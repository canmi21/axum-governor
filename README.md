# Axum Governor

Rate-limiting middleware for [Axum](https://docs.rs/axum) powered by
[Governor](https://docs.rs/governor).

## Quick start

```toml
[dependencies]
axum-governor = "2"
```

```rust
use std::net::SocketAddr;
use axum::{Router, routing::get};
use axum_governor::{GovernorConfigBuilder, GovernorLayer, Quota, nz, extractor::PeerIp};

#[tokio::main]
async fn main() {
    let cfg = GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(Quota::requests_per_second(nz!(50u32)))
        .finish()
        .unwrap();

    let app = Router::new()
        .route("/", get(|| async { "hello" }))
        .layer(GovernorLayer::new(cfg));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
```

See `examples/` for runnable programs covering stacked limits and per-tier overrides.

## Features

- **Flexible key extraction** — object-safe `KeyExtractor` and `AsyncKeyExtractor` traits;
  built-in `PeerIp`, `SmartIp`, `Global`, `Header`, `Extension`, `Cookie`, and `Compound`
  combinators. Custom extractors implement one method on one trait — no `Clone` bound, no
  body-type leakage.
- **Per-method quotas** — attach different limits to GET, POST, or any HTTP method via
  `quota_for(method, quota)`.
- **Stacked / multi-window limits** — ordered chain of named policies, each with its own
  extractor and quota; the first reject wins. `quotas()` expands a slice of quotas into
  labelled entries automatically.
- **Per-tier override** — `KeyOutcome::quota_override` lets an extractor select a different
  quota per request (e.g. free vs pro tier) without separate policy registries.
- **IETF draft-ietf-httpapi-ratelimit-headers** — structured-field `RateLimit:` and
  `RateLimit-Policy:` emitted by default; legacy `X-RateLimit-*` co-emitted for client
  compatibility.
- **Background GC** — periodic `retain_recent` sweep runs in a `Weak`-referenced tokio task;
  `Drop` aborts it automatically. Interval and opt-out are configurable.
- **Type erasure** — `BoxedGovernorLayer` erases the key type parameter for direct use in
  `#[derive(Clone)] struct AppState { rate_limit: BoxedGovernorLayer }`.
- **Observability** — every reject is a `tracing` event; `GovernorLayer::limiter()` returns
  a `LimiterHandle` for live introspection via `snapshot()`.

## Cargo features

| Feature      | Default | Description                                                      |
| ------------ | ------- | ---------------------------------------------------------------- |
| `dashmap`    | yes     | Concurrent `DashMapStateStore` for the per-key limiter cache     |
| `tracing`    | yes     | Per-request span and per-reject tracing event                    |
| `json`       | yes     | `BodyPreset::ProblemJson` for RFC 9457 reject bodies             |
| `test-utils` | no      | `test_utils::drive` and re-exported helpers for downstream tests |

Build without default features (`--no-default-features`) to get a minimal binary that
uses `HashMapStateStore` and emits plain text reject bodies.

## Acknowledgements

This project was inspired by:

- [governor](https://github.com/boinkor-net/governor)
- [actix-governor](https://github.com/AaronErhardt/actix-governor)
- [tower-governor](https://github.com/benwis/tower-governor)

Thanks to the open source community and contributors.

## License

Released under the MIT License © 2025 [Canmi](https://canmi.net)
