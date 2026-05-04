//! Per-tier quota override via a custom KeyExtractor.
//!
//! A Tier extension (normally set by auth middleware) drives the quota:
//!   Free => 10 req/s
//!   Pro  => 1000 req/s
//!
//! Upstream middleware would inject the tier into request extensions, e.g.:
//!
//!   req.extensions_mut().insert(Tier::Pro);

use std::net::SocketAddr;

use axum::Router;
use axum::routing::get;
use axum_governor::{
	ExtractionError, GovernorConfigBuilder, GovernorLayer, Quota,
	extractor::{KeyExtractor, KeyOutcome},
	nz,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tier {
	Free,
	Pro,
}

#[derive(Clone, Debug, Default)]
struct TierExtractor;

impl KeyExtractor for TierExtractor {
	type Key = String;

	fn extract(&self, parts: &http::request::Parts) -> Result<KeyOutcome<String>, ExtractionError> {
		// In production this would come from a validated JWT or session.
		let tier = parts.extensions.get::<Tier>().cloned().unwrap_or(Tier::Free);
		let quota_override = match tier {
			Tier::Free => None,
			Tier::Pro => Some(Quota::requests_per_second(nz!(1000u32))),
		};
		// Key is the tier name; real apps would use user_id here.
		Ok(KeyOutcome { key: format!("{tier:?}"), quota_override })
	}
}

#[tokio::main]
async fn main() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(TierExtractor)
		.quota_default(Quota::requests_per_second(nz!(10u32))) // Free tier default
		.finish()
		.unwrap();

	let app = Router::new().route("/", get(|| async { "hello" })).layer(GovernorLayer::new(cfg));

	let listener = tokio::net::TcpListener::bind("127.0.0.1:3002").await.unwrap();
	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
