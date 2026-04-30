# CLAUDE.md

## Stack

Single-crate Rust library (`axum-governor`) — rate-limiting middleware for `axum`, backed by
`governor`. Toolchain pinned in `rust-toolchain.toml`.

A Nix flake (`flake.nix`) provides a reproducible dev shell — `nix develop` lands you in an
environment with the rust toolchain, `cargo-nextest`, `just`, `bun`, `nixfmt`, and the musl
cross-toolchain, all pinned via `flake.lock`. `nix fmt` formats `.nix` files. Optional — system
rustup / brew works equivalently.

## Quality gates

Everything that can be mechanical is mechanical — treat the gate as authoritative, don't re-check
by hand.

Gates run on commit via lefthook:

- `rustfmt --edition 2024` on staged `.rs` files (config in `rustfmt.toml`)
- `dprint fmt` on staged `.md` / `.json` / `.toml` / `.yml` / `.yaml` files
- `cargo clippy --all-targets -- -D warnings` — lint
- `commitlint` — conventional commit format, 72-char header, lower-case subject,
  body lines ≤ 100 chars

Test baseline is `cargo nextest run` (or `just test`); `cargo test` (or `just test-cargo`) is the
bypass for doctests and runner-suspect debugging. Tests are not gated on commit. `just gate` runs
lint + test as the pre-push bundle.

## Conventions

- **Chat:** Simplified Chinese. **Code / commits / docs-in-repo:** English.
- **Naming:** see [`spec/naming.md`](spec/naming.md).
- **Comments:** see [`spec/comments.md`](spec/comments.md).
- **Testing:** see [`spec/testing.md`](spec/testing.md).
- **Scope:** features planned for v2.0 and what is deferred or out of scope live in
  [`spec/roadmap.md`](spec/roadmap.md). Edit there, not here.
- When the user says "commit this" without a message, write one that passes commitlint.

## Git

Conventional Commits (see `commitlint.config.js`). Prefer the un-scoped `type: subject` form;
default to `feat: …` / `fix: …` / `chore: …` / `test: …` since the diff already shows the scope.
Use a `(scope)` only when the reader genuinely needs disambiguation.

## Keeping this file honest

When the toolchain or a project-wide convention changes, update this file in the **same commit**
as the tooling / code change. A rule that contradicts the running code is worse than no rule.

Keep it tight. If a section starts accumulating step-by-step instructions or "don't forget to…"
reminders, that is a signal to mechanize it — add a lefthook job, a clippy lint, or a test
assertion — and delete the prose.
