# Addresses

## `@canmi/me/urls` holds the addresses whose names are the author's

The author's own site and the addresses of their identity, the world's addresses anyone could use
-- GitHub, the registries, SPDX, the social bases, analytics and fonts -- and the functions a
program reads a request with. **An address goes with whoever owns the name, not with whoever reads
it**: `canmi.net` is the author's even where a platform probes it. A platform's own addresses are
its own package's, which composes this one into the map its programs read; nothing here names one.

## The Rust half is generated, never written

`crates/canmi/src/lib.rs` is rendered from `urls/src/index.ts` and `identity/author.json` by `mise
run urls`: every string under the map as a `pub const` named by its path in capitals, so
`external.github.web` is `EXTERNAL_GITHUB_WEB`, and the author's identity as `AUTHOR_*`. It is
committed, so a checkout compiles without Node having run first, and `rust.test.ts` fails the moment
the two disagree. The generator, `rustConstants`, knows nothing of what the addresses are; another
address package renders its own Rust half with it.

## Every address has one spelling

`normalizePath` is a request path in its one spelling: every CJK full stop a dot, every backslash a
slash, every run of slashes one, and no trailing slash, so the root alone is `''`, the bare host. The
full stops are the three IDNA already reads as a dot in a hostname -- `。`, `．` and `｡` -- so a
reader typing on a CJK keyboard reaches a path the way a browser lets them reach a host.

`normalizedLocation` says where a request belongs and how it is sent there, or that it is already
there. **`/` is always already there**, alone or with a query: the bare host goes on the wire as
`GET /`, so no server can tell the two apart, and a redirect between them would loop. Any other
spelling of the root alone goes to the host itself, no slash, by a 301; any other path to its
spelling with the query kept, by a 308, which keeps the method, so a misspelled API call arrives as
the call it was. The redirect itself is the caller's, in its own framework's terms.
