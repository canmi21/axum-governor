/**
 * What an app is made of, worked out at build time by `disclosure` in `./build` and read by its
 * pages as `import.meta.env.VITE_DISCLOSURE`.
 */
export interface Disclosure {
	/** Installed versions, by package name, of the packages a fingerprint reads a version for. */
	readonly versions: Readonly<Record<string, string>>;
	/** Where the app runs, when a fingerprint names it: set for an app deployed to Workers. */
	readonly runtime?: 'Cloudflare Workers';
}

/**
 * What the page is built with, said where a profiler reads it: each dotted path set on `window`,
 * the objects on the way made as needed, nothing already there replaced, and an undefined value
 * skipped, so an entry the build did not find is simply absent. A patch for
 * Wappalyzer, whose fingerprints read globals -- and read a version only from some. See
 * spec/web/disclose.md.
 */
export function disclose(entries: Readonly<Record<string, unknown>>): void {
	if (typeof window === 'undefined') return;
	for (const [path, value] of Object.entries(entries)) {
		const keys = path.split('.');
		const last = keys.pop();
		if (!last || value === undefined) continue;
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
