# Sentry

## `@canmi/web/sentry` is what every app shares of Sentry

The build's upload decision and plugin options (`./sentry/build`), the client and server init
(`./sentry/client`, `./sentry/server`), the feedback dialog (`./sentry/feedback`), and the control
that opens it or falls back to mail (`./sentry/report`). Which
project an app reports to is its own DSN, so what a reader sends from an app reaches that app's
project.

## `initClient` needs Sentry's Vite plugin, or the page never hydrates

**An app that calls `initClient` installs `sentrySvelteKit` in its Vite config, with `pluginOptions`
from `@canmi/web/sentry/build`, whether or not it uploads source maps.** `@sentry/sveltekit` ships
two browser-tracing variants behind one import, one reading `$app/stores` and one reading
`$app/state`, and only its Vite plugin chooses between them; without the plugin the import resolves
to the `$app/stores` one. SvelteKit 3 removed `$app/stores`, so the SDK throws while it initializes,
the client hook rejects, and the page renders on the server and never hydrates -- no error a reader
sees, only a page whose controls are dead. Found on 2026-10-08 when the console called `initClient`
without the plugin; the site and the status page had the plugin for their uploads, and were spared
by that.

## The feedback dialog is fetched when a reader asks for it, and offers mail when it cannot open

**`openReport()` is imported with `await import`, never at the top of a file -- and only by
`report`, which is what a control calls.** The widget is 24KB gzipped, which in an app's entry is every page paying for a
control most never press; and on the server `@sentry/sveltekit` has no `getFeedback`, so a
top-level import fails the module rather than the call -- a page that did this answered 500. The
module uses named imports, since a namespace import forces every export live and the bundler could
not split it off.

**`report(mailto)` in `./sentry/report` is what a control calls**, and the one that may be
imported anywhere: it imports nothing of Sentry, fetches the dialog on the press, and sends the
reader to `mailto` when the dialog does not open.

**`openReport` answers whether it opened, and `report` offers mail when it did not**: the `mailto`
its caller passes, which on the site and the status page is the `support` box from
`@canmi/me/mail`. It does not open without a client, which an app without a DSN never
makes, nor when the dialog fails; and the `import` itself fails when a tab opened before a deploy
asks for a chunk the deploy replaced, which `report` catches the same way. A control that does
nothing when pressed is the one outcome not allowed.

**On a Worker, both resolve to nothing.** `./sentry/report` and `./sentry/feedback` carry a
`workerd` and a `worker` condition pointing at stubs that import nothing of Sentry: wrangler bundles
every import a Worker's code can reach, dynamic ones too, and `@sentry/sveltekit`'s `workerd` entry
has no `feedbackIntegration` or `getFeedback`, so following the dialog there failed the deploy. A
page's server code may then import `report` like any other module.
