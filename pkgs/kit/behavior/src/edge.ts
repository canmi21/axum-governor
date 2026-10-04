/**
 * A panel that comes out when the pointer runs to one edge of the window, and goes back when the
 * pointer moves well clear of it.
 *
 * Running to the edge is a request that costs nothing to make and nothing to withdraw: no click,
 * and moving on is the way out. Two margins decide it, both in pixels -- how close to the edge
 * counts as having arrived, and how far past the panel's inner side counts as having left. The
 * second is wider than zero so that a pointer following the panel's own edge does not drop it.
 *
 * Only a mouse asks this way. A touch has no pointer resting anywhere to be near an edge.
 */

export type Side = 'left' | 'right';

export type EdgeWatch = {
	side: Side;
	/** How close to the window's edge the pointer comes to bring the panel out. */
	within: number;
	/** How far past the panel's inner side the pointer goes before the panel goes back. */
	release: number;
};

/**
 * What a pointer at `x`, in a window `width` wide, asks of a panel watching `watch.side`: to come
 * out, to go back, or nothing. `panel` is where the panel stands while it is out.
 */
export function edgeIntent(
	x: number,
	width: number,
	watch: EdgeWatch,
	out: boolean,
	panel?: { left: number; right: number },
): 'reveal' | 'conceal' | undefined {
	if (!out) {
		const near = watch.side === 'left' ? x <= watch.within : x >= width - watch.within;
		return near ? 'reveal' : undefined;
	}
	if (!panel) return undefined;
	const away =
		watch.side === 'left' ? x > panel.right + watch.release : x < panel.left - watch.release;
	return away ? 'conceal' : undefined;
}

/**
 * A `pointermove` handler for the document, driving one panel from one edge.
 *
 * `live` says whether the edge is listening at all -- a panel already standing open by a click is
 * not the edge's to take down, and one with nowhere to come out to has no edge. `out` says whether
 * the edge brought it out, which is the only case in which moving away takes it back.
 */
export function edgeReveal(
	watch: EdgeWatch & {
		live: () => boolean;
		out: () => boolean;
		panel: () => Element | undefined;
		reveal: () => void;
		conceal: () => void;
	},
): (event: PointerEvent) => void {
	return (event) => {
		if (event.pointerType !== 'mouse' || !watch.live()) return;
		const intent = edgeIntent(
			event.clientX,
			window.innerWidth,
			watch,
			watch.out(),
			watch.panel()?.getBoundingClientRect(),
		);
		if (intent === 'reveal') watch.reveal();
		else if (intent === 'conceal') watch.conceal();
	};
}
