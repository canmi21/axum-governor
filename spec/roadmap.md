# Roadmap

Every functional capability planned for axum-governor v2 and beyond. Documentation,
examples, and benchmarks are tracked separately. Items are scoped per release; non-goals
at the bottom are explicit and durable.

## v2.0

### Key extraction

- `PeerIp` extractor reading the connecting peer IP from `axum::extract::ConnectInfo`.
  Default masks IPv6 addresses to a /56 prefix to prevent /64 rotation.
- `SmartIp` extractor walking `X-Forwarded-For` → `X-Real-IP` → `Forwarded` → peer.
  Honours a configurable trusted-proxy whitelist; without one, falls back to peer.
- `Global` extractor — single bucket for the whole layer (`type Key = ()`).
- `Header(name)` extractor — derives key from any HTTP header value (e.g. `Authorization`,
  `X-API-Key`).
- `Cookie(name)` extractor — derives key from a named cookie value.
- `Extension<T>` extractor — pulls key from an axum request extension placed by upstream
  middleware (e.g. authenticated user id from an auth layer).
- `Compound(a, b)` combinator — limiter key becomes `(KeyA, KeyB)` for use cases such as
  IP + path.
- `KeyExtractor` trait — object-safe (no `Clone` bound), `extract` returns
  `Result<Key, RejectionReason>`. Type-erasable into `Arc<dyn KeyExtractor>`.
- `AsyncKeyExtractor` trait — parallel async variant for extractors that need to await
  (DB lookup, cache fetch). Distinct trait, not a generic on `KeyExtractor`.

### Quota policy

- `Quota::requests_per_second(N)` / `requests_per_minute(N)` / `requests_per_hour(N)`
  constructors; `seconds_per_request(N)` exists for the inverse case. The
  reverse-meaning `per_second(N)` foot-gun from governor is not re-exported.
- `burst(N)` setter for setting burst capacity above the base rate.
- Per-method quotas — within one Layer, distinct quotas per HTTP method
  (e.g. `GET 100/s` + `POST 10/s`).
- Per-tier quotas — `KeyExtractor` may return `(Key, QuotaOverride)`. When present, the
  override replaces the layer's default quota for that key, powering free/pro tiers
  without stacking layers.
- Stacked limits — one Layer holds multiple `(KeyExtractor, Quota)` pairs and applies all
  of them per request; first to reject wins.
- Method whitelist — listed methods bypass the limiter entirely.
- Path whitelist — listed path patterns bypass the limiter entirely.
- IP whitelist — listed IPs bypass the limiter entirely.
- Builder is `const fn`-compatible where the underlying types allow, so configs can live
  in `const` items.

### Response

- Default 429 response with `Retry-After`, `X-RateLimit-Limit`, `X-RateLimit-Remaining`,
  `X-RateLimit-Reset` headers.
- IETF `RateLimit` and `RateLimit-Policy` headers per draft-ietf-httpapi-ratelimit-headers,
  applied to both successful and rejected responses.
- Custom response closure — `error_handler: Fn(RejectionReason) -> Response`, where
  `RejectionReason` distinguishes "quota exceeded" from "key extraction failed".
- Three default body presets selectable on the builder: plain text, JSON, and
  `application/problem+json` (RFC 7807).

### Lifecycle / runtime

- Background tokio task calling `RateLimiter::retain_recent` on a configurable interval
  (default 60 s); started by the Layer constructor, opt-out via builder.
- Maximum-keys bound with LRU eviction, to cap memory under malicious key floods. Off by
  default, configurable.
- Layer `Drop` aborts the GC task automatically.
- Startup sanity check: if `PeerIp` or `SmartIp` is configured but the axum router was
  not built with `into_make_service_with_connect_info`, the constructor panics with a
  remediation hint, instead of returning HTTP 500 per request.

### Observability

- `tracing` event on every reject, carrying key, configured quota, and wait duration.
- `tracing` span around middleware execution, opt-out via feature flag.
- `Limiter::snapshot()` — returns current key count, rough memory estimate, and the top-N
  hottest keys.

### Ergonomics & safety

- `BoxedGovernorLayer` — type-erased layer for storage in app state, avoids forcing users
  to name `<K, M, RespBody>` generic triples.
- `KeyExtractor` is object-safe (see Key extraction); no `Clone` bound on the trait.
- `MockClock` plus `axum_governor::test_utils` module for deterministic time control in
  downstream tests.
- Builder `.finish()` returns `Result<GovernorConfig, ConfigError>`; misconfigured values
  (zero quota, zero burst, contradictory whitelists) surface at build time rather than at
  runtime panic.
- All builder methods are `#[must_use]`.

### Performance

- Hot-path `check` is non-allocating (preserves governor's existing guarantee).

## deferred

- `metrics` crate integration as an optional feature: `requests_total{outcome}` counter
  and `wait_time_seconds` histogram.
- Explicit `shutdown()` and `drain()` API beyond the existing `Drop` semantics.

## Non-goals

- Redis or any other distributed backend. axum-governor is in-memory only. Users needing
  distributed state should reach for a different middleware.
