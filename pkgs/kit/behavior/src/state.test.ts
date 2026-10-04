import { beforeEach, describe, expect, it } from 'vitest';
import { reader, type Store, tab } from './state';

const KEY = 'state';

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

describe('the two records', () => {
	let local: ReturnType<typeof store>;
	let session: ReturnType<typeof store>;
	beforeEach(() => {
		local = store();
		session = store();
	});

	it('do not see each other, even sharing a key name', () => {
		// Both are called `state`, because the storage area is what says which record it is. That
		// only holds while nothing reaches across.
		reader.remember(local, 'support.preferred', true);
		tab.remember(session, 'support.preferred', false);
		expect(reader.recall(local, 'support.preferred', false)).toBe(true);
		expect(tab.recall(session, 'support.preferred', true)).toBe(false);
		expect(local.items.size).toBe(1);
		expect(session.items.size).toBe(1);
	});

	it('carry their own version, because they will not move together', () => {
		// The failure this separation exists to prevent is a migration written for one record
		// running against the other, and a shared version line is how that starts.
		expect(reader.version).toBeTypeOf('number');
		expect(tab.version).toBeTypeOf('number');
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
