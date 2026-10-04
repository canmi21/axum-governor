/**
 * The one way a disclosure opens and closes on this site.
 *
 * Animating to `height: auto` is not possible, so the height is animated between two measured
 * numbers and handed back to `auto` at rest -- a panel pinned to a measured height would stop
 * following its content on resize or a late font.
 *
 * Shared rather than copied: two disclosures with the same spring written out twice are two
 * numbers to keep in step with no way to tell later whether they were meant to be equal. The
 * search panel uses the same spring for a different measurement -- see web's spec/search.md.
 */

import { NEGLIGIBLE_PIXELS, pressMotion, prefersReducedMotion } from '../../motion/src/index.ts';
import { animate } from 'motion';
import { DEFAULT_PIXELS_PER_REM, remFromMeasuredPixels } from '../../units/src/index.ts';

/**
 * A panel answering a press, not a thing being thrown.
 *
 * Derived from the distance rather than fixed, so a short disclosure is a quick one, and a tween
 * rather than a spring so it ends when it says it will -- see `@canmi/kit/motion` for the
 * measurements behind both.
 */
export const collapseMotion = pressMotion;

export type AnimationControl = { stop: () => void };

/** Where a disclosure is, including the two states it is only passing through. */
export type CollapsePhase = 'collapsed' | 'collapsing' | 'expanded' | 'expanding';

export { prefersReducedMotion };

/**
 * Drive `element`'s height from where it is to `targetPixels`, calling `onSettle` when it lands.
 *
 * Returns the control to stop it, or nothing when no animation ran (reduced motion, or too small
 * a distance to see) -- `onSettle` has already run either way, so a caller need not ask which.
 * `onSettle` gets the control that finished, since a stopped animation is not guaranteed silent
 * and a caller with a newer one running compares before acting on it. `onFrame` gets each height
 * in transit, for anything that must track the panel rather than react once it lands.
 */
export function animateHeight(
	element: HTMLElement,
	targetPixels: number,
	onSettle: (finished?: AnimationControl) => void,
	onFrame?: (heightPixels: number) => void,
): AnimationControl | undefined {
	const currentPixels = element.getBoundingClientRect().height;
	element.style.height = remFromMeasuredPixels(currentPixels);

	if (prefersReducedMotion() || Math.abs(currentPixels - targetPixels) < NEGLIGIBLE_PIXELS) {
		onFrame?.(targetPixels);
		onSettle();
		return undefined;
	}

	const rootPixels =
		Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
		DEFAULT_PIXELS_PER_REM;
	let control: AnimationControl;
	control = animate(currentPixels, targetPixels, {
		...collapseMotion(targetPixels - currentPixels),
		onUpdate: (height) => {
			element.style.setProperty('height', remFromMeasuredPixels(Math.max(0, height), rootPixels));
			onFrame?.(height);
		},
		onComplete: () => onSettle(control),
	});
	return control;
}
