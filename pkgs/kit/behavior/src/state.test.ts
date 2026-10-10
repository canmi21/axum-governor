import { beforeEach, describe, expect, it } from 'vitest';
import { record, type Step, type Store } from './state';

const KEY = 'state';

/** A record with no steps, which is what a project's first record is. */
const reader = record(KEY, []);

const VERSION = reader.version;

/** As much of a store as this module touches, which is why it can be this small. */
function store(): Store & { items: Map<string, string> } {
	const items = new Map<string, string>();
	return {
		items,
		getItem: (key) => items.get(key) ?? null,
		setItem: (key, value) => void items.set(key, value),
	};
}

describe('a stored record', () => {
	let local: ReturnType<typeof store>;
	beforeEach(() => (local = store()));

	it('writes one key, holding the version and flat dotted names', () => {
		reader.remember(local, 'support.preferred', true);
		expect(JSON.parse(local.getItem(KEY) ?? '{}')).toEqual({
			version: VERSION,
			'support.preferred': true,
		});
		expect(local.items.size).toBe(1);
	});

	it('keeps the other facts when one of them changes', () => {
		reader.remember(local, 'support.preferred', true);
		reader.remember(local, 'reader.something', 'else');
		reader.remember(local, 'support.preferred', false);
		expect(reader.recall(local, 'support.preferred', true)).toBe(false);
		expect(reader.recall(local, 'reader.something', '')).toBe('else');
	});

	it('falls back where nothing is stored, or where the type is not what was asked', () => {
		expect(reader.recall(local, 'support.preferred', false)).toBe(false);
		reader.remember(local, 'support.preferred', 'yes');
		expect(reader.recall(local, 'support.preferred', false)).toBe(false);
	});

	it('falls back on a record that is not a record', () => {
		for (const junk of ['null', '[]', '"text"', '{oops', '7']) {
			local.setItem(KEY, junk);
			expect(reader.recall(local, 'support.preferred', false)).toBe(false);
		}
	});

	it('leaves a record from a later version alone and still reads what it knows', () => {
		// The reader's other device runs a later build, which is what cloud sync will make
		// ordinary. Its keys are not this build's to discard.
		local.setItem(
			KEY,
			JSON.stringify({ version: VERSION + 9, 'support.preferred': true, 'from.tomorrow': 1 }),
		);
		expect(reader.recall(local, 'support.preferred', false)).toBe(true);
		reader.remember(local, 'support.preferred', false);
		const stored = JSON.parse(local.getItem(KEY) ?? '{}');
		expect(stored.version).toBe(VERSION + 9);
		expect(stored['from.tomorrow']).toBe(1);
	});

	it('forgets one key without disturbing the record around it', () => {
		reader.remember(local, 'support.preferred', true);
		reader.remember(local, 'reader.something', 'else');
		reader.forget(local, 'support.preferred');
		const stored = JSON.parse(local.getItem(KEY) ?? '{}');
		expect(stored).toEqual({ version: VERSION, 'reader.something': 'else' });
	});
});

describe('a record declared with steps', () => {
	let local: ReturnType<typeof store>;
	beforeEach(() => (local = store()));

	/** Write a record as an older build would have left it. */
	function stored(state: Record<string, unknown>): void {
		local.setItem(KEY, JSON.stringify(state));
	}

	it('stands one version above its steps', () => {
		expect(reader.version).toBe(1);
		expect(record(KEY, [{}, {}]).version).toBe(3);
	});

	it('runs every step from the stored version on, in order', () => {
		const steps: Step[] = [
			{ count: (value) => `${value}+a` },
			{ count: (value) => `${value}+b` },
			{ count: (value) => `${value}+c` },
		];
		stored({ version: 2, count: '2' });
		expect(record(KEY, steps).recall(local, 'count', '')).toBe('2+b+c');
		stored({ version: 1, count: '1' });
		expect(record(KEY, steps).recall(local, 'count', '')).toBe('1+a+b+c');
	});

	it('writes the migrated record back at its own version', () => {
		const steps: Step[] = [{ count: (value) => (value as number) * 10 }];
		stored({ version: 1, count: 4, other: 'kept' });
		const migrated = record(KEY, steps);
		migrated.remember(local, 'more', true);
		expect(JSON.parse(local.getItem(KEY) ?? '{}')).toEqual({
			version: 2,
			count: 40,
			other: 'kept',
			more: true,
		});
	});

	it('drops a key whose change answers undefined', () => {
		stored({ version: 1, gone: 'x', kept: 'y' });
		const migrated = record(KEY, [{ gone: () => undefined }]);
		migrated.remember(local, 'touched', 1);
		expect(JSON.parse(local.getItem(KEY) ?? '{}')).toEqual({ version: 2, kept: 'y', touched: 1 });
	});

	it('drops a key whose change throws, and still reads the rest', () => {
		stored({ version: 1, broken: 'x', kept: 'y' });
		const migrated = record(KEY, [
			{
				broken: () => {
					throw new Error('no sense in it');
				},
			},
		]);
		expect(migrated.recall(local, 'broken', 'fallback')).toBe('fallback');
		expect(migrated.recall(local, 'kept', '')).toBe('y');
	});

	it('leaves a key that is absent absent, rather than calling its change', () => {
		let called = false;
		stored({ version: 1, other: 'y' });
		const migrated = record(KEY, [{ missing: () => ((called = true), 'made up') }]);
		migrated.remember(local, 'touched', 1);
		expect(called).toBe(false);
		expect(JSON.parse(local.getItem(KEY) ?? '{}')).toEqual({ version: 2, other: 'y', touched: 1 });
	});

	it('runs no step on a record from a later version', () => {
		stored({ version: 5, count: 4 });
		const migrated = record(KEY, [{ count: () => 'changed' }]);
		expect(migrated.recall(local, 'count', 0)).toBe(4);
		migrated.remember(local, 'more', true);
		expect(JSON.parse(local.getItem(KEY) ?? '{}')).toEqual({ version: 5, count: 4, more: true });
	});

	it('does not reach a record in another store, even sharing its key', () => {
		// The storage area is what says which record it is; that only holds while nothing reaches
		// across.
		const session = store();
		const tab = record(KEY, [{}]);
		reader.remember(local, 'support.preferred', true);
		tab.remember(session, 'support.preferred', false);
		expect(reader.recall(local, 'support.preferred', false)).toBe(true);
		expect(tab.recall(session, 'support.preferred', true)).toBe(false);
	});
});

describe('a fallback describes the kind of thing wanted, not just its typeof', () => {
	let local: ReturnType<typeof store>;
	beforeEach(() => (local = store()));

	it('refuses null and an array where a map was asked for', () => {
		// `typeof null` and `typeof []` are both 'object'. Handing either back as a map is how the
		// first thing to read a key off it throws.
		for (const junk of [null, [1, 2], 'text', 7]) {
			local.setItem(KEY, JSON.stringify({ version: VERSION, 'video.at': junk }));
			expect(reader.recall<Record<string, number>>(local, 'video.at', {})).toEqual({});
		}
	});

	it('accepts a map, and an array only where an array was asked for', () => {
		local.setItem(KEY, JSON.stringify({ version: VERSION, 'video.at': { abc: 1.5 } }));
		expect(reader.recall<Record<string, number>>(local, 'video.at', {})).toEqual({ abc: 1.5 });
		local.setItem(KEY, JSON.stringify({ version: VERSION, list: [1, 2] }));
		expect(reader.recall<number[]>(local, 'list', [])).toEqual([1, 2]);
		expect(reader.recall<Record<string, number>>(local, 'list', {})).toEqual({});
	});
});
