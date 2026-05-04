// Integration tests run only with --features test-utils.
// To include these in the test suite: cargo nextest run --features test-utils

use std::net::SocketAddr;

use axum::Router;
use axum::extract::ConnectInfo;
use axum::routing::get;
use http::header::AUTHORIZATION;
use http::{Method, Request, StatusCode};
use tower::{Layer as _, ServiceExt as _};

use axum_governor::extractor::{Global, Header, PeerIp};
use axum_governor::{
	BodyPreset, BoxedGovernorLayer, GovernorConfigBuilder, GovernorLayer, Quota, nz,
};

fn req(method: Method, path: &str) -> Request<axum::body::Body> {
	Request::builder().method(method).uri(path).body(axum::body::Body::empty()).unwrap()
}

fn req_with_peer_and_auth(
	method: Method,
	path: &str,
	peer: &str,
	token: &str,
) -> Request<axum::body::Body> {
	let addr: SocketAddr = peer.parse().unwrap();
	let mut r = Request::builder()
		.method(method)
		.uri(path)
		.header(AUTHORIZATION, token)
		.body(axum::body::Body::empty())
		.unwrap();
	r.extensions_mut().insert(ConnectInfo::<SocketAddr>(addr));
	r
}

async fn handler() -> &'static str {
	"ok"
}

// ---------------------------------------------------------------------------
// Test 1: basic admit/reject with byte-level header verification
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admit_then_reject_through_router() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);
	let router = Router::new().route("/api", get(handler));

	let r1 = layer.layer(router.clone()).oneshot(req(Method::GET, "/api")).await.unwrap();
	assert_eq!(r1.status(), StatusCode::OK);
	let policy = r1.headers().get("ratelimit-policy").unwrap();
	assert_eq!(policy, "\"default\";q=1;w=1");

	let r2 = layer.layer(router.clone()).oneshot(req(Method::GET, "/api")).await.unwrap();
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
	assert_eq!(r2.headers().get("retry-after").unwrap(), "1");
	assert_eq!(r2.headers().get("x-ratelimit-limit").unwrap(), "1");
	assert_eq!(r2.headers().get("x-ratelimit-remaining").unwrap(), "0");
}

// ---------------------------------------------------------------------------
// Test 2: ConnectInfo missing returns 500 when PeerIp is the extractor
// ---------------------------------------------------------------------------

#[tokio::test]
async fn connect_info_missing_returns_500_through_router() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(PeerIp::default())
		.expect_connect_info()
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);
	let router = Router::new().route("/", get(handler));

	// No ConnectInfo extension in the request => PeerIp extraction fails
	let r = layer.layer(router).oneshot(req(Method::GET, "/")).await.unwrap();
	assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
	assert!(r.headers().get("ratelimit-policy").is_none());

	let bytes = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
	let body = std::str::from_utf8(&bytes).unwrap().to_lowercase();
	assert!(body.contains("connect"), "body: {body}");
}

// ---------------------------------------------------------------------------
// Test 3: problem+json 429 body round-trips correctly via serde_json
// ---------------------------------------------------------------------------

#[tokio::test]
async fn problem_json_reject_body_round_trips() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.body_preset(BodyPreset::ProblemJson)
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);
	let router = Router::new().route("/", get(handler));

	// exhaust the single token
	let _ = layer.layer(router.clone()).oneshot(req(Method::GET, "/")).await.unwrap();
	// trigger 429
	let r = layer.layer(router.clone()).oneshot(req(Method::GET, "/")).await.unwrap();
	assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);

	let ct = r.headers().get("content-type").unwrap().to_str().unwrap();
	assert!(ct.contains("application/problem+json"), "got: {ct}");

	let bytes = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
	let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
	assert_eq!(v["type"], "about:blank");
	assert_eq!(v["status"], 429);
	assert_eq!(v["title"], "Too Many Requests");
	let detail = v["detail"].as_str().unwrap().to_lowercase();
	assert!(detail.contains("retry"), "detail: {detail}");
}

// ---------------------------------------------------------------------------
// Test 4: BoxedGovernorLayer stored in an AppState pattern (Clone, no Arc)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
	rate_limit: BoxedGovernorLayer,
}

#[tokio::test]
async fn boxed_layer_in_app_state_pattern() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.finish()
		.unwrap();
	let state = AppState { rate_limit: BoxedGovernorLayer::from_config(cfg) };
	let router = Router::new().route("/", get(handler));
	let r = state.rate_limit.layer(router).oneshot(req(Method::GET, "/")).await.unwrap();
	assert_eq!(r.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Test 5: test_utils::drive works from outside-the-crate code
// ---------------------------------------------------------------------------

#[tokio::test]
async fn drive_helper_works_from_outside_crate() {
	use axum_governor::test_utils::drive;

	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		.quota_default(Quota::requests_per_second(nz!(1u32)))
		.finish()
		.unwrap();
	let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
	let s1 = drive(&layer, Method::GET, "/", None).await;
	assert_eq!(s1, StatusCode::OK);
	let s2 = drive(&layer, Method::GET, "/", None).await;
	assert_eq!(s2, StatusCode::TOO_MANY_REQUESTS);
}

// ---------------------------------------------------------------------------
// Test 6: stacked reject emits correct policy in ratelimit header
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stacked_reject_emits_correct_policy_in_ratelimit_header() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(Global)
		// High default so primary limiter does not trigger
		.quota_default(Quota::requests_per_second(nz!(100u32)))
		.stack("peer", PeerIp::default(), Quota::requests_per_second(nz!(1u32)))
		.stack("auth", Header(&AUTHORIZATION), Quota::requests_per_minute(nz!(60u32)))
		.expect_connect_info()
		.finish()
		.unwrap();
	let layer = GovernorLayer::new(cfg);
	let router = Router::new().route("/", get(handler));

	let peer = "1.2.3.4:1234";
	let token = "Bearer testtoken";
	// First request: exhausts peer 1/s bucket
	let r1 = layer
		.layer(router.clone())
		.oneshot(req_with_peer_and_auth(Method::GET, "/", peer, token))
		.await
		.unwrap();
	assert_eq!(r1.status(), StatusCode::OK);
	// Second request: peer stack rejects
	let r2 = layer
		.layer(router.clone())
		.oneshot(req_with_peer_and_auth(Method::GET, "/", peer, token))
		.await
		.unwrap();
	assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

	let ratelimit = r2.headers().get("ratelimit").unwrap().to_str().unwrap();
	// t= is the raw wait seconds (may be 0 for sub-second waits); r= is always 0 on reject
	assert!(ratelimit.starts_with("\"peer\";r=0;t="), "ratelimit: {ratelimit}");

	let policy = r2.headers().get("ratelimit-policy").unwrap().to_str().unwrap();
	assert!(policy.contains("\"peer\";q=1;w=1"), "policy: {policy}");
	assert!(policy.contains("\"auth\";q=60;w=60"), "policy: {policy}");
}
