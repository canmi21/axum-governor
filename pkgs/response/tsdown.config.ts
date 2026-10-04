import { defineConfig } from 'tsdown';

// One output per source file, named as the source is, so a `.svelte.ts` stays a module Svelte
// compiles and a `.stylex.ts` one StyleX reads; what is not TypeScript is copied as it is.
export default defineConfig({
	entry: ["src/index.ts"],
	unbundle: true,
	// The package's own directory, so `dist/` mirrors it whatever the entries have in common.
	root: '.',
	format: 'esm',
	platform: 'neutral',
	dts: true,
	outExtensions: () => ({ js: '.js', dts: '.d.ts' }),
	// The codes stay a file of their own, which the crate reads too, published beside `dist/` and
	// imported from there, so a declaration names the codes as the source does.
	deps: { neverBundle: [/codes\.json$/] },
});
