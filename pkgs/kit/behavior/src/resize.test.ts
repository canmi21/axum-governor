import { describe, expect, it } from 'vitest';
import { clampSpan, dividerScript, dragged, rememberedWidth, type Divider } from './resize';
import { reader, type Store } from './state';

const DIVIDER: Divider = {
	key: 'test.width',
	property: '--test-width',
	span: { min: 10, max: 20, fallback: 15 },
};

function store(): Store {
	const items = new Map<string, string>();
	return {
		getItem: (key) => items.get(key) ?? null,
		setItem: (key, value) => void items.set(key, value),
	};
}

/** Run the inline script against a store and a root, and say what it set. */
function runScript(storage: Store): string | undefined {
	let set: string | undefined;
	const document = {
		documentElement: { style: { setProperty: (_: string, value: string) => (set = value) } },
	};
	new Function('localStorage', 'document', dividerScript(DIVIDER))(storage, document);
	return set;
}

describe('a divider width', () => {
	it('is held inside its span, and anything not a number is the fallback', () => {
		expect(clampSpan(5, DIVIDER.span)).toBe(10);
		expect(clampSpan(25, DIVIDER.span)).toBe(20);
		expect(clampSpan(12.5, DIVIDER.span)).toBe(12.5);
		expect(clampSpan('12', DIVIDER.span)).toBe(15);
		expect(clampSpan(Number.NaN, DIVIDER.span)).toBe(15);
	});

	it('is read back from the reader record it was remembered in', () => {
		const local = store();
		expect(rememberedWidth(local, DIVIDER)).toBe(15);
		reader.remember(local, DIVIDER.key, 18);
		expect(rememberedWidth(local, DIVIDER)).toBe(18);
		reader.remember(local, DIVIDER.key, 99);
		expect(rememberedWidth(local, DIVIDER)).toBe(20);
	});

	// The script repeats the record's shape because it runs before any module; this is what holds
	// the two readings together.
	it('is set before the first frame exactly as the module would read it', () => {
		const local = store();
		expect(runScript(local)).toBeUndefined();
		for (const stored of [18, 99, 3, 12.25]) {
			reader.remember(local, DIVIDER.key, stored);
			expect(runScript(local)).toBe(`${rememberedWidth(local, DIVIDER)}rem`);
		}
		reader.remember(local, DIVIDER.key, 'wide');
		expect(runScript(local)).toBeUndefined();
	});

	it('holds at the minimum through the margin, and asks to fold only past it', () => {
		const { span } = DIVIDER;
		expect(dragged(12, span, 2)).toEqual({ width: 12, folding: false });
		expect(dragged(9, span, 2)).toEqual({ width: 10, folding: false });
		expect(dragged(8, span, 2)).toEqual({ width: 10, folding: false });
		expect(dragged(7.9, span, 2)).toEqual({ width: 10, folding: true });
		expect(dragged(-40, span)).toEqual({ width: 10, folding: false });
	});
});
