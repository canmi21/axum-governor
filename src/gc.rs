//! Background garbage-collection task for the key state store.

use std::sync::Weak;
use std::time::Duration;

use crate::layer::LimiterShared;

pub(crate) fn spawn_gc_inner<K>(
	weak: Weak<LimiterShared<K>>,
	every: Duration,
) -> Option<tokio::task::AbortHandle>
where
	K: std::hash::Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	// If there is no Tokio runtime (e.g. in synchronous unit tests), skip spawning.
	// The GC task is a best-effort background sweep; its absence is safe.
	let rt = match tokio::runtime::Handle::try_current() {
		Ok(h) => h,
		Err(_) => return None,
	};
	let handle = rt.spawn(async move {
		let mut tick = tokio::time::interval(every);
		// First tick fires immediately; skip it so we wait one full interval before the
		// first sweep.
		tick.tick().await;
		loop {
			tick.tick().await;
			let Some(strong) = weak.upgrade() else { return };
			strong.retain_all();
			// Drop the Arc immediately so the gc task does not extend the lifetime
			// of LimiterShared between ticks.
			drop(strong);
		}
	});
	Some(handle.abort_handle())
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::time::Duration;

	use crate::builder::GovernorConfigBuilder;
	use crate::extractor::Global;
	use crate::layer::GovernorLayer;
	use crate::{Quota, nz};

	#[tokio::test]
	async fn gc_task_spawned_when_interval_set() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.gc_interval(Duration::from_millis(50))
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert!(layer.shared.gc_handle.is_some(), "GC task must be spawned");
	}

	#[tokio::test]
	async fn gc_task_not_spawned_when_disabled() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.gc_disable()
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert!(layer.shared.gc_handle.is_none());
	}

	#[tokio::test(flavor = "current_thread", start_paused = true)]
	async fn gc_sweep_drops_keys_whose_window_has_passed() {
		use http::Method;

		use crate::extractor::PeerIp;
		use crate::test_utils::{drive_response, request_with_peer};

		let cfg = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			// 1000/s: one hit puts the key's theoretical arrival time ~1 ms out, so a
			// few real milliseconds later governor considers it stale.
			.quota_default(Quota::requests_per_second(nz!(1000u32)))
			.gc_interval(Duration::from_millis(50))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		// Let the task run once so its interval is registered and the immediate first
		// tick is consumed; until then advancing the clock has nothing to fire.
		tokio::task::yield_now().await;

		drive_response(&layer, request_with_peer(Method::GET, "/", "1.2.3.4:1".parse().unwrap())).await;
		assert_eq!(layer.limiter().snapshot().key_count, 1);

		// governor reads the wall clock, not tokio's paused one, so real time has to pass
		// for the key to age; the interval itself is driven by the paused clock.
		std::thread::sleep(Duration::from_millis(20));
		tokio::time::advance(Duration::from_millis(60)).await;
		tokio::task::yield_now().await;

		assert_eq!(layer.limiter().snapshot().key_count, 0, "GC sweep did not run");
	}

	#[tokio::test(flavor = "current_thread", start_paused = true)]
	async fn gc_task_aborts_on_last_arc_drop() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.gc_interval(Duration::from_millis(50))
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		// Hold a Weak so we can observe the strong count drop to zero.
		let weak = Arc::downgrade(&layer.shared);
		drop(layer);
		// Advance simulated clock past interval, then yield.
		tokio::time::advance(Duration::from_millis(200)).await;
		tokio::task::yield_now().await;
		assert!(weak.upgrade().is_none(), "LimiterShared must be dropped after Layer drop");
	}
}
