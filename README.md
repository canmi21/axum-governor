# Axum-Governor

**Axum-Governor** is a rate-limiting middleware for Axum, designed to enforce request limits based on client IP addresses. It is powered by the modern, Rust 2024-compliant, and async-first [`lazy-limit`](https://github.com/canmi21/lazy-limit) library, offering robust performance, flexible configuration, and built-in memory management to prevent crashes under high load. **Note**: Despite the name, `axum-governor` is **not related** to the `governor`, `tower-governor`, or `actix-governor` crates. The name "governor" is used due to historical naming conventions in rate-limiting libraries, where "xxx-governor" typically denotes rate-limiting middleware.

If you are looking for the `governor` crate or its derivatives, please visit their respective repositories:
- [`governor`](https://github.com/boinkor-net/governor)
- [`tower-governor`](https://github.com/benwis/tower-governor)
- [`actix-governor`](https://github.com/AaronErhardt/actix-governor)

## Why Axum-Governor?

`Axum-Governor` leverages the [`lazy-limit`](https://github.com/canmi21/lazy-limit) library, a modern rate-limiting solution built with Rust 2024 and Tokio for asynchronous, high-performance applications. Unlike traditional rate-limiting libraries, `lazy-limit` prioritizes service availability over strict enforcement in extreme conditions. Its core philosophy is **"running is better than crashing."**

### Key Features

- **IP-Based Rate Limiting**: Uses the `real` crate to accurately identify client IP addresses, ensuring reliable rate limiting.
- **Flexible Rules**: Supports global rate limits and route-specific rules, with an override mode to bypass global limits when needed.
- **Memory Management**: Built-in garbage collection (GC) prevents memory overflow by cleaning up stale request records. If memory limits are reached, `lazy-limit` sacrifices strict rate-limiting enforcement to keep the service running, avoiding crashes.
- **Asynchronous Design**: Fully async, built on Tokio for non-blocking performance in high-concurrency environments.
- **Tower Integration**: Implemented as a `tower::Layer`, making it easy to integrate with Axum and other Tower-based frameworks.
- **Configurable**: Customize global limits, route-specific rules, memory usage, and GC intervals.
- **Robust Testing**: Includes comprehensive unit tests and a demo to verify functionality.

### Why Lazy-Limit?

The underlying [`lazy-limit`](https://github.com/canmi21/lazy-limit) library is designed with modern Rust practices and offers several advantages over traditional rate-limiting libraries:

- **Memory Safety**: Configurable memory limits (default: 64MB) and a garbage collector ensure the rate limiter doesn't consume excessive memory, even under heavy load.
- **Graceful Degradation**: In high-concurrency scenarios, if memory limits are approached, `lazy-limit` triggers aggressive cleanup of older records. While this may relax strict rate-limiting for some requests, it ensures the service remains operational, avoiding crashes.
- **Modern Async Design**: Built with Rust 2024 and Tokio, it integrates seamlessly with async ecosystems like Axum.
- **Flexible Configuration**: Supports global and route-specific rules, with an override mode for fine-grained control.

This approach ensures that even in worst-case scenarios, your service has a better chance of staying online rather than crashing due to memory exhaustion.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
axum-governor = "1"
lazy-limit = "1"
tokio = { version = "1", features = ["full"] }
real = { version = "0.1", features = ["axum"] }
axum = "0.8"
tower = "0.5"
http = "1"
tracing = "0.1"

[dev-dependencies]
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

Ensure you have the required dependencies for `lazy-limit` and Tokio.

## Usage

### Step 1: Initialize the Rate Limiter

Before starting your Axum application, initialize the rate limiter using the `lazy_limit::init_rate_limiter!` macro. This sets up global and route-specific rate-limiting rules.

```rust
use lazy_limit::{init_rate_limiter, Duration, RuleConfig};
use tokio::main;

#[tokio::main]
async fn main() {
    init_rate_limiter!(
        default: RuleConfig::new(Duration::seconds(1), 5), // 5 req/s globally
        max_memory: Some(64 * headed4 * 1024), // 64MB max memory
        routes: [
            ("/api/login", RuleConfig::new(Duration::minutes(1), 3)), // 3 req/min
            ("/api/public", RuleConfig::new(Duration::seconds(1), 10)), // 10 req/s
            ("/api/premium", RuleConfig::new(Duration::seconds(1), 20)), // 20 req/s
        ]
    ).await;

    // Your Axum application setup goes here
}
```

### Step 2: Set Up Your Axum Application

Add the `RealIpLayer` and `GovernorLayer` to your Axum router. The `RealIpLayer` must be added **before** the `GovernorLayer` to ensure client IP addresses are available.

```rust
use axum::{Router, routing::get, Server};
use axum_governor::{GovernorLayer, GovernorConfig};
use real::RealIpLayer;
use std::net::SocketAddr;
use tower::ServiceBuilder;

async fn handler() -> &'static str {
    "Hello, world!"
}

#[tokio::main]
async fn main() {
    // Initialize rate limiter (as shown above)
    init_rate_limiter!(
        default: RuleConfig::new(Duration::seconds(1), 5),
        routes: [
            ("/api/login", RuleConfig::new(Duration::minutes(1), 3)),
        ]
    ).await;

    // Create the Axum router
    let app = Router::new()
        .route("/", get(handler))
        .layer(
            ServiceBuilder::new()
                .layer(RealIpLayer::default()) // Extracts the real IP
                .layer(GovernorLayer::default()) // Applies rate limiting
        );

    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
```

### Step 3: Configure Override Mode (Optional)

To ignore global rate limits and apply only route-specific rules, use the `GovernorConfig` with `override_mode` set to `true`:

```rust
let override_limiter = ServiceBuilder::new()
    .layer(RealIpLayer::default())
    .layer(GovernorLayer::new(GovernorConfig::new().override_mode(true)));

let premium_routes = Router::new()
    .route("/api/premium", get(premium_handler))
    .layer(override_limiter);
```

### Step 4: Test Your Application

The included `examples/demo.rs` provides a comprehensive example showcasing various rate-limiting scenarios:

- **Global Rate Limiting**: 5 requests per second across all routes.
- **Route-Specific Rules**: Custom limits, e.g., 3 requests per minute for `/api/login`.
- **Override Mode**: Bypassing global limits for specific routes like `/api/premium`.
- **Multiple IPs**: Independent rate limits for different client IPs.

Run the demo with:

```bash
cargo run --example demo
```

The demo outputs `curl` commands to test rate limits, such as:

```bash
# Test global limit (5 req/s)
for i in {1..6}; do curl -w '%{http_code}\n' http://127.0.0.1:3000/; done

# Test route-specific limit (3 req/min)
for i in {1..4}; do curl -w '%{http_code}\n' http://127.0.0.1:3000/api/login; done
```

## Advantages of Axum-Governor

1. **Seamless Axum Integration**: Designed specifically for Axum, with a `tower::Layer` interface for easy setup.
2. **Modern Async Architecture**: Built on Rust 2024 and Tokio for high performance in async applications.
3. **Robust Memory Management**: Prevents memory overflow with configurable limits and automatic garbage collection.
4. **Flexible Rate Limiting**: Supports global, route-specific, and override modes for maximum control.
5. **Service-First Philosophy**: Prioritizes keeping your service running under load, even if it means relaxing strict rate limits temporarily.
6. **Comprehensive Testing**: Includes a demo and unit tests to ensure reliability.

## Project Structure

```plaintext
axum-governor/
├── examples/
│   └── demo.rs         # Example showcasing rate-limiting scenarios
├── src/
│   ├── config.rs       # Configuration for the rate limiter
│   ├── layer.rs        # Tower Layer implementation
│   ├── lib.rs          # Main library entry point and exports
│   ├── middleware.rs   # Rate-limiting middleware logic
├── Cargo.toml          # Project metadata and dependencies
├── LICENSE             # MIT License
├── README.md           # This file
```

## Configuration Options

- **Global Rate Limit**: Set a default limit for all requests using `lazy_limit::RuleConfig`.
- **Route-Specific Rules**: Define custom limits for specific routes in the `init_rate_limiter!` macro.
- **Override Mode**: Enable `override_mode` in `GovernorConfig` to ignore global limits for specific routes.
- **Memory Limits**: Configure maximum memory usage (default: 64MB) to prevent excessive memory consumption.
- **Garbage Collection**: Automatically cleans up stale records to maintain performance.

Example configuration:

```rust
init_rate_limiter!(
    default: RuleConfig::new(Duration::seconds(1), 5),
    max_memory: Some(32 * 1024 * 1024), // 32MB
    routes: [
        ("/api/login", RuleConfig::new(Duration::minutes(1), 3)),
    ]
).await;
```

## Testing

Run the included tests to verify functionality:

```bash
cargo test --all
```

The tests cover:
- Basic rate limiting
- Route-specific rules
- Override mode
- Multiple client IPs
- Long-interval rules

## Limitations

- **Single Initialization**: The rate limiter can only be initialized once. Multiple calls to `init_rate_limiter!` will panic.
- **Static Rules**: Rate-limiting rules are set at initialization and cannot be modified at runtime.
- **Dependency on RealIpLayer**: The `RealIpLayer` must be added before `GovernorLayer` to provide client IP addresses.
- **Memory Estimation**: Memory usage calculations are approximate and may vary based on the Rust allocator.

## Contributing

Contributions are welcome! Please submit issues or pull requests to the [GitHub repository](https://github.com/canmi21/axum-governor).

1. Fork the repository.
2. Create a new branch (`git checkout -b feature/your-feature`).
3. Make your changes and commit (`git commit -m "Add your feature"`).
4. Push to the branch (`git push origin feature/your-feature`).
5. Open a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contact

For questions or support, please open an issue on the [GitHub repository](https://github.com/canmi21/axum-governor).

## Acknowledgments

`Axum-Governor` is built on top of the excellent [`lazy-limit`](https://github.com/canmi21/lazy-limit) library, which provides the core rate-limiting functionality. Thanks to the Rust and Axum communities for their contributions to the ecosystem!
