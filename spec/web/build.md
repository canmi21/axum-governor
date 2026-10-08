# Build

## `@canmi/web/build` states what only exists at build time, in one call

**An app's `vite.config.ts` calls `buildDefine(root)` and nothing else for what it was built
from**: the commit, the moment, and what it is made of. It hands Vite three `define` entries --
`import.meta.env.VITE_COMMIT_HASH`, `VITE_BUILD_TIME` and the disclosure [disclose.md](disclose.md)
describes as `VITE_DISCLOSURE` -- and `BuildEnv` is the shape an app merges into its
`ImportMetaEnv`. Before it, the site worked out a commit and a time in its own config and every
app called `discloseDefine` beside it; one function makes the set the same in every app, and an
app that shows its version needs nothing of its own to do it.

- **The commit is the short one, seven characters**, which is what a page shows and what a person
  types back into a search.
- **The commit comes from the builder first, and from git after.** A provider's build names the
  commit it checked out in its environment -- GitHub Actions, Cloudflare's Workers Builds, Vercel
  and Netlify each do -- and that is the commit that was built, whatever the clone's history holds.
  On the author's machine there is no such variable, and git is asked: the repositories are jj
  colocated with git, so `HEAD` is the working copy's parent, the last commit made. A build that
  can learn neither says `unknown` rather than failing.
- **The time is the build's**, as an ISO 8601 string, so a page writes it in whatever zone its
  reader is in.

Decided on 2026-10-08, when the console's sidebar began showing the version it runs.
