import { beforeEach, describe, expect, it } from 'vitest';
import { keepPosition, positionOf } from './progress';
import { record, type Store } from './state';

/** A record for one tab, as a project would declare it. */
const tab = record('state', []);

function store(): Store & { items: Map<string, string> } {
	const items = new Map<string, string>();
	return {
		items,
		getItem: (key) => items.get(key) ?? null,
		setItem: (key, value) => void items.set(key, value),
	};
}

const CLIP = '766abd9d564851362536c6db951e3ce7';
const OTHER = 'cda778566d1e4b0f9a1c2e3d4f5a6b7c';

describe('where a clip had got to', () => {
	let session: ReturnType<typeof store>;
	beforeEach(() => (session = store()));

	it('is nothing until something is kept', () => {
		expect(positionOf(tab, session, CLIP)).toBeUndefined();
	});

	it('comes back for the clip it was filed under, and not for another', () => {
		keepPosition(tab, session, CLIP, 12.4, 25);
		expect(positionOf(tab, session, CLIP)?.at).toBe(12.4);
		expect(positionOf(tab, session, OTHER)).toBeUndefined();
	});

	it('is one key holding a map, inside the tab record', () => {
		keepPosition(tab, session, CLIP, 12.4, 25);
		keepPosition(tab, session, OTHER, 3, 25);
		expect(session.items.size).toBe(1);
		expect(tab.recall(session, 'video.at', {})).toEqual({
			[CLIP]: { at: 12.4 },
			[OTHER]: { at: 3 },
		});
	});

	it('is not kept at all below the floor, because a third of a second is noise', () => {
		keepPosition(tab, session, CLIP, 0.3, 25);
		expect(positionOf(tab, session, CLIP)).toBeUndefined();
	});

	it('is dropped once the clip has finished, so a finished clip starts again', () => {
		keepPosition(tab, session, CLIP, 12.4, 25);
		keepPosition(tab, session, CLIP, 24.8, 25);
		expect(positionOf(tab, session, CLIP)).toBeUndefined();
	});

	it('is dropped when the reader seeks back to the start', () => {
		keepPosition(tab, session, CLIP, 12.4, 25);
		keepPosition(tab, session, CLIP, 0, 25);
		expect(positionOf(tab, session, CLIP)).toBeUndefined();
	});

	it('survives a duration nobody has measured yet', () => {
		// `duration` is NaN until metadata lands, and a clip paused before then still has a
		// position worth keeping.
		keepPosition(tab, session, CLIP, 12.4, Number.NaN);
		expect(positionOf(tab, session, CLIP)?.at).toBe(12.4);
	});

	it('ignores anything in the map that is not a position', () => {
		// Another build, another tab's idea of this key, or a reader with a console. One NaN
		// reaching `currentTime` throws, and the check is per value because `recall` can only say
		// whether the record holds a map at all.
		tab.remember(session, 'video.at', {
			[CLIP]: { at: 'twelve' },
			[OTHER]: { at: 4 },
			flat: 9,
			nope: null,
			never: { at: Number.NaN },
			negative: { at: -3 },
			listed: [4],
		});
		expect(positionOf(tab, session, CLIP)).toBeUndefined();
		expect(positionOf(tab, session, OTHER)?.at).toBe(4);
		expect(positionOf(tab, session, 'flat')).toBeUndefined();
		expect(positionOf(tab, session, 'listed')).toBeUndefined();
		expect(positionOf(tab, session, 'nope')).toBeUndefined();
		expect(positionOf(tab, session, 'never')).toBeUndefined();
		expect(positionOf(tab, session, 'negative')).toBeUndefined();
	});

	it('leaves the rest of the tab record alone', () => {
		tab.remember(session, 'support.preferred', true);
		keepPosition(tab, session, CLIP, 12.4, 25);
		expect(tab.recall(session, 'support.preferred', false)).toBe(true);
	});
});

describe('the picture of where it was', () => {
	let session: ReturnType<typeof store>;
	beforeEach(() => (session = store()));

	it('is kept beside the position and comes back with it', () => {
		keepPosition(tab, session, CLIP, 12.4, 25, 'data:image/webp;base64,abc');
		expect(positionOf(tab, session, CLIP)).toEqual({
			at: 12.4,
			still: 'data:image/webp;base64,abc',
		});
	});

	it('is optional, because the canvas can be refused', () => {
		keepPosition(tab, session, CLIP, 12.4, 25);
		expect(positionOf(tab, session, CLIP)).toEqual({ at: 12.4, still: undefined });
	});

	it('is dropped with the entry when the clip finishes', () => {
		keepPosition(tab, session, CLIP, 12.4, 25, 'data:image/webp;base64,abc');
		keepPosition(tab, session, CLIP, 24.9, 25, 'data:image/webp;base64,def');
		expect(positionOf(tab, session, CLIP)).toBeUndefined();
	});

	it('is ignored where it is not a string', () => {
		tab.remember(session, 'video.at', { [CLIP]: { at: 3, still: 7 } });
		expect(positionOf(tab, session, CLIP)).toEqual({ at: 3, still: undefined });
	});
});
