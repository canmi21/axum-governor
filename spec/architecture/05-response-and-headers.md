# Response and headers

Two header families, three body presets, one `error_handler` hook, and a sharp split
between "quota exceeded" and "key extraction failed" failure paths.

## What we always emit on a reject

```
HTTP/1.1 429 Too Many Requests
Retry-After: 5
RateLimit:        "default";r=0;t=5
RateLimit-Policy: "default";q=100;w=60
X-RateLimit-Limit:     100
X-RateLimit-Remaining: 0
X-RateLimit-Reset:     5
Content-Type: text/plain; charset=utf-8
Content-Length: 32

Too Many Requests, retry in 5s
```

Three header families, all on by default:

1. **`Retry-After`** (RFC 9110) in delta-seconds. The most universally honored header;
   never disable.
2. **IETF draft-10 structured fields** (`RateLimit:` + `RateLimit-Policy:`) per
   [draft-ietf-httpapi-ratelimit-headers-10]. `t=` is delta-seconds — the spec is
   explicit. The policy name (`"default"` above) is the stack entry's name; for
   stacked configs it identifies which limit triggered.
3. **Legacy `X-RateLimit-*`** with `Reset` as delta-seconds by default. We pick delta
   over epoch because (a) it matches `t=` and `Retry-After` so all three numbers are
   coherent, (b) it is what `governor::NotUntil::wait_time_from(now)` natively
   produces without a clock dependency, and (c) it is what Discord and Cloudflare
   emit. Users who must match GitHub's epoch wire format flip
   `GovernorConfigBuilder::legacy_reset_epoch(true)`.

If both `Retry-After` and `RateLimit:` are present, the IETF spec says `Retry-After`
wins on the client side — we emit both anyway because every dashboard scrapes one or
the other.

[draft-ietf-httpapi-ratelimit-headers-10]: https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-ratelimit-headers-10

## Headers on successful (admitted) responses

The same `RateLimit:` / `RateLimit-Policy:` and `X-RateLimit-*` headers are emitted on
non-429 responses, with `r=` and `X-RateLimit-Remaining` reflecting the post-decrement
remaining capacity. Whitelisted requests (see [`04`](04-quota-and-policy.md)) emit
nothing; the request is treated as if the limiter is not on the path.

This matches the IETF draft and every production deployment we surveyed: clients
budget across responses by reading the live counter, not just on rejects.

## Why `StateInformationMiddleware` is mandatory

`StateSnapshot::remaining_burst_capacity()` is the only public source of remaining
capacity in `governor`. With `NoOpMiddleware` we cannot fill `r=` or
`X-RateLimit-Remaining` truthfully. The cost of `StateInformationMiddleware` is one
extra `t / τ` arithmetic per check, which is well below network jitter — the trade
is unambiguous.

## Body presets

Three presets selectable on the builder; default is plain text.

```rust
.body_preset(BodyPreset::Text)         // default
.body_preset(BodyPreset::Json)         // application/json
.body_preset(BodyPreset::ProblemJson)  // application/problem+json (RFC 9457)
```

- **`Text`** — `Too Many Requests, retry in {N}s` (delta-seconds, no localisation).
- **`Json`** — `{"error":"too_many_requests","retry_after_seconds":5}`. Matches the
  conventional axum-style JSON error body; predictable for dashboards.
- **`ProblemJson`** — RFC 9457 (which obsoletes RFC 7807). Default `type` is
  `about:blank`, default `title` is `"Too Many Requests"`. No registered `type` URI
  exists for rate-limit-exceeded; users supply their own when they want one.

```json
{
	"type": "about:blank",
	"title": "Too Many Requests",
	"status": 429,
	"detail": "Rate limit exceeded; retry in 5 seconds"
}
```

The `serde_json` dependency is feature-gated (`features = ["json"]`, default-on); a
user on `default-features = false` who does not enable `json` is restricted to `Text`.

## Custom `error_handler`

Power users override the entire response:

```rust
.error_handler(|reason: RejectionReason| -> Response<Body> {
    match reason {
        RejectionReason::QuotaExceeded { wait, snapshot, key, policy_name } => { /* ... */ }
        RejectionReason::KeyExtractionFailed(err) => { /* ... */ }
    }
})
```

`RejectionReason` is the type the layer hands the user — it is the _only_ way to
distinguish the two cases without resorting to status-code sniffing.

```rust
pub enum RejectionReason {
    QuotaExceeded {
        wait: Duration,
        snapshot: StateSnapshot,
        key: Box<dyn std::any::Any + Send>,   // erased; layer knows its K
        policy_name: &'static str,
    },
    KeyExtractionFailed(ExtractionError),
}
```

When `error_handler` is set, the layer skips its built-in body. Headers are still
filled in by the outer Service wrapper, so the user gets to control the body without
losing the standards-compliant headers. Users who want full control over headers
return their `Response` with the headers they want and the layer leaves them alone.

## Default response status mapping

| Reason                                    | Default status | Notes                                                            |
| ----------------------------------------- | -------------- | ---------------------------------------------------------------- |
| `QuotaExceeded`                           | 429            | RFC 6585.                                                        |
| `KeyExtractionFailed::MissingHeader`      | 400            | Missing required header is the client's fault.                   |
| `KeyExtractionFailed::MalformedHeader`    | 400            | Same.                                                            |
| `KeyExtractionFailed::MissingConnectInfo` | 500            | Server misconfiguration; should have been caught by the builder. |
| `KeyExtractionFailed::UntrustedProxy`     | 400            | Proxy not in whitelist.                                          |
| `KeyExtractionFailed::Other(_)`           | 500            | Conservative default.                                            |

These are defaults; `error_handler` overrides everything.

## Worked example: stacked rejection

Config: `PeerIp` 10/s and `Header(Authorization)` 600/m. A burst trips the second:

```
HTTP/1.1 429 Too Many Requests
Retry-After: 12
RateLimit:        "auth";r=0;t=12
RateLimit-Policy: "peer";q=10;w=1, "auth";q=600;w=60
X-RateLimit-Limit:     600
X-RateLimit-Remaining: 0
X-RateLimit-Reset:     12
```

Both policies appear in `RateLimit-Policy`; only the triggering one populates
`RateLimit:` and the legacy headers.
