//! Rate-limiting middleware for [`axum`], backed by [`governor`].
//!
//! # Quick start
//!
//! ```rust,no_run
//! use std::net::SocketAddr;
//! use axum::{Router, routing::get};
//! use axum_governor::{GovernorConfigBuilder, GovernorLayer, Quota, nz, extractor::PeerIp};
//!
//! #[tokio::main]
//! async fn main() {
//!     let cfg = GovernorConfigBuilder::default()
//!         .with_extractor(PeerIp::default())
//!         .expect_connect_info()
//!         .quota_default(Quota::requests_per_second(nz!(50u32)))
//!         .finish()
//!         .unwrap();
//!
//!     let app = Router::new()
//!         .route("/", get(|| async { "hello" }))
//!         .layer(GovernorLayer::new(cfg));
//!
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
//!     axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//! # Features
//!
//! - **Key extraction** — sync and async extractors; built-in `PeerIp`, `Global`,
//!   `Header`, `Extension`, `SmartIp`, `Cookie`, `Compound`. See [`extractor`].
//! - **Per-method quotas** — different limits for GET vs POST via
//!   [`GovernorConfigBuilder::quota_for`].
//! - **Stacked limits** — ordered chain of named policies with first-reject-wins
//!   semantics and IETF `RateLimit-Policy` header listing all entries.
//! - **Per-tier override** — extractors return [`extractor::KeyOutcome`] with
//!   `quota_override` for per-request quota selection.
//! - **Background GC** — periodic `retain_recent` sweep driven by a `Weak`-referenced
//!   task; configurable interval and opt-out via [`GovernorConfigBuilder::gc_disable`].
//! - **Type erasure** — [`BoxedGovernorLayer`] collapses the `K` parameter for
//!   use in `#[derive(Clone)] struct AppState`.
//!
//! # Cargo features
//!
//! | Feature | Default | Enables |
//! |---------|---------|---------|
//! | `dashmap` | yes | Lock-free per-tier limiter cache |
//! | `tracing` | yes | Per-request span and per-reject event |
//! | `json` | yes | `BodyPreset::ProblemJson` reject bodies |
//! | `test-utils` | no | `test_utils` helpers for downstream tests |
//!
//! [`axum`]: https://docs.rs/axum
//! [`governor`]: https://docs.rs/governor

#![forbid(unsafe_code)]

pub mod boxed;
pub mod builder;
pub mod error;
pub mod extractor;
pub mod gc;
mod glob;
pub mod headers;
pub mod layer;
mod limiters;
pub mod quota;
pub mod response;
pub mod service;
pub mod snapshot;
mod util;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use crate::boxed::BoxedGovernorLayer;
pub use crate::builder::{GovernorConfig, GovernorConfigBuilder};
pub use crate::error::{ConfigError, ExtractionError, RejectionReason};
pub use crate::extractor::{
	AsyncKeyExtractor, Compound, Cookie, Extension, Global, Header, KeyExtractor, KeyOutcome, PeerIp,
	SmartIp,
};
pub use crate::layer::GovernorLayer;
pub use crate::quota::{Quota, nz};
pub use crate::response::{BodyPreset, ErrorHandler};
pub use crate::snapshot::{LimiterHandle, LimiterSnapshot};
pub use governor::clock::FakeRelativeClock as MockClock;

#[cfg(test)]
mod smoke {
	use super::{ConfigError, ExtractionError, RejectionReason};

	#[test]
	fn error_variants_are_nameable() {
		let _: ExtractionError = ExtractionError::MissingConnectInfo;
		let _: ConfigError = ConfigError::NoExtractor;
	}

	#[test]
	fn rejection_reason_variant_is_nameable() {
		let _: RejectionReason = RejectionReason::KeyExtractionFailed(ExtractionError::UntrustedProxy);
	}
}
