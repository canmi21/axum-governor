/**
 * What the last input was, recorded on the document for the focus ring to read: `kbd` after a
 * navigation key, `pointer` after a press. `interaction.css` takes the ring away only where the
 * source is positively a pointer, so with nothing recorded the browser's guess stands. See
 * spec/kit/behavior.md, "Where focus came from".
 */

/** The keys that move focus or act on it; a letter typed into a field is not one of them. */
export const NAVIGATION = new Set([
	'Tab',
	'ArrowUp',
	'ArrowDown',
	'ArrowLeft',
	'ArrowRight',
	'Home',
	'End',
	'PageUp',
	'PageDown',
	'Enter',
	' ',
]);

/**
 * Start recording on `root`'s `data-focus-source`, captured so no handler below can hide an input
 * from it; returns the function that stops. `root` is the document's element unless a test hands
 * over another.
 */
export function trackFocusSource(
	root: HTMLElement = document.documentElement,
	target: Pick<EventTarget, 'addEventListener' | 'removeEventListener'> = document,
): () => void {
	const keyboard = (event: Event) => {
		if (NAVIGATION.has((event as KeyboardEvent).key)) root.dataset.focusSource = 'kbd';
	};
	const pointer = () => {
		root.dataset.focusSource = 'pointer';
	};
	target.addEventListener('keydown', keyboard, true);
	target.addEventListener('pointerdown', pointer, true);
	return () => {
		target.removeEventListener('keydown', keyboard, true);
		target.removeEventListener('pointerdown', pointer, true);
	};
}
