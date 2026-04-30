# axum-governor tasks. `just` lists recipes; full names are canonical
# (used in CLAUDE.md / spec) and short aliases (`c`, `b`, `t`, `t1`, `g`)
# work everywhere a full name works.

default:
	@just --list --unsorted

# ─── aliases ────────────────────────────────────────────────────────
alias c := check
alias b := build
alias t := test
alias t1 := test-one
alias g := gate

# cargo check
check:
	cargo check --all-targets

# cargo build
build:
	cargo build --all-targets

# nextest (default test runner)
test:
	cargo nextest run

# cargo test bypass — runs doctests; useful when nextest output is suspect
test-cargo:
	cargo test

# Run a single test by name via nextest expression filter, e.g. `just t1 my_test`
test-one NAME:
	cargo nextest run -E 'test({{NAME}})'

# Format: rustfmt for .rs, dprint for md/json/toml/yaml (writes changes)
fmt:
	cargo fmt --all
	dprint fmt

# Lint: clippy + rustfmt check + dprint check
lint: lint-clippy lint-fmt lint-prose

# Clippy with -D warnings
lint-clippy:
	cargo clippy --all-targets -- -D warnings

# Workspace rustfmt --check
lint-fmt:
	cargo fmt --all -- --check

# dprint --check for prose / config files
lint-prose:
	dprint check

# Pre-push gate: full lint pass + test run
gate: lint test

# Clean build artifacts
clean:
	cargo clean
