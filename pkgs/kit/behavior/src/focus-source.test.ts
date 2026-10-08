import { describe, expect, it } from 'vitest';
import { trackFocusSource } from './focus-source';

/** A stand-in for the document: listeners kept by type, fired by hand. */
function stage() {
	const listeners = new Map<string, (event: Event) => void>();
	const root = { dataset: {} as Record<string, string> } as unknown as HTMLElement;
	const target = {
		addEventListener: (type: string, listener: (event: Event) => void) => listeners.set(type, listener),
		removeEventListener: (type: string) => listeners.delete(type),
	} as unknown as Pick<EventTarget, 'addEventListener' | 'removeEventListener'>;
	const fire = (type: string, key?: string) => listeners.get(type)?.({ key } as unknown as Event);
	return { root, target, fire, listeners };
}

describe('trackFocusSource', () => {
	it('records nothing until there is an input, so the browser guess stands', () => {
		const { root, target } = stage();
		trackFocusSource(root, target);
		expect(root.dataset.focusSource).toBeUndefined();
	});

	it('marks a navigation key as the keyboard, and a press as the pointer', () => {
		const { root, target, fire } = stage();
		trackFocusSource(root, target);
		fire('keydown', 'Tab');
		expect(root.dataset.focusSource).toBe('kbd');
		fire('pointerdown');
		expect(root.dataset.focusSource).toBe('pointer');
	});

	it('leaves a letter typed into a field alone', () => {
		const { root, target, fire } = stage();
		trackFocusSource(root, target);
		fire('pointerdown');
		fire('keydown', 'a');
		expect(root.dataset.focusSource).toBe('pointer');
	});

	it('stops listening when it is stopped', () => {
		const { root, target, listeners } = stage();
		trackFocusSource(root, target)();
		expect(listeners.size).toBe(0);
		expect(root.dataset.focusSource).toBeUndefined();
	});
});
