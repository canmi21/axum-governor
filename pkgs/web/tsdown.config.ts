import { defineConfig } from 'tsdown';

// One output per source file, named as the source is, so a `.svelte.ts` stays a module Svelte
// compiles and a `.stylex.ts` one StyleX reads; what is not TypeScript is copied as it is.
export default defineConfig({
	entry: [
		'compat/src/index.ts',
		'compat/src/build.ts',
		'disclose/src/build.ts',
		'disclose/src/hono.ts',
		'disclose/src/index.ts',
		'referer/src/index.ts',
		'sentry/src/build.ts',
		'sentry/src/client.ts',
		'sentry/src/feedback.ts',
		'sentry/src/report.ts',
		'sentry/src/server.ts',
	],
	unbundle: true,
	// The package's own directory, so `dist/` mirrors it whatever the entries have in common.
	root: '.',
	format: 'esm',
	platform: 'neutral',
	dts: true,
	outExtensions: () => ({ js: '.js', dts: '.d.ts' }),
	copy: [],
});
