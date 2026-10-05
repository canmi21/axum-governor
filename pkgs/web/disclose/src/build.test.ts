import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vitest';
import { disclosure, discloseDefine } from './build.ts';

function app(manifest: object, installed: Record<string, string>): string {
	const root = mkdtempSync(join(tmpdir(), 'disclose-'));
	writeFileSync(join(root, 'package.json'), JSON.stringify(manifest));
	for (const [name, version] of Object.entries(installed)) {
		mkdirSync(join(root, 'node_modules', name), { recursive: true });
		writeFileSync(join(root, 'node_modules', name, 'package.json'), JSON.stringify({ version }));
	}
	return root;
}

it('reads the versions of the packages an app lists, and only those', () => {
	const root = app(
		{ dependencies: { 'd3-hierarchy': '^3.1.2', svelte: '^5.0.0' } },
		{ 'd3-hierarchy': '3.1.2', svelte: '5.57.1', algoliasearch: '5.59.0' },
	);
	expect(disclosure(root)).toEqual({
		versions: { 'd3-hierarchy': '3.1.2' },
		globals: {},
		references: [],
	});
});

it('names Workers for an app deployed with the Cloudflare adapter or wrangler', () => {
	expect(
		disclosure(app({ devDependencies: { '@sveltejs/adapter-cloudflare': '^7' } }, {})),
	).toEqual({
		versions: {},
		globals: {},
		references: [],
		runtime: 'Cloudflare Workers',
	});
	expect(disclosure(app({ devDependencies: { wrangler: '^4' } }, {})).runtime).toBe(
		'Cloudflare Workers',
	);
	expect(
		disclosure(app({ devDependencies: { '@sveltejs/adapter-vercel': '^6' } }, {})).runtime,
	).toBe(undefined);
});

it('defines it for Vite as one object', () => {
	const root = app({ dependencies: { algoliasearch: '^5' } }, { algoliasearch: '5.59.0' });
	expect(JSON.parse(discloseDefine(root)['import.meta.env.VITE_DISCLOSURE']!).globals).toEqual({
		'__algolia.algoliasearch.version': '5.59.0',
	});
});

it('sets the globals and references of what every page uses', () => {
	const root = app(
		{ dependencies: { motion: '^14', '@tanstack/svelte-query': '^6' } },
		{ motion: '14.0.0', '@tanstack/svelte-query': '6.0.0' },
	);
	expect(disclosure(root)).toEqual({
		versions: {},
		globals: { MotionIsMounted: true },
		references: ['https://tanstack.com/query'],
	});
});
