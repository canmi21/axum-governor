//! Behind a reverse proxy: key on the client address the proxy reports, but only when the
//! request really came from the proxy, and leave the health check alone.
//!
//! `SmartIp` walks `X-Forwarded-For`, `X-Real-IP` and `Forwarded` only when the peer is in
//! the trusted list. A request arriving from anywhere else with those headers set is
//! rejected with 400 rather than trusted, because a spoofed header would otherwise let a
//! client choose its own bucket.

use std::net::SocketAddr;

use axum::Router;
use axum::routing::get;
use axum_governor::{
	BodyPreset, GovernorConfigBuilder, GovernorLayer, Quota, extractor::SmartIp, nz,
};
use ipnet::IpNet;

#[tokio::main]
async fn main() {
	let proxies: Vec<IpNet> =
		["10.0.0.0/8", "127.0.0.0/8"].iter().map(|n| n.parse().unwrap()).collect();

	let cfg = GovernorConfigBuilder::default()
		.with_extractor(SmartIp::new().with_trusted_proxies(proxies))
		.expect_connect_info()
		.quota_default(Quota::requests_per_second(nz!(20u32)))
		.whitelist_paths(["/health", "/internal/**"])
		.body_preset(BodyPreset::ProblemJson)
		.finish()
		.unwrap();

	let app = Router::new()
		.route("/", get(|| async { "hello" }))
		.route("/health", get(|| async { "ok" }))
		.layer(GovernorLayer::new(cfg));

	let listener = tokio::net::TcpListener::bind("127.0.0.1:3003").await.unwrap();
	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
