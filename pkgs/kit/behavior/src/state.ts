/**
 * The mechanism behind every record a project keeps in the browser. The records themselves --
 * which key, which storage area, which steps -- are the project's, declared as data and handed to
 * `record`; this file only executes them.
 *
 * Why keys are flat and dotted, why the version is an integer from the first write, why a record
 * from a newer version is left alone, and why the store is passed in rather than reached for -- all
 * covered in spec/kit/state.md.
 */

export type State = { version: number; [key: string]: unknown };

/** As much of `Storage` as this needs, so a test can hand over a plain object. */
export type Store = Pick<Storage, 'getItem' | 'setItem'>;

/**
 * One version's step, as data: each key it changes, and that key's value in its new form.
 *
 * `steps[0]` takes a record at version 1 to version 2. A change sees only its own key's value,
 * runs only where the key is present, and returning `undefined` drops the key. It may not fail:
 * there is nowhere to report to and nothing a reader could do, so a change that throws drops its
 * key and lets the default stand.
 */
export type Step = Readonly<Record<string, (value: unknown) => unknown>>;

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
	/** The storage key the record lives under; the storage area is the caller's to pair. */
	readonly key: string;
	/** The shape `remember` produces: one more than the record's steps. */
	readonly version: number;
	/** What is stored under `key`, or `fallback` where there is none, or its kind does not match. */
	recall<T>(storage: Store, key: string, fallback: T): T;
	/** Store `value` under `key`, migrating whatever is already there on the way past. */
	remember(storage: Store, key: string, value: unknown): void;
	/** Forget one key, leaving the rest of the record and its version alone. */
	forget(storage: Store, key: string): void;
}

/** Carry `state` through one step, in place. */
function apply(state: State, step: Step): void {
	for (const [name, change] of Object.entries(step)) {
		if (!Object.hasOwn(state, name)) continue;
		let next: unknown;
		try {
			next = change(state[name]);
		} catch {
			next = undefined;
		}
		if (next === undefined) delete state[name];
		else state[name] = next;
	}
}

/** A record kept under `key`, migrated through `steps`; its version is one more than its steps. */
export function record(key: string, steps: readonly Step[]): Container {
	const version = steps.length + 1;
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
				const step = steps[state.version - 1];
				if (step) apply(state, step);
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
		key,
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
