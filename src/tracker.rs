//! Sidecar key tracker that enforces `max_keys` and powers `LimiterSnapshot::top_n`.
//!
//! governor 0.10 only lets us read `len()` and call `retain_recent()` on a keyed state
//! store; it does not expose per-key removal or iteration. So we keep our own table
//! of observed keys: a hit counter plus a monotonic sequence stamp for approximate LRU
//! eviction. Operations are O(log n) on touch (one `BTreeMap` insert + one remove of
//! the previous stamp) and only run under `max_keys`-bound — for unbounded limiters
//! we skip the LRU bookkeeping entirely.

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::sync::Mutex;

/// Per-limiter sidecar that pairs governor's keyed state store with our own LRU and
/// hit-count tables. `K_repr` is the key's `Debug` rendering; this lets the tracker
/// stay independent of `K` while still exposing meaningful labels via `top_n`.
pub(crate) struct KeyTracker<K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	inner: Mutex<TrackerInner<K>>,
	max_keys: Option<usize>,
}

struct TrackerInner<K> {
	by_key: HashMap<K, KeyState>,
	by_seq: BTreeMap<u64, K>,
	next_seq: u64,
}

struct KeyState {
	seq: u64,
	hits: u64,
}

/// Outcome of a single `touch`. `evicted` is the key bumped out of our LRU when
/// the tracker is over `max_keys`; the caller is expected to react (typically by
/// calling `RateLimiter::retain_recent()` to nudge governor's DashMap).
pub(crate) struct TouchOutcome<K> {
	pub evicted: Option<K>,
}

impl<K> KeyTracker<K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	pub(crate) fn new(max_keys: Option<usize>) -> Self {
		Self {
			inner: Mutex::new(TrackerInner {
				by_key: HashMap::new(),
				by_seq: BTreeMap::new(),
				next_seq: 0,
			}),
			max_keys,
		}
	}

	/// Record an access for `key`. Returns the key evicted from the tracker when the
	/// LRU is over `max_keys`. When `max_keys` is `None` we still keep counts for
	/// `top_n` but never evict; tracker memory is bounded by governor's own
	/// `retain_recent` sweep in that mode.
	pub(crate) fn touch(&self, key: &K) -> TouchOutcome<K> {
		let mut g = self.inner.lock().expect("KeyTracker mutex poisoned");
		let seq = g.next_seq;
		g.next_seq = g.next_seq.wrapping_add(1);

		match g.by_key.get_mut(key) {
			Some(state) => {
				let old_seq = state.seq;
				state.seq = seq;
				state.hits = state.hits.saturating_add(1);
				g.by_seq.remove(&old_seq);
				g.by_seq.insert(seq, key.clone());
			}
			None => {
				g.by_key.insert(key.clone(), KeyState { seq, hits: 1 });
				g.by_seq.insert(seq, key.clone());
			}
		}

		let evicted = match self.max_keys {
			Some(cap) if g.by_key.len() > cap => pop_oldest(&mut g),
			_ => None,
		};

		TouchOutcome { evicted }
	}

	#[cfg(test)]
	pub(crate) fn key_count(&self) -> usize {
		self.inner.lock().expect("KeyTracker mutex poisoned").by_key.len()
	}

	#[cfg(test)]
	pub(crate) fn forget(&self, key: &K) {
		let mut g = self.inner.lock().expect("KeyTracker mutex poisoned");
		if let Some(state) = g.by_key.remove(key) {
			g.by_seq.remove(&state.seq);
		}
	}

	/// Snapshot the top-`n` hottest keys in descending hit-count order. Returns the
	/// keys' `Debug` form so the snapshot type stays free of `K`.
	pub(crate) fn top_n(&self, n: usize) -> Vec<(String, u64)> {
		if n == 0 {
			return Vec::new();
		}
		let g = self.inner.lock().expect("KeyTracker mutex poisoned");
		let mut entries: Vec<(&K, u64)> = g.by_key.iter().map(|(k, s)| (k, s.hits)).collect();
		entries.sort_unstable_by_key(|e| std::cmp::Reverse(e.1));
		entries.into_iter().take(n).map(|(k, h)| (format!("{k:?}"), h)).collect()
	}
}

fn pop_oldest<K>(g: &mut TrackerInner<K>) -> Option<K>
where
	K: Hash + Eq + Clone,
{
	let (oldest_seq, oldest_key) = g.by_seq.iter().next().map(|(s, k)| (*s, k.clone()))?;
	g.by_seq.remove(&oldest_seq);
	g.by_key.remove(&oldest_key);
	Some(oldest_key)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn touch_with_no_cap_never_evicts() {
		let t: KeyTracker<u32> = KeyTracker::new(None);
		for k in 0..1000u32 {
			let out = t.touch(&k);
			assert!(out.evicted.is_none());
		}
		assert_eq!(t.key_count(), 1000);
	}

	#[test]
	fn touch_with_cap_evicts_oldest_first() {
		let t: KeyTracker<u32> = KeyTracker::new(Some(3));
		t.touch(&1);
		t.touch(&2);
		t.touch(&3);
		// Re-touching 1 promotes it; the next insertion should evict 2 (now oldest).
		t.touch(&1);
		let out = t.touch(&4);
		assert_eq!(out.evicted, Some(2), "expected 2 to be evicted, key 1 was just touched");
		assert_eq!(t.key_count(), 3);
	}

	#[test]
	fn top_n_orders_by_hits_descending() {
		let t: KeyTracker<&'static str> = KeyTracker::new(None);
		for _ in 0..5 {
			t.touch(&"a");
		}
		for _ in 0..2 {
			t.touch(&"b");
		}
		t.touch(&"c");
		let top = t.top_n(2);
		assert_eq!(top.len(), 2);
		assert_eq!(top[0].1, 5);
		assert_eq!(top[1].1, 2);
		assert!(top[0].0.contains('a'));
		assert!(top[1].0.contains('b'));
	}

	#[test]
	fn top_n_zero_returns_empty() {
		let t: KeyTracker<u32> = KeyTracker::new(None);
		t.touch(&1);
		assert!(t.top_n(0).is_empty());
	}

	#[test]
	fn forget_removes_key_and_seq() {
		let t: KeyTracker<u32> = KeyTracker::new(Some(2));
		t.touch(&1);
		t.touch(&2);
		t.forget(&1);
		assert_eq!(t.key_count(), 1);
		// 1 is gone, so adding 3 should not evict anything.
		let out = t.touch(&3);
		assert!(out.evicted.is_none());
	}

	#[test]
	fn cap_zero_evicts_every_insertion() {
		// Edge case: max_keys = 0 should reject everything by evicting on every touch.
		let t: KeyTracker<u32> = KeyTracker::new(Some(0));
		let out = t.touch(&1);
		assert_eq!(out.evicted, Some(1));
		assert_eq!(t.key_count(), 0);
	}
}
