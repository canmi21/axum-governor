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

The packages under `pkgs/` and the crates `canmi`, `response` and `geocode` came from the
workspace's lattice project, and a `spec/...` their comments cite is lattice's until the sections
about them move here. Moving them is owed: each section that rules on one of these packages comes
here, and its citations are rewritten as it does; one that rules on the site stays there.

## Versions and publishing

**A dated package is versioned by the UTC day**, `2026.10.4`, never zero-padded, which semver
forbids: `@canmi/me`, `@canmi/kit`, `@canmi/ui` and `@canmi/web`, and the `canmi` crate under the
same version as `@canmi/me`. `.github/workflows/release.yml` runs `.mise/tasks/release` just after
midnight UTC for the day that ended, or by hand for today, and publishes each package whose
directory changed since its last tag, `<name>@<version>`, tagging it as it goes.

**npm's copy is packed by pnpm and published by npm**: `pnpm pack` writes the versions `workspace:`
names, and `npm publish` of the tarball reaches trusted publishing, which `pnpm publish` does not.
The crate goes through `rust-lang/crates-io-auth-action`. No token is kept anywhere.

**A package's first version was published by hand**, as `0.0.0`, since both registries attach a
trusted publisher only to a package that exists; the first dated version is the pipeline's.

**A semver package -- `axum-governor`, `response` and `@canmi/response`, `geocode` -- is published
by hand until it has a pipeline of its own**, its first version 1.0.0, or 2.0.0 for `response`,
which redoes the crate of that name.

**Each package is built by tsdown into `dist/`**, one output per source file, so a `.svelte.ts`
stays a module Svelte compiles and a `.stylex.ts` one StyleX reads; CSS, JSON and Svelte components
are copied as they are. A consumer that is not a bundler -- node, loading a Vite config -- does
not strip types inside `node_modules`, so nothing is published as TypeScript.
