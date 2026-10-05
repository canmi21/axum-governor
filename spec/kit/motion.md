# What moves, and what moves it

**"Motion" means the `motion` package, the library that was Framer Motion** until it took the
shorter name, and nothing else -- in a spec, in a comment and in conversation. A Svelte page uses
its framework-free half, `animate` imported from `motion`; the React components it also ships are
never used. A rule elsewhere that says "`motion` springs this" or "on `motion`'s spring" means that
`animate`.

**Svelte's own motion is not used.** No `svelte/motion` -- its `Tween` and `Spring` -- no
`svelte/transition` and no `svelte/animate`, and so no `transition:`, `in:`, `out:` or `animate:`
directive in markup. Their timings would be a second set of numbers beside the one below, which is
the thing every gesture here is built to avoid.

**`@canmi/kit/motion` is the timing, not an engine.** It holds the numbers a movement is made with
-- `pressMotion` for a surface answering a press, `travelMotion` for an indicator crossing a strip,
`contentMotion` for a control resizing with its content -- and imports nothing that animates. What
takes those numbers and moves something is `motion`'s `animate`, as `@canmi/kit/behavior/collapse`
does for every height that opens and closes.
