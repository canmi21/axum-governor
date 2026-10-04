/**
 * Authored lengths and measured ones, kept apart.
 *
 * Geometry read from the DOM comes back in CSS pixels, and anything written back to a style is
 * divided by the live root size and serialised as rem -- see spec/styling/lengths.md. Two
 * applications now animate lengths, which is what moved these out of the site.
 */

export const DEFAULT_PIXELS_PER_REM = 16;

export function remFromDefaultPixels(value: number): string {
	return `${value / DEFAULT_PIXELS_PER_REM}rem`;
}

export function remFromMeasuredPixels(value: number, measuredRoot?: number): string {
	const root =
		measuredRoot ??
		(typeof document === 'undefined'
			? DEFAULT_PIXELS_PER_REM
			: Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
				DEFAULT_PIXELS_PER_REM);
	return `${value / root}rem`;
}
