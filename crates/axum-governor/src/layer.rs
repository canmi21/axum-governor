//! `tower::Layer` constructor for the rate-limit middleware.

use std::hash::Hash;
use std::sync::Arc;

use governor::clock::DefaultClock;
use governor::middleware::StateInformationMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;

use http::HeaderValue;
use http::Method;

use crate::Quota;
use crate::builder::{ExtractorSlot, GovernorConfig, Settings};
use crate::headers::{PolicyDescriptor, render_policy_value};
use crate::limiters::{LimiterCache, StackedRunner};
use crate::tracker::KeyTracker;

pub(crate) type KeyedRateLimiter<K> =
	governor::RateLimiter<K, DefaultKeyedStateStore<K>, DefaultClock, StateInformationMiddleware>;

/// Pre-rendered `RateLimit-Policy` header value plus its descriptor list, kept
/// alongside the limiter that produced it. The descriptor list is `Arc<[…]>` so
/// the hot path can lend it as a slice without copying.
pub(crate) struct PolicyEntry {
	pub header: HeaderValue,
	pub descriptors: Arc<[PolicyDescriptorOwned]>,
}

/// Owned twin of `headers::PolicyDescriptor` so we can store it in `LimiterShared`
/// without lifetimes. The hot path borrows from this when it needs to hand a
/// `&[PolicyDescriptor<'_>]` to a header writer.
#[derive(Clone)]
pub(crate) struct PolicyDescriptorOwned {
	pub name: Arc<str>,
	pub quota: Quota,
}

pub(crate) struct LimiterShared<K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	pub extractor: ExtractorSlot<K>,
	pub settings: Settings,
	pub default_limiter: Option<KeyedRateLimiter<K>>,
	pub default_tracker: Option<KeyTracker<K>>,
	/// Per-method limiters built from `config.quota_methods` at layer construction.
	pub method_limiters: Vec<(Method, KeyedRateLimiter<K>)>,
	pub method_trackers: Vec<(Method, KeyTracker<K>)>,
	/// Ordered stack of type-erased limiters built from `config.stack` factories.
	pub stack: Vec<Box<dyn StackedRunner>>,
	/// Cache mapping quota overrides to their limiters, used for per-tier dispatch.
	pub tier_cache: LimiterCache<K>,
	/// Handle to the background GC task; `None` when GC is disabled.
	pub gc_handle: Option<tokio::task::AbortHandle>,
	/// `Arc<str>` reused for the `"default"` policy label so per-request rejections
	/// can share one heap allocation instead of `Arc::from("default")`-ing each call.
	pub default_label: Arc<str>,
	/// Pre-computed policy header for the no-method-override case (default + stack).
	/// Empty when the layer has neither a default quota nor any stack entries.
	pub policy_default: Option<PolicyEntry>,
	/// Pre-computed policy header per HTTP method override. Tier overrides are still
	/// rendered inline because their quota is request-time data.
	pub policy_per_method: Vec<(Method, PolicyEntry)>,
}

impl<K> LimiterShared<K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	pub(crate) fn retain_all(&self) {
		if let Some(l) = &self.default_limiter {
			l.retain_recent();
		}
		for (_, l) in &self.method_limiters {
			l.retain_recent();
		}
		for entry in &self.stack {
			entry.retain_recent();
		}
		self.tier_cache.retain_all();
	}
}

impl<K> Drop for LimiterShared<K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	fn drop(&mut self) {
		if let Some(handle) = self.gc_handle.take() {
			handle.abort();
		}
	}
}

/// Tower `Layer` that wraps services with governor-backed rate limiting.
#[derive(Clone)]
pub struct GovernorLayer<K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	pub(crate) shared: Arc<LimiterShared<K>>,
}

impl<K> GovernorLayer<K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	/// Return a handle for live introspection of the running limiter state.
	pub fn limiter(&self) -> crate::snapshot::LimiterHandle<K> {
		crate::snapshot::LimiterHandle { shared: Arc::clone(&self.shared) }
	}

	/// Build the limiters described by `config` and start the background GC task.
	///
	/// Every `Service` produced by this layer shares one set of limiters, so construct
	/// the layer once per process and clone it; a layer per request is a limiter per
	/// request, which limits nothing. The GC task needs a Tokio runtime to be current;
	/// without one it is skipped, which only ever happens in synchronous tests.
	pub fn new(config: GovernorConfig<K>) -> Self {
		let GovernorConfig { extractor, stack, settings } = config;
		let max_keys = settings.max_keys;

		let default_limiter = settings.quota_default.map(|q: Quota| {
			governor::RateLimiter::keyed(q.inner()).with_middleware::<StateInformationMiddleware>()
		});
		let default_tracker = default_limiter.as_ref().map(|_| KeyTracker::new(max_keys));

		let method_limiters: Vec<(Method, KeyedRateLimiter<K>)> = settings
			.quota_methods
			.iter()
			.map(|(method, q)| {
				let limiter =
					governor::RateLimiter::keyed(q.inner()).with_middleware::<StateInformationMiddleware>();
				(method.clone(), limiter)
			})
			.collect();
		let method_trackers: Vec<(Method, KeyTracker<K>)> = settings
			.quota_methods
			.iter()
			.map(|(method, _)| (method.clone(), KeyTracker::new(max_keys)))
			.collect();

		// Each stack entry has its own state store, so each gets its own tracker under the
		// same max_keys cap.
		let stack: Vec<Box<dyn StackedRunner>> = stack.into_iter().map(|f| f.build(max_keys)).collect();

		let default_label: Arc<str> = Arc::from("default");

		// Pre-compute policy headers — these fold the static portion of every admit /
		// reject response so the hot path emits header bytes without rebuilding the
		// `RateLimit-Policy` value per request.
		let stack_descriptors: Vec<PolicyDescriptorOwned> = stack
			.iter()
			.map(|entry| PolicyDescriptorOwned { name: entry.name_arc(), quota: entry.quota() })
			.collect();

		let policy_default = build_policy_entry(
			settings
				.quota_default
				.map(|q| PolicyDescriptorOwned { name: Arc::clone(&default_label), quota: q }),
			&stack_descriptors,
		);

		let policy_per_method: Vec<(Method, PolicyEntry)> = settings
			.quota_methods
			.iter()
			.filter_map(|(method, q)| {
				build_policy_entry(
					Some(PolicyDescriptorOwned { name: Arc::clone(&default_label), quota: *q }),
					&stack_descriptors,
				)
				.map(|entry| (method.clone(), entry))
			})
			.collect();

		let shared = Arc::new_cyclic(|weak: &std::sync::Weak<LimiterShared<K>>| {
			let gc_handle = match (settings.gc_interval, settings.gc_disabled) {
				(Some(every), false) => crate::gc::spawn_gc_inner(weak.clone(), every),
				_ => None,
			};
			LimiterShared {
				extractor,
				settings,
				default_limiter,
				default_tracker,
				method_limiters,
				method_trackers,
				stack,
				tier_cache: LimiterCache::<K>::new(),
				gc_handle,
				default_label,
				policy_default,
				policy_per_method,
			}
		});

		Self { shared }
	}
}

fn build_policy_entry(
	primary: Option<PolicyDescriptorOwned>,
	stack: &[PolicyDescriptorOwned],
) -> Option<PolicyEntry> {
	let mut owned: Vec<PolicyDescriptorOwned> = Vec::with_capacity(1 + stack.len());
	if let Some(p) = primary {
		owned.push(p);
	}
	owned.extend_from_slice(stack);
	if owned.is_empty() {
		return None;
	}
	let view: Vec<PolicyDescriptor<'_>> =
		owned.iter().map(|d| PolicyDescriptor { name: &d.name, quota: d.quota }).collect();
	let header = render_policy_value(&view)?;
	Some(PolicyEntry { header, descriptors: owned.into() })
}

impl<S, K> tower::Layer<S> for GovernorLayer<K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	type Service = crate::service::Governor<S, K>;

	fn layer(&self, inner: S) -> Self::Service {
		crate::service::Governor { inner, shared: Arc::clone(&self.shared) }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::builder::GovernorConfigBuilder;
	use crate::extractor::{AsyncExtractFuture, AsyncKeyExtractor, Global, KeyOutcome};
	use crate::{Quota, nz};
	use http::request::Parts;

	#[test]
	fn sync_extractor_constructs_ok() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let _layer: GovernorLayer<()> = GovernorLayer::new(cfg);
	}

	#[test]
	fn async_extractor_constructs_ok() {
		#[derive(Debug)]
		struct DummyAsync;
		impl AsyncKeyExtractor for DummyAsync {
			type Key = ();
			fn extract<'a>(&'a self, _parts: &'a Parts) -> AsyncExtractFuture<'a, ()> {
				Box::pin(async { Ok(KeyOutcome { key: (), quota_override: None }) })
			}
		}

		let cfg = GovernorConfigBuilder::default()
			.with_async_extractor(DummyAsync)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let _: GovernorLayer<()> = GovernorLayer::new(cfg);
	}

	#[test]
	fn default_limiter_is_none_without_quota() {
		let cfg = GovernorConfigBuilder::default().with_extractor(Global).finish().unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert!(layer.shared.default_limiter.is_none());
		assert!(layer.shared.policy_default.is_none());
	}

	#[test]
	fn method_limiters_built_from_config() {
		use http::Method;
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_for(Method::GET, Quota::requests_per_second(nz!(10u32)))
			.quota_for(Method::POST, Quota::requests_per_second(nz!(5u32)))
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert_eq!(layer.shared.method_limiters.len(), 2);
		assert_eq!(layer.shared.method_trackers.len(), 2);
		assert_eq!(layer.shared.policy_per_method.len(), 2);
	}

	#[test]
	fn stack_entries_built_from_factories() {
		use crate::extractor::PeerIp;
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.stack("peer", PeerIp::default(), Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert_eq!(layer.shared.stack.len(), 1);
		assert_eq!(layer.shared.stack[0].name(), "peer");
	}

	#[test]
	fn policy_default_is_precomputed_when_default_quota_present() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		let entry = layer.shared.policy_default.as_ref().expect("policy entry");
		assert_eq!(entry.header.to_str().unwrap(), "\"default\";q=10;w=1");
	}
}
