import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { Disclosure } from './index.ts';

/**
 * The packages whose version a Wappalyzer fingerprint reads, by the name an app installs them as.
 *
 * Only an app that lists one in its own `package.json` gets its version; the others are absent.
 */
const VERSIONED = ['@codemirror/view', '@videojs/core', 'algoliasearch', 'd3-hierarchy'] as const;

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
 * What an app is made of, read from its `package.json` at build time.
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
	const versions: Record<string, string> = {};
	for (const name of VERSIONED) {
		if (!names.has(name)) continue;
		versions[name] = readJson<{ version: string }>(
			join(root, 'node_modules', name, 'package.json'),
		).version;
	}
	return {
		versions,
		...(WORKERS.some((name) => names.has(name)) ? { runtime: 'Cloudflare Workers' } : {}),
	};
}

/** `disclosure(root)` as a Vite `define` entry, so a page reads it as `import.meta.env.VITE_DISCLOSURE`. */
export function discloseDefine(root: string): Record<string, string> {
	return { 'import.meta.env.VITE_DISCLOSURE': JSON.stringify(disclosure(root)) };
}
