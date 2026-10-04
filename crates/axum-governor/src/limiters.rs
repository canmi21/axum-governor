//! Per-request limiter primitives: `StackedRunner`, `LimiterCache`, and `StackEntryFactory`.
//!
//! The builder stores a typed factory per stack entry and the `RateLimiter` is only built
//! inside `GovernorLayer::new`. Building eagerly would force the builder to hold a
//! type-erased extractor and recover `E::Key` later; the factory keeps the key type until
//! the layer is finalized.

use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use governor::clock::DefaultClock;
use governor::middleware::StateInformationMiddleware;

use crate::extractor::KeyExtractor;
use crate::layer::KeyedRateLimiter;
use crate::tracker::{EvictionReason, KeyTracker};

/// Outcome of checking one stacked limiter entry.
pub(crate) enum StackedResult {
	Admit { remaining: u32 },
	Reject { wait: Duration, key_repr: String },
	ExtractionFailed(crate::ExtractionError),
}

/// Object-safe trait for one entry in the ordered stack of limiters.
///
/// The Layer holds `Vec<Box<dyn StackedRunner>>`. On each request every entry is
/// checked in insertion order; the first `Reject` wins.
pub(crate) trait StackedRunner: Send + Sync + 'static {
	fn name(&self) -> &str;
	fn name_arc(&self) -> Arc<str>;
	fn quota(&self) -> crate::Quota;
	fn check(&self, parts: &http::request::Parts, redact: bool) -> StackedResult;
	fn retain_recent(&self);
	fn len(&self) -> usize;
	fn top_n(&self, n: usize) -> Vec<(String, u64)>;
}

/// Concrete implementation of `StackedRunner` for a given `KeyExtractor`.
pub(crate) struct StackedEntry<E: KeyExtractor> {
	name: Arc<str>,
	quota: crate::Quota,
	extractor: Arc<E>,
	limiter: KeyedRateLimiter<E::Key>,
	tracker: KeyTracker<E::Key>,
}

impl<E: KeyExtractor> StackedRunner for StackedEntry<E> {
	fn name(&self) -> &str {
		&self.name
	}

	fn name_arc(&self) -> Arc<str> {
		Arc::clone(&self.name)
	}

	fn quota(&self) -> crate::Quota {
		self.quota
	}

	fn check(&self, parts: &http::request::Parts, redact: bool) -> StackedResult {
		use governor::clock::Clock as _;
		match self.extractor.extract(parts) {
			Err(e) => StackedResult::ExtractionFailed(e),
			Ok(outcome) => {
				let result = match self.limiter.check_key(&outcome.key) {
					Ok(snapshot) => StackedResult::Admit { remaining: snapshot.remaining_burst_capacity() },
					Err(not_until) => {
						let now = DefaultClock::default().now();
						let key_repr = crate::util::format_key(&outcome.key, redact);
						StackedResult::Reject { wait: not_until.wait_time_from(now), key_repr }
					}
				};
				if self.tracker.touch(&outcome.key) == Some(EvictionReason::MaxKeys) {
					crate::trace::eviction(&self.name);
					self.limiter.retain_recent();
				}
				result
			}
		}
	}

	fn retain_recent(&self) {
		self.limiter.retain_recent();
	}

	fn len(&self) -> usize {
		self.limiter.len()
	}

	fn top_n(&self, n: usize) -> Vec<(String, u64)> {
		self.tracker.top_n(n)
	}
}

/// Type-erased factory that builds one `Box<dyn StackedRunner>` when the Layer is
/// finalized. The builder stores `Vec<Box<dyn StackEntryFactory>>` and calls `build()`
/// inside `Layer::new()`.
pub(crate) trait StackEntryFactory: Send + Sync + 'static {
	fn build(self: Box<Self>, max_keys: Option<usize>) -> Box<dyn StackedRunner>;
}

/// Concrete factory for `StackedEntry<E>`. Holds everything except the `RateLimiter`,
/// which is constructed inside `build()`.
pub(crate) struct TypedStackFactory<E: KeyExtractor> {
	pub(crate) name: Arc<str>,
	pub(crate) quota: crate::Quota,
	pub(crate) extractor: Arc<E>,
}

impl<E: KeyExtractor> StackEntryFactory for TypedStackFactory<E> {
	fn build(self: Box<Self>, max_keys: Option<usize>) -> Box<dyn StackedRunner> {
		let limiter = governor::RateLimiter::keyed(self.quota.inner())
			.with_middleware::<StateInformationMiddleware>();
		Box::new(StackedEntry {
			name: self.name,
			quota: self.quota,
			extractor: self.extractor,
			limiter,
			tracker: KeyTracker::new(max_keys),
		})
	}
}

/// Per-quota limiter cache for tier overrides.
///
/// State is not shared across quotas: a key that changes tier mid-session starts a fresh
/// bucket with a full burst for the new quota. Acceptable because the key itself is
/// unchanged and only the wrapper differs.
#[cfg(feature = "dashmap")]
pub(crate) struct LimiterCache<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	inner: dashmap::DashMap<crate::Quota, Arc<KeyedRateLimiter<K>>>,
}

#[cfg(feature = "dashmap")]
impl<K> LimiterCache<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	pub(crate) fn new() -> Self {
		Self { inner: dashmap::DashMap::new() }
	}

	pub(crate) fn get_or_insert(&self, quota: crate::Quota) -> Arc<KeyedRateLimiter<K>> {
		self
			.inner
			.entry(quota)
			.or_insert_with(|| {
				Arc::new(
					governor::RateLimiter::keyed(quota.inner())
						.with_middleware::<StateInformationMiddleware>(),
				)
			})
			.clone()
	}

	pub(crate) fn retain_all(&self) {
		for kv in self.inner.iter() {
			kv.value().retain_recent();
		}
	}

	pub(crate) fn total_len(&self) -> usize {
		self.inner.iter().map(|kv| kv.value().len()).sum()
	}
}

#[cfg(not(feature = "dashmap"))]
pub(crate) struct LimiterCache<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	inner: std::sync::Mutex<std::collections::HashMap<crate::Quota, Arc<KeyedRateLimiter<K>>>>,
}

#[cfg(not(feature = "dashmap"))]
impl<K> LimiterCache<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	pub(crate) fn new() -> Self {
		Self { inner: std::sync::Mutex::new(std::collections::HashMap::new()) }
	}

	pub(crate) fn get_or_insert(&self, quota: crate::Quota) -> Arc<KeyedRateLimiter<K>> {
		let mut map = self.inner.lock().expect("LimiterCache mutex poisoned");
		map
			.entry(quota)
			.or_insert_with(|| {
				Arc::new(
					governor::RateLimiter::keyed(quota.inner())
						.with_middleware::<StateInformationMiddleware>(),
				)
			})
			.clone()
	}

	pub(crate) fn retain_all(&self) {
		let map = self.inner.lock().expect("LimiterCache mutex poisoned");
		for l in map.values() {
			l.retain_recent();
		}
	}

	pub(crate) fn total_len(&self) -> usize {
		let map = self.inner.lock().expect("LimiterCache mutex poisoned");
		map.values().map(|l| l.len()).sum()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Quota, nz};

	// Test that inserting the same Quota twice returns the same Arc pointer (both cfg paths).

	#[cfg(feature = "dashmap")]
	#[test]
	fn dashmap_cache_reuses_same_arc() {
		let cache: LimiterCache<()> = LimiterCache::new();
		let q = Quota::requests_per_second(nz!(10u32));
		let a = cache.get_or_insert(q);
		let b = cache.get_or_insert(q);
		assert!(Arc::ptr_eq(&a, &b), "expected same Arc on duplicate Quota insert");
	}

	#[cfg(not(feature = "dashmap"))]
	#[test]
	fn mutex_cache_reuses_same_arc() {
		let cache: LimiterCache<()> = LimiterCache::new();
		let q = Quota::requests_per_second(nz!(10u32));
		let a = cache.get_or_insert(q);
		let b = cache.get_or_insert(q);
		assert!(Arc::ptr_eq(&a, &b), "expected same Arc on duplicate Quota insert");
	}

	#[test]
	fn different_quotas_get_different_arcs() {
		let cache: LimiterCache<()> = LimiterCache::new();
		let q1 = Quota::requests_per_second(nz!(10u32));
		let q2 = Quota::requests_per_minute(nz!(10u32));
		let a = cache.get_or_insert(q1);
		let b = cache.get_or_insert(q2);
		assert!(!Arc::ptr_eq(&a, &b), "different quotas must produce different limiters");
	}
}
