/**
 * The gestures shared by everything here that moves.
 *
 * Only the shape of a movement lives here, never the thing being moved: a height animation needs
 * rem conversion the site owns, and a sliding indicator needs geometry a management surface owns.
 * What both would otherwise write out separately is the timing.
 *
 * The site's disclosure was first and the desktop client's tab indicator was the second, which is
 * the pair that moved this out of `apps/site/src/lib/client/collapse.ts`.
 */

/**
 * The move this scale is anchored to: a group of four articles folding, measured.
 *
 * Everything else is derived from it, so the one gesture that was already right stays exactly
 * where it was and the others move toward it.
 */
const REFERENCE_PIXELS = 178;
const REFERENCE_SECONDS = 0.222;

/**
 * Floors and ceilings on the derived time, in seconds.
 *
 * A few pixels would otherwise finish inside one frame and read as a jump, and a very tall panel
 * would take long enough that the press stops feeling connected to it.
 */
const MIN_SECONDS = 0.1;
const MAX_SECONDS = 0.45;

/** Under this a move is a flicker rather than a motion, so it plays at all only above it. */
export const NEGLIGIBLE_PIXELS = 0.5;

/**
 * CSS's own `ease`, which is the flattest curve that still starts promptly.
 *
 * Measured as peak movement in the worst 16ms frame over the reference fold: a linear ramp
 * gives 13.7px, the ease-out quintic this replaces gave 56.8px -- a third of the distance in
 * one frame, reported as graininess rather than a dropped frame. This curve peaks at 30.6px
 * and opens at 11.1, 46% flatter at the top and still visible on the first frame. A curve
 * starting at rest is flatter still, and was rejected: stillness reads as unheard.
 */
const EASE = [0.25, 0.1, 0.25, 1] as const;

/**
 * How long a surface takes to travel a distance.
 *
 * Not a constant speed: a fixed spring settles in about the same time regardless of distance
 * (0.19px/ms for a short move against 0.40 for a long one), while a constant speed instead
 * makes a small panel finish in a tenth of a second, a flash rather than a movement.
 *
 * So time grows with the square root of distance: four times as far takes twice as long, not
 * four times. Against it: 86px goes 108ms to 154ms, 600px goes 750ms to 408ms.
 */
export function pressMotion(distancePixels: number): {
	duration: number;
	ease: readonly [number, number, number, number];
} {
	const scaled = REFERENCE_SECONDS * Math.sqrt(Math.abs(distancePixels) / REFERENCE_PIXELS);
	return { duration: Math.min(MAX_SECONDS, Math.max(MIN_SECONDS, scaled)), ease: EASE };
}

/**
 * The two curves an indicator travels on, and they describe different things.
 *
 * A bar crossing a strip has a centre that moves and a width that adapts; driving both on one
 * curve conflates them into a rectangle redrawn at successive positions -- correct, and inert.
 * `CENTRE` and `WIDTH` run separately over one duration instead, so the bar reads as moving and
 * resizing at once: `CENTRE` leaves decisively, `WIDTH` flatter so it neither snaps ahead nor
 * lags at the wrong length.
 */
const CENTRE = [0.32, 0.72, 0.24, 1] as const;
const WIDTH = [0.4, 0, 0.2, 1] as const;

/** A bar travelling between two tabs, rather than a surface opening. */
const TRAVEL_REFERENCE_PIXELS = 60;
const TRAVEL_REFERENCE_SECONDS = 0.24;
const TRAVEL_MIN_SECONDS = 0.18;
const TRAVEL_MAX_SECONDS = 0.38;

/**
 * How an indicator crosses to its new tab.
 *
 * Slower for its distance than a panel opening, and deliberately: a tab strip's hops are short
 * enough that the panel curve would be over before the movement could be read as one. Scaled by
 * the same square root for the same reason, with its own anchor because it is a different gesture.
 */
export function travelMotion(distancePixels: number): {
	duration: number;
	centre: readonly [number, number, number, number];
	width: readonly [number, number, number, number];
} {
	const scaled =
		TRAVEL_REFERENCE_SECONDS * Math.sqrt(Math.abs(distancePixels) / TRAVEL_REFERENCE_PIXELS);
	return {
		duration: Math.min(TRAVEL_MAX_SECONDS, Math.max(TRAVEL_MIN_SECONDS, scaled)),
		centre: CENTRE,
		width: WIDTH,
	};
}

/**
 * A control resizing because its content changed.
 *
 * A spring here where a panel gets a tween, and the overshoot is deliberate: a swapped label
 * is a thing pushed or pulled, and a little give at the end reads as elastic. Stiffness 420
 * against damping 28 is a ratio near 0.74, so it overshoots and comes back.
 *
 * Rest thresholds are in pixels, since the animated value is a width: the library's own
 * default of 0.01 held a spring open over a hundredth of a pixel, slow enough to measure.
 */
export function contentMotion(): {
	type: 'spring';
	stiffness: number;
	damping: number;
	mass: number;
	restDelta: number;
	restSpeed: number;
} {
	return { type: 'spring', stiffness: 420, damping: 28, mass: 0.85, restDelta: 0.5, restSpeed: 10 };
}

export function prefersReducedMotion(): boolean {
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
