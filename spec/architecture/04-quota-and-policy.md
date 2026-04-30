# Quota and policy

The policy surface is the builder API of `GovernorConfigBuilder`. Every knob below maps
to exactly one builder method that returns `Self`, is `#[must_use]`, and (where the
underlying operation allows) is `const fn`.

## Constructors

```rust
impl Quota {
    pub const fn requests_per_second(n: NonZeroU32) -> Self;
    pub const fn requests_per_minute(n: NonZeroU32) -> Self;
    pub const fn requests_per_hour(n: NonZeroU32) -> Self;
    pub const fn seconds_per_request(n: NonZeroU32) -> Self;
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
    .stack("peer", PeerIp::default(),       Quota::requests_per_second(nz!(10)))
    .stack("auth", Header(&AUTHORIZATION),  Quota::requests_per_minute(nz!(600)))
    .finish()?;
```

Internally `Vec<(name, KeyExtractorKind, Quota, RateLimiter)>`. On each request, every
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

- `ConfigError::ZeroBurst` — burst override of zero.
- `ConfigError::EmptyChain` — `stack(...)` was called once with no entries.
- `ConfigError::ContradictoryWhitelist` — e.g. an IP is in `whitelist_ips` and is also
  the only key the configured extractor could produce.
- `ConfigError::NoExtractor` — the builder went straight to `finish()` without picking
  an extractor.
- `ConfigError::MissingConnectInfoAcknowledgement` — `PeerIp` / `SmartIp` configured
  but `expect_connect_info()` was not called
  ([`06`](06-runtime-and-lifecycle.md)).

The split between `finish()` errors (config-level, recoverable) and the construction
panic (ConnectInfo missing at runtime, [`06`](06-runtime-and-lifecycle.md)) is
deliberate: config errors are something the developer can fix with a different value;
the runtime panic fires only when the deployment-level acknowledgement was lied about.

## `const fn` reach

Every builder method that does not allocate is `const fn`:

```rust
const CONFIG: GovernorConfig = GovernorConfigBuilder::default()
    .quota_default(Quota::requests_per_second(nz!(50)))
    .const_finish();
```

`const_finish` is the panicking variant of `finish` for const contexts; it inherits the
same validation but turns errors into compile-time failures. Builder methods that take
heap-allocated arguments (`whitelist_paths`, `stack` with custom extractors) are not
`const fn`, but their absence does not block configurations that fit in const space.
