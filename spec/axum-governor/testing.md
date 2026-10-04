# Testing

## Local dev environment

Local development happens on **`aarch64-apple-darwin`** (macOS arm64) exclusively. `mise run verify` is the gate: type check, clippy with warnings denied, and the test suite. It is what `mise run check governor` dispatches to from the workspace.

**Verify runs clippy and the tests at both feature extremes**, `--all-features` and `--no-default-features`. Every optional feature here has a `not(feature)` twin (the tracing no-ops, the mutex-backed `LimiterCache`, the text-only `BodyPreset`) that only compiles when the feature is off, so a gate that only enables everything never looks at half the cfg branches. The first run with defaults off found a clippy error that had been sitting in the mutex cache for as long as it existed. Intermediate combinations are not run; the two extremes cover every cfg branch once, and the middle only adds build time.

Unit tests live beside their code in `#[cfg(test)] mod tests` blocks; integration tests live in the top-level `tests/` directory.

## Coverage target

95 % line coverage on tested modules. This is a floor, not a ceiling. I/O-heavy code may fall below when the uncovered branches are genuinely error paths with no observable behaviour — document the exemption in-module.

A function C that orchestrates tested functions A and B is covered by testing C's orchestration (call order, short-circuits, data threading) — not by re-testing A's and B's internal branches through C. The 95 % target is satisfied when every C-level branch runs in some test, not when C's tests cover every leaf A and B could reach.

## What to cover

Every public function gets a unit test. Each test module must cover:

- **All correct paths** — every branch that produces a valid result.
- **One error / edge path** — a single representative bad-input case. Exhaustive negative testing is not worth the maintenance cost.

## Redundancy rule

If function C orchestrates functions A and B, and A / B each have their own tests:

- C's tests cover **orchestration logic only** — call order, data threading, short-circuit behaviour.
- C's tests do **not** re-verify A's or B's business logic.

Duplication between layers makes refactors painful and signals nothing useful.

### Anti-over-testing — trust upstream

Do not re-verify behaviour that belongs to a dependency:

- **Do not re-test `governor`'s GCRA math.** Our limiter tests assert that we call `check_key` with the right `Quota` and translate the `NotUntil` error into the right HTTP response. We do not re-prove that GCRA admits the correct number of bursts at boundary conditions — that is `governor`'s test suite.
- **Do not re-test `axum`'s extractor framework.** Our `KeyExtractor` tests verify that `extract` returns the right `Key` for a given `Request`; we do not assert how axum dispatches handlers or threads request extensions.
- **Do not re-test `tokio` runtime mechanics.** Assert observable outcomes (a value lands in a channel, a `JoinHandle` finishes, a GC pass shrinks the key set); do not count `spawn` calls.
- **Do not re-test `serde`** when used for config types — assert our own type round-trips for one representative case, not every malformed-input variant.

## Timing and readiness

A `tokio::time::sleep(N)` used as a happens-before barrier between an async producer and consumer is flake fuel — `N` rarely covers the worst case under parallel load. Gate on observable state.

For tests that depend on time (rate-limit windows, GC cadence), use deterministic time control rather than wall-clock sleeps. Our own GC tests use `tokio::time::pause` + `advance` (see `src/gc.rs`); downstream tests of governor-level math can use `governor::clock::FakeRelativeClock`, re-exported as `axum_governor::MockClock`. Threading a `Clock` through our `GovernorLayer` is deferred for v2.0 — see [`spec/architecture/07-ergonomics-and-testing.md`](architecture/07-ergonomics-and-testing.md).

**The one place a real sleep is correct: the GC sweep test.** Tokio's paused clock drives the interval, but governor decides staleness on the wall clock, so a test that wants to see `retain_recent` drop a key has to let a few real milliseconds pass before advancing tokio time. It is a `std::thread::sleep` on purpose, small, and with a comment naming the two clocks; a `tokio::time::sleep` there would advance nothing governor can see. The other trap in that test: the GC task registers its interval on its first poll, so the test yields once after building the layer, or advancing the clock fires nothing.

Layer-level tests, in-crate or downstream, go through `test_utils` (`drive_response` and the request builders) rather than a private `oneshot` wrapper. See [`architecture/07`](architecture/07-ergonomics-and-testing.md) for why the helper returns the whole response.

When a wall-clock loop is genuinely needed, the trigger-loop idiom — `Option<Instant>` avoids `clippy::unchecked_time_subtraction` and fires on the first iteration:

```rust
let deadline = Instant::now() + Duration::from_secs(5);
let mut last_trigger: Option<Instant> = None;
while Instant::now() < deadline {
	if last_trigger.is_none_or(|t| t.elapsed() >= Duration::from_millis(200)) {
		fire_trigger();
		last_trigger = Some(Instant::now());
	}
	if observed() { break; }
	tokio::time::sleep(Duration::from_millis(50)).await;
}
```

Short sleeps **inside** a polling loop are fine — they are a backoff, not a barrier.

## Test types

| Type        | When to use                                 | Location                         |
| ----------- | ------------------------------------------- | -------------------------------- |
| Unit        | Pure functions, key extraction logic        | `#[cfg(test)] mod tests` in-file |
| Integration | Layer-level behaviour, public API contracts | top-level `tests/` directory     |

Start with unit tests. Introduce integration tests when a feature needs end-to-end verification through the Layer — don't pre-build harnesses for behaviour that doesn't yet exist.
