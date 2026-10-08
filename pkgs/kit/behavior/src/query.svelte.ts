/**
 * A field whose text is searched once typing stops, and whose answers arrive in order. See
 * spec/kit/behavior.md.
 *
 * The quiet period and the ticket are the rules of web's spec/search.md, "A request goes out when
 * typing stops": a request per keystroke spends ten on one search, and debouncing alone does not
 * order the answers.
 */

/** How long the field stays quiet before a request goes out, in milliseconds. */
export const QUIET_MS = 300;

export type QueryOptions<Hit> = {
	fetch: (text: string) => Promise<Hit[]>;
	/** Whether a text is worth a request yet; non-empty unless told otherwise. */
	worth?: (text: string) => boolean;
	quietMs?: number;
};

export class DebouncedQuery<Hit> {
	/** What was typed, as typed. */
	text = $state('');
	/** The newest answer, or nothing. */
	hits = $state<Hit[]>([]);
	/** A request has gone out for the text on screen and not come back. */
	searching = $state(false);
	/** The newest request failed. */
	failed = $state(false);

	#fetch: (text: string) => Promise<Hit[]>;
	#worth: (text: string) => boolean;
	#quietMs: number;
	#timer: ReturnType<typeof setTimeout> | undefined;
	/**
	 * Which request the answer on screen is allowed to come from.
	 *
	 * Type, pause, type, pause, and the first response can still arrive second; without this the
	 * reader sees the results for a prefix of what they typed. A counter rather than an
	 * `AbortController`, because the client exposes no signal to abort with and the request has
	 * left either way -- what matters is which answer may be believed. See web's spec/search.md.
	 */
	#issued = 0;

	constructor(options: QueryOptions<Hit>) {
		this.#fetch = options.fetch;
		this.#worth = options.worth ?? ((text) => text.trim() !== '');
		this.#quietMs = options.quietMs ?? QUIET_MS;
	}

	/** Take new text: search it once the field is quiet, or clear at once if it is not worth one. */
	input(text: string): void {
		this.text = text;
		clearTimeout(this.#timer);
		const query = text.trim();
		if (!this.#worth(query)) {
			// Deleting back to one letter must clear the answer to two, not leave it standing.
			this.#issued += 1;
			this.hits = [];
			this.searching = false;
			this.failed = false;
			return;
		}
		this.#timer = setTimeout(() => void this.#run(query), this.#quietMs);
	}

	/** Back to empty; nothing already in flight may write to a field that was closed and reopened. */
	reset(): void {
		clearTimeout(this.#timer);
		this.#issued += 1;
		this.text = '';
		this.hits = [];
		this.searching = false;
		this.failed = false;
	}

	/** Stop the pending request and any answer still on its way, for a field that is going away. */
	dispose(): void {
		clearTimeout(this.#timer);
		this.#issued += 1;
	}

	async #run(text: string): Promise<void> {
		const ticket = ++this.#issued;
		this.searching = true;
		try {
			const found = await this.#fetch(text);
			if (ticket !== this.#issued) return;
			this.hits = found;
			this.failed = false;
		} catch {
			if (ticket !== this.#issued) return;
			this.hits = [];
			this.failed = true;
		} finally {
			if (ticket === this.#issued) this.searching = false;
		}
	}
}
