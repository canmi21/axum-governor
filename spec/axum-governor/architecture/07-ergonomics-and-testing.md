# Ergonomics and testing

The pieces that don't fit cleanly into one of the request-path docs: type erasure for
app state, builder validation, and the deterministic-time test infrastructure.

## `BoxedGovernorLayer`

```rust
pub struct BoxedGovernorLayer { /* GovernorLayer<String> */ }

impl BoxedGovernorLayer {
    pub fn from_config<K>(config: GovernorConfig<K>) -> Self where K: /* key bounds */;
    pub fn limiter(&self) -> LimiterHandle<String>;
}
```

It takes the config rather than a built layer because erasure has to wrap the extractor,
and the extractor is only reachable before `GovernorLayer::new` moves it into the shared
state.

Why it exists: a typical app has

```rust
struct AppState {
    rate_limit_layer: GovernorLayer<???>,   // what's K?
}
```

— and `K` infects the entire state struct, every handler that takes `State<AppState>`,
and every test fixture. `BoxedGovernorLayer` erases `K` to a `String` (formatted via
`Debug` on first use). The cost is one allocation and one hash per request; for the
99 % of users who do not have a measured rate-limit hot path, the trade is correct.

Users who are sure they need zero overhead keep `GovernorLayer<K>` and pay the generic
tax everywhere.

## `ConfigError`

Listed in [`04-quota-and-policy.md`](04-quota-and-policy.md). The principle:
`finish()` returns `Result`, never panics. The runtime fault path in
[`06`](06-runtime-and-lifecycle.md) (per-request 500 for missing `ConnectInfo` after
the user has `expect_connect_info()`'d) is the only failure mode the layer surfaces
at runtime; everything else is build-time.

## `#[must_use]`

Every builder method that returns `Self` carries `#[must_use]`. Callers who write

```rust
config.quota_default(Quota::requests_per_second(nz!(50)));   // discarded
```

— a real bug we have seen in production code reviews of similar APIs — get a compiler
warning. The cost is a few attribute lines; the value is catching the highest-frequency
mistake.

## `MockClock`

```rust
pub use governor::clock::FakeRelativeClock as MockClock;
```

Just an alias. governor's `FakeRelativeClock` already has the API we need:

```rust
let clock = MockClock::default();
clock.advance(Duration::from_secs(60));
```

Re-exporting under a non-vendor-name keeps the public surface stable across governor
versions and matches the project's naming rule
([`spec/naming.md`](../naming.md): names describe what the thing does, not which crate
provided it).

Threading a custom `Clock` through `GovernorLayer` would require making `Clock` a
generic parameter on every user-facing type (`GovernorLayer<K, C = DefaultClock>`,
`Governor<S, K, C>`, the builder, the snapshot handle, the limiter cache). That
refactor is **deferred** for v2.0. Users needing deterministic time control should
construct a `governor::RateLimiter` directly with `FakeRelativeClock` for the units
they want to test, rather than going through our Layer. Our own tests use
`tokio::time::pause` + `advance` for time-dependent assertions (see
`src/gc.rs::tests::gc_task_aborts_on_last_arc_drop`).

## `axum_governor::test_utils`

A small module gated by `feature = "test-utils"` (off by default; `cfg(test)` of this
crate enables it implicitly):

```rust
pub use crate::MockClock;

pub struct OkService;                       // inner service: always 200, empty body
pub fn request(method, path) -> Request<Body>;
pub fn request_with_peer(method, path, peer: SocketAddr) -> Request<Body>;
pub async fn drive_response<L: Layer<OkService>>(layer: &L, req) -> Response<Body>;
pub async fn drive(layer: &GovernorLayer<K>, method, path, peer: Option<SocketAddr>) -> StatusCode;
pub async fn drive_boxed(layer: &BoxedGovernorLayer, ..) -> StatusCode;
```

The surface stops at "build a request, push it through the layer, read the response".
Anything that needs a router or a socket is an integration test in `tests/`.

`drive_response` returns the whole response, not a status, and takes any layer. The
first version returned only a `StatusCode`, and the consequence was that every test
that wanted to look at a header (most of them) wrote its own `oneshot` wrapper, so the
crate ended up with five private copies of the same three helpers. A helper that
cannot answer the common question is not used, and then it is not a helper. Being
generic over the layer is what lets `drive_boxed` stop being a second copy of `drive`.

## Test redundancy rule

Already in [`spec/testing.md`](../testing.md). Repeated here in shorthand because it
shapes what we put in `test_utils`:

- We do not re-test `governor`'s GCRA math.
- We do not re-test `axum`'s extractor framework.
- Our tests assert orchestration: that `extract` is called with the right `Parts`,
  that `check_key` is called with the right `Quota`, that the `NotUntil` is mapped
  to the right HTTP response.

`test_utils` is sized for those orchestration assertions, no larger.

## Anti-patterns we want to make hard

- **One config per request.** tower-governor's docs call this out explicitly; v2
  does not enforce it programmatically (the runtime cost of detection is worse than
  the bug), but `BoxedGovernorLayer` plus a one-liner setup in the README make the
  one-config-per-process pattern the path of least resistance.
- **Wall-clock sleep in tests.** `MockClock` is the answer. The test helpers
  deliberately do not export any `tokio::time::sleep` re-export.
- **Spreading `<K>` through app state.** `BoxedGovernorLayer` is the answer.

## `Service::Future` shape

Worth noting because it appears in user-visible signatures (Tower service
composition):

```rust
pin_project! {
    pub struct GovernorFuture<F, E> { #[pin] state: GovernorFutureState<F, E> }
}
// state is one of:
//   Admit  { inner: F, headers: Option<HeaderMap> }   sync path, headers merged on Ready
//   Reject { response: Option<Response> }              sync path, resolved immediately
//   Boxed  { fut: Pin<Box<dyn Future + Send>> }        async-extractor path
```

Only the async-extractor path boxes; a sync extractor pays no allocation for the future.
Both paths hand their extraction result to one `decide()` in `service.rs`, so the
precedence rules (whitelist, per-method, tier override, default, stack) are written once.

`pin-project-lite` over `pin-project` because the future shape is small enough for
the macro-free form, and the proc-macro dep would lose the only place we currently
avoid one.
