//! `tower::Layer` constructor for the rate-limit middleware.

use std::hash::Hash;
use std::sync::Arc;

use governor::clock::DefaultClock;
use governor::middleware::StateInformationMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;

use http::Method;

use crate::Quota;
use crate::builder::GovernorConfig;
use crate::limiters::{LimiterCache, StackedRunner};

pub(crate) type KeyedRateLimiter<K> =
	governor::RateLimiter<K, DefaultKeyedStateStore<K>, DefaultClock, StateInformationMiddleware>;

pub(crate) struct LimiterShared<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	pub config: GovernorConfig<K>,
	pub default_limiter: Option<KeyedRateLimiter<K>>,
	/// Per-method limiters built from `config.quota_methods` at layer construction.
	pub method_limiters: Vec<(Method, KeyedRateLimiter<K>)>,
	/// Ordered stack of type-erased limiters built from `config.stack` factories.
	pub stack: Vec<Box<dyn StackedRunner>>,
	/// Cache mapping quota overrides to their limiters, used for per-tier dispatch.
	pub tier_cache: LimiterCache<K>,
	/// Handle to the background GC task; `None` when GC is disabled.
	pub gc_handle: Option<tokio::task::AbortHandle>,
}

impl<K> LimiterShared<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
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
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	fn drop(&mut self) {
		if let Some(handle) = self.gc_handle.take() {
			handle.abort();
		}
	}
}

/// Tower `Layer` that wraps services with governor-backed rate limiting.
pub struct GovernorLayer<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	pub(crate) shared: Arc<LimiterShared<K>>,
}

impl<K> GovernorLayer<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	pub fn new(config: GovernorConfig<K>) -> Self {
		let default_limiter = config.quota_default.map(|q: Quota| {
			governor::RateLimiter::keyed(q.inner()).with_middleware::<StateInformationMiddleware>()
		});

		let method_limiters: Vec<(Method, KeyedRateLimiter<K>)> = config
			.quota_methods
			.iter()
			.map(|(method, q)| {
				let limiter =
					governor::RateLimiter::keyed(q.inner()).with_middleware::<StateInformationMiddleware>();
				(method.clone(), limiter)
			})
			.collect();

		// Consume the stack factories, building each StackedRunner.  The factories are
		// consumed here so the config field is left empty; the built runners live in `stack`.
		// We rebuild quota_methods from config, so we need to drain stack only.
		// Because GovernorConfig.stack is Vec<Box<dyn StackEntryFactory>>, we need to
		// temporarily take ownership.  The config is moved in, so we can destructure it.

		// Build stack before moving config into LimiterShared.
		let stack: Vec<Box<dyn StackedRunner>> = config.stack.into_iter().map(|f| f.build()).collect();

		// Rebuild config without the consumed stack field.
		let config_rebuilt = GovernorConfig {
			extractor: config.extractor,
			quota_default: config.quota_default,
			quota_methods: config.quota_methods,
			stack: Vec::new(), // consumed above
			whitelist_methods: config.whitelist_methods,
			whitelist_paths: config.whitelist_paths,
			whitelist_ips: config.whitelist_ips,
			body_preset: config.body_preset,
			error_handler: config.error_handler,
			gc_interval: config.gc_interval,
			gc_disabled: config.gc_disabled,
			// stored but not yet enforced: governor 0.10 does not expose state-store injection
			max_keys: config.max_keys,
			connect_info_required: config.connect_info_required,
			legacy_reset_epoch: config.legacy_reset_epoch,
		};

		let gc_interval = config_rebuilt.gc_interval;
		let gc_disabled = config_rebuilt.gc_disabled;

		let shared = Arc::new_cyclic(|weak: &std::sync::Weak<LimiterShared<K>>| {
			let gc_handle = match (gc_interval, gc_disabled) {
				(Some(every), false) => crate::gc::spawn_gc_inner(weak.clone(), every),
				_ => None,
			};
			LimiterShared {
				config: config_rebuilt,
				default_limiter,
				method_limiters,
				stack,
				tier_cache: LimiterCache::<K>::new(),
				gc_handle,
			}
		});

		Self { shared }
	}
}

impl<S, K> tower::Layer<S> for GovernorLayer<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
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
}
