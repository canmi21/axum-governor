//! Live introspection for the layer's limiter state.

use std::sync::Arc;

use crate::layer::LimiterShared;

/// Handle for read-only inspection of the running limiter set.
pub struct LimiterHandle<K>
where
	K: std::hash::Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	pub(crate) shared: Arc<LimiterShared<K>>,
}

/// Snapshot of the limiter's runtime state. Intended for monitoring; values are
/// point-in-time.
///
/// `top_n` is sourced from the per-limiter sidecar tracker (see `tracker.rs`),
/// not from governor's keyed state store, which 0.10 does not let us iterate.
/// As a consequence:
///   * Keys appear in `top_n` only after they have been touched at least once.
///   * Hit counts are tracked per key and saturate at `u64::MAX`.
///   * When `max_keys` is set, the tracker bounds itself to that value. When unset,
///     it uses an internal observability budget and evicts least-recently-touched
///     tracker entries after the budget is reached. This keeps `top_n` approximate
///     instead of letting monitoring state grow without bound.
#[derive(Clone, Debug)]
pub struct LimiterSnapshot {
	pub key_count: usize,
	/// Best-effort estimate; computed as key_count * per_entry + constant_overhead.
	pub approx_bytes: usize,
	/// Top-N most-active keys (default N = 10), descending by hit count, formatted
	/// via `Debug` so the snapshot type stays free of `K`.
	pub top_n: Vec<(String, u64)>,
}

impl<K> LimiterHandle<K>
where
	K: std::hash::Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	pub fn snapshot(&self) -> LimiterSnapshot {
		self.snapshot_top_n(10)
	}

	/// Same as `snapshot()` but lets the caller choose the size of `top_n`. Useful
	/// from dashboards that want a longer or shorter list than the default 10.
	pub fn snapshot_top_n(&self, n: usize) -> LimiterSnapshot {
		let mut key_count = self.shared.default_limiter.as_ref().map(|l| l.len()).unwrap_or(0);
		for (_, l) in &self.shared.method_limiters {
			key_count += l.len();
		}
		for entry in &self.shared.stack {
			key_count += entry.len();
		}
		key_count += self.shared.tier_cache.total_len();

		// Per-entry estimate: K + governor's TAT (~24 bytes) + DashMap shard overhead (~16 bytes amortized).
		let per_entry = std::mem::size_of::<K>() + 24 + 16;
		let approx_bytes = key_count.saturating_mul(per_entry).saturating_add(64);

		let top_n = self.gather_top_n(n);

		LimiterSnapshot { key_count, approx_bytes, top_n }
	}

	fn gather_top_n(&self, n: usize) -> Vec<(String, u64)> {
		if n == 0 {
			return Vec::new();
		}
		let mut buckets: Vec<(String, u64)> = Vec::new();
		if let Some(t) = self.shared.default_tracker.as_ref() {
			buckets.extend(t.top_n(n));
		}
		for (_, t) in &self.shared.method_trackers {
			buckets.extend(t.top_n(n));
		}
		for entry in &self.shared.stack {
			buckets.extend(entry.top_n(n));
		}

		// Multiple trackers can each contribute their own slot; merge by key string and
		// keep the largest hit count, then take the top N.
		let mut merged: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
		for (k, h) in buckets {
			merged.entry(k).and_modify(|cur| *cur = (*cur).max(h)).or_insert(h);
		}
		let mut entries: Vec<(String, u64)> = merged.into_iter().collect();
		entries.sort_unstable_by_key(|e| std::cmp::Reverse(e.1));
		entries.truncate(n);
		entries
	}
}

#[cfg(test)]
mod tests {
	use http::Method;

	use crate::builder::GovernorConfigBuilder;
	use crate::extractor::{Global, PeerIp};
	use crate::layer::GovernorLayer;
	use crate::test_utils::{drive_response, request, request_with_peer};
	use crate::{Quota, nz};

	fn req(method: Method, path: &str) -> http::Request<axum::body::Body> {
		request(method, path)
	}

	fn req_with_peer(method: Method, path: &str, peer: &str) -> http::Request<axum::body::Body> {
		request_with_peer(method, path, peer.parse().unwrap())
	}

	#[tokio::test]
	async fn snapshot_global_one_request_key_count_gte_one() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		drive_response(&layer, req(Method::GET, "/")).await;

		let snap = layer.limiter().snapshot();
		assert!(snap.key_count >= 1, "expected at least one key, got {}", snap.key_count);
		assert!(snap.approx_bytes > 0, "expected non-zero approx_bytes");
		assert!(!snap.top_n.is_empty(), "top_n should be populated by the tracker");
		assert_eq!(snap.top_n[0].1, 1, "single-request key should have one hit");
	}

	#[tokio::test]
	async fn snapshot_peer_ip_three_ips_key_count_equals_three() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		drive_response(&layer, req_with_peer(Method::GET, "/", "1.2.3.4:1234")).await;
		drive_response(&layer, req_with_peer(Method::GET, "/", "5.6.7.8:1234")).await;
		drive_response(&layer, req_with_peer(Method::GET, "/", "9.10.11.12:1234")).await;

		let snap = layer.limiter().snapshot();
		assert_eq!(snap.key_count, 3, "expected 3 distinct IP keys, got {}", snap.key_count);
		assert_eq!(snap.top_n.len(), 3);
		assert!(snap.top_n.iter().all(|(_, h)| *h == 1));
	}

	#[tokio::test]
	async fn snapshot_top_n_orders_by_hits_descending() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			.quota_default(Quota::requests_per_second(nz!(50u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		// 5x ip1, 2x ip2, 1x ip3 — top should be ip1 then ip2 then ip3.
		for _ in 0..5 {
			drive_response(&layer, req_with_peer(Method::GET, "/", "1.1.1.1:1")).await;
		}
		for _ in 0..2 {
			drive_response(&layer, req_with_peer(Method::GET, "/", "2.2.2.2:1")).await;
		}
		drive_response(&layer, req_with_peer(Method::GET, "/", "3.3.3.3:1")).await;

		let snap = layer.limiter().snapshot();
		assert!(snap.top_n.len() >= 3);
		assert_eq!(snap.top_n[0].1, 5);
		assert_eq!(snap.top_n[1].1, 2);
		assert_eq!(snap.top_n[2].1, 1);

		let two = layer.limiter().snapshot_top_n(2);
		assert_eq!(two.top_n.len(), 2);
		assert_eq!(two.key_count, snap.key_count, "top_n size must not change the key count");
	}
}
