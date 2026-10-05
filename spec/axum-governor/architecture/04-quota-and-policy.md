# Quota and policy

The policy surface is the builder API of `GovernorConfigBuilder`. Every knob below maps
to exactly one builder method that returns `Self` and is `#[must_use]`.

## Constructors

```rust
impl Quota {
    pub const fn requests_per_second(n: NonZeroU32) -> Self;
    pub const fn requests_per_minute(n: NonZeroU32) -> Self;
    pub const fn requests_per_hour(n: NonZeroU32) -> Self;
    pub fn seconds_per_request(n: NonZeroU32) -> Self;
}
```

These are thin wrappers over `governor::Quota::per_second` etc. — same math, unambiguous
names. `seconds_per_request(n)` is the inverse case (one cell per N seconds, burst 1)
expressed without forcing users to construct a `Duration`.

`burst(n)` overrides the implied burst capacity:

```rust
let q = Quota::requests_per_second(nz!(50)).burst(nz!(200));
```

The macro `nz!` is a re-export of `nonzero_ext::nonzero!` — keep one dep, one macro
name.

## Per-method quotas

Within one Layer, distinct quotas per HTTP method:

```rust
GovernorConfigBuilder::default()
    .with_extractor(Global)
    .quota_for(Method::GET,  Quota::requests_per_second(nz!(100)))
    .quota_for(Method::POST, Quota::requests_per_second(nz!(10)))
    .quota_default(Quota::requests_per_second(nz!(50)))   // anything else
    .finish()?;
```

Internally, one keyed `RateLimiter` per method-bucket plus one for the default,
indexed by `Method`. State stores are independent — a `GET` flood does not consume
`POST` budget. This subsumes the "10/s on writes, 100/s on reads" use case without
stacking layers.

## Per-tier override

No builder knob — the override comes from the extractor through
`KeyOutcome::quota_override` (see [`03-key-extraction.md`](03-key-extraction.md)). The
builder fixes the _default_ quota; the extractor can replace it per request.

When `quota_override` is `Some`, the limiter applies the override quota to the check
against the same state store. Bucket state is keyed by `key`, so a tier upgrade between
requests is observed on the next request without touching state. The implementation
maintains a small cache of `Quota -> RateLimiter` wrappers sharing the underlying
store; cold quotas allocate one limiter wrapper, never a state store.

## Stacked limits (cross-key chain)

```rust
GovernorConfigBuilder::default()
    .with_extractor(Global)
    .expect_connect_info()                  // the stacked PeerIp reads the peer too
    .stack("peer", PeerIp::default(),       Quota::requests_per_second(nz!(10)))
    .stack("auth", Header(&AUTHORIZATION),  Quota::requests_per_minute(nz!(600)))
    .finish()?;
```

A stack sits on top of the primary extractor, which is still required. Internally each entry is a
type-erased runner -- its name, its extractor, its quota and its limiter -- held in order in the
layer's shared state. On each request, every
entry is checked in order; the first reject wins and is reported with its own policy
name in `RateLimit:` (see [`05`](05-response-and-headers.md)). The full set of policies
is advertised in `RateLimit-Policy`; only the entry that triggered the reject populates
the live `RateLimit:` counter.

This is the shape ASP.NET Core's `PartitionedRateLimiter::CreateChained` settled on,
and it is what every sufficiently-large API gateway (Envoy, Apache APISIX, Kong) ends
up with. Order matters — put the cheapest extractor first.

## Multi-window same-key sugar

```rust
GovernorConfigBuilder::default()
    .with_extractor(Global)
    .expect_connect_info()
    .quotas("peer", PeerIp::default(), [
        Quota::requests_per_second(nz!(10)),
        Quota::requests_per_minute(nz!(600)),
        Quota::requests_per_hour(nz!(20_000)),
    ])
    .finish()?;
```

Typed shortcut for "stack the same extractor against several quotas". Expands to
multiple `stack(...)` entries with names `peer:1s`, `peer:1m`, `peer:1h`; the only
difference is that the multiple state stores share an extractor and therefore share
the key-allocation cost.

## Whitelists

Three independent whitelist axes — all bypass the limiter entirely on match:

```rust
.whitelist_methods([Method::OPTIONS, Method::HEAD])
.whitelist_paths(["/health", "/metrics", "/internal/*"])      // glob
.whitelist_ips(["127.0.0.0/8".parse()?, "::1/128".parse()?])  // CIDR
```

Precedence is **whitelist beats limit, always**. A request matching any whitelist axis
is admitted with no header writes — we explicitly do _not_ emit `RateLimit:` for
whitelisted requests, since "remaining = ∞" is meaningless and confuses dashboards.

Path matching uses a small glob (`*` matches one segment, `**` matches any) instead of
a full regex engine — keeps the dep tree clean and matches what production gateways
typically support.

## Builder validation

`finish()` returns `Result<GovernorConfig, ConfigError>`. The variants:

- `ConfigError::ZeroBurst` — deprecated and never produced; see
  [`api-stability.md`](../api-stability.md).
- `ConfigError::EmptyChain` — `quotas(...)` was given no quotas.
- `ConfigError::ContradictoryWhitelist` — `whitelist_ips` covers every IPv4 and every IPv6
  address, which turns the limiter off.
- `ConfigError::NoExtractor` — the builder went straight to `finish()` without picking
  an extractor.
- `ConfigError::MissingConnectInfoAcknowledgement` — `PeerIp` / `SmartIp` configured,
  as the primary extractor or in the stack, but `expect_connect_info()` was not called
  ([`06`](06-runtime-and-lifecycle.md)).

The split between `finish()` errors (config-level, recoverable) and the runtime 500
(ConnectInfo missing despite the acknowledgement, [`06`](06-runtime-and-lifecycle.md)) is
deliberate: config errors are something the developer can fix with a different value;
the 500 answers only when the deployment-level acknowledgement was lied about.
