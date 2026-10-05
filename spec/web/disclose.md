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
  library that sets its own global keeps it.
- **One function, every app.** Each app names its own entries; what every app with Sentry shares
  is set in `initClient`, `Sentry.SDK_VERSION`.

A version shows only where a fingerprint reads one -- Sentry's `Sentry.SDK_VERSION` and Algolia's
`__algolia.algoliasearch.version` among what the sites use. The others name a technology and no
version, so they are disclosed only where they would otherwise go unseen: OpenPanel's client as
`openpanel`, since its fingerprint wants `openpanel.api` and the bundled client sets no global.
