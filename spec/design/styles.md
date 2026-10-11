# Design: the system the console and space are built on

`@canmi/design` is the design system: a contract of names every style gives a value to, the styles,
a scale of measures, and in time the primitives and the components standing on them. What it draws is
[components.md](components.md), and what it frames a page with [layouts.md](layouts.md). Its layers and who is on it are web's
`spec/architecture/space.md`, "The design system: its layers, and a style across them".

## Kit is frozen

**`@canmi/kit` takes nothing new and is to be deprecated**: its name says nothing of what it holds,
and half of it is a foundation the other half's behavior has nothing to do with. Renaming it would
move every app that depends on it at once, so the new system is a package of its own instead, the
console and space move to it first, and each other consumer moves when there is time -- the site
and the CMS are web's `spec/issues/site.md` and `spec/issues/cms.md`. Kit is deprecated once
nothing reads it. What `@canmi/design` needs of kit's it takes as its own, so it depends on none of
it: mono's palette was copied rather than imported.

## A component reads a role, and a style gives every role a value

**The contract is a fixed set of names, each a role; a style is a value for every name**, in light
and in dark. A component reads only the contract's names and never a palette's color, so one
family of components takes any style, and a style is changed without touching a component. The
palettes under it -- `palettes/mono.css` and the rest -- keep their own names, which only a style
reads. What an app reads beyond the contract is its own and declared in its own stylesheet; a
value two apps declare alike moves into the contract.

**The names are listed once, in code: `@canmi/design/contract`**, which a page drawing the
system reads, and whose test holds every style to it -- each name given a value on the root and on
either theme's mark, and nothing declared outside the contract's groups.

**Mono is the first style, `@canmi/design/styles/mono.css`**, the console's neutral look. It imports
its palette itself, so an app imports the style and nothing under it.

## A theme is a mark, and any element may carry one

**A theme is `data-theme="light"` or `data-theme="dark"`, and the nearest mark decides.** The
document's root carries one, set before the first frame by `@canmi/design/theme`'s script from the
reader's cookie or their system and moved by its `applyTheme`; any element below may carry the
other, and every token inside it takes that theme's value. A style declares its light on the root
and on `[data-theme='light']` and its dark over it on `[data-theme='dark']`, its palette too,
**every name declared again on each mark**, since a name made of others is worked out where it is
declared. There is no `.dark` class: the kit's `.dark` stays with the kit and the apps still on it
-- [../kit/theme.md](../kit/theme.md) -- and an app on the system marks its root as the system does.
Settled with the author on 2026-10-11.

**A component reads tokens and never asks which theme it is in**: no `dark:` variant, no branch on
the mark. A theme is nothing but the tokens' values, so a component placed in a part held in the
other theme is right there with no case of its own. A variant keyed on an ancestor's mark cannot be,
since a selector cannot ask for the nearest of two marks; the system's own pages are where that
matters, where one specimen may hold both themes side by side.

## How a name is made

**`--{group}-{role}`**: the group is the property the value is laid on, the role what it does
there. Where a group ranks its roles, the ranks are one scale for every group: `subtle`, `muted`,
`default`, `strong`. Every word is the full word, and none is a component's: a name says what a
value is for, not who first wanted it.

| group         | names                                                                                              |
| ------------- | -------------------------------------------------------------------------------------------------- |
| `background`  | `base`, under everything; `surface`, a card on it                                                 |
| `border`      | `subtle`, `default`, `strong`                                                                      |
| `foreground`  | `subtle`, `muted`, `default`, `strong`: text, and the icons and marks drawn as text is             |
| `interaction` | `subtle`, `muted`, `default`, `strong`: ranks of one tint, laid over a ground; and `focus`         |
| `status`      | `success`, `info`, `warning`, `critical`                                                           |
| `shadow`      | `surface`, the shadow a surface casts, `none` in mono                                              |
| `font`        | `sans`, `mono`, and `sans-cap-height`                                                              |

**`--font-sans-cap-height` is the sans's capital height**, `H` from its baseline to its top, what
CSS's `cap` unit should give and Chrome does not read right from this sans. It names its font,
since another face's capitals stand at another height.

**The focus ring is information's hue at full OKHSL saturation, a shade off `--status-info`, APCA Lc
5**, counted before APCA's clip at 10: a shade apart, not a second blue, so a ring drawn on a blue
mark still reads as a ring. In mono `oklch(0.674 0.173 258.6)` in the light and `oklch(0.543 0.207
258.6)` in the dark. It is told apart by its lightness, not by a gap around it. Lc 25 and 15 were
tried first and stood too far off. **There is no `--accent`**: what was the system's own and live is
information's blue, held by `--status-info` as every other status holds its hue. Settled with the
author on 2026-10-11.

**A selection is drawn as focus is**: `--interaction-focus` solid, with white on it, declared by the
style on `::selection`, since a selection is the reader's own choice as focus is. Mono's palette
kept Geist's, the ramp's far end under the text, which went gray in the light and in the dark alike.
Settled with the author on 2026-10-11.

## The scale

**`@canmi/design/scale.stylex` holds the measures, and they are the same under every style**: a
style changes colors and never a size. Each is a StyleX const, so a reading declaration keeps the
value as written. A measure joins when a component on the system first reads it, not before.

- **Type sizes are named for what they set**: `caption`, `label`, `body`, and above them in time
  `title`, `headline`, `display`. Tailwind already holds the t-shirt sizes for type, with other
  values -- its `text-sm` is 14px -- and two meanings of one word in files that mix the two would be
  read wrong; a number named only an order a reader would take for a share of the body.
- **Radii keep the t-shirt sizes**, `sm` to `xl` and `full`, since they are Tailwind's `rounded-*`
  steps at Tailwind's values: one word, one value, either way it is read.
- **Weights are `regular`, `medium`, `semibold`; a border's width is `hairline`; a transition's
  length is `duration.base`.**

A style may derive its values from parameters no component reads: mono's hover and selection are
one tint of the foreground, `--interaction-tint`, at `--interaction-strength` and a share of it.

**A surface is a step lighter than the base in both themes, and the light's step is the dark's over
the ratio.** The dark's base is black and its surface 10 levels above it; the light's surface is
white and its base 7 levels under, `oklch(0.979 0 0)`, where mono's palette page stood 5 under and
read as a smaller step than the dark's. Regions follow the step -- a card, the shell's top bar and
its sidebar -- and what is set into a region, a field or a framed button, stands on the base again.
States follow the text, below. **A surface casts no shadow in either theme**: it stands off the base
by its ground and its border alone, the same way in both. Set with the author on 2026-10-11.

**Two grounds, and every other ground an interaction rank laid over one of them.** A ground that is
neither the page nor a card -- a hover, a selection, a control's fill, a skeleton -- is not a gray
of its own but a rank: the text's own color laid thin, so it leaves whatever it lies on toward the
text in either theme, where a fixed gray lighter than a white card would sink back toward the page.
A rank is translucent, so it adapts to the ground under it. A component takes a rank, never a share.
The ranks in mono are shares of one strength:

| rank      | light | dark |
| --------- | ----- | ---- |
| `subtle`  | 0.7   | 0.7  |
| `muted`   | 0.85  | 0.8  |
| `default` | 1     | 1    |
| `strong`  | 1.5   | 1.5  |

**The dark's strength, 7 percent, is the reference, and the light's is it over
`--interaction-ratio`, 1.4**, since near black a share reads larger: one rank is one visual weight
in both themes. The light holds `muted` further off `subtle`, where the two would otherwise read as
one. **Where something chosen stands among what can be pointed at, the chosen takes `muted` and the
hover `subtle`; where only a hover shows, it takes `default`.** `inset` and `raised` were dropped
for this on 2026-10-11, with the author: `inset` had the base's value in both themes, and `raised`
was a gray darker than the light's base, so a hover on a white field sank toward the page. Ranks and
names settled with the author on 2026-10-11.

**A border is the text's own color laid thin too, by the same ratio**: `subtle` the ranks' full
strength, `default` 9 percent in the dark and `strong` 24, the light's each over 1.4. Each was a
gray or a share of its own before, `default` 8 and 9 percent and `strong` two palette grays, so the
light's borders stood nearly as heavy as the dark's. Set with the author on 2026-10-11.

**Text is set by its APCA contrast on a surface, one target per tier in both themes**: `subtle` Lc
60, `muted` 75, `default` 90, `strong` the most there is, black or white, and each theme's lightness
worked back from the target. APCA weighs a light text on a dark ground apart from a dark one on a
light ground, which is the difference the eye makes and the ratio above cannot, since text stands
far off its ground where a rank stands close. Mono's palette text had the light's lower tiers near
Lc 80 and 93 and the dark's near 33 and 51, the dark's secondary text under what a label needs; the
light's `default` sat at the light's most, beside `strong`. Set with the author on 2026-10-11.

**A hue is drawn as graphics, and is set in OKHSL: saturation 100 in both themes, and its lightness
9 deeper in the dark than in the light.** OKHSL's saturation is a share of the most chroma the
screen shows at that hue and lightness, so 100 is the hue at its purest whatever its lightness,
which oklch's absolute chroma cannot say; the dark's lightness is the author's per hue. In mono
information is OKHSL lightness 50.8 at 258.6 in the dark, success 58.9 at 147.2, critical 56.5 at
23.1, each 9 lighter in the light. **Warning is the one exception: one value in both themes**, 79.1
at 76, since a yellow already at its brightest pure point turns cream when lifted. The lift first
went the other way, the dark 9 lighter, after Geist's palette; set side by side, the dark read right
with the deeper hues and the light with the brighter. Contrast was tried as the generator before
either, a single APCA target per theme, and lost each hue's own lightness: yellow went brown and red
pink. Set with the author on 2026-10-11.

## Who is on it

The console's polished parts and space, first -- web's `spec/architecture/space.md`, "The design
system: its layers, and a style across them". The rest of the console reads its old names, each
declared in its own stylesheet as the contract's value, until each part is polished.
