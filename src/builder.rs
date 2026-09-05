//! Builder for assembling a `GovernorConfig`.

use std::sync::Arc;
use std::time::Duration;

use http::Method;
use ipnet::IpNet;

use crate::Quota;
use crate::error::ConfigError;
use crate::extractor::{AsyncKeyExtractor, KeyExtractor};
use crate::headers::quota_window_seconds;
use crate::limiters::{StackEntryFactory, TypedStackFactory};
use crate::response::{BodyPreset, ErrorHandler};

/// Holds the primary extractor configured on the builder.
pub(crate) enum ExtractorSlot<K> {
	None,
	Sync(Arc<dyn KeyExtractor<Key = K>>),
	Async(Arc<dyn AsyncKeyExtractor<Key = K>>),
}

/// Everything in a config that does not depend on the key type. Kept apart from the
/// generic fields so builder type-state transitions, `GovernorLayer::new` and the boxed
/// adapter can move the whole block in one expression instead of naming every field.
pub(crate) struct Settings {
	pub quota_default: Option<Quota>,
	pub quota_methods: Vec<(Method, Quota)>,
	pub whitelist_methods: Vec<Method>,
	pub whitelist_paths: Vec<String>,
	pub whitelist_ips: Vec<IpNet>,
	pub body_preset: BodyPreset,
	pub error_handler: Option<ErrorHandler>,
	pub gc_interval: Option<Duration>,
	pub gc_disabled: bool,
	pub max_keys: Option<usize>,
	pub legacy_reset_epoch: bool,
	pub redact_keys: bool,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			quota_default: None,
			quota_methods: Vec::new(),
			whitelist_methods: Vec::new(),
			whitelist_paths: Vec::new(),
			whitelist_ips: Vec::new(),
			body_preset: BodyPreset::default(),
			error_handler: None,
			gc_interval: Some(Duration::from_secs(60)),
			gc_disabled: false,
			max_keys: None,
			legacy_reset_epoch: false,
			redact_keys: false,
		}
	}
}

/// Validated rate-limit configuration produced by [`GovernorConfigBuilder::finish`].
pub struct GovernorConfig<K> {
	pub(crate) extractor: ExtractorSlot<K>,
	pub(crate) stack: Vec<Box<dyn StackEntryFactory>>,
	pub(crate) settings: Settings,
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
	connect_info_acknowledged: bool,
	stack: Vec<Box<dyn StackEntryFactory>>,
	empty_chain_names: Vec<&'static str>,
	settings: Settings,
}

impl Default for GovernorConfigBuilder<()> {
	fn default() -> Self {
		Self {
			extractor: ExtractorSlot::None,
			requires_connect_info: false,
			connect_info_acknowledged: false,
			stack: Vec::new(),
			empty_chain_names: Vec::new(),
			settings: Settings::default(),
		}
	}
}

impl GovernorConfigBuilder<()> {
	/// Set the synchronous key extractor; transitions the builder's key type to `E::Key`.
	///
	/// Only callable on a fresh (`K = ()`) builder — type-state prevents a second call.
	#[must_use]
	pub fn with_extractor<E: KeyExtractor>(self, e: E) -> GovernorConfigBuilder<E::Key> {
		let requires_connect_info = e.requires_connect_info();
		self.with_slot(ExtractorSlot::Sync(Arc::new(e)), requires_connect_info)
	}

	/// Set an async key extractor; transitions the builder's key type to `E::Key`.
	///
	/// Only callable on a fresh (`K = ()`) builder.
	#[must_use]
	pub fn with_async_extractor<E: AsyncKeyExtractor>(self, e: E) -> GovernorConfigBuilder<E::Key> {
		self.with_slot(ExtractorSlot::Async(Arc::new(e)), false)
	}

	fn with_slot<K>(
		self,
		extractor: ExtractorSlot<K>,
		requires_connect_info: bool,
	) -> GovernorConfigBuilder<K> {
		GovernorConfigBuilder {
			extractor,
			requires_connect_info,
			connect_info_acknowledged: self.connect_info_acknowledged,
			stack: self.stack,
			empty_chain_names: self.empty_chain_names,
			settings: self.settings,
		}
	}
}

impl<K> GovernorConfigBuilder<K> {
	/// Set the default quota applied when no per-method quota matches.
	#[must_use]
	pub fn quota_default(mut self, q: Quota) -> Self {
		self.settings.quota_default = Some(q);
		self
	}

	/// Add a per-HTTP-method quota; overrides the default for that method only.
	#[must_use]
	pub fn quota_for(mut self, method: Method, q: Quota) -> Self {
		self.settings.quota_methods.push((method, q));
		self
	}

	/// Push a named limiter onto the stack with its own extractor and quota.
	///
	/// Entries are checked in insertion order; the first reject wins.
	#[must_use]
	pub fn stack<E: KeyExtractor>(mut self, name: &'static str, extractor: E, quota: Quota) -> Self {
		self.stack.push(Box::new(TypedStackFactory {
			name: Arc::from(name),
			quota,
			extractor: Arc::new(extractor),
		}));
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
		let shared = Arc::new(extractor);
		for (idx, q) in entries.into_iter().enumerate() {
			let label = quota_label(name, &q, idx);
			self.stack.push(Box::new(TypedStackFactory {
				name: label,
				quota: q,
				extractor: Arc::clone(&shared),
			}));
		}
		self
	}

	/// Bypass rate limiting for requests using any of the given HTTP methods.
	#[must_use]
	pub fn whitelist_methods(mut self, methods: impl IntoIterator<Item = Method>) -> Self {
		self.settings.whitelist_methods.extend(methods);
		self
	}

	/// Bypass rate limiting for requests whose path matches any of the given glob patterns.
	///
	/// `*` matches one path segment; `**` matches any number of segments.
	#[must_use]
	pub fn whitelist_paths(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
		self.settings.whitelist_paths.extend(paths.into_iter().map(Into::into));
		self
	}

	/// Bypass rate limiting for requests originating from any of the given IP CIDRs.
	#[must_use]
	pub fn whitelist_ips(mut self, ips: impl IntoIterator<Item = IpNet>) -> Self {
		self.settings.whitelist_ips.extend(ips);
		self
	}

	/// Select the response body format used for 429 responses.
	#[must_use]
	pub fn body_preset(mut self, preset: BodyPreset) -> Self {
		self.settings.body_preset = preset;
		self
	}

	/// Override the default 429 response with a custom builder function.
	#[must_use]
	pub fn error_handler(
		mut self,
		f: impl Fn(crate::RejectionReason) -> http::Response<axum::body::Body> + Send + Sync + 'static,
	) -> Self {
		self.settings.error_handler = Some(Arc::new(f));
		self
	}

	/// Switch `X-RateLimit-Reset` from delta-seconds (default) to Unix epoch seconds.
	///
	/// Most APIs use delta; flip this if you need to match GitHub-style epoch wire format.
	#[must_use]
	pub fn legacy_reset_epoch(mut self, on: bool) -> Self {
		self.settings.legacy_reset_epoch = on;
		self
	}

	/// Override the GC sweep interval (default: 60 s).
	#[must_use]
	pub fn gc_interval(mut self, interval: Duration) -> Self {
		self.settings.gc_interval = Some(interval);
		self
	}

	/// Disable the background GC task entirely.
	#[must_use]
	pub fn gc_disable(mut self) -> Self {
		self.settings.gc_disabled = true;
		self
	}

	/// Cap the sidecar tracker used for `snapshot().top_n` and best-effort key shedding.
	///
	/// governor 0.10 does not expose per-key removal, so exceeding this value evicts the
	/// oldest key from axum-governor's tracker and forces a `retain_recent()` pass on the
	/// underlying limiter. Fresh governor entries may remain until they become stale.
	#[must_use]
	pub fn max_keys(mut self, n: usize) -> Self {
		self.settings.max_keys = Some(n);
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

	/// When `true`, keys are hashed with SipHash and rendered as `hash:<hex>` in tracing
	/// events instead of their `Debug` representation. Use this to avoid logging sensitive
	/// values such as API keys or session tokens.
	#[must_use]
	pub fn redact_keys(mut self, on: bool) -> Self {
		self.settings.redact_keys = on;
		self
	}

	/// Validate the configuration and produce a [`GovernorConfig`].
	///
	/// Checks are applied in order:
	/// `NoExtractor` → `EmptyChain` → `ContradictoryWhitelist` →
	/// `MissingConnectInfoAcknowledgement`.
	pub fn finish(self) -> Result<GovernorConfig<K>, ConfigError> {
		if matches!(self.extractor, ExtractorSlot::None) {
			return Err(ConfigError::NoExtractor);
		}

		if !self.empty_chain_names.is_empty() {
			return Err(ConfigError::EmptyChain);
		}

		// Whitelisting every v4 and every v6 address makes the limiter a no-op.
		let any_v4 = self.settings.whitelist_ips.iter().any(|n| matches!(n, IpNet::V4(v) if v.prefix_len() == 0));
		let any_v6 = self.settings.whitelist_ips.iter().any(|n| matches!(n, IpNet::V6(v) if v.prefix_len() == 0));
		if any_v4 && any_v6 {
			return Err(ConfigError::ContradictoryWhitelist);
		}

		if self.requires_connect_info && !self.connect_info_acknowledged {
			return Err(ConfigError::MissingConnectInfoAcknowledgement);
		}

		Ok(GovernorConfig { extractor: self.extractor, stack: self.stack, settings: self.settings })
	}
}

fn quota_label(name: &'static str, q: &Quota, idx: usize) -> Arc<str> {
	// `Arc<str>` lets the layer outlive each entry's label without leaking memory:
	// when the layer is dropped, every Arc is freed. Cost is one allocation per
	// label at config time and one atomic-ref-count clone per request that needs
	// the name (e.g. when constructing a `RejectionReason`).
	let suffix: &str = match quota_window_seconds(q) {
		1 => "1s",
		60 => "1m",
		3600 => "1h",
		_ => return Arc::from(format!("{name}:{idx}")),
	};
	Arc::from(format!("{name}:{suffix}"))
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
		assert_eq!(cfg.settings.quota_methods.len(), 2);
		assert_eq!(cfg.settings.quota_methods[0].0, Method::GET);
		assert_eq!(cfg.settings.quota_methods[1].0, Method::POST);
	}

	#[test]
	fn stack_single_entry_added() {
		use crate::layer::GovernorLayer;
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.stack("peer", PeerIp::default(), q1s())
			.finish()
			.unwrap();
		assert_eq!(cfg.stack.len(), 1);
		// Build the layer to get the finalized runners and verify name propagated.
		// The primary extractor is Global (K = ()), stack entries are type-erased.
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert_eq!(layer.shared.stack[0].name(), "peer");
	}

	#[test]
	fn quotas_expands_to_three_stack_entries_with_labels() {
		use crate::layer::GovernorLayer;
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quotas("peer", PeerIp::default(), [q1s(), q1m(), q1h()])
			.finish()
			.unwrap();
		assert_eq!(cfg.stack.len(), 3);
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert_eq!(layer.shared.stack[0].name(), "peer:1s");
		assert_eq!(layer.shared.stack[1].name(), "peer:1m");
		assert_eq!(layer.shared.stack[2].name(), "peer:1h");
	}

	#[test]
	fn quotas_unknown_window_uses_index() {
		use crate::layer::GovernorLayer;
		// seconds_per_request(5) → burst=1, replenish=5s, window=5s — not 1/60/3600
		let odd = Quota::seconds_per_request(nz!(5u32));
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quotas("x", Global, [odd])
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert_eq!(layer.shared.stack[0].name(), "x:0");
	}

	#[test]
	fn whitelist_methods_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.whitelist_methods([Method::OPTIONS, Method::HEAD])
			.finish()
			.unwrap();
		assert_eq!(cfg.settings.whitelist_methods, [Method::OPTIONS, Method::HEAD]);
	}

	#[test]
	fn whitelist_paths_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.whitelist_paths(["/health", "/metrics"])
			.finish()
			.unwrap();
		assert_eq!(cfg.settings.whitelist_paths, ["/health", "/metrics"]);
	}

	#[test]
	fn whitelist_ips_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.whitelist_ips([net("127.0.0.0/8")])
			.finish()
			.unwrap();
		assert_eq!(cfg.settings.whitelist_ips, [net("127.0.0.0/8")]);
	}

	#[test]
	fn gc_interval_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.gc_interval(Duration::from_secs(30))
			.finish()
			.unwrap();
		assert_eq!(cfg.settings.gc_interval, Some(Duration::from_secs(30)));
	}

	#[test]
	fn gc_disable_stored() {
		let cfg =
			GovernorConfigBuilder::default().with_extractor(Global).gc_disable().finish().unwrap();
		assert!(cfg.settings.gc_disabled);
	}

	#[test]
	fn max_keys_stored() {
		let cfg =
			GovernorConfigBuilder::default().with_extractor(Global).max_keys(50_000).finish().unwrap();
		assert_eq!(cfg.settings.max_keys, Some(50_000));
	}

	#[test]
	fn body_preset_stored() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.body_preset(BodyPreset::Text)
			.finish()
			.unwrap();
		assert_eq!(cfg.settings.body_preset, BodyPreset::Text);
	}

	#[test]
	fn error_handler_replaces_default() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.error_handler(|_| http::Response::new(axum::body::Body::empty()))
			.finish()
			.unwrap();
		assert!(cfg.settings.error_handler.is_some());
	}

	#[test]
	fn gc_interval_defaults_to_60s() {
		let cfg = GovernorConfigBuilder::default().with_extractor(Global).finish().unwrap();
		assert_eq!(cfg.settings.gc_interval, Some(Duration::from_secs(60)));
		assert!(!cfg.settings.gc_disabled);
	}

	#[test]
	fn legacy_reset_epoch_defaults_false_and_round_trips() {
		let cfg = GovernorConfigBuilder::default().with_extractor(Global).finish().unwrap();
		assert!(!cfg.settings.legacy_reset_epoch);

		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.legacy_reset_epoch(true)
			.finish()
			.unwrap();
		assert!(cfg.settings.legacy_reset_epoch);
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

	#[test]
	fn redact_keys_default_false_and_round_trips() {
		let cfg = GovernorConfigBuilder::default().with_extractor(Global).finish().unwrap();
		assert!(!cfg.settings.redact_keys);
		let cfg =
			GovernorConfigBuilder::default().with_extractor(Global).redact_keys(true).finish().unwrap();
		assert!(cfg.settings.redact_keys);
	}
}
