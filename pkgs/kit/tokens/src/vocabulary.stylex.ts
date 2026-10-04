/**
 * The values the visual layer repeats, each with one name, already written character for
 * character by three or more components -- a value only two share stays a literal. Two spellings
 * of one idea keep two names: they differ outside the default root size. See spec/todo/css.md.
 *
 * `defineConsts`, never `defineVars`: a var group rewrites every reading declaration into a
 * `var()`, where a const inlines and leaves the declaration as written; only the hashed class
 * name changes. See spec/architecture/css/extraction.md.
 *
 * The `.stylex`/`.stylex.const` filename is the compiler's own requirement (`vite.config.ts`
 * states the alias), and a key not defined here is not an error here: the importing file gets a
 * proxy resolving any string, so a typo becomes a `var()` nothing declares and the browser drops
 * silently -- only `tsc` catches it.
 */

import * as stylex from '@stylexjs/stylex';

/**
 * Tailwind's radius scale, which is where all five of these came from -- `--radius-sm` through
 * `--radius-xl`, and the pill. `624.9375rem` is the site's other pill and is deliberately
 * not here: it computes to a different number and it is the one value on the site that loses a
 * digit in serialisation. See spec/todo/todo.md.
 */
export const radius = stylex.defineConsts({
	sm: '0.25rem',
	md: '0.375rem',
	lg: '0.5rem',
	xl: '0.75rem',
	full: 'calc(infinity * 1px)',
});

/**
 * The type ladder, named by its size in pixels at the default root.
 *
 * Seven steps, one pixel apart, and only two of them are Tailwind's -- its `xs` step is `px12`
 * and its `sm` step is `px14`. A t-shirt ladder would therefore have had to give those letters
 * meanings that disagree with the utilities the components' own comments still cite, so the
 * figure is the name and the unit stays `rem`. `px9` is the floor and a treemap tile is the only
 * thing down there: a ladder holds a step because it is one. See spec/todo/todo.md.
 */
export const text = stylex.defineConsts({
	px9: '0.5625rem',
	px10: '0.625rem',
	px11: '0.6875rem',
	px12: '0.75rem',
	px13: '0.8125rem',
	px14: '0.875rem',
	px15: '0.9375rem',
});

/**
 * Line heights written as a ratio: Tailwind's ladder, an eighth apart above the floor.
 *
 * Five steps, and every unitless line in the tree is its nearest one at a worst error of 0.075.
 * What stood here before was nine ratios between 1.2 and 1.625, which is nine decisions taken
 * separately rather than a scale. `none` is the floor and not a step on the eighths: a box that
 * is the padding around one line, never two.
 */
export const line = stylex.defineConsts({
	none: 1,
	tight: 1.25,
	snug: 1.375,
	base: 1.5,
	relaxed: 1.625,
});

/**
 * Line heights written as a length, which is a different kind of thing from the ratios above
 * and is named apart from them for that reason.
 *
 * Each is the line its own step computes to, where the ratio that would say so does not
 * terminate: `calc(1.25 / 0.875)` under `text.px14`, `calc(1 / 0.75)` under `text.px12`. So the
 * name is the length, on the same footing as the type ladder. See
 * spec/architecture/css/authoring.md.
 */
export const leading = stylex.defineConsts({
	px16: '1rem',
	px20: '1.25rem',
});

/**
 * The body weight a reset puts back, Tailwind's medium, the site's own step above it that no
 * utility writes, and Tailwind's semibold, which a section title and the player's value badges
 * take. See spec/todo/todo.md.
 */
export const weight = stylex.defineConsts({
	normal: 400,
	medium: 500,
	strong: 560,
	semibold: 600,
});

/**
 * The tracking small uppercase takes, which is the site's one letter-spacing decision. The other
 * value in the tree is on lowercase at one site, and is a different one. See spec/todo/css.md.
 */
export const tracking = stylex.defineConsts({
	caps: '0.02em',
});

/**
 * Figures that hold their width as they change, which eight components ask for in eight places.
 * One value rather than a scale, and the only ramp property the tree spells exactly one way.
 */
export const figures = stylex.defineConsts({
	tabular: 'tabular-nums',
});

/**
 * Border widths, and the two hairlines are two names on purpose.
 *
 * They are the same length at the default root size and at no other, so a reader who enlarges
 * text sees them part company. Which one a hairline should be is a real question and it is not
 * answered here. See spec/todo/todo.md.
 */
export const border = stylex.defineConsts({
	hairlinePx: '1px',
	hairlineRem: '0.0625rem',
	doublePx: '2px',
});

/** Tailwind's 200ms step. No other duration on the site repeats across three components. */
export const duration = stylex.defineConsts({
	base: '200ms',
});

/** Tailwind's `--ease-in-out`, which is also the curve its transition utilities default to. */
export const easing = stylex.defineConsts({
	inOut: 'cubic-bezier(0.4, 0, 0.2, 1)',
});

/**
 * What Tailwind's colour transition compiles to, the longest string the site retypes.
 *
 * Three of the ten are Tailwind's private gradient variables and this site sets none of them.
 * They stay because the measure of a migration is the computed value, and whether they belong in
 * our source at all is a question one name does not answer -- it only gives it one place to be
 * answered in. See spec/todo/todo.md.
 */
export const transition = stylex.defineConsts({
	colors:
		'color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, --tw-gradient-from, --tw-gradient-via, --tw-gradient-to',
});

/**
 * The two monospace stacks, which are not the same stack.
 *
 * Tailwind's theme variable carries Monaco, Liberation Mono and Courier New; the spelled-out one
 * does not, so a machine with Monaco and without Menlo renders two different fonts on one page.
 * Which is right is a question about the fonts rather than the layering. See spec/todo/css.md.
 */
export const family = stylex.defineConsts({
	monoTheme: 'var(--font-mono)',
	monoSpelled: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
});
