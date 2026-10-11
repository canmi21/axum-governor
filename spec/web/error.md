# Error: which page an error is, without the page

`@canmi/web/error` is what an app's error page is made of that is not its look or its words: which
side failed, which of two pages that makes, and the protocol's name for a status. The page drawn
from it is a style's, in that style's tokens and languages -- the site's own, in every language it
answers in; the console's and space's, `@canmi/design`'s in English, [../design/components.md](../design/components.md),
"An error page" -- so this half holds no component, no style and no message. Decided with the
author on 2026-10-09; the neutral style's page moved into `@canmi/design` on 2026-10-10.

## Only a failure is stamped

**`stamp(side)` is the function each side's `handleErrorWithSentry` is given**: an error of kind
`unknown` -- thrown by code, not by `error()` or by SvelteKit itself -- comes back stamped `origin:
'client'` or `origin: 'server'`, and every other keeps the status and message it came with. So no
stamp means an answer rather than a failure: a 404 is a 404 whichever side worked it out. An app's
`App.Error` extends `Stamped` for the field. One gap: an error surfaced through `__data.json`
arrives as a status and a string and loses the stamp, which draws the page with the status, the
right fallback, since the status is the part that survived.

## Two pages

**`pageOf(error)` is `client` where the browser broke, and `status` for everything else.** The
client page has no status, because nothing answered: a code is what a server said about a request,
and inventing a 500 for a page that was served and could not finish would claim the one thing known
not to have happened. Its title is `CLIENT_TITLE`, `Unexpected Client Behavior`, in the Title Case
of the protocol's names. The status page shows the number, and `statusText(status)` is the
protocol's name for it, from `STATUS_TEXT`, in English whatever language the page answers in: a
reader who meets `404` meets `Not Found` with it everywhere else. A status the table does not hold
is `Error`. The sentence for a person beside the number is the app's, never `error.message`, which
carries whatever words the framework or an `error()` call used and is for a log.
