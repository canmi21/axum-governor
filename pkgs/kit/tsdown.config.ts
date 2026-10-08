import { defineConfig } from 'tsdown';

// One output per source file, named as the source is, so a `.svelte.ts` stays a module Svelte
// compiles and a `.stylex.ts` one StyleX reads; what is not TypeScript is copied as it is.
export default defineConfig({
	entry: [
		'theme/src/index.ts',
		'tokens/src/surfaces.ts',
		'tokens/src/vocabulary.stylex.ts',
		'motion/src/index.ts',
		'behavior/src/arrival.ts',
		'behavior/src/brevity.svelte.ts',
		'behavior/src/collapse.ts',
		'behavior/src/cursor.svelte.ts',
		'behavior/src/edge.ts',
		'behavior/src/jump.ts',
		'behavior/src/progress.ts',
		'behavior/src/query.svelte.ts',
		'behavior/src/resize.ts',
		'behavior/src/shortcut.ts',
		'behavior/src/state.ts',
		'units/src/index.ts',
	],
	unbundle: true,
	// The package's own directory, so `dist/` mirrors it whatever the entries have in common.
	root: '.',
	format: 'esm',
	platform: 'neutral',
	dts: true,
	outExtensions: () => ({ js: '.js', dts: '.d.ts' }),
	copy: [
		{ from: 'theme/src/palettes/concrete.css', to: 'dist/theme/src/palettes' },
		{ from: 'theme/src/palettes/mono.css', to: 'dist/theme/src/palettes' },
		{ from: 'theme/src/palettes/nord.css', to: 'dist/theme/src/palettes' },
		{ from: 'tokens/src/interaction.css', to: 'dist/tokens/src' },
		{ from: 'tokens/src/player.css', to: 'dist/tokens/src' },
		{ from: 'behavior/src/title.svelte', to: 'dist/behavior/src' },
	],
});
