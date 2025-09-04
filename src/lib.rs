/* src/lib.rs */

//! # Axum Governor
//!
//! A rate-limiting middleware for Axum, powered by `lazy-limit` and `real`.
//!
//! This crate provides a simple and configurable Tower layer to enforce rate limits
//! on your Axum application based on the client's real IP address.
//!
//! ## Features
//!
//! - **IP-Based Limiting**: Uses the `real` crate to accurately identify the client's IP address.
//! - **Flexible Rules**: Leverages `lazy-limit` to support global and route-specific rate limits.
//! - **Two Modes**: Supports both standard mode (respecting global and route rules) and override mode (ignoring global rules).
//! - **Easy Integration**: Implemented as a standard Tower `Layer`.
//!
//! ## Quick Start
//!
//! 1.  **Add Dependencies**:
//!
//!     ```toml
//!     [dependencies]
//!     axum-governor = "0.1.0"
//!     lazy-limit = "1"
//!     tokio = { version = "1", features = ["full"] }
//!     real = { version = "0.1", features = ["axum"] }
//!     ```
//!
//! 2.  **Initialize the Rate Limiter**:
//!
//!     Before starting your application, initialize `lazy-limit` with your desired rules.
//!
//!     ```rust
//!     use lazy_limit::{init_rate_limiter, Duration, RuleConfig};
//!
//!     #[tokio::main]
//!     async fn main() {
//!         init_rate_limiter!(
//!             default: RuleConfig::new(Duration::seconds(1), 5), // 5 req/s globally
//!             routes: [
//!                 ("/api/special", RuleConfig::new(Duration::seconds(1), 10)),
//!             ]
//!         ).await;
//!
//!         // ... your Axum app setup
//!     }
//!     ```
//!
//! 3.  **Add Layers to Your Router**:
//!
//!     The `GovernorLayer` requires the `RealIpLayer` to be present. Always add `RealIpLayer` first.
//!
//!     ```rust
//!     # use axum::{Router, routing::get, Server};
//!     # use axum_governor::GovernorLayer;
//!     # use real::RealIpLayer;
//!     # use std::net::SocketAddr;
//!     # async fn handler() -> &'static str { "Hello!" }
//!     # async {
//!     let app = Router::new()
//!         .route("/", get(handler))
//!         .layer(
//!             tower::ServiceBuilder::new()
//!                 .layer(RealIpLayer::default()) // Extracts the real IP
//!                 .layer(GovernorLayer::default())   // Applies rate limiting
//!         );
//!
//!     let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
//!     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
//!     axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
//!         .await
//!         .unwrap();
//!     # };
//!     ```

// Public exports
pub use config::GovernorConfig;
pub use layer::GovernorLayer;
pub use middleware::GovernorMiddleware;

// Module declarations
mod config;
mod layer;
mod middleware;
