# Disclose

## `@canmi/web/disclose` says what a page is built with, for a profiler to read

**Wappalyzer names a technology by its fingerprints, and some of the author's go unseen**: a
library bundled into the page sets none of the globals its fingerprint reads, and a request that
would name it is made only when a reader does something. `disclose` sets the global a fingerprint
reads, so the page is named for what it really uses. It is a patch for Wappalyzer, and nothing else
reads what it sets.

- **Only what the page uses, and the true value.** A version is the installed package's, read at
  build time or from the library itself; an object is the real one where the fingerprint asks for
  one. Nothing is claimed that is not so.
- **Nothing already there is replaced.** A path is filled in only where it is missing, so a
  library that sets its own global keeps it. An undefined value is skipped, so an entry the build
  did not find is simply absent.
- **One function, every app.** Each app names its own entries; what every app with Sentry shares
  is set in `initClient`, `Sentry.SDK_VERSION`.
- **Where a library is used on some pages, its patch is set by the component that uses it**, so
  only those pages say so.

## What a build knows is read from the app's `package.json`

**`disclosure(root)` in `@canmi/web/disclose/build` reads it once, at build time**, and
`discloseDefine(root)` hands it to Vite as `import.meta.env.VITE_DISCLOSURE`, so no app reads a
version by hand.

- **`versions`** holds the installed version of each package a fingerprint reads one for -- the
  `VERSIONED` list in `build.ts` -- and only those the app lists itself. It is read from the
  package's own `package.json` beside the app, since exports rarely offer it.
- **`runtime`** is `Cloudflare Workers` when the app depends on `@sveltejs/adapter-cloudflare` or
  `wrangler`, and absent otherwise. The app writes it as `<meta name="runtime">` in its root
  layout, which is all Wappalyzer's Cloudflare Workers fingerprint reads.

## What each technology is disclosed as

A version shows only where a fingerprint reads one.

| Technology | Fingerprint read                  | Disclosed as                                                |
| ---------- | --------------------------------- | ----------------------------------------------------------- |
| Sentry     | `Sentry.SDK_VERSION`              | the SDK's own constant, in `initClient`                     |
| Algolia    | `__algolia.algoliasearch.version` | the installed `algoliasearch`                               |
| CodeMirror | `CodeMirror.version`              | the installed `@codemirror/view`, by the editor on mounting |
| Video.js   | `videojs.VERSION`                 | the installed `@videojs/core`, by a video on mounting       |
| D3         | `d3.version`                      | `d3-hierarchy@<version>`, by the blocks that draw with it   |
| OpenPanel  | `openpanel.api`                   | the real client, as `openpanel`                             |
| Motion     | `MotionIsMounted`                 | `true`                                                      |
| Workers    | `<meta name="runtime">`           | `runtime`, from the build                                   |
| Hono       | `X-Powered-By` on the page        | `Hono`, on the site's pages                                 |
| Iconify    | `iconify` beside `data-icon`      | on one MingCute icon, which is drawn from Iconify's data    |
| MingCute   | `i-mingcute-<name>-line`          | on the icons it draws                                       |

- **D3 is named without a version.** A site carries only `d3-hierarchy`, whose 3.x would read as
  D3's own; `d3-hierarchy@<version>` is refused by Wappalyzer's version check, which allows only
  letters, digits, `.`, `_` and `-`.
- **Motion's flag is one its React components set on mounting**, and its `animate` -- all a
  Svelte page uses -- never sets. Wappalyzer still lists Motion under its old name, Framer Motion.
- **Hono's header goes on the site's pages, not only its API**, because Wappalyzer reads headers
  from the page's own response alone and only the host of a request the page makes. The Worker
  that serves the page answers its API with Hono.
- **core-js is not disclosed.** It loads only where a browser lacks an API it covers, and then it
  sets `__core-js_shared__` itself, which is the fingerprint; a browser that never needs it is not
  told it was used.
