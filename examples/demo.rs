/* examples/demo.rs */
//! Demo for `axum-governor` showcasing various rate-limiting scenarios.

use axum::{
    extract::Path,
    routing::{get, post},
    Router,
};
use axum_governor::{GovernorConfig, GovernorLayer};
use lazy_limit::{init_rate_limiter, Duration, HttpMethod, RuleConfig};
use real::RealIpLayer;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// --- Handlers ---
async fn root_handler() -> &'static str {
    "Welcome! This endpoint has a global limit of 5 req/s."
}

async fn public_api_handler() -> &'static str {
    "Public API. Effective limit is min(global, route) = min(5, 10) = 5 req/s."
}

async fn premium_api_handler() -> &'static str {
    "Premium API (Override Mode). Limit is 20 req/s, ignoring the global limit."
}

async fn login_handler() -> &'static str {
    "Login endpoint. Limit is 3 req/min."
}

async fn prefix_handler(Path(test): Path<String>) -> String {
    format!("Prefix route. Param: {}", test)
}

async fn contact_handler() -> &'static str {
    "Contact endpoint (POST). Limit is 5 req/s."
}

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Initializing rate limiter with rules...");

    // Initialize the global rate limiter using lazy-limit
    init_rate_limiter!(
        default: RuleConfig::new(Duration::seconds(1), 5), // 5 req/s global default
        max_memory: Some(64 * 1024 * 1024), // 64MB max memory
        routes: [
            ("/api/login", RuleConfig::new(Duration::minutes(1), 3)),  // 3 req/min
            ("/api/public", RuleConfig::new(Duration::seconds(1), 10)), // 10 req/s
            ("/api/premium", RuleConfig::new(Duration::seconds(1), 20)), // 20 req/s
            ("/api/prefix/", RuleConfig::new(Duration::seconds(1), 6).match_prefix(true)), // 6 req/s for route prefix
            ("/api/contact", RuleConfig::new(Duration::seconds(1), 7).for_methods(vec![HttpMethod::POST])), // 6 req/s for HTTP Method
        ]
    )
    .await;

    info!("Rate limiter initialized.");

    // --- Layer Configurations ---

    // Default layer: respects both global and route-specific rules.
    let default_limiter = ServiceBuilder::new()
        .layer(RealIpLayer::default())
        .layer(GovernorLayer::default());

    // Override layer: ignores global rules, only applies route-specific rules.
    let override_limiter =
        ServiceBuilder::new()
            .layer(RealIpLayer::default())
            .layer(GovernorLayer::new(
                GovernorConfig::new().override_mode(true),
            ));

    // --- Router Definitions ---

    // Routes with default rate limiting
    let default_routes = Router::new()
        .route("/", get(root_handler))
        .route("/api/public", get(public_api_handler))
        .route("/api/login", get(login_handler))
        .route("/api/prefix/{test}", get(prefix_handler))
        .route("/api/contact", post(contact_handler))
        .layer(default_limiter);

    // Routes with override rate limiting
    let premium_routes = Router::new()
        .route("/api/premium", get(premium_api_handler))
        .layer(override_limiter);

    // Combine all routers into a single app
    let app = Router::new().merge(default_routes).merge(premium_routes);

    // --- Server Startup ---
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server listening on http://{}", addr);

    // --- Instructions for Testing ---
    println!("\n--- Server is running! Try these curl commands to test rate limits: ---\n");
    println!("1. Test Global Limit (5 req/s). Sixth request should fail:");
    println!("   for i in {{1..6}}; do curl -w '%{{http_code}}\\n' http://127.0.0.1:3000/; done\n");

    println!("2. Test Route-Specific Limit (effective 5 req/s). Sixth request should fail:");
    println!(
        "   for i in {{1..6}}; do curl -w '%{{http_code}}\\n' http://127.0.0.1:3000/api/public; done\n"
    );

    println!("3. Test Override Mode (20 req/s). 21st request should fail:");
    println!(
        "   for i in {{1..21}}; do curl -w '%{{http_code}}\\n' http://127.0.0.1:3000/api/premium; done\n"
    );

    println!("4. Test Long Interval Limit (3 req/min). Fourth request should fail:");
    println!(
        "   for i in {{1..4}}; do curl -w '%{{http_code}}\\n' http://127.0.0.1:3000/api/login; done\n"
    );

    println!("5. Test with a different IP using X-Real-IP header:");
    println!(
        "   for i in {{1..6}}; do curl -H 'X-Real-IP: 2.2.2.2' -w '%{{http_code}}\\n' http://127.0.0.1:3000/; done\n"
    );

    println!("6. Test Prefix Route (effective 6 req/s). Seventh request should fail:");
    println!(
        "   for i in {{1..7}}; do curl -w '%{{http_code}}\\n' http://127.0.0.1:3000/api/prefix/test; done\n"
    );

    println!("7. Test Contact POST (7 req/s). Eighth should fail:");
    println!(
        "   for i in {{1..8}}; do curl -X POST -w '%{{http_code}}\\n' http://127.0.0.1:3000/api/contact; done\n"
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
