import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// The runes are the Svelte compiler's, which this suite does not run; outside a component a field
// declared with `$state` holds its value the way a plain field does, which is all these read.
vi.hoisted(() => {
	Object.assign(globalThis, { $state: <T>(value: T) => value });
});

import { DebouncedQuery, QUIET_MS } from './query.svelte';

/** A request that answers when told to. */
function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}

/** A query whose every request is held, in the order they went out. */
function held() {
	const requests: { text: string; answer: ReturnType<typeof deferred<string[]>> }[] = [];
	const query = new DebouncedQuery<string>({
		fetch: (text) => {
			const answer = deferred<string[]>();
			requests.push({ text, answer });
			return answer.promise;
		},
	});
	return { query, requests };
}

describe('a debounced query', () => {
	beforeEach(() => void vi.useFakeTimers());
	afterEach(() => void vi.useRealTimers());

	it('sends one request for a burst of typing, once the field is quiet', () => {
		const { query, requests } = held();
		for (const text of ['r', 'ru', 'rus', 'rust ']) {
			query.input(text);
			vi.advanceTimersByTime(QUIET_MS - 1);
		}
		expect(requests).toHaveLength(0);
		vi.advanceTimersByTime(1);
		expect(requests.map((request) => request.text)).toEqual(['rust']);
		expect(query.text).toBe('rust ');
		expect(query.searching).toBe(true);
	});

	it('lets only the newest answer write, whichever arrives first', async () => {
		const { query, requests } = held();
		query.input('ru');
		vi.advanceTimersByTime(QUIET_MS);
		query.input('rust');
		vi.advanceTimersByTime(QUIET_MS);
		const [older, newer] = requests;
		newer?.answer.resolve(['rust']);
		await vi.runAllTimersAsync();
		older?.answer.resolve(['ruby']);
		await vi.runAllTimersAsync();
		expect(query.hits).toEqual(['rust']);
		expect(query.searching).toBe(false);
	});

	it('discards what is in flight once reset', async () => {
		const { query, requests } = held();
		query.input('rust');
		vi.advanceTimersByTime(QUIET_MS);
		query.reset();
		requests[0]?.answer.resolve(['rust']);
		await vi.runAllTimersAsync();
		expect(query.hits).toEqual([]);
		expect(query.text).toBe('');
		expect(query.searching).toBe(false);
	});

	it('clears at once on a text not worth a request, and forgets what was in flight', async () => {
		const { query, requests } = held();
		query.input('rust');
		vi.advanceTimersByTime(QUIET_MS);
		query.input('  ');
		expect(query.searching).toBe(false);
		requests[0]?.answer.resolve(['rust']);
		await vi.runAllTimersAsync();
		expect(query.hits).toEqual([]);
		expect(requests).toHaveLength(1);
	});

	it('says a request failed, and stops searching', async () => {
		const { query, requests } = held();
		query.input('rust');
		vi.advanceTimersByTime(QUIET_MS);
		requests[0]?.answer.reject(new Error('down'));
		await vi.runAllTimersAsync();
		expect(query.failed).toBe(true);
		expect(query.searching).toBe(false);
		expect(query.hits).toEqual([]);
	});

	it('sends nothing once disposed', () => {
		const { query, requests } = held();
		query.input('rust');
		query.dispose();
		vi.advanceTimersByTime(QUIET_MS);
		expect(requests).toHaveLength(0);
	});
});
