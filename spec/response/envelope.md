# The envelope

## Every answer is one envelope

An API built on this package, in TypeScript or in Rust, answers in one shape:
`{ "status": "success", "data": ... }` or `{ "status": "error", "code": ..., "message": ... }`. The
npm package and the `response` crate are its two halves, and both read the one catalogue of codes,
`codes.json`, and are tested against the same fixtures, `src/fixtures.json`, so the two languages
cannot drift apart. A success carries what the route answers and nothing else; there is nothing to
say about a call that worked. A body that is the thing itself -- an image, a file, a redirect -- is
not wrapped.

**A failure carries a code and a message, both always.** The code is for a program: lowercase with
underscores, and not the HTTP status, which the response already has. The message is for a person:
one line of English, objective, short without being curt, opening with a capital and ending without
a stop. Each code has a default message in the catalogue, so a refusal names its code, and a moment
with something more exact to say -- a port and who holds it -- says it instead.

**A code is one of four families.** `no_such_*` for something asked for by a name or an id that
does not exist, `invalid_*` for a request that is malformed, `*_unavailable` for something that
could not be reached or read at this moment, and `forbidden_*` for a request that is well formed and
refused to this caller -- a 403, where `invalid_*` is a 400. `rate_limited` is the one code outside
them.

## `codes.json` lives in the npm package and is linked into the crate

`cargo package` takes only what is under the crate's own directory, and follows a symbolic link to
copy what it names, so `crates/response/codes.json` and `src/fixtures.json` are links to the npm
package's. The catalogue is edited there, once.
