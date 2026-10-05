import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { Disclosure } from './index.ts';

/**
 * The packages whose version a component's own patch reads, by the name an app installs them as:
 * used on some pages only, so the component that uses one discloses it. See spec/web/disclose.md.
 */
const VERSIONED = [
	'@codemirror/view',
	'@videojs/core',
	'd3-array',
	'd3-hierarchy',
	'd3-scale',
	'd3-shape',
] as const;

/** The globals a package used on every page is disclosed as, given its installed version. */
const GLOBALS: Readonly<Record<string, (version: string) => Record<string, unknown>>> = {
	algoliasearch: (version) => ({ '__algolia.algoliasearch.version': version }),
	motion: () => ({ MotionIsMounted: true }),
};

/** The address a package is named by in the page's data block, for a fingerprint that reads one. */
const REFERENCES: Readonly<Record<string, string>> = {
	'@tanstack/svelte-query': 'https://tanstack.com/query',
};

/** A dependency an app is deployed to Cloudflare Workers with. */
const WORKERS = ['@sveltejs/adapter-cloudflare', 'wrangler'];

interface Manifest {
	dependencies?: Record<string, string>;
	devDependencies?: Record<string, string>;
}

function readJson<T>(path: string): T {
	return JSON.parse(readFileSync(path, 'utf8')) as T;
}

/**
 * What an app is made of, read from its `package.json` at build time: only what it lists itself.
 *
 * A version is the installed package's, read from its own `package.json` beside the app rather than
 * through its exports, which rarely offer it. See spec/web/disclose.md.
 */
export function disclosure(root: string): Disclosure {
	const manifest = readJson<Manifest>(join(root, 'package.json'));
	const names = new Set([
		...Object.keys(manifest.dependencies ?? {}),
		...Object.keys(manifest.devDependencies ?? {}),
	]);
	const installed = (name: string): string =>
		readJson<{ version: string }>(join(root, 'node_modules', name, 'package.json')).version;

	const versions: Record<string, string> = {};
	for (const name of VERSIONED) if (names.has(name)) versions[name] = installed(name);
	const globals: Record<string, unknown> = {};
	for (const [name, entries] of Object.entries(GLOBALS)) {
		if (names.has(name)) Object.assign(globals, entries(installed(name)));
	}
	const references = Object.entries(REFERENCES)
		.filter(([name]) => names.has(name))
		.map(([, address]) => address);

	return {
		versions,
		globals,
		references,
		...(WORKERS.some((name) => names.has(name)) ? { runtime: 'Cloudflare Workers' } : {}),
	};
}

/** `disclosure(root)` as a Vite `define` entry, so a page reads it as `import.meta.env.VITE_DISCLOSURE`. */
export function discloseDefine(root: string): Record<string, string> {
	return { 'import.meta.env.VITE_DISCLOSURE': JSON.stringify(disclosure(root)) };
}
