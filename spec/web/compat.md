# Compatibility

## The API floor: a short list of canaries, and all of core-js behind it

`@canmi/web/compat` checks for each canary -- `Array.prototype.toSorted`, `URL.canParse` -- and, if
any is absent, dynamically imports `core-js/stable` before hydration.

**There is no list of modules, and that is the point.** A hand-written list has no knowable correct
length: an entry missing from it is a crash in somebody's browser, discovered the way the first one
was. Loading the whole stable set removes the question instead of answering it. Being generous is
free because the import is dynamic: the check compiles into an app's eager entry at under two
hundred bytes, core-js is in none of its static chunks, a current browser fetches nothing, and a
browser below the line fetches the stable set once, before hydration.

**The canaries grow one at a time, each an edge case met in production.** `toSorted` was the first.
`URL.canParse` is the second: Chrome 120, above the floor, so a Chrome 110 to 119 reader passes the
first canary and loads nothing, and a Chrome 99 reader crashed on it. Either missing loads core-js.
The floor itself does not move for a canary: it is where the syntax must parse, and a canary is only
a cheaper way of noticing a browser that needs the rest. A new one is one line in `src/index.ts` and
one in `canaries.test.ts`.

**So few checks, because browser support is strongly ordered**: a browser new enough to have a
canary has the decade of features before it, and a browser without it needs everything anyway. A
canary is not a claim about which API the next crash involves -- it is a cheap proxy for "this
browser is old", and the whole of core-js is what answers the crash. The canaries are where the API
floor is declared, not a config file or a table: moving that line is editing that list.

`stable` rather than `es`, which omits `URL` and `structuredClone`, or `actual`, which adds proposals
nothing here writes.

## The syntax floor is an app's `browserslist`, read into esbuild's target

`esbuildTarget` reads an app's `browserslist` into esbuild's `build.target`, so the floor is written
once, in the app's `package.json`, and the build derives from it.

**A rescue only happens if the browser could parse the code doing the rescuing.** A target above the
canaries' line hands exactly the readers this exists for a bundle that dies before the check runs,
and core-js sitting in a chunk they never reach helps nobody. Two apps that read their floor through
this agree by construction rather than by somebody keeping them in step.

**Floors, never a relative query.** `> 0.5%` or `last 2 versions` is resolved against
`caniuse-lite`, so the compiled output would change on an unrelated dependency update and
rebuilding one commit twice would not produce the same bytes. `esbuildTarget` refuses a query that
is not a floor.
