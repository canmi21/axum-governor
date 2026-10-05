/**
 * What the page is built with, said where a profiler reads it: each dotted path set on `window`,
 * the objects on the way made as needed, and nothing already there replaced. A patch for
 * Wappalyzer, whose fingerprints read globals -- and read a version only from some. See
 * spec/web/disclose.md.
 */
export function disclose(entries: Readonly<Record<string, unknown>>): void {
	if (typeof window === 'undefined') return;
	for (const [path, value] of Object.entries(entries)) {
		const keys = path.split('.');
		const last = keys.pop();
		if (!last) continue;
		let target = window as unknown as Record<string, unknown>;
		for (const key of keys) {
			const next = target[key];
			if (next === null || (typeof next !== 'object' && typeof next !== 'function')) {
				target[key] = {};
			}
			target = target[key] as Record<string, unknown>;
		}
		if (target[last] === undefined) target[last] = value;
	}
}
