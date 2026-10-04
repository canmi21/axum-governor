# The repository

One repository for every package the author publishes that depends on nothing else of theirs:
npm packages under `pkgs/`, crates under `crates/`, one directory each. A package with a half in
each registry is two directories of one name. It is split by registry rather than by purpose
because what it is read for is what it publishes.

Each package keeps its own spec beside the others here, under a directory of its name:
[axum-governor/](axum-governor/).

It took this shape from `axum-governor`'s own repository, renamed, so the crate's history and its
stars stayed where they were. How the rest arrives, and how each package is versioned and
published, is the workspace's lattice project's `spec/architecture/layers.md` until it moves here
with them.
