/**
 * Where a reader came from, taken out of the address bar. See spec/architecture/referer.md.
 */

/** Parameters a link may carry into any page, taken out once the page is running. */
export const ARRIVAL_PARAMETERS = ['ref'] as const;

/** A query pair's name, decoded; a name that does not decode is compared as written. */
function nameOf(pair: string): string {
	const name = pair.split('=', 1)[0]!.replaceAll('+', ' ');
	try {
		return decodeURIComponent(name);
	} catch {
		return name;
	}
}

/**
 * `url`'s path, query and hash with every `name` pair taken out and every other pair left as it
 * arrived, spelling and order included; undefined when there is no `name` to take.
 */
export function withoutParameter(url: URL, name: string): string | undefined {
	if (!url.search) return undefined;
	const pairs = url.search.slice(1).split('&');
	const kept = pairs.filter((pair) => nameOf(pair) !== name);
	if (kept.length === pairs.length) return undefined;
	const search = kept.length > 0 ? `?${kept.join('&')}` : '';
	return `${url.pathname}${search}${url.hash}`;
}

/**
 * Take `name` out of the address bar, in place: no history entry, and no page view either. A
 * tracker counts views by wrapping `history.replaceState` on the instance, so the prototype's is
 * the browser's own. Returns the value taken, or null when there was none.
 */
export function takeParameter(name: string): string | null {
	const url = new URL(window.location.href);
	const value = url.searchParams.get(name);
	const replacement = withoutParameter(url, name);
	if (replacement !== undefined) {
		History.prototype.replaceState.call(history, history.state, '', replacement);
	}
	return value;
}

/** Take every arrival parameter. Called once a page has hydrated; nothing reads them yet. */
export function takeArrivalParameters(): void {
	for (const name of ARRIVAL_PARAMETERS) takeParameter(name);
}
