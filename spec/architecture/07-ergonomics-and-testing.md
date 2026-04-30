# Ergonomics and testing

The pieces that don't fit cleanly into one of the request-path docs: type erasure for
app state, builder validation, and the deterministic-time test infrastructure.

## `BoxedGovernorLayer`

```rust
pub struct BoxedGovernorLayer(/* private */);

impl<K> GovernorLayer<K> {
    pub fn boxed(self) -> BoxedGovernorLayer where K: 'static + Send + Sync;
}
```

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
provided it). The Layer accepts a `Clock` impl through the builder:

```rust
.with_clock(MockClock::default())
```

## `axum_governor::test_utils`

A small module gated by `feature = "test-utils"` (off by default; `cfg(test)` of this
crate enables it implicitly):

```rust
pub use crate::MockClock;
pub use axum::body::Body;
pub use http::{Method, Request, StatusCode};

pub fn drive(
    layer: &GovernorLayer<...>,
    method: Method,
    path: &str,
    peer: Option<SocketAddr>,
) -> StatusCode { /* ... */ }

pub fn fast_forward(clock: &MockClock, by: Duration) -> StateSnapshot { /* ... */ }
```

The test utility surface deliberately stops at "send a synthetic request and read
back the StateSnapshot". Anything more is a real integration test (`tests/`); the
helper exists to remove `tower::Service::oneshot` boilerplate from unit tests.

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
use pin_project_lite::pin_project;

pin_project! {
    pub struct GovernorFuture<F> {
        #[pin] inner: F,
        // pre-computed header data so the future allocates nothing once polled
    }
}
```

`pin-project-lite` over `pin-project` because the future shape is small enough for
the macro-free form, and the proc-macro dep would lose the only place we currently
avoid one.
