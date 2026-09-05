# API stability

## Within a major, nothing public is removed

The crate follows semver, and the part that needs writing down is what happens to a public
item that turns out to be wrong. It is marked `#[deprecated]` with the minor it was deprecated
in and a note saying why, and it stays until the next major. Removing it would be the cheaper
edit, and it is not taken because a published crate has callers nobody here can see; a match
arm on a variant that no longer exists breaks their build on a minor bump they had every reason
to take blindly.

A minor may add: new methods, new constructors, new modules, new `test_utils` helpers, new
examples. A minor may not change the shape of anything already public, including the field
list of a `pub` tuple struct.

## Waiting on 3.0

Each of these is a known wart kept for the reason above. The list is the release checklist for
the next major, so an entry is only removed when the change ships.

- `ConfigError::ZeroBurst` is unreachable. Every `Quota` constructor takes `NonZeroU32`, so no
  configuration can produce it. Deprecated in 2.1; delete the variant.
- `RejectionReason::QuotaExceeded.snapshot` is fabricated. It is built from a fresh direct
  limiter per rejection, so its values describe no real request. Replace it with the `Quota`
  that rejected, which is what an `error_handler` actually wants.
- `RejectionReason::QuotaExceeded.key` is `()` for stack rejections. Stack entries are
  type-erased and cannot hand back a typed key; the formatted key is only in the tracing event.
  Either carry the formatted key or drop the field.
- `Header(pub &'static HeaderName)` forces a `static` for any custom header name because the
  error type carries the name as `&'static str`. Move the name into an owned `HeaderName` on
  both.
- `PeerIp::ipv6_prefix(u8)` is a constructor while `SmartIp::ipv6_prefix(self, u8)` is a
  setter. Same name, different shape; make both setters.
