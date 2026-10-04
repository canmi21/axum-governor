import { beforeEach, describe, expect, it } from 'vitest';
import { keepPosition, positionOf } from './progress';
import { tab, type Store } from './state';

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
		expect(positionOf(session, CLIP)).toBeUndefined();
	});

	it('comes back for the clip it was filed under, and not for another', () => {
		keepPosition(session, CLIP, 12.4, 25);
		expect(positionOf(session, CLIP)?.at).toBe(12.4);
		expect(positionOf(session, OTHER)).toBeUndefined();
	});

	it('is one key holding a map, inside the tab record', () => {
		keepPosition(session, CLIP, 12.4, 25);
		keepPosition(session, OTHER, 3, 25);
		expect(session.items.size).toBe(1);
		expect(tab.recall(session, 'video.at', {})).toEqual({
			[CLIP]: { at: 12.4 },
			[OTHER]: { at: 3 },
		});
	});

	it('is not kept at all below the floor, because a third of a second is noise', () => {
		keepPosition(session, CLIP, 0.3, 25);
		expect(positionOf(session, CLIP)).toBeUndefined();
	});

	it('is dropped once the clip has finished, so a finished clip starts again', () => {
		keepPosition(session, CLIP, 12.4, 25);
		keepPosition(session, CLIP, 24.8, 25);
		expect(positionOf(session, CLIP)).toBeUndefined();
	});

	it('is dropped when the reader seeks back to the start', () => {
		keepPosition(session, CLIP, 12.4, 25);
		keepPosition(session, CLIP, 0, 25);
		expect(positionOf(session, CLIP)).toBeUndefined();
	});

	it('survives a duration nobody has measured yet', () => {
		// `duration` is NaN until metadata lands, and a clip paused before then still has a
		// position worth keeping.
		keepPosition(session, CLIP, 12.4, Number.NaN);
		expect(positionOf(session, CLIP)?.at).toBe(12.4);
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
		expect(positionOf(session, CLIP)).toBeUndefined();
		expect(positionOf(session, OTHER)?.at).toBe(4);
		expect(positionOf(session, 'flat')).toBeUndefined();
		expect(positionOf(session, 'listed')).toBeUndefined();
		expect(positionOf(session, 'nope')).toBeUndefined();
		expect(positionOf(session, 'never')).toBeUndefined();
		expect(positionOf(session, 'negative')).toBeUndefined();
	});

	it('leaves the rest of the tab record alone', () => {
		tab.remember(session, 'support.preferred', true);
		keepPosition(session, CLIP, 12.4, 25);
		expect(tab.recall(session, 'support.preferred', false)).toBe(true);
	});
});

describe('the picture of where it was', () => {
	let session: ReturnType<typeof store>;
	beforeEach(() => (session = store()));

	it('is kept beside the position and comes back with it', () => {
		keepPosition(session, CLIP, 12.4, 25, 'data:image/webp;base64,abc');
		expect(positionOf(session, CLIP)).toEqual({ at: 12.4, still: 'data:image/webp;base64,abc' });
	});

	it('is optional, because the canvas can be refused', () => {
		keepPosition(session, CLIP, 12.4, 25);
		expect(positionOf(session, CLIP)).toEqual({ at: 12.4, still: undefined });
	});

	it('is dropped with the entry when the clip finishes', () => {
		keepPosition(session, CLIP, 12.4, 25, 'data:image/webp;base64,abc');
		keepPosition(session, CLIP, 24.9, 25, 'data:image/webp;base64,def');
		expect(positionOf(session, CLIP)).toBeUndefined();
	});

	it('is ignored where it is not a string', () => {
		tab.remember(session, 'video.at', { [CLIP]: { at: 3, still: 7 } });
		expect(positionOf(session, CLIP)).toEqual({ at: 3, still: undefined });
	});
});

describe('the step from one shape to the next', () => {
	it('carries positions across and leaves nothing of the old shape', () => {
		// The first migration this record has had, written while there is nothing worth losing.
		const session = store();
		session.setItem(
			'state',
			JSON.stringify({ version: 1, 'video.at': { [CLIP]: 12.4, [OTHER]: -1 }, keep: 'me' }),
		);
		expect(positionOf(session, CLIP)).toEqual({ at: 12.4, still: undefined });
		expect(positionOf(session, OTHER)).toBeUndefined();
		expect(tab.recall(session, 'keep', '')).toBe('me');
	});

	it('discards a `video.at` that was never a map', () => {
		const session = store();
		session.setItem('state', JSON.stringify({ version: 1, 'video.at': 'nonsense' }));
		expect(positionOf(session, CLIP)).toBeUndefined();
	});
});
