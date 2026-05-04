//! Minimal axum app with per-IP rate limiting at 50 req/s.

use std::net::SocketAddr;

use axum::Router;
use axum::routing::get;
use axum_governor::{GovernorConfigBuilder, GovernorLayer, Quota, extractor::PeerIp, nz};

#[tokio::main]
async fn main() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(PeerIp::default())
		.expect_connect_info()
		.quota_default(Quota::requests_per_second(nz!(50u32)))
		.finish()
		.unwrap();

	let app = Router::new().route("/", get(|| async { "hello" })).layer(GovernorLayer::new(cfg));

	let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
