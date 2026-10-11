# Layouts

What `@canmi/design` frames a page with. A layout stands between the components and an app's own
interface: it may draw components, and no component draws it -- [components.md](components.md).

## A layout owns the viewport, so it is never put inside anything

**A component may be put anywhere, as many times as wanted; a layout cannot be put inside
anything.** That is the line between the two, and size is not: a layout holds the viewport whole --
which regions are fixed, which one scrolls, how they stack where the screen is narrow -- so a page
has one, and nothing holds a second inside it. Like a component it knows nothing of an app's data:
it leaves places, and the app fills each.

**Where a place stands is the layout's, and what fills it is the app's.** The widths, the heights,
the insets, which region scrolls and where they stack are decided once here, so two apps on one
layout cannot drift apart by a pixel; the app passes what goes in each place and never sets how it
sits. Made after the console's and space's sidebars were found 4px apart on 2026-10-10, each drawn
by hand from the other.

## The shell

**`@canmi/design/layouts/shell.svelte` is the console's frame.** The sidebar down the left, 15rem
wide: its head as tall as the top bar, 3.5rem, inset 0.75rem; its pages below, inset as the head is
and a quarter rem from the head, scrolling on their own; its foot under them. The top bar across the
rest, 3.5rem tall, its three places in a grid of `1fr auto 1fr` so the middle is centered whatever
the sides hold, inset 2rem. The page between them, the one region that scrolls, its column at most
90rem wide, inset 2rem, its parts 1.5rem apart. Where the viewport is narrower than 48rem the three
stack into one column that scrolls whole.

**The page's region is `main#content`**, focusable, so a skip link lands on it, and given to the
app bound and with its scroll, for an app that keeps a reader's place in it.

**Neither bar is selectable, and the page is**; what a reader would quote in a bar is made
selectable by the app where it stands.
