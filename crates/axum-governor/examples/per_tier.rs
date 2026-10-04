//! Per-tier quota override: every user gets their own bucket, and the size of the bucket
//! depends on their plan.
//!
//! Auth middleware upstream of the limiter puts a `User` into request extensions. The
//! extractor keys on the user id and lets the tier pick the quota:
//!   Free => the layer default, 10 req/s
//!   Pro  => 1000 req/s via `KeyOutcome::with_quota_override`
//!
//! Try it: `curl -i localhost:3002/` is free tier; add `-H 'x-plan: pro'` for the other.

use axum::Router;
use axum::http::request::Parts;
use axum::routing::get;
use axum_governor::{
	ExtractionError, GovernorConfigBuilder, GovernorLayer, Quota,
	extractor::{KeyExtractor, KeyOutcome},
	nz,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
	Free,
	Pro,
}

#[derive(Clone, Debug)]
struct User {
	id: u64,
	tier: Tier,
}

#[derive(Clone, Copy, Debug)]
struct ByUser;

impl KeyExtractor for ByUser {
	type Key = u64;

	fn extract(&self, parts: &Parts) -> Result<KeyOutcome<u64>, ExtractionError> {
		// Absent only when the auth layer is missing, which is a deployment bug, so it
		// surfaces as a 500 rather than a 400.
		let user = parts
			.extensions
			.get::<User>()
			.ok_or_else(|| ExtractionError::Other("auth middleware did not run".into()))?;
		let outcome = KeyOutcome::new(user.id);
		Ok(match user.tier {
			Tier::Free => outcome,
			Tier::Pro => outcome.with_quota_override(Quota::requests_per_second(nz!(1000u32))),
		})
	}
}

/// Stand-in for real authentication: a header picks the tier, the id is fixed.
async fn fake_auth(
	mut req: axum::extract::Request,
	next: axum::middleware::Next,
) -> axum::response::Response {
	let tier = match req.headers().get("x-plan").and_then(|v| v.to_str().ok()) {
		Some("pro") => Tier::Pro,
		_ => Tier::Free,
	};
	req.extensions_mut().insert(User { id: 42, tier });
	next.run(req).await
}

#[tokio::main]
async fn main() {
	let cfg = GovernorConfigBuilder::default()
		.with_extractor(ByUser)
		.quota_default(Quota::requests_per_second(nz!(10u32)))
		.finish()
		.unwrap();

	// Layers run outermost first, so the limiter is added before auth: auth then runs
	// first on the way in and the limiter sees the User it inserted.
	let app = Router::new()
		.route("/", get(|| async { "hello" }))
		.layer(GovernorLayer::new(cfg))
		.layer(axum::middleware::from_fn(fake_auth));

	let listener = tokio::net::TcpListener::bind("127.0.0.1:3002").await.unwrap();
	axum::serve(listener, app).await.unwrap();
}
