# Response

One JSON response envelope for TypeScript APIs, shared with the
[`response`](https://crates.io/crates/response) crate on the Rust side.

## Quick start

```sh
npm install @canmi/response
```

```ts
import { failure, success, unwrap } from '@canmi/response';

// On the server, a standard Response either way.
success(['rust', 'svelte']);
failure(404, 'no_such_route');

// On the client, the data, or an error naming the code.
const tags = unwrap<string[]>(await response.json(), 'tags');
```

Every answer is one of two shapes:

```json
{ "status": "success", "data": ["rust", "svelte"] }
{ "status": "error", "code": "no_such_route", "message": "No route answers this path" }
```

## Features

- **One shape, two languages** — `ApiResponse<T>` here and `Envelope<T>` in the `response` crate
  serialize the same, and both are tested against one set of fixtures.
- **Typed codes** — `Code` is the union of every code in the catalog, so a typo is a type error,
  and each code carries a default message.
- **Plain `Response`** — `success` and `failure` return a Web `Response`, for Workers, Hono,
  SvelteKit or anything else that speaks fetch. A failure is `no-store` unless you say otherwise.
- **One way back** — `unwrap` returns the data, or throws with the code and the message.

## License

Released under the MIT License © 2024 [Canmi](https://canmi.net)
