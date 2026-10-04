/**
 * A divider that a reader drags to size the region beside it, and that remembers where it was left.
 *
 * Three halves of one thing, kept in one file so they cannot disagree about the record: the range a
 * width may take, the handle that changes it, and the script that puts the remembered width on the
 * page before the first frame. The width lives in the `reader` record (`state.ts`), because how
 * wide somebody likes a column is a fact about them rather than about this tab.
 *
 * The width is a custom property on the root element, not a style on the region. The script runs
 * before the region exists, so the root is the only element both it and the handle can reach, and
 * the region reads the property with a fallback -- which is also what the server renders, so a
 * page with nothing remembered draws the same width before and after hydration.
 */

import { reader, type Store } from './state.ts';

/** The widths a region may take, in rem so they follow the reader's text size. */
export type Span = { min: number; max: number; fallback: number };

/** Where a remembered width is kept, and what it is written to. */
export type Divider = {
	/** The key in the `reader` record, flat and dotted: see spec/engagement.md. */
	key: string;
	/** The custom property the region reads, set on the root element. */
	property: `--${string}`;
	span: Span;
};

/** `rem` held inside the span, and the fallback in place of anything not a finite number. */
export function clampSpan(rem: unknown, span: Span): number {
	if (typeof rem !== 'number' || !Number.isFinite(rem)) return span.fallback;
	return Math.min(span.max, Math.max(span.min, rem));
}

/** The width remembered for this divider, held inside its span. */
export function rememberedWidth(storage: Store, divider: Divider): number {
	return clampSpan(reader.recall(storage, divider.key, divider.span.fallback), divider.span);
}

/**
 * The script that sets the remembered width before the first frame, as a string for the head. It
 * reads the record itself, since it runs before any module loads; what it repeats of the record's
 * shape -- one JSON object under `state`, the width a number under the key -- is held against
 * `rememberedWidth` by the test beside this file. Every value goes through `JSON.stringify`, and
 * all are this repository's own constants.
 */
export function dividerScript(divider: Divider): string {
	const { key, property, span } = divider;
	return (
		`(function(){try{var r=localStorage.getItem("state");if(!r)return;` +
		`var v=JSON.parse(r)[${JSON.stringify(key)}];if(typeof v!=="number"||!isFinite(v))return;` +
		`v=Math.min(${span.max},Math.max(${span.min},v));` +
		`document.documentElement.style.setProperty(${JSON.stringify(property)},v+"rem")}catch(e){}})()`
	);
}

/** The step the arrow keys move a divider by, in rem. */
const KEY_STEP = 1;

/**
 * Dragging past the minimum to fold the region away, for a divider whose region can fold.
 *
 * Past the minimum the width holds, and for `beyond` rem the drag means nothing more: a writer
 * who overshot the edge by a little has not asked for anything. Past that it is a request to fold,
 * `intent` says so while it holds so the page can show what letting go will do, and letting go
 * then calls `fold`. The remembered width is left as it was before the drag, so unfolding returns
 * to it rather than to the minimum the drag was pinned at.
 */
export type Fold = {
	beyond: number;
	intent: (folding: boolean) => void;
	fold: () => void;
};

/** Where a drag has taken a divider: the width it shows, and whether it is asking to fold. */
export function dragged(
	rem: number,
	span: Span,
	beyond?: number,
): { width: number; folding: boolean } {
	return {
		width: clampSpan(rem, span),
		folding: beyond !== undefined && rem < span.min - beyond,
	};
}

/**
 * Make `handle` drag the divider: a Svelte action, and a plain function anywhere else. A drag
 * writes the property every move and the record once on release; the arrows move a step and
 * remember at once; a double click forgets, which returns the fallback. While held, the document
 * takes the resize cursor and gives up selection, since the pointer outruns the handle and the
 * cursor would flicker and the page be selected. With `fold`, dragging well past the minimum folds
 * the region; see `Fold`.
 */
export function resizeHandle(
	handle: HTMLElement,
	divider: Divider & { fold?: Fold },
	storage: Store = localStorage,
): { destroy: () => void } {
	const root = document.documentElement;
	let width = rememberedWidth(storage, divider);
	let start: { x: number; width: number } | undefined;
	let folding = false;

	const rootPixels = () => Number.parseFloat(getComputedStyle(root).fontSize) || 16;

	function show(rem: number) {
		width = clampSpan(rem, divider.span);
		root.style.setProperty(divider.property, `${width}rem`);
		handle.setAttribute('aria-valuenow', width.toFixed(1));
	}

	function down(event: PointerEvent) {
		if (event.button !== 0) return;
		event.preventDefault();
		handle.setPointerCapture(event.pointerId);
		start = { x: event.clientX, width };
		root.style.cursor = 'col-resize';
		root.style.userSelect = 'none';
	}

	function move(event: PointerEvent) {
		if (!start) return;
		const at = dragged(
			start.width + (event.clientX - start.x) / rootPixels(),
			divider.span,
			divider.fold?.beyond,
		);
		show(at.width);
		if (at.folding !== folding) {
			folding = at.folding;
			divider.fold?.intent(folding);
		}
	}

	/** Let go of the drag; `released` is a real release, where a cancelled pointer is not one. */
	function end(event: PointerEvent, released: boolean) {
		if (!start) return;
		const before = start.width;
		start = undefined;
		if (handle.hasPointerCapture(event.pointerId)) handle.releasePointerCapture(event.pointerId);
		root.style.removeProperty('cursor');
		root.style.removeProperty('user-select');
		if (folding) {
			folding = false;
			divider.fold?.intent(false);
			show(before);
			if (released) divider.fold?.fold();
			return;
		}
		reader.remember(storage, divider.key, width);
	}

	const up = (event: PointerEvent) => end(event, true);
	const cancel = (event: PointerEvent) => end(event, false);

	function key(event: KeyboardEvent) {
		const step = event.key === 'ArrowLeft' ? -KEY_STEP : event.key === 'ArrowRight' ? KEY_STEP : 0;
		if (step === 0) return;
		event.preventDefault();
		show(width + step);
		reader.remember(storage, divider.key, width);
	}

	function reset() {
		reader.forget(storage, divider.key);
		show(divider.span.fallback);
	}

	// The values are rem, the unit the width is kept in; a separator's range need only be consistent.
	handle.setAttribute('aria-valuemin', String(divider.span.min));
	handle.setAttribute('aria-valuemax', String(divider.span.max));
	handle.setAttribute('aria-valuenow', width.toFixed(1));
	handle.addEventListener('pointerdown', down);
	handle.addEventListener('pointermove', move);
	handle.addEventListener('pointerup', up);
	handle.addEventListener('pointercancel', cancel);
	handle.addEventListener('keydown', key);
	handle.addEventListener('dblclick', reset);

	return {
		destroy() {
			handle.removeEventListener('pointerdown', down);
			handle.removeEventListener('pointermove', move);
			handle.removeEventListener('pointerup', up);
			handle.removeEventListener('pointercancel', cancel);
			handle.removeEventListener('keydown', key);
			handle.removeEventListener('dblclick', reset);
		},
	};
}
