//! The limiter as part of application state, with a monitoring endpoint.
//!
//! `BoxedGovernorLayer` erases the key type so `AppState` derives `Clone` without naming
//! `<K>`, and `limiter().snapshot()` exposes the live key count and hottest keys to a
//! dashboard. A custom `error_handler` shows the reject body is fully in the app's hands.

use std::net::SocketAddr;

use axum::extract::State;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use axum::{Router, http::StatusCode};
use axum_governor::{
	BoxedGovernorLayer, GovernorConfigBuilder, Quota, RejectionReason, extractor::PeerIp, nz,
};

#[derive(Clone)]
struct AppState {
	rate_limit: BoxedGovernorLayer,
}

fn reject(reason: RejectionReason) -> Response {
	match reason {
		RejectionReason::QuotaExceeded { wait, policy_name, .. } => (
			StatusCode::TOO_MANY_REQUESTS,
			Json(serde_json::json!({ "policy": &*policy_name, "retry_after_ms": wait.as_millis() })),
		)
			.into_response(),
		RejectionReason::KeyExtractionFailed(err) => {
			(StatusCode::BAD_REQUEST, err.to_string()).into_response()
		}
	}
}

async fn metrics(State(state): State<AppState>) -> Json<serde_json::Value> {
	let snap = state.rate_limit.limiter().snapshot();
	Json(serde_json::json!({
		"keys": snap.key_count,
		"approx_bytes": snap.approx_bytes,
		"top": snap.top_n,
	}))
}

#[tokio::main]
async fn main() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(PeerIp::default())
		.expect_connect_info()
		.quota_default(Quota::requests_per_second(nz!(5u32)))
		.error_handler(reject)
		.finish()
		.unwrap();
	let state = AppState { rate_limit: BoxedGovernorLayer::from_config(cfg) };

	let app = Router::new()
		.route("/", get(|| async { "hello" }))
		.layer(state.rate_limit.clone())
		// Registered after the layer so the dashboard itself is never rate limited.
		.route("/metrics", get(metrics))
		.with_state(state);

	let listener = tokio::net::TcpListener::bind("127.0.0.1:3004").await.unwrap();
	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
