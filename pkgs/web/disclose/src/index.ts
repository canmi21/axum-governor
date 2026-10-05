/**
 * What an app is made of, worked out at build time by `disclosure` in `./build` and read by its
 * pages as `import.meta.env.VITE_DISCLOSURE`.
 */
export interface Disclosure {
	/** Installed versions, by package name, of the packages a fingerprint reads a version for. */
	readonly versions: Readonly<Record<string, string>>;
	/** Globals for what every page uses, by dotted path, set by `discloseGlobals`. */
	readonly globals: Readonly<Record<string, unknown>>;
	/** Addresses named in the page's data block, written by `disclosureHead`. */
	readonly references: readonly string[];
	/** Where the app runs, when a fingerprint names it: set for an app deployed to Workers. */
	readonly runtime?: 'Cloudflare Workers';
}

/**
 * The head an app's root layout writes with `{@html}`: the `runtime` meta, and a data block naming
 * `references`, which the browser never runs and a profiler reads as script text. Empty when there
 * is nothing to say. See spec/web/disclose.md.
 */
export function disclosureHead(disclosure: Disclosure | undefined): string {
	if (!disclosure) return '';
	const parts: string[] = [];
	if (disclosure.runtime) parts.push(`<meta name="runtime" content="${disclosure.runtime}">`);
	if (disclosure.references.length > 0) {
		// Every slash written `\/`, which JSON reads as a slash: Wappalyzer escapes each `/` in a
		// pattern again, so its TanStack pattern only matches a slash that follows a backslash.
		const data = JSON.stringify(disclosure.references)
			.replaceAll('<', '\\u003c')
			.replaceAll('/', '\\/');
		parts.push(`<script type="application/json" data-disclosure>${data}</script>`);
	}
	return parts.join('');
}

/** The D3 modules an app may draw with, any one of which is D3 to a fingerprint. */
const D3_MODULES = ['d3-hierarchy', 'd3-scale', 'd3-shape', 'd3-array'];

/**
 * D3, from the component that draws with it: `d3.version`, which no D3 module sets. The value names
 * the module, which Wappalyzer's version check refuses, so D3 shows without the module's own major
 * read as D3's. Nothing when the app's build found no D3 module. See spec/web/disclose.md.
 */
export function discloseD3(disclosure: Disclosure | undefined): void {
	const versions = disclosure?.versions ?? {};
	const name = D3_MODULES.find((module) => versions[module]);
	if (name) disclose({ 'd3.version': `${name}@${versions[name]}` });
}

/** Sets `globals` on `window`, from an app's root layout; nothing on the server. */
export function discloseGlobals(disclosure: Disclosure | undefined): void {
	if (disclosure) disclose(disclosure.globals);
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
