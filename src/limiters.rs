//! Per-request limiter primitives: StackedRunner, LimiterCache, and StackEntryFactory.
//!
//! Structural choice (option a): the builder constructs stack entries immediately at
//! stack()/quotas() call time for the factory objects, and the actual RateLimiter is
//! constructed when Layer::new() is called (via StackEntryFactory::build()). This
//! avoids storing Arc<dyn ErasedSyncExtractor> and re-extracting the key type at build
//! time, keeping type information alive through the factory pattern until the moment the
//! layer is finalized.

use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use governor::clock::DefaultClock;
use governor::middleware::StateInformationMiddleware;

use crate::extractor::KeyExtractor;
use crate::layer::KeyedRateLimiter;

// ---------------------------------------------------------------------------
// StackedResult
// ---------------------------------------------------------------------------

/// Outcome of checking one stacked limiter entry.
pub(crate) enum StackedResult {
	Admit { remaining: u32 },
	Reject { wait: Duration, key_repr: String },
	ExtractionFailed(crate::ExtractionError),
}

// ---------------------------------------------------------------------------
// StackedRunner
// ---------------------------------------------------------------------------

/// Object-safe trait for one entry in the ordered stack of limiters.
///
/// The Layer holds `Vec<Box<dyn StackedRunner>>`. On each request every entry is
/// checked in insertion order; the first `Reject` wins.
pub(crate) trait StackedRunner: Send + Sync + 'static {
	fn name(&self) -> &'static str;
	fn quota(&self) -> crate::Quota;
	fn check(&self, parts: &http::request::Parts, redact: bool) -> StackedResult;
	fn retain_recent(&self);
	fn len(&self) -> usize;
}

// ---------------------------------------------------------------------------
// StackedEntry<E>
// ---------------------------------------------------------------------------

/// Concrete implementation of `StackedRunner` for a given `KeyExtractor`.
pub(crate) struct StackedEntry<E: KeyExtractor> {
	name: &'static str,
	quota: crate::Quota,
	extractor: Arc<E>,
	limiter: KeyedRateLimiter<E::Key>,
}

impl<E: KeyExtractor> StackedRunner for StackedEntry<E> {
	fn name(&self) -> &'static str {
		self.name
	}

	fn quota(&self) -> crate::Quota {
		self.quota
	}

	fn check(&self, parts: &http::request::Parts, redact: bool) -> StackedResult {
		use governor::clock::Clock as _;
		match self.extractor.extract(parts) {
			Err(e) => StackedResult::ExtractionFailed(e),
			Ok(outcome) => match self.limiter.check_key(&outcome.key) {
				Ok(snapshot) => StackedResult::Admit { remaining: snapshot.remaining_burst_capacity() },
				Err(not_until) => {
					let now = DefaultClock::default().now();
					let key_repr = crate::util::format_key(&outcome.key, redact);
					StackedResult::Reject { wait: not_until.wait_time_from(now), key_repr }
				}
			},
		}
	}

	fn retain_recent(&self) {
		self.limiter.retain_recent();
	}

	fn len(&self) -> usize {
		self.limiter.len()
	}
}

// ---------------------------------------------------------------------------
// StackEntryFactory
// ---------------------------------------------------------------------------

/// Type-erased factory that builds one `Box<dyn StackedRunner>` when the Layer is
/// finalized. The builder stores `Vec<Box<dyn StackEntryFactory>>` and calls `build()`
/// inside `Layer::new()`.
pub(crate) trait StackEntryFactory: Send + Sync + 'static {
	fn build(self: Box<Self>) -> Box<dyn StackedRunner>;
}

/// Concrete factory for `StackedEntry<E>`. Holds everything except the `RateLimiter`,
/// which is constructed inside `build()`.
pub(crate) struct TypedStackFactory<E: KeyExtractor> {
	pub(crate) name: &'static str,
	pub(crate) quota: crate::Quota,
	pub(crate) extractor: Arc<E>,
}

impl<E: KeyExtractor> StackEntryFactory for TypedStackFactory<E> {
	fn build(self: Box<Self>) -> Box<dyn StackedRunner> {
		let limiter = governor::RateLimiter::keyed(self.quota.inner())
			.with_middleware::<StateInformationMiddleware>();
		Box::new(StackedEntry {
			name: self.name,
			quota: self.quota,
			extractor: self.extractor,
			limiter,
		})
	}
}

// ---------------------------------------------------------------------------
// LimiterCache<K>
// ---------------------------------------------------------------------------
//
// limitation: state stores are NOT shared across cached limiters.  A user
// upgrading tier mid-session gets a fresh bucket keyed on the new quota.  This
// is acceptable because the key itself stays the same; only the quota wrapper
// changes, and the new limiter starts with a full burst for the new tier.

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
		for (_, l) in map.iter() {
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
