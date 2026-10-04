# Key extraction

Two object-safe traits, a `KeyOutcome` carrier, an `ExtractionError`, and a fixed set of
built-in extractors. The Layer holds `Arc<dyn KeyExtractor<Key = K>>` (or the async
counterpart), so user code names `K` and not the concrete extractor.

## The traits

```rust
use http::request::Parts;
use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;

pub struct KeyOutcome<K> {
    pub key: K,
    pub quota_override: Option<crate::Quota>,
}

#[derive(Debug)]
pub enum ExtractionError {
    MissingConnectInfo,
    MissingHeader(&'static str),
    MalformedHeader(&'static str),
    UntrustedProxy,
    Other(Box<dyn std::error::Error + Send + Sync>),
}

pub trait KeyExtractor: Send + Sync + 'static {
    type Key: Hash + Eq + Clone + Debug + Send + Sync + 'static;
    fn extract(&self, parts: &Parts)
        -> Result<KeyOutcome<Self::Key>, ExtractionError>;
}

pub trait AsyncKeyExtractor: Send + Sync + 'static {
    type Key: Hash + Eq + Clone + Debug + Send + Sync + 'static;
    fn extract<'a>(&'a self, parts: &'a Parts)
        -> Pin<Box<
            dyn Future<Output = Result<KeyOutcome<Self::Key>, ExtractionError>>
                + Send + 'a
        >>;
}
```

Why these shapes (the [`02`](02-prior-art.md) gap table is the short version):

- **No `Clone` bound.** `Clone` belongs on the wrapping `Service`, not on a predicate
  trait. The Layer stores the extractor as `Arc<dyn ...>`, which is itself trivially
  cloneable. tower-governor and actix-governor both got this wrong.
- **`&Parts`, not `&Request<B>`.** Axum's `FromRequestParts` uses the same input. It
  removes the body-type generic that breaks dyn-safety and forces correct middleware
  ordering — limiters run before any body extractor.
- **Hand-rolled `Pin<Box<dyn Future>>` for the async trait.** AFIT (`async fn` in trait,
  stable since 1.75) is **not** dyn-compatible in 1.95. The hand-rolled form is exactly
  what `async-trait` would expand to; we avoid the proc-macro to keep build times tight.
- **Two traits, not one async trait.** Roughly 99 % of extractors (peer IP, header,
  cookie, connect-info) are sync. Forcing them through a `Pin<Box<dyn Future>>`
  allocates per request on the hot path with zero benefit.
- **`KeyOutcome<K>` carries the per-tier override.** A free / pro / enterprise lookup
  that has already happened in an upstream auth middleware can be pulled from
  `parts.extensions` and turned into a `quota_override` here, with no extra trait.

## Built-in extractors

All of these implement `KeyExtractor` (sync) unless noted.

- **`PeerIp`** — `IpAddr` from the `axum::extract::ConnectInfo<SocketAddr>` extension.
  Returns `MissingConnectInfo` if the extension is absent (the Layer constructor will
  have refused this case via `expect_connect_info`; this is per-request defense in
  depth). IPv6 addresses are masked to a `/56` prefix by default — `/64` is the typical
  rotation budget so anything smaller is the right floor — configurable via
  `PeerIp::ipv6_prefix(u8)`.
- **`SmartIp`** — header walk in priority order: `X-Forwarded-For` → `X-Real-IP` →
  `Forwarded` (`for=`) → peer. Honors a configurable trusted-proxy CIDR list:
  `SmartIp::with_trusted_proxies([&"10.0.0.0/8".parse()?])`. Without an explicit
  whitelist, only the peer IP is consulted — header spoofing is otherwise trivial.
- **`Global`** — `type Key = ();`. One bucket for the whole Layer; useful for hard caps
  on total HTTP load.
- **`Header(name: &'static HeaderName)`** — value of a header treated as the key. Common
  uses: `Authorization`, `X-API-Key`. Missing or non-UTF-8 headers produce
  `MissingHeader` / `MalformedHeader`.
- **`Cookie(name: &'static str)`** — value of a named cookie. Walks `Cookie` headers
  without pulling in a full cookie crate; users who need parsed attributes should use
  `Extension<T>` after a cookie-parsing middleware.
- **`Extension<T: Clone + Hash + Eq + Debug + Send + Sync + 'static>`** — pulls a value of type
  `T` from `parts.extensions`. The conventional shape for "auth has populated a
  `UserId`, rate-limit by that".

## Combinators

- **`Compound<A, B>`** — `A: KeyExtractor` + `B: KeyExtractor` ⇒ `KeyExtractor` with
  `Key = (A::Key, B::Key)`. Failure of either propagates. Typical use: rate-limit by
  `(IpAddr, MethodPath)` so one IP cannot exhaust a single-route budget.

`Compound` is sync-only by design. Mixing async extractors into a tuple key forces
every sync sibling onto the async hot path; users who need async + composition write
the composing extractor by hand and place the await inside it.

## Per-tier override flow

The intended flow when offering tiered quotas:

1. Auth middleware (upstream of the Layer) reads the API key, looks up the user, and
   inserts `Tier(Free | Pro | Enterprise)` into `parts.extensions`.
2. A user-defined `KeyExtractor` reads `parts.extensions::<Tier>()`, picks the user-id
   key, and returns `KeyOutcome { key, quota_override: Some(quota_for_tier) }`.
3. The Layer applies `quota_override` for this request only; the bucket state is keyed
   by `key`, so a tier upgrade between requests is observed on the next request without
   touching state.

The lookup is already done by auth — we never re-do it. If the lookup itself is async
(no upstream auth, tier comes from a cache fetch), use `AsyncKeyExtractor` instead.

## What the Layer holds

```rust
pub struct GovernorLayer<K> {
    extractor: KeyExtractorKind<K>,
    // ...
}

enum KeyExtractorKind<K> {
    Sync(Arc<dyn KeyExtractor<Key = K>>),
    Async(Arc<dyn AsyncKeyExtractor<Key = K>>),
}
```

Users pick the variant by builder method; the Service implementation branches once at
construction time and selects the right code path per request.

## Object-erased Layer

`BoxedGovernorLayer` (see [`07-ergonomics-and-testing.md`](07-ergonomics-and-testing.md))
re-erases `K` to a string-keyed inner store, paying one allocation and one hash per
request in exchange for a single non-generic Layer type that fits in app state without
naming `K`. This is the explicit cost; users who want zero-overhead generic dispatch
keep the `GovernorLayer<K>` form.
