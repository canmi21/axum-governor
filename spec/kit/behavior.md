# Behavior: what a page does, with none of its look

## `@canmi/kit/behavior` holds the logic two apps would otherwise each write

**A behavior is what a control does, apart from what it looks like**: which keys it answers, what
it keeps in state, where focus goes and what a reader of assistive technology is told. It draws
nothing -- no StyleX, no Tailwind, no CSS, no copy -- so an app with a styling system of its own
takes the behavior and dresses it in its own. A piece of logic moves here when a second app needs
it, not before: the first app is where it proves itself, and the second is what shows which half
of it was the app's own.

**What stays in the app is everything that says what it is about**: the backend a search asks, the
records it gets back and how they are drawn, the copy in each language, the styles, and any rule
of the app's own passed in rather than built in -- a predicate, a fetch, a number.

## A search palette's three parts

Taken out of the site's palette on 2026-10-08, when the console needed the same one, and written
as the site's spec states the behavior -- web's `spec/search.md`.

- **`shortcut`** answers a key with the platform's command modifier, Meta or Control, from
  anywhere on the page, and **stands down while focus is in a text field**, so typing a `k` into a
  form is never taken for a command; a shortcut that must also fire there says so. It matches the
  modifiers it is given and no others, so `⌘⇧K` stays free while `⌘K` is bound, and it hands back
  the function that unbinds it, which `$effect` takes as its cleanup. Its label is
  `⌘K` on Apple platforms and `Ctrl K` elsewhere, and `⌘` on the server, so the first paint and an
  Apple browser agree.
- **`query`** holds a debounced search: a quiet period before a request, 300 ms unless told
  otherwise, and a ticket per request so only the newest may write -- not an `AbortController`,
  because a client may expose no signal to abort with. A text not worth searching clears at once.
- **`cursor`** is the active row of a list of results: the arrow keys wrap, Home and End go to
  the ends, Enter chooses, the pointer moves it, and it is clamped when the list shrinks. It
  carries the WAI-ARIA combobox pattern the palette's markup takes as attribute bags -- the input a
  `combobox` naming its `listbox` and its active `option` -- and keeps the active row in view.

The dialog around them is the app's, from Bits UI, which already traps focus, closes on Escape and
outside clicks, locks scrolling and returns focus; nothing here repeats it.

## Where focus came from

**`focus-source` records what the last input was, on the document, for the focus ring to read**:
`data-focus-source="kbd"` after a key that moves or acts on focus, `"pointer"` after a press, touch
arriving as a pointer like any other. `tokens/interaction.css` takes the ring away only where the
source is positively a pointer, because `:focus-visible` is the browser's guess and is least
reliable for focus a script moved -- a menu handing it back to its trigger. Written as a suppression
and never as a requirement, so with nothing recorded the guess stands: it can show a ring once too
often and never leave a keyboard with none. The rule is web's `spec/styling/focus.md`,
"`:focus-visible` is the browser's guess, and the site keeps its own answer"; the tracker moved here
from the site on 2026-10-08, when the console took the same ring.
