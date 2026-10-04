# The repository

One repository for every package the author publishes that depends on nothing else of theirs:
npm packages under `pkgs/`, crates under `crates/`, one directory each. A package with a half in
each registry is two directories of one name -- except `@canmi/me`, whose crate is `canmi`: npm
refused the bare name as too near two others, so the npm half took the scope. It is split by registry rather than by purpose
because what it is read for is what it publishes.

Each package keeps its own spec beside the others here, under a directory of its name:
[axum-governor/](axum-governor/).

It took this shape from `axum-governor`'s own repository, renamed, so the crate's history and its
stars stayed where they were. How the rest arrives, and how each package is versioned and
published, is the workspace's lattice project's `spec/architecture/layers.md` until it moves here
with them.

## Citations that still point at lattice

The packages under `pkgs/` and the crates `canmi`, `response` and `whereabouts` came from the
workspace's lattice project, and a `spec/...` their comments cite is lattice's until the sections
about them move here. Moving them is owed: each section that rules on one of these packages comes
here, and its citations are rewritten as it does; one that rules on the site stays there.

## Versions and publishing

**A dated package is versioned `YYYY.MDD.N`**: the UTC year, the month times a hundred plus the
day, and the release's number within that day from 0 -- `2026.1004.0`, then `2026.1004.1`; the
fourth of January is `2026.104.0`. Nothing is zero-padded, which semver forbids. A day holds as many
releases as it needs, so a change is never held back for the date to turn. `@canmi/me`,
`@canmi/kit`, `@canmi/ui` and `@canmi/web` are dated, and the `canmi` crate takes `@canmi/me`'s
version. The earlier `2026.10.3` and `2026.10.4` were `YYYY.M.D`, and sort below every one since.

**A push to main that changes a package publishes it**: `.github/workflows/release.yml` runs
`.mise/tasks/release`, which publishes each package whose directory changed since its last tag,
`<name>@<version>`, and tags it as it goes -- the `canmi` crate under its own name too, beside
`@canmi/me`'s. Runs queue in the order pushed and none is cancelled
or dropped, since each numbers the day from the tags the one before pushed.

**npm's copy is packed by pnpm and published by npm**: `pnpm pack` writes the versions `workspace:`
names, and `npm publish` of the tarball reaches trusted publishing, which `pnpm publish` does not.
The crate goes through `rust-lang/crates-io-auth-action`. No token is kept anywhere.

**A package's first version was published by hand**, as `0.0.0`, since both registries attach a
trusted publisher only to a package that exists; the first dated version is the pipeline's.

**A semver package -- `axum-governor`, `response` and `@canmi/response`, `whereabouts` -- is
published by hand until it has a pipeline of its own**, its first version 1.0.0, or 2.0.0 for
`response`, which redoes the crate of that name. The two halves of `response` share one version,
since they share one `codes.json`.

**A file two halves share lives in the npm package and is linked into the crate**: `cargo package`
takes only what is under the crate's own directory, and follows a symbolic link to copy what it
names. `crates/response/codes.json` is one, to `pkgs/response/codes.json`.

**`whereabouts` is `geocode` renamed, with `geo`'s address lookup taken in beside it**, each
behind a feature of its own and none on by default. Fetching the data stays with the caller.

**Each package is built by tsdown into `dist/`**, one output per source file, so a `.svelte.ts`
stays a module Svelte compiles and a `.stylex.ts` one StyleX reads; CSS, JSON and Svelte components
are copied as they are. A consumer that is not a bundler -- node, loading a Vite config -- does
not strip types inside `node_modules`, so nothing is published as TypeScript.
