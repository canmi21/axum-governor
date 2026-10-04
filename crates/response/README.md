# Response

One JSON response envelope for [Axum](https://docs.rs/axum), shared with
[`@canmi/response`](https://www.npmjs.com/package/@canmi/response) on the TypeScript side.

## Quick start

```toml
[dependencies]
response = { version = "2", features = ["axum"] }
```

```rust
use axum::{Router, http::StatusCode, response::Response, routing::get};
use response::{failure, success};

async fn tags() -> Response {
    success(StatusCode::OK, ["rust", "svelte"])
}

async fn missing() -> Response {
    failure(StatusCode::NOT_FOUND, "no_such_route")
}

let app: Router = Router::new().route("/tags", get(tags)).fallback(missing);
```

Every answer is one of two shapes:

```json
{ "status": "success", "data": ["rust", "svelte"] }
{ "status": "error", "code": "no_such_route", "message": "No route answers this path" }
```

## Features

- **One shape, two languages** — `Envelope<T>` here and `ApiResponse<T>` in `@canmi/response`
  serialize the same, and both are tested against one set of fixtures.
- **A catalog of codes** — every code lives in `codes.json` with its message; `message_of` looks
  one up, and `Envelope::error` fills the message in for you.
- **Axum helpers** — `success`, `failure` and `failure_with` turn a status and data, or a code,
  into a response.

## Cargo features

| Feature | Default | Description                                               |
| ------- | ------- | --------------------------------------------------------- |
| `axum`  | No      | `success`, `failure` and `failure_with` for Axum handlers |

Without `axum` the crate is the envelope and the catalog alone, on `serde` and `serde_json`.

Version 2 is a rewrite; 1.x was an unrelated crate of mine under the same name.

## License

Released under the MIT License © 2026 [Canmi](https://canmi.net)
