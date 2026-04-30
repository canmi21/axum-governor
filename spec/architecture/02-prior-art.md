# Prior art and gap analysis

This document is the receipt for the choices in [`01-overview.md`](01-overview.md). It
catalogues what `governor`, `tower-governor`, and `actix-governor` do, where they diverge
from what a 2026 Axum middleware should look like, and which patterns we copy from outside
the Rust ecosystem.

## `governor` (the underlying crate)

What we use as-is:

- The GCRA limiter and its `RateLimiter<K, S, C, MW>` parameterisation.
- `Quota::per_*` math (we wrap with renamed constructors, see
  [`04-quota-and-policy.md`](04-quota-and-policy.md)).
- `clock::Clock` and `clock::Reference` traits, `DefaultClock`, `MonotonicClock`,
  `FakeRelativeClock` (re-exported as `MockClock`).
- `state::keyed::DefaultKeyedStateStore<K>` — defaults to `HashMapStateStore`; flipping
  the `dashmap` feature swaps in `DashMapStateStore`. Our default features include
  `dashmap`.
- `RateLimiter::retain_recent`, `shrink_to_fit`, `len`.

What forces a design choice:

- `middleware::NoOpMiddleware` returns `Ok(())` on positive outcomes.
  `middleware::StateInformationMiddleware` returns `Ok(StateSnapshot)`, where
  `StateSnapshot::remaining_burst_capacity()` is the only public source of "how many
  cells remain before the next reject?". Filling IETF `r=` and legacy
  `X-RateLimit-Remaining` requires this snapshot, so v2 mandates
  `StateInformationMiddleware`. The cost is one extra arithmetic step per check, well
  below network jitter; the value is the headers.

What we deliberately do NOT re-export:

- `Quota::per_second(N)` — semantically correct (N cells per second) but textually
  ambiguous against the "seconds per request" reading found in other ecosystems. v2
  exposes `requests_per_second(N)` and `seconds_per_request(N)` as named alternatives.
- `Quota::new(burst, replenish_all_per)` — already deprecated upstream.

## `tower-governor`

Source-level inspection (`src/key_extractor.rs`):

```rust
pub trait KeyExtractor: Clone {
    type Key: Clone + Hash + Eq + Debug;
    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError>;
}
```

Five concrete defects we fix:

| Defect                                                        | Consequence                                                        | Fix in v2                                                                                      |
| ------------------------------------------------------------- | ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| `Clone` bound on the trait                                    | `Box<dyn KeyExtractor>` impossible                                 | Remove the bound; Layer holds `Arc<dyn ...>`.                                                  |
| Generic method `extract<T>`                                   | dyn-incompat even without `Clone`; body type leaks into the trait  | Use `&http::request::Parts` (axum's `FromRequestParts` precedent).                             |
| Single `GovernorError` for all failures                       | Caller cannot distinguish "no key" from "over quota" in one branch | `RejectionReason` enum at the response layer; `ExtractionError` for the extractor side.        |
| No automatic GC                                               | Users must spawn their own retain task or leak keys                | Layer constructor spawns a tokio task; `Drop` aborts it ([`06`](06-runtime-and-lifecycle.md)). |
| Default `PeerIpKeyExtractor` errors out without `ConnectInfo` | Every request becomes 500                                          | Builder requires `expect_connect_info()` ([`06`](06-runtime-and-lifecycle.md)).                |

What we keep from `tower-governor`:

- The `default()` / `secure()` preset idea (burst 8 / 500 ms, burst 2 / 4 s) is
  reasonable enough to copy as named presets on `GovernorConfigBuilder`.
- `SmartIpKeyExtractor` walking `X-Forwarded-For` → `X-Real-IP` → `Forwarded` → peer is
  the right priority order. We tighten it with a trusted-proxy whitelist by default.
- `use_headers()` opt-in for `X-RateLimit-*` was the right instinct; v2 makes it the
  default and adds the IETF form.

## `actix-governor`

Source-level inspection (`src/key_extractor.rs`):

```rust
pub trait KeyExtractor: Clone {
    type Key: Clone + Hash + Eq;
    type KeyExtractionError: ResponseError + 'static;
    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError>;
    fn exceed_rate_limit_response(&self, _: &NotUntil<_>, _: HttpResponseBuilder) -> HttpResponse;
    fn whitelisted_keys(&self) -> Vec<Self::Key> { Vec::new() }
}
```

Defects:

- Same `Clone` and associated-type problems → not object-safe.
- `exceed_rate_limit_response` couples response shaping into the extractor. v2 keeps
  these responsibilities apart: extraction in [`03`](03-key-extraction.md), response in
  [`05`](05-response-and-headers.md).
- `whitelisted_keys` lives on the trait. v2 puts whitelists on the builder so they
  compose with method/path/IP whitelists uniformly ([`04`](04-quota-and-policy.md)).
- Same lack of automatic GC and async support.

Worth borrowing:

- The associated `KeyExtractionError: ResponseError` decoupling — the extractor decides
  what the failure means, not the layer. v2's `ExtractionError` plays the same role.
- `SimpleKeyExtractionError` for users who don't want to define a type — v2 ships an
  equivalent default error type usable inline.

## Borrowed concepts from outside the Rust ecosystem

- **ASP.NET Core `PartitionedRateLimiter::CreateChained`** — chain multiple limiters
  into one, run them in sequence per request. This is the model behind v2's stacked
  limits.
- **Multi-window same-key** (10/s + 1000/m + 100k/d), shipped in `AspNetCoreRateLimit`
  and most API gateways — modeled as sugar over stacked limits
  ([`04`](04-quota-and-policy.md)).
- **Cloudflare's adoption of IETF draft-10 structured fields** — only major vendor on
  the draft as of 2026-04. We follow the wire format but co-emit legacy `X-RateLimit-*`
  since most clients still grep for it ([`05`](05-response-and-headers.md)).
- **AWS API Gateway "Usage Plans"** — tiers (free / pro / enterprise) attached to API
  keys. v2 expresses this as `KeyOutcome::quota_override` rather than separate plan
  registries; the lookup belongs in the extractor (after auth has populated the request
  extension), not in the limiter.
- **Envoy `local_rate_limit` descriptors** — ordered list of `(key, value)` pairs
  forming the bucket key. v2's `Compound(a, b)` combinator is the same idea, type-safe.

## Anti-patterns we explicitly avoid

- Per-request lock around shared state. governor's `DashMapStateStore` already shards;
  we don't add a second layer.
- Building a config inside an `axum::middleware::from_fn` closure (a common
  StackOverflow answer). Each call would construct a fresh limiter. v2's
  `BoxedGovernorLayer` makes the one-config-per-Layer pattern the path of least
  resistance.
- Putting body extraction (e.g. JSON username) into the rate-limit key. Body bytes
  belong downstream of any limiter; the `&Parts` interface enforces this in the type
  system.
