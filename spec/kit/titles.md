# Titles: the full one for a first load, the short one for a known reader

**A page has two titles: the full one a search result shows, and the short one a tab shows.** The
full one leads with the name and then says, in plain words, what the page is; the short one is the
name alone. Two subpaths hold the halves: `@canmi/kit/behavior/brevity` decides which one a load shows, and
`@canmi/kit/behavior/title.svelte` renders the page's `<title>` from it.

- **A page loaded fresh in a browser the app has no mark in shows the full title**, and the server
  renders it. Then it marks the browser: `visit.seen` in the `reader` record, see
  [state.md](state.md).
- **A page loaded fresh where the mark is shows the short title once it has hydrated.**
- **A page reached by a navigation inside the app shows the short title.** Only the landing page of
  a first visit is the full one.

An app's root layout calls `settleBrevity` on mount, and `shortenTitles` on any navigation that is
not the first.

## Why this and not something else

**Google renders JavaScript, on every page, and reads a title the script changed.** Shortening every
title after hydration would put the short one in the results, and nothing in a page can ask Google
not to render it.

**Its renderer keeps no state between loads**: storage and cookies are cleared for each URL it
renders, and it loads each URL fresh rather than following a link inside the app. So it always sees
what a first-time reader sees, which is the full title. That is also why this is not cloaking: no
visitor is told apart by who it is, only by what its own browser holds, and Google is shown exactly
what any new reader is shown. Telling crawlers apart by user agent or by automation traits is the
alternative not taken -- it is cloaking by definition, and the traits do not even mark Google's
renderer reliably.

A browser that keeps nothing -- a private window, storage turned off, data cleared -- is always a
first visit and always shows the full title. Nothing breaks; the title is just longer.
