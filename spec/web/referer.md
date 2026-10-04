# Referer: where a reader came from, taken out of the address bar

`@canmi/web/referer` is spelled as HTTP spells its header -- a misspelling the header has carried
since it was specified, kept so the name reads as the header's job.

**A page accepts `?ref=` and takes it out once it has hydrated.** A link from one of the author's
own places carries it -- `?ref=status`, `?ref=app`, `?ref=api` -- to say where the reader came
from. That is analytics, and a reader cannot read it, so it does not stay in the address they copy.
`ARRIVAL_PARAMETERS` is the list of such parameters, and an app's root layout calls
`takeArrivalParameters` on mount.

- **Only the named parameter goes.** Every other pair stays as it arrived, its spelling and order
  included, which a round trip through `URLSearchParams` would not keep: it rewrites `%20` as `+`
  and `?flag` as `?flag=`. A query left empty takes its `?` with it.
- **Taking one is not a page view.** umami and OpenPanel each count a view by wrapping
  `history.replaceState` on the `history` object, so a cleanup through it would count the page
  twice, once with the parameter and once without. This calls the method on `History.prototype`
  instead, which neither wraps, and the first view keeps the address the reader arrived at. No
  history entry is added either way.
- **SvelteKit's `page.url` keeps the parameter** until the next navigation, since the router is not
  told.
