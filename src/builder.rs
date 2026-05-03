//! Builder for assembling a `GovernorConfig`.

// Fields on GovernorConfig are consumed by the Service layer (Stage 6).
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use http::Method;
use ipnet::IpNet;

use crate::Quota;
use crate::error::ConfigError;
use crate::extractor::{AsyncKeyExtractor, KeyExtractor};
use crate::headers::quota_window_seconds;
use crate::response::{BodyPreset, ErrorHandler};

/// Marker trait for type-erased sync extractors stored in the stack.
pub(crate) trait ErasedSyncExtractor: Send + Sync + 'static {}

impl<T: KeyExtractor> ErasedSyncExtractor for T {}

/// An `Arc`-wrapped sync extractor with its key type erased, for use in stacked limiters.
pub struct BoxedSyncExtractor(pub(crate) Arc<dyn ErasedSyncExtractor>);

/// One entry in a stacked or multi-window limiter chain.
pub struct StackEntry {
	pub(crate) name: &'static str,
	pub(crate) extractor: BoxedSyncExtractor,
	pub(crate) quota: Quota,
}

/// Holds the primary extractor configured on the builder.
pub(crate) enum ExtractorSlot<K> {
	None,
	Sync(Arc<dyn KeyExtractor<Key = K>>),
	Async(Arc<dyn AsyncKeyExtractor<Key = K>>),
}

/// Validated rate-limit configuration produced by [`GovernorConfigBuilder::finish`].
pub struct GovernorConfig<K> {
	pub(crate) extractor: ExtractorSlot<K>,
	pub(crate) quota_default: Option<Quota>,
	pub(crate) quota_methods: Vec<(Method, Quota)>,
	pub(crate) stack: Vec<StackEntry>,
	pub(crate) whitelist_methods: Vec<Method>,
	pub(crate) whitelist_paths: Vec<String>,
	pub(crate) whitelist_ips: Vec<IpNet>,
	pub(crate) body_preset: BodyPreset,
	pub(crate) error_handler: Option<ErrorHandler>,
	pub(crate) gc_interval: Option<Duration>,
	pub(crate) gc_disabled: bool,
	pub(crate) max_keys: Option<usize>,
	pub(crate) connect_info_required: bool,
}

// ExtractorSlot contains Arc<dyn ...> which isn't Debug; minimal impl avoids K: Debug bound.
impl<K> std::fmt::Debug for GovernorConfig<K> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GovernorConfig").finish_non_exhaustive()
	}
}

/// Type-state builder for [`GovernorConfig`].
///
/// Start with `GovernorConfigBuilder::default()` (`K = ()`), then call
/// [`with_extractor`](Self::with_extractor) or [`with_async_extractor`](Self::with_async_extractor)
/// to fix the key type, then configure quotas and call [`finish`](Self::finish).
pub struct GovernorConfigBuilder<K = ()> {
	extractor: ExtractorSlot<K>,
	requires_connect_info: bool,
	quota_default: Option<Quota>,
	quota_methods: Vec<(Method, Quota)>,
	stack: Vec<StackEntry>,
	empty_chain_names: Vec<&'static str>,
	whitelist_methods: Vec<Method>,
	whitelist_paths: Vec<String>,
	whitelist_ips: Vec<IpNet>,
	body_preset: BodyPreset,
	error_handler: Option<ErrorHandler>,
	gc_interval: Option<Duration>,
	gc_disabled: bool,
	max_keys: Option<usize>,
	connect_info_acknowledged: bool,
}

impl Default for GovernorConfigBuilder<()> {
	fn default() -> Self {
		Self {
			extractor: ExtractorSlot::None,
			requires_connect_info: false,
			quota_default: None,
			quota_methods: Vec::new(),
			stack: Vec::new(),
			empty_chain_names: Vec::new(),
			whitelist_methods: Vec::new(),
			whitelist_paths: Vec::new(),
			whitelist_ips: Vec::new(),
			body_preset: BodyPreset::default(),
			error_handler: None,
			gc_interval: None,
			gc_disabled: false,
			max_keys: None,
			connect_info_acknowledged: false,
		}
	}
}

impl GovernorConfigBuilder<()> {
	/// Set the synchronous key extractor; transitions the builder's key type to `E::Key`.
	///
	/// Only callable on a fresh (`K = ()`) builder — type-state prevents a second call.
	#[must_use]
	pub fn with_extractor<E: KeyExtractor>(self, e: E) -> GovernorConfigBuilder<E::Key> {
		let needs_ci = e.requires_connect_info();
		GovernorConfigBuilder {
			extractor: ExtractorSlot::Sync(Arc::new(e)),
			requires_connect_info: needs_ci,
			quota_default: self.quota_default,
			quota_methods: self.quota_methods,
			stack: self.stack,
			empty_chain_names: self.empty_chain_names,
			whitelist_methods: self.whitelist_methods,
			whitelist_paths: self.whitelist_paths,
			whitelist_ips: self.whitelist_ips,
			body_preset: self.body_preset,
			error_handler: self.error_handler,
			gc_interval: self.gc_interval,
			gc_disabled: self.gc_disabled,
			max_keys: self.max_keys,
			connect_info_acknowledged: self.connect_info_acknowledged,
		}
	}

	/// Set an async key extractor; transitions the builder's key type to `E::Key`.
	///
	/// Only callable on a fresh (`K = ()`) builder.
	#[must_use]
	pub fn with_async_extractor<E: AsyncKeyExtractor>(self, e: E) -> GovernorConfigBuilder<E::Key> {
		GovernorConfigBuilder {
			extractor: ExtractorSlot::Async(Arc::new(e)),
			requires_connect_info: false,
			quota_default: self.quota_default,
			quota_methods: self.quota_methods,
			stack: self.stack,
			empty_chain_names: self.empty_chain_names,
			whitelist_methods: self.whitelist_methods,
			whitelist_paths: self.whitelist_paths,
			whitelist_ips: self.whitelist_ips,
			body_preset: self.body_preset,
			error_handler: self.error_handler,
			gc_interval: self.gc_interval,
			gc_disabled: self.gc_disabled,
			max_keys: self.max_keys,
			connect_info_acknowledged: self.connect_info_acknowledged,
		}
	}
}

impl<K> GovernorConfigBuilder<K> {
	/// Set the default quota applied when no per-method quota matches.
	#[must_use]
	pub fn quota_default(mut self, q: Quota) -> Self {
		self.quota_default = Some(q);
		self
	}

	/// Add a per-HTTP-method quota; overrides the default for that method only.
	#[must_use]
	pub fn quota_for(mut self, method: Method, q: Quota) -> Self {
		self.quota_methods.push((method, q));
		self
	}

	/// Push a named limiter onto the stack with its own extractor and quota.
	///
	/// Entries are checked in insertion order; the first reject wins.
	#[must_use]
	pub fn stack<E: KeyExtractor>(mut self, name: &'static str, extractor: E, quota: Quota) -> Self {
		self.stack.push(StackEntry { name, extractor: BoxedSyncExtractor(Arc::new(extractor)), quota });
		self
	}

	/// Push multiple same-extractor entries onto the stack, one per quota window.
	///
	/// All entries share a single `Arc` to the extractor, avoiding redundant key allocations.
	/// Names are derived from the quota window: 1 s → `"name:1s"`, 60 s → `"name:1m"`,
	/// 3600 s → `"name:1h"`, other → `"name:N"` (0-based index).
	///
	/// Passing an empty iterator records the name for an `EmptyChain` error deferred to `finish`.
	#[must_use]
	pub fn quotas<E: KeyExtractor>(
		mut self,
		name: &'static str,
		extractor: E,
		quotas: impl IntoIterator<Item = Quota>,
	) -> Self {
		let entries: Vec<Quota> = quotas.into_iter().collect();
		if entries.is_empty() {
			self.empty_chain_names.push(name);
			return self;
		}
		let shared: Arc<dyn ErasedSyncExtractor> = Arc::new(extractor);
		for (idx, q) in entries.into_iter().enumerate() {
			let label = quota_label(name, &q, idx);
			self.stack.push(StackEntry {
				name: label,
				extractor: BoxedSyncExtractor(Arc::clone(&shared)),
				quota: q,
			});
		}
		self
	}

	/// Bypass rate limiting for requests using any of the given HTTP methods.
	#[must_use]
	pub fn whitelist_methods(mut self, methods: impl IntoIterator<Item = Method>) -> Self {
		self.whitelist_methods.extend(methods);
		self
	}

	/// Bypass rate limiting for requests whose path matches any of the given glob patterns.
	///
	/// `*` matches one path segment; `**` matches any number of segments.
	#[must_use]
	pub fn whitelist_paths(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
		self.whitelist_paths.extend(paths.into_iter().map(Into::into));
		self
	}

	/// Bypass rate limiting for requests originating from any of the given IP CIDRs.
	#[must_use]
	pub fn whitelist_ips(mut self, ips: impl IntoIterator<Item = IpNet>) -> Self {
		self.whitelist_ips.extend(ips);
		self
	}

	/// Select the response body format used for 429 responses.
	#[must_use]
	pub fn body_preset(mut self, preset: BodyPreset) -> Self {
		self.body_preset = preset;
		self
	}

	/// Override the default 429 response with a custom builder function.
	#[must_use]
	pub fn error_handler(
		mut self,
		f: impl Fn(crate::RejectionReason) -> http::Response<axum::body::Body> + Send + Sync + 'static,
	) -> Self {
		self.error_handler = Some(Arc::new(f));
		self
	}

	/// Override the GC sweep interval (default: 60 s).
	#[must_use]
	pub fn gc_interval(mut self, interval: Duration) -> Self {
		self.gc_interval = Some(interval);
		self
	}

	/// Disable the background GC task entirely.
	#[must_use]
	pub fn gc_disable(mut self) -> Self {
		self.gc_disabled = true;
		self
	}

	/// Cap the number of tracked keys; entries exceeding the limit evict the oldest key.
	#[must_use]
	pub fn max_keys(mut self, n: usize) -> Self {
		self.max_keys = Some(n);
		self
	}

	/// Acknowledge that the router is built with `into_make_service_with_connect_info`.
	///
	/// Required when using `PeerIp` or `SmartIp`; omitting it causes `finish()` to return
	/// `ConfigError::MissingConnectInfoAcknowledgement`.
	#[must_use]
	pub fn expect_connect_info(mut self) -> Self {
		self.connect_info_acknowledged = true;
		self
	}

	/// Validate the configuration and produce a [`GovernorConfig`].
	///
	/// Checks are applied in order:
	/// `NoExtractor` → `ZeroBurst` → `EmptyChain` → `ContradictoryWhitelist` →
	/// `MissingConnectInfoAcknowledgement`.
	pub fn finish(self) -> Result<GovernorConfig<K>, ConfigError> {
		// 1. NoExtractor
		if matches!(self.extractor, ExtractorSlot::None) {
			return Err(ConfigError::NoExtractor);
		}

		// 2. ZeroBurst — governor enforces NonZeroU32 on burst, so this is a defensive guard.
		let all_q = self
			.quota_default
			.iter()
			.chain(self.quota_methods.iter().map(|(_, q)| q))
			.chain(self.stack.iter().map(|e| &e.quota));
		for q in all_q {
			if q.inner().burst_size().get() == 0 {
				return Err(ConfigError::ZeroBurst);
			}
		}

		// 3. EmptyChain
		if !self.empty_chain_names.is_empty() {
			return Err(ConfigError::EmptyChain);
		}

		// 4. ContradictoryWhitelist — universal IP coverage makes the limiter a no-op.
		let any_v4_any = self.whitelist_ips.iter().any(|n| n.to_string() == "0.0.0.0/0");
		let any_v6_any = self.whitelist_ips.iter().any(|n| n.to_string() == "::/0");
		if any_v4_any && any_v6_any {
			return Err(ConfigError::ContradictoryWhitelist);
		}

		// 5. MissingConnectInfoAcknowledgement
		if self.requires_connect_info && !self.connect_info_acknowledged {
			return Err(ConfigError::MissingConnectInfoAcknowledgement);
		}

		Ok(GovernorConfig {
			extractor: self.extractor,
			quota_default: self.quota_default,
			quota_methods: self.quota_methods,
			stack: self.stack,
			whitelist_methods: self.whitelist_methods,
			whitelist_paths: self.whitelist_paths,
			whitelist_ips: self.whitelist_ips,
			body_preset: self.body_preset,
			error_handler: self.error_handler,
			gc_interval: self.gc_interval,
			gc_disabled: self.gc_disabled,
			max_keys: self.max_keys,
			connect_info_required: self.requires_connect_info,
		})
	}
}

fn quota_label(name: &'static str, q: &Quota, idx: usize) -> &'static str {
	let suffix = match quota_window_seconds(q) {
		1 => "1s".to_owned(),
		60 => "1m".to_owned(),
		3600 => "1h".to_owned(),
		_ => idx.to_string(),
	};
	Box::leak(format!("{name}:{suffix}").into_boxed_str())
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use http::Method;
	use ipnet::IpNet;

	use super::*;
	use crate::extractor::{Global, PeerIp, SmartIp};
	use crate::{Quota, nz};

	fn q1s() -> Quota {
		Quota::requests_per_second(nz!(10u32))
	}

	fn q1m() -> Quota {
		Quota::requests_per_minute(nz!(600u32))
	}

	fn q1h() -> Quota {
		Quota::requests_per_hour(nz!(20_000u32))
	}

	fn net(s: &str) -> IpNet {
		s.parse().unwrap()
	}

	// --- finish() error variants ---

	#[test]
	fn finish_no_extractor_returns_error() {
		let err = GovernorConfigBuilder::default().finish().unwrap_err();
		assert!(matches!(err, crate::ConfigError::NoExtractor));
	}

	#[test]
	fn finish_empty_chain_returns_error() {
		let err = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quotas("x", Global, [])
			.finish()
			.unwrap_err();
		assert!(matches!(err, crate::ConfigError::EmptyChain));
	}

	#[test]
	fn finish_contradictory_whitelist_v4_and_v6_any() {
		let err = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.whitelist_ips([net("0.0.0.0/0"), net("::/0")])
			.finish()
			.unwrap_err();
		assert!(matches!(err, crate::ConfigError::ContradictoryWhitelist));
	}

	#[test]
	fn finish_only_v4_any_whitelist_ok() {
		let result = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.whitelist_ips([net("0.0.0.0/0")])
			.finish();
		assert!(result.is_ok());
	}

	#[test]
	fn finish_peer_ip_without_ack_returns_error() {
		let err = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.quota_default(q1s())
			.finish()
			.unwrap_err();
		assert!(matches!(err, crate::ConfigError::MissingConnectInfoAcknowledgement));
	}

	#[test]
	fn finish_smart_ip_without_ack_returns_error() {
		let err = GovernorConfigBuilder::default()
			.with_extractor(SmartIp::new())
			.quota_default(q1s())
			.finish()
			.unwrap_err();
		assert!(matches!(err, crate::ConfigError::MissingConnectInfoAcknowledgement));
	}

	// --- finish() success paths ---

	#[test]
	fn finish_global_with_quota_ok() {
		let result =
			GovernorConfigBuilder::default().with_extractor(Global).quota_default(q1s()).finish();
		assert!(result.is_ok());
	}

	#[test]
	fn finish_peer_ip_with_ack_ok() {
		let result = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			.quota_default(q1s())
			.finish();
		assert!(result.is_ok());
	}

	#[test]
	fn finish_global_no_quota_ok() {
		let result = GovernorConfigBuilder::default().with_extractor(Global).finish();
		assert!(result.is_ok());
	}

	// --- field storage ---

	#[test]
	fn quota_for_method_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_for(Method::GET, q1s())
			.quota_for(Method::POST, q1m())
			.finish()
			.unwrap();
		assert_eq!(cfg.quota_methods.len(), 2);
		assert_eq!(cfg.quota_methods[0].0, Method::GET);
		assert_eq!(cfg.quota_methods[1].0, Method::POST);
	}

	#[test]
	fn stack_single_entry_added() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.stack("peer", PeerIp::default(), q1s())
			.finish()
			.unwrap();
		assert_eq!(cfg.stack.len(), 1);
		assert_eq!(cfg.stack[0].name, "peer");
	}

	#[test]
	fn quotas_expands_to_three_stack_entries_with_labels() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quotas("peer", PeerIp::default(), [q1s(), q1m(), q1h()])
			.finish()
			.unwrap();
		assert_eq!(cfg.stack.len(), 3);
		assert_eq!(cfg.stack[0].name, "peer:1s");
		assert_eq!(cfg.stack[1].name, "peer:1m");
		assert_eq!(cfg.stack[2].name, "peer:1h");
	}

	#[test]
	fn quotas_unknown_window_uses_index() {
		// seconds_per_request(5) → burst=1, replenish=5s, window=5s — not 1/60/3600
		let odd = Quota::seconds_per_request(nz!(5u32));
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quotas("x", Global, [odd])
			.finish()
			.unwrap();
		assert_eq!(cfg.stack[0].name, "x:0");
	}

	#[test]
	fn whitelist_methods_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.whitelist_methods([Method::OPTIONS, Method::HEAD])
			.finish()
			.unwrap();
		assert_eq!(cfg.whitelist_methods, [Method::OPTIONS, Method::HEAD]);
	}

	#[test]
	fn whitelist_paths_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.whitelist_paths(["/health", "/metrics"])
			.finish()
			.unwrap();
		assert_eq!(cfg.whitelist_paths, ["/health", "/metrics"]);
	}

	#[test]
	fn whitelist_ips_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.whitelist_ips([net("127.0.0.0/8")])
			.finish()
			.unwrap();
		assert_eq!(cfg.whitelist_ips, [net("127.0.0.0/8")]);
	}

	#[test]
	fn gc_interval_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.gc_interval(Duration::from_secs(30))
			.finish()
			.unwrap();
		assert_eq!(cfg.gc_interval, Some(Duration::from_secs(30)));
	}

	#[test]
	fn gc_disable_stored() {
		let cfg =
			GovernorConfigBuilder::default().with_extractor(Global).gc_disable().finish().unwrap();
		assert!(cfg.gc_disabled);
	}

	#[test]
	fn max_keys_stored() {
		let cfg =
			GovernorConfigBuilder::default().with_extractor(Global).max_keys(50_000).finish().unwrap();
		assert_eq!(cfg.max_keys, Some(50_000));
	}

	#[test]
	fn body_preset_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.body_preset(BodyPreset::Text)
			.finish()
			.unwrap();
		assert_eq!(cfg.body_preset, BodyPreset::Text);
	}

	#[test]
	fn error_handler_replaces_default() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.error_handler(|_| http::Response::new(axum::body::Body::empty()))
			.finish()
			.unwrap();
		assert!(cfg.error_handler.is_some());
	}

	#[test]
	fn connect_info_required_propagated_to_config() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			.quota_default(q1s())
			.finish()
			.unwrap();
		assert!(cfg.connect_info_required);
	}

	#[test]
	fn global_connect_info_not_required() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(q1s())
			.finish()
			.unwrap();
		assert!(!cfg.connect_info_required);
	}

	// --- KeyExtractor::requires_connect_info ---

	#[test]
	fn peer_ip_requires_connect_info() {
		assert!(PeerIp::default().requires_connect_info());
	}

	#[test]
	fn smart_ip_requires_connect_info() {
		assert!(SmartIp::new().requires_connect_info());
	}

	#[test]
	fn global_does_not_require_connect_info() {
		assert!(!Global.requires_connect_info());
	}
}
