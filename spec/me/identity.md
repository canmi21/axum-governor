# Identity

## `@canmi/me/identity` holds the author, declared once

The name a page signs with, the full name, the role, the email, and the handle on each service --
GitHub, X, the fediverse, Bluesky, Telegram. Telegram is two: the author's own account, and the
group they run. A program reads the author from here and never spells the name itself. A handle is
this package's and a service's address is whoever's owns it; a link is the two put together.

**The record is a JSON file**, `identity/author.json`, published as `@canmi/me/identity/author.json`
and rendered into the `canmi` crate as `AUTHOR_*` constants, so a Rust program and a script outside
any bundler read what the TypeScript does. `mailbox` is the author named in full anywhere they are
not being introduced: the name, then the address.
