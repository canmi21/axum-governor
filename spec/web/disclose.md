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
version or names a library by hand. Only what the app lists in its own `package.json` counts, so a
dependency it declares and never uses is a false claim -- remove it rather than leave it.

- **`globals`** are what a package used on every page is disclosed as -- the `GLOBALS` table in
  `build.ts`, given the installed version.
- **`references`** are addresses a fingerprint looks for in a page's script text -- the
  `REFERENCES` table.
- **`runtime`** is `Cloudflare Workers` when the app depends on `@sveltejs/adapter-cloudflare` or
  `wrangler`, and absent otherwise.
- **`versions`** hold the installed version of each package a component's own patch reads -- the
  `VERSIONED` list -- for a library used on some pages only.

A version is read from the package's own `package.json` beside the app, since exports rarely offer
it.

**Every app that has pages and depends on `@canmi/web` wires it once**, the same way: the define
in `vite.config.ts`, and in its root layout `discloseGlobals(disclosure)` and
`{@html disclosureHead(disclosure)}` in the head. The head is the `runtime` meta and a
`<script type="application/json" data-disclosure>` naming the `references`: a data block the
browser never runs or fetches, which Wappalyzer reads as script text. An app then discloses
whatever it gains a dependency on without another line.

## What each technology is disclosed as

A version shows only where a fingerprint reads one.

| Technology | Fingerprint read                  | Disclosed as                                                |
| ---------- | --------------------------------- | ----------------------------------------------------------- |
| Sentry     | `Sentry.SDK_VERSION`              | the SDK's own constant, in `initClient`                     |
| Algolia    | `__algolia.algoliasearch.version` | the installed `algoliasearch`, from `globals`               |
| CodeMirror | `CodeMirror.version`              | the installed `@codemirror/view`, by the editor on mounting |
| Video.js   | `videojs.VERSION`                 | the installed `@videojs/core`, by a video on mounting       |
| D3         | `d3.version`                      | `<module>@<version>`, by `discloseD3` where it draws        |
| OpenPanel  | `openpanel.api`                   | the real client, as `openpanel`                             |
| Motion     | `MotionIsMounted`                 | `true`, from `globals`                                      |
| TanStack   | `tanstack.com` in script text     | `https://tanstack.com/query`, from `references`             |
| Workers    | `<meta name="runtime">`           | `runtime`, from the build                                   |
| Hono       | `X-Powered-By` on the page        | `Hono`, from every Hono app and on the site's pages         |
| Iconify    | `iconify` beside `data-icon`      | on one MingCute icon, which is drawn from Iconify's data    |
| MingCute   | `i-mingcute-<name>-line`          | on the icons it draws                                       |

- **D3 is named without a version.** An app carries D3's modules -- `d3-hierarchy`, `d3-scale`,
  `d3-shape`, `d3-array` -- never D3 itself, and a module's own major would read as D3's;
  `<module>@<version>` is refused by Wappalyzer's version check, which allows only letters,
  digits, `.`, `_` and `-`. `discloseD3(disclosure)` names the first module the app lists, from
  the component that draws with it.
- **Motion's flag is one its React components set on mounting**, and its `animate` -- all a
  Svelte page uses -- never sets. Wappalyzer still lists Motion under its old name, Framer Motion.
- **Every Hono app names itself with `poweredBy()` from `@canmi/web/disclose/hono`**, the first
  middleware it registers. Hono's own sets the header in place and throws on an answer passed
  through from a `fetch`, whose headers are immutable; this one copies such an answer first.
- **Hono's header goes on the site's pages too, not only its API**, because Wappalyzer reads
  headers from the page's own response alone and only the host of a request the page makes. The
  Worker that serves the page answers its API with Hono.
- **The data block writes every slash as `\/`**, which JSON reads as a plain slash. Wappalyzer
  escapes each `/` of a pattern once more when it compiles it, so its TanStack pattern, already
  written `\/\/`, matches only a slash that follows a backslash.
- **core-js is not disclosed.** It loads only where a browser lacks an API it covers, and then it
  sets `__core-js_shared__` itself, which is the fingerprint; a browser that never needs it is not
  told it was used.
