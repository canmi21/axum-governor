import { describe, expect, it } from 'vitest';
import { label, matches, shortcut } from './shortcut';

type Press = Partial<KeyboardEvent> & { prevented?: boolean };

/** As much of a keydown as this module reads, with `preventDefault` recorded. */
function press(over: Partial<KeyboardEvent> = {}): KeyboardEvent & Press {
	const event: Press = {
		key: 'k',
		code: 'KeyK',
		metaKey: true,
		ctrlKey: false,
		shiftKey: false,
		altKey: false,
		target: null,
		...over,
		preventDefault() {
			event.prevented = true;
		},
	};
	return event as KeyboardEvent & Press;
}

/** A window that only holds one keydown listener, and can be pressed. */
function host() {
	let listener: ((event: Event) => void) | undefined;
	return {
		addEventListener: (_: string, next: (event: Event) => void) => void (listener = next),
		removeEventListener: () => void (listener = undefined),
		press: (event: KeyboardEvent) => listener?.(event),
		bound: () => listener !== undefined,
	};
}

describe('a binding', () => {
	it('takes either command key, and the letter in either case', () => {
		expect(matches(press(), 'k')).toBe(true);
		expect(matches(press({ metaKey: false, ctrlKey: true }), 'K')).toBe(true);
		expect(matches(press({ key: 'K', shiftKey: true }), 'k', { shift: true })).toBe(true);
	});

	it('wants the command key, and exactly the modifiers asked for', () => {
		expect(matches(press({ metaKey: false }), 'k')).toBe(false);
		expect(matches(press({ key: 'j' }), 'k')).toBe(false);
		expect(matches(press({ shiftKey: true }), 'k')).toBe(false);
		expect(matches(press({ altKey: true }), 'k')).toBe(false);
		expect(matches(press(), 'k', { shift: true })).toBe(false);
	});

	// Option on a Mac rewrites the key it modifies; the physical key still says which it was.
	it('knows the key under Option by where it is', () => {
		expect(matches(press({ key: '˚', altKey: true }), 'k', { alt: true })).toBe(true);
	});

	// An autofill or a harness dispatches a plain Event under the keydown name, with no key at all.
	it('reads a keydown without a key as no match rather than throwing', () => {
		expect(matches(press({ key: undefined }), 'k')).toBe(false);
	});
});

describe('the window binding', () => {
	it('runs on a match and keeps the browser from acting on it too', () => {
		const target = host();
		let ran = 0;
		shortcut('k', () => (ran += 1), { target });
		const event = press();
		target.press(event);
		target.press(press({ key: 'j' }));
		expect(ran).toBe(1);
		expect(event.prevented).toBe(true);
	});

	it('stands down inside a text field, unless told the field is its too', () => {
		const fields = [
			{ tagName: 'INPUT' },
			{ tagName: 'TEXTAREA' },
			{ tagName: 'SELECT' },
			{ tagName: 'DIV', isContentEditable: true },
		] as unknown as EventTarget[];
		const target = host();
		let ran = 0;
		shortcut('k', () => (ran += 1), { target });
		for (const field of fields) {
			const event = press({ target: field });
			target.press(event);
			expect(event.prevented).toBeUndefined();
		}
		target.press(press({ target: { tagName: 'BUTTON' } as unknown as EventTarget }));
		expect(ran).toBe(1);

		const anywhere = host();
		shortcut('k', () => (ran += 1), { target: anywhere, inFields: true });
		for (const field of fields) anywhere.press(press({ target: field }));
		expect(ran).toBe(1 + fields.length);
	});

	it('unbinds with the function it returns', () => {
		const target = host();
		const stop = shortcut('k', () => {}, { target });
		expect(target.bound()).toBe(true);
		stop();
		expect(target.bound()).toBe(false);
	});
});

describe('the label', () => {
	it('is the command glyph on Apple platforms and Ctrl elsewhere', () => {
		expect(label('k', 'MacIntel')).toBe('⌘K');
		expect(label('k', 'macOS')).toBe('⌘K');
		expect(label('k', 'iPhone')).toBe('⌘K');
		expect(label('k', 'Win32')).toBe('Ctrl K');
		expect(label('k', 'Linux x86_64')).toBe('Ctrl K');
	});

	// The server has no document, and answers what an Apple browser will hydrate to.
	it('is the command glyph on the server', () => {
		expect(label('k')).toBe('⌘K');
	});
});
