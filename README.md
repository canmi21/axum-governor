# lib

Everything of the author's that depends on nothing else of theirs, published: npm packages under
`pkgs/`, crates under `crates/`, each with its own README.

| Crate                                    | What it is                                                       |
| ---------------------------------------- | ---------------------------------------------------------------- |
| [`axum-governor`](crates/axum-governor/) | Rate-limiting middleware for Axum, on Governor                   |
| [`whereabouts`](crates/whereabouts/)     | A position to a place name, an IP address to a location, offline |
| [`response`](crates/response/)           | The envelope the author's APIs answer in, from Rust              |

| Package                             | What it is                                                |
| ----------------------------------- | --------------------------------------------------------- |
| [`@canmi/response`](pkgs/response/) | The envelope the author's APIs answer in, from TypeScript |

How the repository is laid out and how each package is versioned and published is
[spec/repository.md](spec/repository.md).
