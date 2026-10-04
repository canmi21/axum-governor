/**
 * Where each clip had got to, for the length of one tab.
 *
 * Restores a position, never a stage and never playback -- see spec/architecture/video/player.md,
 * "A reload finds a clip where the tab left it".
 *
 * Lives in the `tab` record in `sessionStorage` (see `state.ts`), as one key holding a map rather
 * than one key per clip, and uncapped -- see spec/engagement.md, "What this site remembers is
 * two records and one mechanism", for why both hold here.
 */

import { rgbaToThumbHash, thumbHashToRGBA } from 'thumbhash';
import { tab, type Store } from './state';

const KEY = 'video.at';

/**
 * How wide the frame is sampled before it is hashed.
 *
 * `rgbaToThumbHash` takes at most a hundred pixels on a side and keeps a handful of coefficients,
 * so this only has to be large enough that each region averages to the right colour.
 */
const HASH_WIDTH = 64;

/**
 * Quality for the encode, which is the build's own number.
 *
 * Higher is wasted on a picture that has already discarded everything but an impression of colour
 * and shape, and this only has to avoid adding artefacts of its own. See
 * `services/libs/compile/src/placeholder.ts`, where the same number encodes the same kind of
 * picture.
 */
const STILL_QUALITY = 0.7;

/**
 * Below this there is nothing worth resuming.
 *
 * Restoring a third of a second is noise: the reader gets a clip that looks like it starts at the
 * beginning and a scrubber that says it does not.
 */
const FLOOR = 1;

/** Within this of the end the clip is finished, and a finished clip starts again. */
const ENDING = 0.5;

/** Where a clip was, and what it looked like there. */
export type Place = {
	at: number;
	/**
	 * A data URI of the frame at `at`, to blur behind the clip until it decodes that frame again.
	 *
	 * Optional because the canvas can be refused. The poster's own thumbhash is the fallback, and
	 * it is a picture of the first frame rather than of this one -- right for a clip nobody has
	 * moved, a little wrong for one that has, and much better than nothing either way.
	 */
	still?: string;
};

/**
 * Every position in the record, with anything that is not a finite number dropped.
 *
 * The check is per value rather than on the object, because `recall` can only answer whether the
 * record holds a map at all. What is inside it came from an older build, another tab's idea of
 * this key, or a reader with a console, and one `NaN` reaching `currentTime` throws.
 */
function positions(storage: Store): Record<string, Place> {
	const stored = tab.recall<Record<string, unknown>>(storage, KEY, {});
	const clean: Record<string, Place> = {};
	for (const [clip, place] of Object.entries(stored)) {
		if (typeof place !== 'object' || place === null || Array.isArray(place)) continue;
		const { at, still } = place as { at?: unknown; still?: unknown };
		if (typeof at !== 'number' || !Number.isFinite(at) || at <= 0) continue;
		clean[clip] = { at, still: typeof still === 'string' ? still : undefined };
	}
	return clean;
}

/** Where this clip had got to, or nothing if it had not got anywhere worth returning to. */
export function positionOf(storage: Store, clip: string): Place | undefined {
	return positions(storage)[clip];
}

/**
 * A picture of the frame an element is showing, small enough to keep.
 *
 * Returns nothing rather than throwing where the canvas is tainted. Clips are served with
 * `Access-Control-Allow-Origin` and requested with `crossorigin`, so this is normally clean, but
 * a deployment that dropped the header would turn every pause into an exception -- and the ground
 * falls back cleanly to the poster's own thumbhash either way. `getImageData` taints on reading
 * back the same as writing would, so one `catch` covers both.
 */
export function stillOf(element: HTMLVideoElement): string | undefined {
	const width = element.videoWidth;
	const height = element.videoHeight;
	if (!width || !height) return undefined;
	try {
		const sampled = document.createElement('canvas');
		sampled.width = HASH_WIDTH;
		sampled.height = Math.max(1, Math.round((HASH_WIDTH * height) / width));
		const source = sampled.getContext('2d');
		if (!source) return undefined;
		source.drawImage(element, 0, 0, sampled.width, sampled.height);
		const pixels = source.getImageData(0, 0, sampled.width, sampled.height);

		// The round trip is the point, not a formality. A small copy of a photograph is a small
		// photograph: stretched back across a 668px frame it reads as a picture out of focus,
		// which is exactly what this looked like. A thumbhash keeps a handful of coefficients and
		// throws the rest away, so what comes back is a field of colour that was never pretending
		// to be in focus -- and that is the appearance every picture on this site already has,
		// because they are placed the same way. See `services/libs/compile/src/placeholder.ts`.
		const hash = rgbaToThumbHash(pixels.width, pixels.height, pixels.data);
		const { w, h, rgba } = thumbHashToRGBA(hash);
		const out = document.createElement('canvas');
		out.width = w;
		out.height = h;
		const target = out.getContext('2d');
		if (!target) return undefined;
		target.putImageData(new ImageData(new Uint8ClampedArray(rgba), w, h), 0, 0);
		// WebP through the canvas rather than `thumbHashToDataURL`, which writes an uncompressed
		// PNG: the build measured that at 3.3KB against a 144-byte WebP of the same pixels.
		return out.toDataURL('image/webp', STILL_QUALITY);
	} catch {
		return undefined;
	}
}

/**
 * Record where this clip has got to, or forget it.
 *
 * Forgetting is the same call rather than a second one, because every caller that knows a
 * position also knows whether it is one worth keeping, and splitting them would put that judgement
 * at each of the call sites instead of here.
 */
export function keepPosition(
	storage: Store,
	clip: string,
	at: number,
	duration: number,
	still?: string,
): void {
	const finished = duration > 0 && at >= duration - ENDING;
	const map = positions(storage);
	if (!Number.isFinite(at) || at < FLOOR || finished) {
		if (!(clip in map)) return;
		delete map[clip];
	} else {
		if (map[clip]?.at === at && map[clip]?.still === still) return;
		map[clip] = { at, still };
	}
	tab.remember(storage, KEY, map);
}
