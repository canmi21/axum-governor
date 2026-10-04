# response

The envelope every API of the author's answers in, the Rust half of
[`@canmi/response`](https://www.npmjs.com/package/@canmi/response). Both halves read the same
`codes.json`, so a code means the same message on either side.

```json
{ "status": "success", "data": { "count": 3 } }
{ "status": "error", "code": "no_such_route", "message": "No route answers this path" }
```

```toml
[dependencies]
response = { version = "2", features = ["axum"] }
```

`Envelope<T>` is the shape, and `message_of` a code's message; with `axum`, `success`, `failure`
and `failure_with` build a response from a status and a body or a code.

Version 2 is a new crate under the name of an older one of the author's.
