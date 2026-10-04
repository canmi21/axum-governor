# @canmi/response

The envelope every API of the author's answers in, the TypeScript half of the
[`response`](https://crates.io/crates/response) crate. Both halves read the same `codes.json`, so a
code means the same message on either side.

```json
{ "status": "success", "data": { "count": 3 } }
{ "status": "error", "code": "no_such_route", "message": "No route answers this path" }
```

`ApiResponse<T>` is the shape and `Code` a code from the catalogue; `success` and `failure` build a
`Response`, `errorBody` the body of one, and `unwrap` reads a body back into its data or throws.
