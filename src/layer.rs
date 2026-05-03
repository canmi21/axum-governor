//! `tower::Layer` constructor for the rate-limit middleware.

use std::hash::Hash;
use std::sync::Arc;

use governor::clock::DefaultClock;
use governor::middleware::StateInformationMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;

use crate::Quota;
use crate::builder::{ExtractorSlot, GovernorConfig};

pub(crate) type KeyedRateLimiter<K> =
	governor::RateLimiter<K, DefaultKeyedStateStore<K>, DefaultClock, StateInformationMiddleware>;

pub(crate) struct LimiterShared<K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	pub config: GovernorConfig<K>,
	pub default_limiter: Option<KeyedRateLimiter<K>>,
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
		// Stage 6b removes this guard.
		if matches!(config.extractor, ExtractorSlot::Async(_)) {
			unimplemented!("async extractor wiring is Stage 6b; configure with_extractor(...) for now");
		}
		let default_limiter = config.quota_default.map(|q: Quota| {
			// Use keyed() rather than dashmap() so the build works without the dashmap feature.
			governor::RateLimiter::keyed(q.inner()).with_middleware::<StateInformationMiddleware>()
		});
		Self { shared: Arc::new(LimiterShared { config, default_limiter }) }
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
	use crate::extractor::Global;
	use crate::{Quota, nz};

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
	#[should_panic(expected = "async extractor wiring is Stage 6b")]
	fn async_extractor_panics() {
		use crate::builder::GovernorConfigBuilder;
		use crate::extractor::{AsyncExtractFuture, AsyncKeyExtractor, KeyOutcome};
		use http::request::Parts;

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
}
