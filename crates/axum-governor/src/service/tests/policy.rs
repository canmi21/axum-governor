use http::{Method, Request, StatusCode, header::AUTHORIZATION};

use crate::builder::GovernorConfigBuilder;
use crate::extractor::{Global, Header, KeyExtractor, KeyOutcome};
use crate::layer::GovernorLayer;
use crate::test_utils::drive_response;
use crate::{Quota, nz};

use super::{policy_header, req};

#[tokio::test]
async fn per_method_quota_get_exhausted_post_passes() {
	// GET has 1/s quota; POST falls through to default 100/s.
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_for(Method::GET, Quota::requests_per_second(nz!(1u32)))
		.quota_default(Quota::requests_per_second(nz!(100u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// First GET passes.
	let r1 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r1.status(), StatusCode::OK);

	// Second GET is rejected.
	let r2 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

	// POST uses default quota (100/s) — passes.
	let r3 = drive_response(&layer, req(Method::POST, "/")).await;
	assert_eq!(r3.status(), StatusCode::OK);

	// GET still rejected.
	let r4 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r4.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn stacked_peer_and_auth_first_reject_wins() {
	// Stack: peer 1/s (Global key), auth 600/min (Global key).
	// Exhaust peer on 2nd request; rejection names "peer" and lists both policies.
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.stack("peer", Global, Quota::requests_per_second(nz!(1u32)))
		.stack("auth", Global, Quota::requests_per_minute(nz!(600u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// First request passes.
	let r1 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r1.status(), StatusCode::OK);

	// Second: peer (1/s) exhausted — 429.
	let r2 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

	// RateLimit: header names the triggering entry.
	let rl = r2.headers().get("ratelimit").unwrap().to_str().unwrap();
	assert!(rl.contains("\"peer\""), "expected 'peer' in RateLimit header, got: {rl}");

	// RateLimit-Policy: lists both.
	let policy = r2.headers().get("ratelimit-policy").unwrap().to_str().unwrap();
	assert!(policy.contains("\"peer\""), "expected 'peer' in policy header");
	assert!(policy.contains("\"auth\""), "expected 'auth' in policy header");
}

#[tokio::test]
async fn multi_window_quotas_first_rejects_on_per_second() {
	// quotas("peer", Global, [1/s, 60/m]) expands to peer:1s and peer:1m.
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quotas(
			"peer",
			Global,
			[Quota::requests_per_second(nz!(1u32)), Quota::requests_per_minute(nz!(60u32))],
		)
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// First request passes.
	let r1 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r1.status(), StatusCode::OK);

	// Second request: peer:1s exhausted → 429.
	let r2 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

	// Policy header lists both windows.
	let policy = r2.headers().get("ratelimit-policy").unwrap().to_str().unwrap();
	assert!(policy.contains("\"peer:1s\""), "expected peer:1s in policy, got: {policy}");
	assert!(policy.contains("\"peer:1m\""), "expected peer:1m in policy, got: {policy}");
}

#[derive(Clone)]
struct TierExtractor;
impl KeyExtractor for TierExtractor {
	type Key = ();
	fn extract(
		&self,
		_parts: &http::request::Parts,
	) -> Result<KeyOutcome<()>, crate::ExtractionError> {
		Ok(KeyOutcome { key: (), quota_override: Some(Quota::requests_per_second(nz!(100u32))) })
	}
}

#[tokio::test]
async fn per_tier_quota_override_admits_burst() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(TierExtractor)
		// Default very tight; override allows more.
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	// With default 1/s the second would fail; with override 100/s it passes.
	let r1 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r1.status(), StatusCode::OK);
	let r2 = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r2.status(), StatusCode::OK, "override quota should admit second request");
}

#[tokio::test]
async fn stacked_reject_names_correct_entry_in_ratelimit_header() {
	// peer 100/s (never exhausted), auth (via Header) 1/s.
	// On the second request: peer passes (100 burst), auth rejects.
	// The RateLimit: header should name "auth".
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.stack("peer", Global, Quota::requests_per_second(nz!(100u32)))
		.stack("auth", Header(&AUTHORIZATION), Quota::requests_per_second(nz!(1u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	let make_req = || {
		Request::builder()
			.method(Method::GET)
			.uri("/")
			.header(AUTHORIZATION, "Bearer token123")
			.body(axum::body::Body::empty())
			.unwrap()
	};

	// First request passes.
	let r1 = drive_response(&layer, make_req()).await;
	assert_eq!(r1.status(), StatusCode::OK);

	// Second request: auth (1/s) exhausted → 429 naming "auth".
	let r2 = drive_response(&layer, make_req()).await;
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
	let rl = r2.headers().get("ratelimit").unwrap().to_str().unwrap();
	assert!(rl.contains("\"auth\""), "expected 'auth' in RateLimit header, got: {rl}");
}

#[tokio::test]
async fn tier_override_renders_its_own_quota_in_the_policy_header() {
	// The precomputed "default" policy header carries the default quota; a tier
	// override must fall back to inline rendering so the header tells the truth.
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(TierExtractor)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	let r = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r.status(), StatusCode::OK);
	assert_eq!(policy_header(&r), Some("\"default\";q=100;w=1"));
	assert_eq!(r.headers()["x-ratelimit-limit"], "100");
}

#[tokio::test]
async fn stack_only_admit_names_the_first_entry() {
	// No primary quota: the RateLimit: header still has to name one policy, and the
	// first advertised stack entry is the one a client would want to pace against.
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.stack("peer", Global, Quota::requests_per_second(nz!(10u32)))
		.stack("auth", Global, Quota::requests_per_minute(nz!(600u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	let r = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r.status(), StatusCode::OK);
	let rl = r.headers()["ratelimit"].to_str().unwrap();
	assert!(rl.starts_with("\"peer\";r=9;"), "got {rl}");
	assert_eq!(policy_header(&r), Some("\"peer\";q=10;w=1, \"auth\";q=600;w=60"));
}

#[tokio::test]
async fn stack_extraction_failure_rejects_with_extraction_status() {
	// A stack entry whose extractor fails is a request problem, not a quota problem:
	// 400 for a missing header, and no rate-limit headers because nothing was counted.
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.stack("auth", Header(&AUTHORIZATION), Quota::requests_per_second(nz!(10u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);

	let r = drive_response(&layer, req(Method::GET, "/")).await;
	assert_eq!(r.status(), StatusCode::BAD_REQUEST);
	assert!(policy_header(&r).is_none());
}
