# Runtime, lifecycle, and observability

Three concerns covered together because they share a single resource — the Layer's
inner state — and one piece of infrastructure: the background tokio task that owns GC
and the `Drop` cleanup.

## Background GC

`governor::RateLimiter::retain_recent` walks the keyed state and removes entries
whose last activity is older than the longest replenishment interval in the current
quota set. It is O(n) over keys and is the only mechanism that prevents unbounded
memory growth under malicious key floods.

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

## Maximum keys with LRU

Even with `retain_recent`, a malicious key flood within one GC interval can grow the
state to OOM. v2 adds an upper bound:

```rust
.max_keys(200_000)   // off by default
```

Implementation: a thin wrapper over the chosen state store that tracks an LRU queue
per shard. On insertion beyond `max_keys`, the LRU shard's tail is evicted. The
eviction emits a `tracing` event at `WARN` so operators see floods in dashboards
rather than as latency spikes.

This is off by default because (a) most apps never approach the bound, (b) the LRU
adds a small constant factor to insert, and (c) picking a number is a deployment
concern.

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
still fails — but the failure path is a deterministic 500 with a `tracing::error!`
naming the request line and the same docs URL. The library cannot inspect the router
topology, so it cannot do better than a typed acknowledgement; the acknowledgement
makes the responsibility explicit.

## `tracing` integration

Two emission points:

1. **Per-request span** at `DEBUG`, named `axum_governor::layer`, with fields
   `method`, `path` (template, not raw), `outcome` (`admit` / `reject`), `policy`
   (when stacked). Opt-out by disabling the `tracing` feature.
2. **Per-reject event** at `INFO`, with fields `key` (string-formatted), `quota`
   (`{n}/{period}`), `wait_ms`, `policy`. The `key` is _opaque-debug-formatted_ — IPs
   render as `1.2.3.4`, headers render as `<header:authorization=...>` with the value
   truncated to the first 8 bytes to avoid leaking secrets verbatim into logs. A
   `.redact_keys(true)` switch hashes the key with SipHash and renders the hex
   instead.

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

`approx_bytes` is `key_count * (key_size + state_size)` plus a constant for DashMap
overhead; documented as approximate. `top_n` defaults to N=10 and is computed by
walking shards and tracking the most-recent-tat values — O(n) but only on demand.

## Resource ownership summary

| Resource                | Owner                                 | Released by                        |
| ----------------------- | ------------------------------------- | ---------------------------------- |
| GC tokio task           | `GovernorLayer<K>`                    | `Drop` calls `AbortHandle::abort`. |
| State store DashMap     | `Arc<...>` shared with cloned Service | last `Service` clone drops.        |
| `Arc<dyn KeyExtractor>` | `GovernorLayer`                       | last clone drops.                  |
| `AbortHandle`           | `GovernorLayer`                       | dropped with the Layer.            |

The Service is `Clone` (Tower requires it); the Layer is the singleton, so cloning
the Service inside Tower's stacking does not duplicate any GC tasks. We do not
implement `Clone` on `GovernorLayer` — users who think they need to clone the Layer
should instead clone the `BoxedGovernorLayer` or share through `Arc`.
