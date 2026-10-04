/**
 * Moving a reader inside one page, without the move becoming an address.
 *
 * The rule these serve is spec/styling/notes.md's: navigation within one document leaves the URL
 * alone, because a reader walking six notes should not have to press Back six times to leave a page
 * they never left. The control stays an ordinary `<a href="#id">`, so without JavaScript the native
 * jump happens instead -- worse than this, and far better than a dead control.
 */

/**
 * Whether this click is the reader asking *this page* to move.
 *
 * A modified click -- meta, control, shift, alt, or any button but the first -- is a request for a
 * new tab or window, and belongs to the browser untouched. Cancelling it would turn a familiar
 * gesture into a control that appears broken.
 */
export function movesThisPage(event: {
	button: number;
	metaKey: boolean;
	ctrlKey: boolean;
	shiftKey: boolean;
	altKey: boolean;
	defaultPrevented: boolean;
}): boolean {
	return (
		!event.defaultPrevented &&
		event.button === 0 &&
		!event.metaKey &&
		!event.ctrlKey &&
		!event.shiftKey &&
		!event.altKey
	);
}

/**
 * Scroll to something in this document, smoothly unless the reader asked for less motion.
 *
 * `scrollIntoView` rather than a computed offset, so a target keeps whatever `scroll-margin-top`
 * its own styling gives it -- `.jump-target` holds a share of the viewport above anything reached
 * this way, and that number belongs beside the rule that explains it rather than in here.
 */
export function jumpTo(target: Element): void {
	const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	target.scrollIntoView({ behavior: reduced ? 'instant' : 'smooth', block: 'start' });
}

/** The element an in-page link points at, or nothing if it points outside this document. */
export function targetOf(link: HTMLAnchorElement): Element | null {
	const hash = link.getAttribute('href');
	if (!hash?.startsWith('#') || hash.length < 2) return null;
	return document.getElementById(decodeURIComponent(hash.slice(1)));
}
