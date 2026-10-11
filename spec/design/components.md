# Components

What `@canmi/design` draws: each a unit an interface is made of, standing on the contract and the
scale -- [styles.md](styles.md). What frames a whole page is a layout, not a component --
[layouts.md](layouts.md). Its layers and who is on it are web's
`spec/architecture/space.md`, "The design system: its layers, and a style across them".

## What makes a component, and what does not

**A component knows nothing of any app's data**: it takes props and snippets and draws them. What
knows an app's own things -- the console's levels, its live socket, its author -- is that app's
interface, and stays in the app, however small it is. Size is not the line: a component may be one
element or several put together.

**A component may stand on smaller components and on primitives, never on a larger one**, so the
graph between them runs one way and has no loop; no directory says how large a component is, so one
that grows moves nowhere.

**One made of parts the app arranges is a compound component** -- `Menu.Root`, `Menu.Trigger`,
`Menu.Item` -- rather than one element configured by a heap of props. A fixed arrangement, a button
holding an icon, is one component.

**A component joins when a polished part of an app first draws it; one put together from others
joins once two apps have put it together the same way.** Until then it is made and used in the app
-- space first, where space needs something nothing has -- then split into its layers and moved
here, and the app reads it back.

## How one is written

**A component is styled with StyleX alone**, on the contract's names and the scale. An app's
Tailwind reads the app's own files and never `node_modules`, so a Tailwind class written here would
draw nothing; a caller's `class` is still taken and set after the component's own.

**A `.svelte` file is published as written** and imports its neighbors by the name the build gives
them -- `./optics.js` for `optics.ts` -- since the app's Svelte compiles it and the build only
copies it.

## A link to a page

**`@canmi/design/components/nav-link.svelte` is one page in a list of pages**: an 18px icon and the
page's name, 2.25rem tall, `--interaction-subtle` under the pointer and `--interaction-muted` when
it is the page being read -- the two lower ranks, since a chosen page stands among pages that can be
pointed at, and the hover must stay below it. Its `current` is what assistive technology is told --
`page` where it is the page, `true` where the page is somewhere under it -- and either draws it
selected. It is a component and not the sidebar's: a list of pages may stand anywhere.

## An error page

**A page that fails is drawn as the site draws one**: the status, a hairline, and a sentence for a
person on one line in the middle -- `This page could not be found` for a 404, `Something went
wrong` for the rest -- and the way out at the foot;
`@canmi/design/components/error-page/status.svelte`. A browser that broke, with no status, gets
`client.svelte`: `This page crashed in the browser. Describing what you were doing helps`, and the
way out under it. Which of the two a page is, and the protocol's names, are `@canmi/web/error`'s --
[../web/error.md](../web/error.md).

**The way out is `offer.svelte`, and the app gives it what it names**: the address to write to, and
the function that opens its report form. A component knows no app's mail and no error reporter, so
an app with neither still draws the page; with no report it offers the address alone, as a 404
does, since nothing failed. Each is told from the muted sentence around it by its ink alone, with
no underline.

## Icons

**Tabler is the system's icon family**, a peer every app that draws an icon depends on itself.

### An icon button

**A button that is only an icon is one component, `@canmi/design/components/icon-button.svelte`, in
one of two kinds, square or round.** `ghost` has no ground of its own and takes
`--interaction-default` on hover -- a bar's own controls, where nothing is chosen beside it.
`framed` is filled with the surface and ruled by a one-pixel shadow rather than a border, as Geist's
secondary button is, so the rule takes no room and the button is exactly its size; its hover lays
`--interaction-strong` over the fill and leaves the rule -- a control set apart from what is around
it. A kind is added to it rather than drawn again where it is wanted, so every icon answers a hover
the same way; its `label` is its name and its title both.

### An icon is drawn in layers

**An icon's box, its drawing's offset, its drawing's scale, and where its box sits among words are
four things, and only the first and the last are layout's.** The box is a square of the size asked
for, and its center is the one point everything around it aligns to. The drawing is then moved off
that center, and scaled about it, by the icon's row in `components/icon/src/optics.ts` -- in the
units of its own 24-unit grid, so one correction holds at every size it is drawn at. Both are done
through the drawing's `viewBox`, as the site's player's cog is -- web's `spec/styling/player.md` --
so the box, and any focus ring or frame around it, never moves; and a scaled drawing is given its
stroke back, so it keeps the weight of the icons beside it. `@canmi/design/components/icon.svelte`
draws every icon that way.

**An icon beside words is set in their line, its box's center on the middle of the letter nearest
it.** Find the words' baseline; take the height that letter stands to -- a lowercase to `1ex`, since
lowercase reaches neither up to the capitals nor down past the baseline as a whole; a capital or a
figure, which stand as tall, to the capital height -- and the middle of it is where the box's center
goes. The letter is the one the icon touches: the first of the words it stands before, the last of
the words it stands after, so `Deploys` centers its rocket on the capitals and a lowercase label on
its lowercase. The capital height is the contract's `--font-sans-cap-height` -- [styles.md](styles.md).
`words` on the icon does this, and `heightBeside` in `optics.ts` names the letter. The drawing's own
correction stays inside the box and the caller never sees it: the caller places the box, the icon's
row places the ink. **A dot set by words follows the same rule**, its middle on the middle of the
letter nearest it, whichever side it stands on. **A state an icon carries is a dot in the icon's
lower corner**, ringed in the surface -- or in `--badge-ground` where it stands on something else --
`badge` on the icon, so a line of icons, names and words needs no dot of its own beside the words.
Decided with the author on 2026-10-10.

**A correction is measured, and then judged.** An outline icon's ink is weighed by rendering it and
taking its centroid against the grid's center: `git-merge` centers at x 9.97, two units left,
because two circles and the stem sit on the left and one on the right. Its correction is three
quarters of that, 1.5 units right, since a drawing moved all the way to its centroid reads as having
overshot. The table holds one row per icon, so an icon is corrected once and is the same wherever it
appears.
