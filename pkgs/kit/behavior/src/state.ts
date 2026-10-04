/**
 * The two records this site keeps in the browser, and the one mechanism behind both.
 *
 * Why there are two records, why keys are flat and dotted, why `video.at` is the exception to
 * that, why the version is an integer from the first write, why a record from a newer version is
 * left alone, and why the store is passed in rather than reached for -- all covered in
 * spec/engagement.md, "What this site remembers is two records and one mechanism". This file is
 * the mechanism itself.
 */

export type State = { version: number; [key: string]: unknown };

/** As much of `Storage` as this needs, so a test can hand over a plain object. */
export type Store = Pick<Storage, 'getItem' | 'setItem'>;

/**
 * One step per version, in order: `migrations[0]` takes a record at version 1 to version 2.
 *
 * A step edits the record in place and may assume every earlier step has run. It may not fail:
 * there is nowhere to report to and nothing a reader could do, so a step that cannot make sense
 * of what it finds deletes it and lets the default stand.
 */
export type Migration = (state: State) => void;

/**
 * Whether a stored value is the same kind of thing as the fallback asked for.
 *
 * `typeof` alone was enough while every fact was a boolean, a number or a string. It stopped
 * being enough the moment one of them became a map: `typeof null` and `typeof []` are both
 * `'object'`, so a record holding either would have handed it back as if it were the map, and the
 * first thing to read a key off it would have thrown.
 */
function alike(value: unknown, fallback: unknown): boolean {
	if (typeof value !== typeof fallback) return false;
	if (typeof fallback !== 'object') return true;
	if (fallback === null || value === null) return fallback === value;
	return Array.isArray(value) === Array.isArray(fallback);
}

export interface Container {
	/** The shape `remember` produces. Raise it in the same commit that adds the step to reach it. */
	readonly version: number;
	/** What is stored under `key`, or `fallback` where there is none, or its kind does not match. */
	recall<T>(storage: Store, key: string, fallback: T): T;
	/** Store `value` under `key`, migrating whatever is already there on the way past. */
	remember(storage: Store, key: string, value: unknown): void;
	/** Forget one key, leaving the rest of the record and its version alone. */
	forget(storage: Store, key: string): void;
}

function container(key: string, version: number, migrations: Migration[]): Container {
	const fresh = (): State => ({ version });

	/**
	 * The stored record, migrated up to `version`.
	 *
	 * A record from a *newer* version is returned untouched rather than reset. That case is a
	 * reader whose other device runs a later build -- which is the case cloud sync will make
	 * ordinary -- and the keys this build understands are still readable inside it. Discarding it
	 * would throw away facts this build simply has no opinion about.
	 */
	function read(storage: Store): State {
		try {
			const raw = storage.getItem(key);
			if (raw === null) return fresh();
			const parsed: unknown = JSON.parse(raw);
			if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return fresh();
			const state = parsed as State;
			if (!Number.isInteger(state.version) || state.version < 1) return fresh();
			while (state.version < version) {
				migrations[state.version - 1]?.(state);
				state.version += 1;
			}
			return state;
		} catch {
			// No storage at all, or a record that is not JSON. Either way the defaults stand.
			return fresh();
		}
	}

	function save(storage: Store, state: State): void {
		try {
			storage.setItem(key, JSON.stringify(state));
		} catch {
			// Private browsing, storage the reader has turned off, or a quota that is full.
			// Nothing to record into, and nothing a reader could do about it.
		}
	}

	return {
		version,
		recall<T>(storage: Store, name: string, fallback: T): T {
			const value = read(storage)[name];
			return alike(value, fallback) ? (value as T) : fallback;
		},
		remember(storage: Store, name: string, value: unknown): void {
			const state = read(storage);
			state[name] = value;
			save(storage, state);
		},
		forget(storage: Store, name: string): void {
			const state = read(storage);
			if (!(name in state)) return;
			delete state[name];
			save(storage, state);
		},
	};
}

/** What is true of the person. Pair it with `localStorage`. */
export const reader = container('state', 1, []);

/** What is true of this sitting. Pair it with `sessionStorage`. */
export const tab = container('state', 2, [
	/**
	 * 1 to 2: `video.at` went from a position to a position and a picture of it.
	 *
	 * The old shape would have expired on its own -- it only ever lives for one tab -- so this is
	 * not a step anybody needed. It is the step that proves the mechanism works before there is a
	 * record worth losing, which is the only time that can be checked cheaply.
	 */
	(state) => {
		const map = state['video.at'];
		if (typeof map !== 'object' || map === null || Array.isArray(map)) {
			delete state['video.at'];
			return;
		}
		const carried: Record<string, unknown> = {};
		for (const [clip, at] of Object.entries(map as Record<string, unknown>)) {
			if (typeof at === 'number' && Number.isFinite(at) && at > 0) carried[clip] = { at };
		}
		state['video.at'] = carried;
	},
]);
