/**
 * Each icon's optical correction, in the units of its own 24-unit grid: how far its drawing moves
 * off the center of its box, and how much it is scaled about that center. One table, so an icon is
 * corrected the same way in every app that draws it. Tabler is the system's icon family. See lib's
 * spec/design/components.md, "An icon is drawn in layers".
 */
import GitMerge from '@tabler/icons-svelte-runes/icons/git-merge';

export type IconComponent = typeof GitMerge;

export interface Optics {
	/** Right, in grid units. */
	readonly x?: number;
	/** Down, in grid units. */
	readonly y?: number;
	/** About the center; 1 is the drawing as drawn. */
	readonly scale?: number;
}

/** The side of every Tabler icon's grid. */
export const GRID = 24;

export const OPTICS: ReadonlyMap<IconComponent, Optics> = new Map([
	// Two circles and the stem on the left, one on the right: its ink centers at x 9.97, two units
	// left of the box's, and three quarters of that brings it to the eye's center.
	[GitMerge, { x: 1.5 }],
]);

/** The `viewBox` that moves and scales a drawing by `optics` while its box stays where it is. */
export function viewBoxOf({ x = 0, y = 0, scale = 1 }: Optics): string {
	const side = GRID / scale;
	const origin = (GRID - side) / 2;
	return `${origin - x} ${origin - y} ${side} ${side}`;
}

/**
 * How high the letter of `words` nearest an icon stands, as a CSS length: a capital, or a figure,
 * which stands as tall, to the capital height `--font-sans-cap-height`; anything else to the
 * lowercase's `1ex`. The icon stands `before` the words, by their first letter, or `after`, by
 * their last.
 */
export function heightBeside(
	words: string,
	side: 'before' | 'after',
): 'var(--font-sans-cap-height)' | '1ex' {
	const nearest = side === 'before' ? words.trimStart()[0] : words.trimEnd().at(-1);
	return nearest && /[\p{Lu}\p{Nd}]/u.test(nearest) ? 'var(--font-sans-cap-height)' : '1ex';
}
