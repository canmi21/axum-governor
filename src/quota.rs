//! Rate quota types exposed in the public API.

use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::time::Duration;

/// Re-export of `nonzero_ext::nonzero!` under a shorter name.
pub use nonzero_ext::nonzero as nz;

/// A rate quota: a burst size and a replenishment interval.
///
/// Construct with the named methods rather than through `governor::Quota` directly;
/// this crate's names are unambiguous where governor's `per_second` is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quota(pub(crate) governor::Quota);

// governor::Quota does not derive Hash; implement manually by hashing the two
// observable fields that fully characterize a quota for rate-limiting purposes.
impl Hash for Quota {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.burst_size().get().hash(state);
		self.0.replenish_interval().hash(state);
	}
}

impl Quota {
	/// Allow at most `n` requests per second, with burst equal to `n`.
	pub const fn requests_per_second(n: NonZeroU32) -> Self {
		Self(governor::Quota::per_second(n))
	}

	/// Allow at most `n` requests per minute, with burst equal to `n`.
	pub const fn requests_per_minute(n: NonZeroU32) -> Self {
		Self(governor::Quota::per_minute(n))
	}

	/// Allow at most `n` requests per hour, with burst equal to `n`.
	pub const fn requests_per_hour(n: NonZeroU32) -> Self {
		Self(governor::Quota::per_hour(n))
	}

	/// Allow one request per `n` seconds, with burst 1.
	pub fn seconds_per_request(n: NonZeroU32) -> Self {
		let q = governor::Quota::with_period(Duration::from_secs(n.get() as u64))
			.expect("NonZeroU32 produces non-zero Duration");
		Self(q)
	}

	/// Override the burst capacity to `n` while keeping the replenishment rate unchanged.
	pub const fn burst(self, n: NonZeroU32) -> Self {
		Self(self.0.allow_burst(n))
	}

	pub(crate) const fn inner(self) -> governor::Quota {
		self.0
	}
}

impl From<Quota> for governor::Quota {
	fn from(q: Quota) -> Self {
		q.inner()
	}
}

impl From<governor::Quota> for Quota {
	fn from(q: governor::Quota) -> Self {
		Self(q)
	}
}

#[cfg(test)]
mod tests {
	use std::collections::hash_map::DefaultHasher;
	use std::hash::{Hash, Hasher};
	use std::num::NonZeroU32;
	use std::time::Duration;

	use super::{Quota, nz};

	fn hash_of(q: Quota) -> u64 {
		let mut h = DefaultHasher::new();
		q.hash(&mut h);
		h.finish()
	}

	const _: core::num::NonZeroU32 = nz!(50u32);

	#[test]
	fn requests_per_second_round_trips() {
		let n = NonZeroU32::new(10).unwrap();
		assert_eq!(Quota::requests_per_second(n).inner(), governor::Quota::per_second(n));
	}

	#[test]
	fn requests_per_minute_round_trips() {
		let n = NonZeroU32::new(60).unwrap();
		assert_eq!(Quota::requests_per_minute(n).inner(), governor::Quota::per_minute(n));
	}

	#[test]
	fn requests_per_hour_round_trips() {
		let n = NonZeroU32::new(3600).unwrap();
		assert_eq!(Quota::requests_per_hour(n).inner(), governor::Quota::per_hour(n));
	}

	#[test]
	fn seconds_per_request_interval_and_burst() {
		let n = NonZeroU32::new(5).unwrap();
		let inner = Quota::seconds_per_request(n).inner();
		assert_eq!(inner.replenish_interval(), Duration::from_secs(5));
		assert_eq!(inner.burst_size().get(), 1);
	}

	#[test]
	fn burst_updates_burst_size_leaves_interval() {
		let n = NonZeroU32::new(10).unwrap();
		let burst_n = NonZeroU32::new(100).unwrap();
		let base = Quota::requests_per_second(n);
		let interval = base.inner().replenish_interval();
		let q = base.burst(burst_n).inner();
		assert_eq!(q.burst_size(), burst_n);
		assert_eq!(q.replenish_interval(), interval);
	}

	#[test]
	fn equal_quotas_hash_equal() {
		let a = Quota::requests_per_second(nz!(50u32));
		let b = Quota::requests_per_second(nz!(50u32));
		assert_eq!(a, b);
		assert_eq!(hash_of(a), hash_of(b));
	}

	#[test]
	fn different_quotas_hash_different() {
		let a = Quota::requests_per_second(nz!(50u32));
		let b = Quota::requests_per_minute(nz!(50u32));
		assert_ne!(hash_of(a), hash_of(b));
	}
}
