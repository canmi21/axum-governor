/**
 * The measures every component draws with, one value each and the same under every style: a style
 * changes colors, never a size. `defineConsts`, so a reading declaration keeps the value as written.
 * The `.stylex` in the name is the compiler's own requirement. See lib's spec/design/styles.md, "The
 * scale".
 */

import * as stylex from '@stylexjs/stylex';

/** A corner's rounding, smallest to the pill; the same steps and values as Tailwind's `rounded-*`. */
export const radius = stylex.defineConsts({
	sm: '0.25rem',
	md: '0.375rem',
	lg: '0.5rem',
	xl: '0.75rem',
	full: 'calc(infinity * 1px)',
});

/** Type sizes named for what they set, smallest first. */
export const fontSize = stylex.defineConsts({
	/** A note beside something else: a status, a count under a name. */
	caption: '0.75rem',
	/** A label, and what is said second about a thing. */
	label: '0.8125rem',
	/** Running text, a link, a name. */
	body: '0.875rem',
});

/** A line's height to its type, named and valued as Tailwind's `leading-*`. */
export const lineHeight = stylex.defineConsts({
	relaxed: 1.625,
});

export const fontWeight = stylex.defineConsts({
	regular: 400,
	medium: 500,
	semibold: 600,
});

export const borderWidth = stylex.defineConsts({
	/** One device pixel's rule at the default density. */
	hairline: '1px',
});

export const duration = stylex.defineConsts({
	/** A color or a ground changing under the pointer. */
	base: '200ms',
});
