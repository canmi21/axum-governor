# Roadmap

Functional capabilities planned for axum-governor v2 and beyond. Each section links to the
architecture document that justifies the design. Items are scoped per release; non-goals at
the bottom are explicit and durable.

## v2.0

### Key extraction

See [`architecture/03-key-extraction.md`](architecture/03-key-extraction.md).

- `PeerIp` — peer IP from `axum::extract::ConnectInfo`. Default masks IPv6 to /56.
- `SmartIp` — walks `X-Forwarded-For` → `X-Real-IP` → `Forwarded` → peer, gated by a
  trusted-proxy whitelist.
- `Global` — single bucket (`type Key = ()`).
- `Header(name)`, `Cookie(name)`, `Extension<T>` — pull key from named header / cookie /
  request extension.
- `Compound(a, b)` — combines two extractors into a tuple key.
- `KeyExtractor` trait — sync, object-safe, returns `KeyOutcome<K>` carrying an optional
  per-tier quota override.
- `AsyncKeyExtractor` trait — async sibling for extractors that must await.

### Quota and policy

See [`architecture/04-quota-and-policy.md`](architecture/04-quota-and-policy.md).

- `Quota::requests_per_second(N)` / `requests_per_minute(N)` / `requests_per_hour(N)` /
  `seconds_per_request(N)`. The ambiguous `per_second(N)` from governor is not
  re-exported.
- `burst(N)` setter for setting burst capacity above the base rate.
- Per-method quotas — distinct quotas per HTTP method within one Layer.
- Per-tier override via `KeyOutcome::quota_override`, no extra layer needed.
- Stacked limits — multiple `(KeyExtractor, Quota)` pairs in one Layer; first to reject
  wins.
- Same-key multi-window sugar — one extractor, several quotas (`10/s` + `1k/m` + `100k/d`).
- Method / path / IP whitelists bypass the limiter entirely.
- Builder methods are `const fn` where the underlying types allow.

### Response and headers

See [`architecture/05-response-and-headers.md`](architecture/05-response-and-headers.md).

- Default 429 with `Retry-After`, IETF structured-fields `RateLimit:` and `RateLimit-Policy:`
  per draft-ietf-httpapi-ratelimit-headers-10, and legacy `X-RateLimit-Limit` / `-Remaining`
  / `-Reset` (delta-seconds by default; epoch via `legacy_reset_epoch()`).
- Same headers on successful responses, reflecting post-decrement remaining capacity.
- Three body presets selectable on the builder: plain text (default), JSON, RFC 9457
  `application/problem+json`.
- Custom `error_handler: Fn(RejectionReason) -> Response`. `RejectionReason` distinguishes
  quota-exceeded from key-extraction-failed.

### Runtime and lifecycle

See [`architecture/06-runtime-and-lifecycle.md`](architecture/06-runtime-and-lifecycle.md).

- Background tokio task running `RateLimiter::retain_recent` on a configurable interval
  (default 60 s); started by the Layer constructor, opt-out via builder.
- Maximum-keys bound with LRU eviction, off by default.
- Layer `Drop` aborts the GC task automatically.
- Startup acknowledgement — builders that select `PeerIp` or `SmartIp` must call
  `expect_connect_info()` before `finish()`, surfacing the deployment requirement at
  build time instead of as a per-request 500.

### Observability

See [`architecture/06-runtime-and-lifecycle.md`](architecture/06-runtime-and-lifecycle.md).

- `tracing` event on every reject, carrying key, configured quota, and wait duration.
- `tracing` span around middleware execution, opt-out via the `tracing` feature.
- `Limiter::snapshot()` — current key count, rough memory estimate, top-N hottest keys.

### Ergonomics and testing

See [`architecture/07-ergonomics-and-testing.md`](architecture/07-ergonomics-and-testing.md).

- `BoxedGovernorLayer` — type-erased layer; app state holds it without naming `<K>`.
- Builder `.finish()` returns `Result<GovernorConfig, ConfigError>` — misconfiguration
  surfaces at build time.
- All builder methods are `#[must_use]`.
- `MockClock` (re-export of `governor::clock::FakeRelativeClock`) plus
  `axum_governor::test_utils` for deterministic time control in downstream tests.

### Performance

- Hot-path `check` is non-allocating (preserves governor's existing guarantee).

## Deferred

- `metrics` crate integration as an optional feature: `requests_total{outcome}` counter
  and `wait_time_seconds` histogram.
- Explicit `shutdown()` and `drain()` API beyond the existing `Drop` semantics.

## Non-goals

- Redis or any other distributed backend. axum-governor is in-memory only. Users needing
  distributed state should reach for a different middleware.
