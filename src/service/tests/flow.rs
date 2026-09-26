use http::{Method, Response, StatusCode};
use ipnet::IpNet;

use crate::builder::GovernorConfigBuilder;
use crate::extractor::{AsyncExtractFuture, AsyncKeyExtractor, Global, KeyOutcome, PeerIp};
use crate::layer::GovernorLayer;
use crate::test_utils::drive_response;
use crate::{Quota, nz};

use super::{req, req_with_peer};

#[tokio::test]
async fn whitelist_method_bypasses_exhausted_limiter() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.whitelist_methods([Method::OPTIONS])
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// exhaust the single token
	let r1 = drive_response(&layer, req(Method::POST, "/")).await;
	assert_eq!(r1.status(), StatusCode::OK);

	// confirm bucket is exhausted
	let r2 = drive_response(&layer, req(Method::POST, "/")).await;
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

	// OPTIONS is whitelisted — must pass even though bucket is empty
	let r3 = drive_response(&layer, req(Method::OPTIONS, "/")).await;
	assert_eq!(r3.status(), StatusCode::OK);
	assert!(r3.headers().get("ratelimit-policy").is_none());
}

#[tokio::test]
async fn whitelist_path_bypasses_exhausted_limiter() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.whitelist_paths(["/health"])
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// exhaust the bucket
	let _ = drive_response(&layer, req(Method::GET, "/api")).await;

	// whitelisted path passes even though bucket is empty
	let r = drive_response(&layer, req(Method::GET, "/health")).await;
	assert_eq!(r.status(), StatusCode::OK);
	assert!(r.headers().get("ratelimit-policy").is_none());
}

#[tokio::test]
async fn whitelist_ip_bypasses_exhausted_limiter() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.whitelist_ips(["127.0.0.0/8".parse::<IpNet>().unwrap()])
		.expect_connect_info()
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// exhaust the bucket from a non-whitelisted IP
	let _ = drive_response(&layer, req_with_peer(Method::GET, "/", "1.2.3.4:1234")).await;

	// localhost is whitelisted
	let r = drive_response(&layer, req_with_peer(Method::GET, "/", "127.0.0.1:1234")).await;
	assert_eq!(r.status(), StatusCode::OK);
	assert!(r.headers().get("ratelimit-policy").is_none());
}

#[tokio::test]
async fn successful_admit_adds_headers() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	let r = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r.status(), StatusCode::OK);
	assert!(r.headers().get("ratelimit-policy").is_some());
	assert!(r.headers().get("ratelimit").is_some());
	assert!(r.headers().get("x-ratelimit-limit").is_some());
	assert!(r.headers().get("x-ratelimit-remaining").is_some());
	assert!(r.headers().get("x-ratelimit-reset").is_some());
}

#[tokio::test]
async fn reject_returns_429_with_all_headers() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// first request succeeds
	let r1 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r1.status(), StatusCode::OK);

	// second request is rejected
	let r2 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
	assert!(r2.headers().get("retry-after").is_some());
	assert!(r2.headers().get("ratelimit-policy").is_some());
	assert!(r2.headers().get("ratelimit").is_some());
	assert!(r2.headers().get("x-ratelimit-limit").is_some());
	assert!(r2.headers().get("x-ratelimit-remaining").is_some());
	assert!(r2.headers().get("x-ratelimit-reset").is_some());
}

#[tokio::test]
async fn key_extraction_failed_returns_500_no_rl_headers() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(PeerIp::default())
		.expect_connect_info()
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// request has no ConnectInfo → PeerIp fails with MissingConnectInfo → 500
	let r = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
	assert!(r.headers().get("ratelimit-policy").is_none());
}

#[tokio::test]
async fn legacy_reset_epoch_produces_epoch_value() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.legacy_reset_epoch(true)
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	let r = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r.status(), StatusCode::OK);

	let reset_val: u64 =
		r.headers().get("x-ratelimit-reset").unwrap().to_str().unwrap().parse().unwrap();
	// Any value greater than this threshold is clearly an epoch, not a delta.
	assert!(reset_val > 1_700_000_000, "expected epoch seconds, got {reset_val}");
}

#[tokio::test]
async fn error_handler_response_used_with_rl_headers_attached() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.error_handler(|_| {
			Response::builder().status(418).body(axum::body::Body::from("custom-body")).unwrap()
		})
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// exhaust bucket
	let _ = drive_response(&layer, req(Method::GET, "/")).await;

	// second request triggers handler
	let r = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r.status(), StatusCode::from_u16(418).unwrap());
	// rate-limit headers are still injected on top of the custom response
	assert!(r.headers().get("ratelimit-policy").is_some());

	let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
	assert!(body_bytes.starts_with(b"custom-body"));
}

#[tokio::test]
async fn async_extractor_admits_and_rejects() {
	#[derive(Clone, Debug)]
	struct SimpleAsync;
	impl AsyncKeyExtractor for SimpleAsync {
		type Key = ();
		fn extract<'a>(&'a self, _parts: &'a http::request::Parts) -> AsyncExtractFuture<'a, ()> {
			Box::pin(async {
				tokio::task::yield_now().await;
				Ok(KeyOutcome { key: (), quota_override: None })
			})
		}
	}

	let cfg = GovernorConfigBuilder::default()
		.with_async_extractor(SimpleAsync)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// First request: 200.
	let r1 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r1.status(), StatusCode::OK);

	// Second request: 429.
	let r2 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn max_keys_evicts_oldest_under_pressure() {
	// max_keys=2 with PeerIp: after 3 distinct IPs, the oldest should have been
	// dropped from the sidecar tracker. governor's internal keyed state is nudged
	// via retain_recent(), but it cannot delete that exact fresh key in governor 0.10.
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(PeerIp::default())
		.expect_connect_info()
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.max_keys(2)
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	for ip in ["1.1.1.1:1", "2.2.2.2:1", "3.3.3.3:1"] {
		let r = drive_response(&layer, req_with_peer(Method::GET, "/", ip)).await;
		assert_eq!(r.status(), StatusCode::OK);
	}

	let snap = layer.limiter().snapshot();
	assert_eq!(snap.top_n.len(), 2, "tracker-backed top_n should be capped by max_keys");
	assert!(
		!snap.top_n.iter().any(|(k, _)| k.contains("1.1.1.1")),
		"oldest key should have been evicted from tracker: {:?}",
		snap.top_n
	);
}

#[test]
fn format_key_redact_off_uses_debug() {
	let k: String = "alice".into();
	assert_eq!(crate::util::format_key(&k, false), "\"alice\"");
}

#[test]
fn format_key_redact_on_hashes() {
	let s = crate::util::format_key(&String::from("alice"), true);
	assert!(s.starts_with("hash:"), "got {s}");
	assert_eq!(s.len(), "hash:".len() + 16);
}

#[tokio::test]
async fn error_handler_sees_key_extraction_failed() {
	use std::sync::Arc;
	use std::sync::Mutex;

	let seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
	let sink = Arc::clone(&seen);
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(PeerIp::default())
		.expect_connect_info()
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.error_handler(move |reason| {
			*sink.lock().unwrap() = Some(format!("{reason:?}"));
			Response::builder().status(503).body(axum::body::Body::empty()).unwrap()
		})
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	let r = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
	let seen = seen.lock().unwrap().clone().expect("handler was called");
	assert!(seen.contains("KeyExtractionFailed(MissingConnectInfo)"), "got {seen}");
}

#[tokio::test]
async fn whitelist_ip_is_judged_on_the_peer_not_forwarded_headers() {
	// The whitelist runs before extraction and only ever sees ConnectInfo, so an
	// X-Forwarded-For naming a whitelisted address must not bypass the limiter.
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.whitelist_ips(["127.0.0.0/8".parse::<IpNet>().unwrap()])
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	let spoofed = || {
		let mut r = req_with_peer(Method::GET, "/", "8.8.8.8:1");
		r.headers_mut().insert("x-forwarded-for", "127.0.0.1".parse().unwrap());
		r
	};
	drive_response(&layer, spoofed()).await;
	let r = drive_response(&layer, spoofed()).await;
	assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[cfg(feature = "tracing")]
#[tokio::test]
async fn tracing_smoke_compiles_and_runs() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);
	drive_response(&layer, req(Method::GET, "/")).await;
	drive_response(&layer, req(Method::GET, "/")).await;
}
