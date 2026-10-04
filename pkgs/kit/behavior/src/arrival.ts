/**
 * Whether this document is still the one the reader arrived in.
 *
 * What it gates is the settle a measurement needs -- see spec/styling/first-paint.md. A component
 * may not decide this for itself: it mounts on every navigation that renders it, and one asking
 * whether it is mounting for the first time would answer yes every time.
 */

/**
 * False from the moment a client navigation starts, and never true again.
 *
 * Set in `beforeNavigate` and not in `afterNavigate`, because a navigation builds its page before
 * `afterNavigate` runs and the components being built have to see the new answer already.
 */
let navigated = false;

/** Called once, by the root layout. */
export function leaveArrival(): void {
	navigated = true;
}

/**
 * Whether a settle drawn from a measurement may animate.
 *
 * Read at init rather than in an effect: an effect runs after the frame this is deciding about.
 */
export function arriving(): boolean {
	return !navigated;
}
