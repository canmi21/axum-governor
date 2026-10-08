import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vitest';
import { buildDefine, commitOf } from './index.ts';

function dir(): string {
	return mkdtempSync(join(tmpdir(), 'build-'));
}

const SHA = '0123456789abcdef0123456789abcdef01234567';

it('takes the first host variable that is set, shortened to 7', () => {
	const root = dir();
	expect(commitOf(root, { GITHUB_SHA: SHA, COMMIT_REF: 'ffffffffff' })).toBe('0123456');
	expect(commitOf(root, { WORKERS_CI_COMMIT_SHA: SHA, VERCEL_GIT_COMMIT_SHA: 'ffffffffff' })).toBe(
		'0123456',
	);
	expect(commitOf(root, { VERCEL_GIT_COMMIT_SHA: SHA, COMMIT_REF: 'ffffffffff' })).toBe('0123456');
	expect(commitOf(root, { COMMIT_REF: SHA })).toBe('0123456');
	expect(commitOf(root, { GITHUB_SHA: '', COMMIT_REF: SHA })).toBe('0123456');
});

it('is unknown when no variable is set and git cannot answer', () => {
	expect(commitOf(dir(), {})).toBe('unknown');
});

it('defines the commit, the time and the disclosure for Vite', () => {
	const root = dir();
	writeFileSync(join(root, 'package.json'), JSON.stringify({ devDependencies: { wrangler: '^4' } }));
	mkdirSync(join(root, 'node_modules'), { recursive: true });
	const define = buildDefine(root, { GITHUB_SHA: SHA });
	expect(JSON.parse(define['import.meta.env.VITE_COMMIT_HASH']!)).toBe('0123456');
	const time = JSON.parse(define['import.meta.env.VITE_BUILD_TIME']!) as string;
	expect(new Date(time).toISOString()).toBe(time);
	expect(JSON.parse(define['import.meta.env.VITE_DISCLOSURE']!).runtime).toBe('Cloudflare Workers');
});
