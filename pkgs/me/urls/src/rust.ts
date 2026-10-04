import { author } from '../../identity/src/index.ts';
import { CONTACT, EXTERNAL, SITE, SOURCE } from './index.ts';

/**
 * Every string under `map` as a Rust `pub const`, named by its path in capitals, so
 * `external.github.web` is `EXTERNAL_GITHUB_WEB`. The generator each address package renders its
 * Rust half with; it knows nothing of what the addresses are. See spec/me/addresses.md.
 */
export function rustConstants(map: Record<string, unknown>): string {
	const pairs: Array<[name: string, value: string]> = [];
	walk(map, [], pairs);
	return pairs.map(([name, value]) => `pub const ${name}: &str = "${value}";`).join('\n');
}

function walk(value: unknown, path: string[], out: Array<[string, string]>): void {
	if (typeof value === 'string') {
		const name = path
			.map((segment) => segment.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toUpperCase())
			.join('_');
		out.push([name, value]);
		return;
	}
	// Only strings become constants: a number such as `githubId` is a fact Rust has no use for yet.
	if (typeof value !== 'object' || value === null) return;
	for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
		walk(child, [...path, key], out);
	}
}

/**
 * This package's Rust half, the `canmi` crate: committed, because a checkout must compile without
 * a Node toolchain having run first. `mise run urls` rewrites it and `rust.test.ts` fails when the
 * two drift. The author's identity comes along as `AUTHOR_*`, so a Rust program reads it compiled
 * in rather than from a path into this repository.
 */
export function rustCanmi(): string {
	return [
		'//! @generated from pkgs/me/urls/src/index.ts and pkgs/me/identity/author.json by',
		'//! `mise run urls`; do not edit.',
		"//! The author's own addresses and identity, for Rust -- see spec/me/addresses.md.",
		'',
		rustConstants({ site: SITE, source: SOURCE, contact: CONTACT, external: EXTERNAL }),
		'',
		rustConstants({ author }),
		'',
	].join('\n');
}
