import { execFileSync } from 'node:child_process';
import { discloseDefine } from '../../disclose/src/build.ts';
import type { Disclosure } from '../../disclose/src/index.ts';

export type { Disclosure };

/**
 * What an app's `import.meta.env` carries because of how it was built: merge it into the app's own
 * `ImportMetaEnv`. See spec/web/build.md.
 */
export interface BuildEnv {
	readonly VITE_COMMIT_HASH: string;
	readonly VITE_BUILD_TIME: string;
	readonly VITE_DISCLOSURE: Disclosure;
}

/**
 * The variable a build host sets to the commit it builds, in the order they are tried: GitHub
 * Actions, Cloudflare Workers Builds, Vercel, Netlify. See spec/web/build.md.
 */
const COMMIT_VARIABLES = [
	'GITHUB_SHA',
	'WORKERS_CI_COMMIT_SHA',
	'VERCEL_GIT_COMMIT_SHA',
	'COMMIT_REF',
] as const;

/**
 * The short (7 character) commit a build is of: the first host variable that is set, else what git
 * says of `root`, else `unknown`. See spec/web/build.md.
 */
export function commitOf(root: string, env: NodeJS.ProcessEnv = process.env): string {
	for (const name of COMMIT_VARIABLES) {
		const value = env[name];
		if (value) return value.slice(0, 7);
	}
	try {
		return execFileSync('git', ['rev-parse', '--short=7', 'HEAD'], {
			cwd: root,
			encoding: 'utf8',
			stdio: ['ignore', 'pipe', 'ignore'],
		}).trim();
	} catch {
		return 'unknown';
	}
}

/**
 * Everything an app states at build time as Vite `define` entries: the commit, the moment, and what
 * it is made of. See spec/web/build.md.
 */
export function buildDefine(
	root: string,
	env: NodeJS.ProcessEnv = process.env,
): Record<string, string> {
	return {
		'import.meta.env.VITE_COMMIT_HASH': JSON.stringify(commitOf(root, env)),
		'import.meta.env.VITE_BUILD_TIME': JSON.stringify(new Date().toISOString()),
		...discloseDefine(root),
	};
}
