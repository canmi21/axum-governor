# Sentry

## `@canmi/web/sentry` is what every app shares of Sentry

The build's upload decision and plugin options (`./sentry/build`), the client and server init
(`./sentry/client`, `./sentry/server`), and the feedback dialog (`./sentry/feedback`). Which
project an app reports to is its own DSN, so what a reader sends from an app reaches that app's
project.

## The feedback dialog is fetched when a reader asks for it, and offers mail when it cannot open

**`openReport()` is imported by the control that opens it, with `await import`, never at the top
of a file.** The widget is 24KB gzipped, which in an app's entry is every page paying for a
control most never press; and on the server `@sentry/sveltekit` has no `getFeedback`, so a
top-level import fails the module rather than the call -- a page that did this answered 500. The
module uses named imports, since a namespace import forces every export live and the bundler could
not split it off.

**`report(mailto)` in `./sentry/report` is what a control calls**, and the one that may be
imported anywhere: it imports nothing of Sentry, fetches the dialog on the press, and sends the
reader to `mailto` when the dialog does not open.

**`openReport` answers whether it opened, and `report` offers mail when it did not**, the `support`
box from `@canmi/me/mail`. It does not open without a client, which an app without a DSN never
makes, nor when the dialog fails; and the `import` itself fails when a tab opened before a deploy
asks for a chunk the deploy replaced, which `report` catches the same way. A control that does
nothing when pressed is the one outcome not allowed.
