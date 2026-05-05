# Runtime, lifecycle, and observability

Three concerns covered together because they share a single resource — the Layer's
inner state — and one piece of infrastructure: the background tokio task that owns GC
and the `Drop` cleanup.

## Background GC

`governor::RateLimiter::retain_recent` walks the keyed state and removes entries
whose last activity is older than the longest replenishment interval in the current
quota set. It is O(n) over keys and is governor's primary built-in mechanism for
controlling keyed-store growth.

v2 starts a tokio task on Layer construction:

```rust
fn spawn_gc(state: Arc<DashMapStateStore<K>>, every: Duration) -> AbortHandle {
    let handle = tokio::spawn(async move {
        let mut tick = tokio::time::interval(every);
        loop {
            tick.tick().await;
            state.retain_recent();
        }
    });
    handle.abort_handle()
}
```

Default interval: 60 seconds. Configurable via `.gc_interval(Duration)`. Disable with
`.gc_disable()`.

The `AbortHandle` lives in `GovernorLayer`. Layer `Drop` calls `abort()` — the task
terminates by the next yield. Users who construct one Layer per request leak a task
per Layer, which is documented as an anti-pattern in
[`07-ergonomics-and-testing.md`](07-ergonomics-and-testing.md); the `BoxedGovernorLayer`
ergonomics aim to make the one-config-per-process pattern the obvious one.

## Key tracker and `max_keys`

`governor` 0.10 does not expose iteration or per-key removal for keyed limiters. That
means axum-governor cannot implement a strict "delete this exact key now" bound over
governor's internal state without replacing governor's state store. v2 therefore uses
a sidecar tracker with explicit, documented limits:

```rust
.max_keys(200_000)   // optional, best-effort shed threshold
```

The sidecar tracks observed keys, hit counts, and a monotonic sequence stamp used as
an approximate LRU. On insertion beyond `max_keys`, the oldest sidecar entry is
evicted, a `WARN`-level `tracing` event names the affected limiter, and
`RateLimiter::retain_recent()` is forced on the underlying governor limiter. This is
best-effort cleanup for governor's own keyed state: fresh entries may remain until
they become stale enough for `retain_recent()` to remove.

When `max_keys` is not configured, the sidecar still has an internal observability
budget so `snapshot().top_n` cannot introduce unbounded memory growth. Evicting from
that budget only affects the `top_n` view; it does not warn and does not change
limiter decisions.

## Startup acknowledgement

The single largest production foot-gun in tower-governor and actix-governor: the user
configures `PeerIp`, forgets `into_make_service_with_connect_info`, and every request
becomes 500 because `ConnectInfo` is missing. v2 catches this at build time by
requiring an explicit acknowledgement on the builder:

```rust
GovernorConfigBuilder::default()
    .with_extractor(PeerIp::default())
    .expect_connect_info()                 // typed acknowledgement
    .quota_default(Quota::requests_per_second(nz!(50)))
    .finish()?;
```

Builders that select `PeerIp` or `SmartIp` and skip `expect_connect_info()` make
`finish()` return `ConfigError::MissingConnectInfoAcknowledgement`. The error message
points to the docs for `into_make_service_with_connect_info`.

The first request that arrives without `ConnectInfo` despite the acknowledgement
still fails — but the failure path is a deterministic 500 with a `tracing::warn!`
naming the extraction error. The library cannot inspect the router topology, so it
cannot do better than a typed acknowledgement; the acknowledgement makes the
responsibility explicit.

## `tracing` integration

Two emission points:

1. **Per-request span** at `DEBUG`, named `axum_governor::layer`, with fields
   `method` and request URI path. Opt-out by disabling the `tracing` feature.
2. **Per-reject event** at `INFO`, with fields `key` (string-formatted), `quota`
   burst, `wait_ms`, and `policy`. By default the key uses `Debug`; callers limiting
   on sensitive values should enable `.redact_keys(true)`, which hashes the key with
   SipHash and renders `hash:<hex>` instead.

The span is _around_ the inner Service call, so downstream tracing fields (status
code, latency) attach to it naturally.

## `Limiter::snapshot()`

Live introspection without scraping logs:

```rust
let snap: LimiterSnapshot = layer.limiter().snapshot();
println!("keys={} bytes~={} top={:?}",
    snap.key_count, snap.approx_bytes, snap.top_n);
```

```rust
pub struct LimiterSnapshot {
    pub key_count: usize,
    pub approx_bytes: usize,
    pub top_n: Vec<(String, u64)>,   // (key debug-format, recent hits)
}
```

`approx_bytes` is `key_count * (key_size + state_size)` plus a constant for keyed
store overhead; documented as approximate. `top_n` defaults to N=10 and is computed
from the sidecar's hit counters, not from governor's internal state. The list is
therefore an observability sample: bounded by `max_keys` when configured and by the
internal tracker budget otherwise.

## Resource ownership summary

| Resource                | Owner                                      | Released by                        |
| ----------------------- | ------------------------------------------ | ---------------------------------- |
| GC tokio task           | `LimiterShared<K>`                         | `Drop` calls `AbortHandle::abort`. |
| Governor keyed store    | `LimiterShared<K>` shared with services    | last shared clone drops.           |
| Sidecar key tracker     | one tracker per limiter in `LimiterShared` | last shared clone drops.           |
| `Arc<dyn KeyExtractor>` | `LimiterShared<K>`                         | last shared clone drops.           |
| `AbortHandle`           | `LimiterShared<K>`                         | dropped with shared state.         |

The Service is `Clone` (Tower requires it); the Layer is the singleton, so cloning
the Service inside Tower's stacking does not duplicate any GC tasks. `GovernorLayer`
is also `Clone`, but clones share the same `LimiterShared`, limiter state, tracker
state, and GC task.
