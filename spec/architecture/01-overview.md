# Architecture overview

axum-governor v2 is a `tower::Layer` that wraps the [`governor`] crate's GCRA rate-limiter
into a fully-featured, modern Axum middleware. This document fixes the scope, the design
principles, and the layered structure that the rest of `spec/architecture/` decomposes.

[`governor`]: https://docs.rs/governor

## What "modern" means here

Six concrete commitments distinguish v2 from existing wrappers (see
[`02-prior-art.md`](02-prior-art.md) for the full comparison):

1. **Object-safe traits.** `Arc<dyn KeyExtractor<Key = K>>` works. Users do not propagate
   `<K, M, RespBody>` triples through their app state.
2. **Async-aware where it pays.** Sync hot path; a separate `AsyncKeyExtractor` trait for
   extractors that genuinely await (DB tier lookup, cache fetch).
3. **Standards-compliant headers.** IETF draft-10 structured-fields `RateLimit:` and
   `RateLimit-Policy:` by default, with legacy `X-RateLimit-*` co-emitted for client
   compatibility.
4. **Lifecycle that just works.** GC runs in the background by construction; `Drop` cleans
   it up. Misconfiguration is caught at build time, not on the request path.
5. **Production observability.** Every reject is a `tracing` event; the limiter exposes a
   `snapshot()` API for live introspection.
6. **Test ergonomics.** A `test_utils` module removes synthetic-request boilerplate;
   `MockClock` is re-exported for direct use with governor's `RateLimiter`. Threading
   a `Clock` through the Layer itself is deferred (see
   [`07-ergonomics-and-testing.md`](07-ergonomics-and-testing.md)).

## Layered structure

```
+------------------+   +------------------+   +-------------------+
|  GovernorLayer   |-->|  Governor svc    |-->|  RateLimiter (gov)|
|  (tower::Layer)  |   |  (tower::Service)|   |  + State store    |
+------------------+   +------------------+   +-------------------+
        |                       |                        |
        | holds Arc<dyn>        | per-request flow       | check_key()
        v                       v                        v
+------------------+   +------------------+   +-------------------+
|  KeyExtractor /  |   |  RejectionReason |   | StateInformation  |
|  AsyncKeyExtr.   |   |  + error_handler |   | Middleware        |
+------------------+   +------------------+   +-------------------+
        |                       |                        |
        v                       v                        v
   03-key-                 05-response-              06-runtime-
   extraction              and-headers               and-lifecycle
```

Each box maps to a focused architecture document.

## Document map

- [`02-prior-art.md`](02-prior-art.md) — what governor / tower-governor / actix-governor
  get right and wrong, and what we borrow from ASP.NET Core, Envoy, and Cloudflare.
- [`03-key-extraction.md`](03-key-extraction.md) — `KeyExtractor` and `AsyncKeyExtractor`
  trait shapes; built-in extractors; combinators.
- [`04-quota-and-policy.md`](04-quota-and-policy.md) — quota constructors, stacked limits,
  per-method, per-tier, whitelists.
- [`05-response-and-headers.md`](05-response-and-headers.md) — 429 default, IETF draft-10,
  legacy compat, body presets, `error_handler`.
- [`06-runtime-and-lifecycle.md`](06-runtime-and-lifecycle.md) — GC task, max-keys,
  startup acknowledgement, tracing.
- [`07-ergonomics-and-testing.md`](07-ergonomics-and-testing.md) — `BoxedGovernorLayer`,
  `ConfigError`, `MockClock`, `test_utils`.

## Default Cargo features

```toml
default = ["dashmap", "tracing"]
```

`dashmap` is on because every Axum app runs on a multi-threaded tokio runtime; the
`HashMapStateStore` fallback is preserved for `no-default-features` consumers but is not
the recommended path. `tracing` is on because all production Axum stacks use it; turning
it off removes both the span and the per-reject event, with no other consequence.

## Crate dependencies (proposed)

| Crate              | Why                                                             |
| ------------------ | --------------------------------------------------------------- |
| `governor`         | The underlying GCRA limiter and clock abstractions.             |
| `tower`            | `Layer` and `Service` traits.                                   |
| `axum`             | `ConnectInfo` extractor and response types.                     |
| `http`             | `request::Parts`, `HeaderMap`, status codes.                    |
| `pin-project-lite` | `#[pin_project]` on `Service::Future` without a proc-macro dep. |
| `tokio`            | Background GC task; `JoinHandle` and `interval`.                |
| `tracing`          | Optional, on by default; spans and events.                      |
| `dashmap`          | Optional, on by default; concurrent state store.                |
| `serde_json`       | Optional, on by default via `json` feature; for the JSON and    |
|                    | problem+json body presets.                                      |
| `nonzero_ext`      | `nz!` macro for compile-time `NonZeroU32` literals.             |

`pin-project-lite` is preferred over `pin-project` to avoid pulling in a proc-macro on
the hot compile path; the `Service::Future` we project is shaped simply enough for the
lite form to suffice.

## Design rules that bind every doc

1. Anything that can be a build-time error is one (after `finish()`) — but `finish()`
   itself returns `Result<_, ConfigError>` rather than `panic!`. Panics are reserved for
   the `expect_connect_info` acknowledgement case at Layer construction time
   ([`06-runtime-and-lifecycle.md`](06-runtime-and-lifecycle.md)).
2. The hot path allocates only when a header value is being formatted into the response.
3. No re-export of governor's ambiguously-named constructors — every public name in this
   crate is unambiguous in isolation.
4. No `Clone` bound on extension traits. Layer / Service hold `Arc<dyn ...>`.
