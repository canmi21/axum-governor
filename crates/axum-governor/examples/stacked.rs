//! Two-policy stack: peer IP at 10 req/s and Authorization header at 600 req/min.
//!
//! A reject from the peer bucket produces:
//!   RateLimit: "peer";r=0;t=1
//!   RateLimit-Policy: "peer";q=10;w=1, "auth";q=600;w=60

use std::net::SocketAddr;

use axum::Router;
use axum::routing::get;
use axum_governor::{
	GovernorConfigBuilder, GovernorLayer, Quota,
	extractor::{Header, PeerIp},
	nz,
};
use http::header::AUTHORIZATION;

#[tokio::main]
async fn main() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(PeerIp::default())
		.expect_connect_info()
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.stack("auth", Header(&AUTHORIZATION), Quota::requests_per_minute(nz!(600u32)))
		.finish()
		.unwrap();

	let app = Router::new().route("/", get(|| async { "hello" })).layer(GovernorLayer::new(cfg));

	let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await.unwrap();
	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
