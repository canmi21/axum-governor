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
the dark's own element, and nothing declared outside the contract's groups.

**Mono is the first style, `@canmi/design/styles/mono.css`**, the console's neutral look. It
imports its palette itself, so an app imports the style and nothing under it. Its dark is `.dark`
and `[data-theme='dark']`, as the palette's is, set by `@canmi/kit/theme` until the theme moves here too -- [../kit/theme.md](../kit/theme.md).
**Every name is declared on the dark's own element as well as the root**, since a name made of
others is worked out where it is declared: declared on the root alone, a dark part of a light page
would keep the light's values.

## How a name is made

**`--{group}-{role}`**: the group is the property the value is laid on, the role what it does
there. Where a group ranks its roles, the ranks are one scale for every group: `subtle`, `muted`,
`default`, `strong`. Every word is the full word, and none is a component's: a name says what a
value is for, not who first wanted it.

| group         | names                                                                                              |
| ------------- | -------------------------------------------------------------------------------------------------- |
| `background`  | `base`, under everything; `inset`, set into a surface; `surface`; `raised`, a step above a surface |
| `border`      | `subtle`, `default`, `strong`                                                                      |
| `foreground`  | `subtle`, `muted`, `default`, `strong`: text, and the icons and marks drawn as text is             |
| `interaction` | `hover`, `selected`, `selected-opaque`, `focus`                                                    |
| `accent`      | `--accent` alone: what is the system's own and live                                                |
| `status`      | `success`, `info`, `warning`, `critical`                                                           |
| `shadow`      | `surface`, the shadow a surface casts                                                              |
| `font`        | `sans`, `mono`, and `sans-cap-height`                                                              |

**`--font-sans-cap-height` is the sans's capital height**, `H` from its baseline to its top, what
CSS's `cap` unit should give and Chrome does not read right from this sans. It names its font,
since another face's capitals stand at another height.

**`--interaction-focus` and `--accent` are the same blue in mono, under two names**, so the focus
ring can move without every mark moving with it.

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

## Who is on it

The console's polished parts and space, first -- web's `spec/architecture/space.md`, "The design
system: its layers, and a style across them". The rest of the console reads its old names, each
declared in its own stylesheet as the contract's value, until each part is polished.
