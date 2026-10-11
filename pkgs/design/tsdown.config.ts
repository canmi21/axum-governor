import { defineConfig } from 'tsdown';

// One output per source file, named as the source is; what is not TypeScript is copied as it is,
// a style beside the palette it imports.
export default defineConfig({
	entry: ['contract/src/index.ts', 'scale/src/scale.stylex.ts', 'components/icon/src/optics.ts'],
	unbundle: true,
	// The package's own directory, so `dist/` mirrors it whatever the entries have in common.
	root: '.',
	format: 'esm',
	platform: 'neutral',
	dts: true,
	outExtensions: () => ({ js: '.js', dts: '.d.ts' }),
	copy: [
		{ from: 'styles/src/mono.css', to: 'dist/styles/src' },
		{ from: 'styles/src/mono/palette.css', to: 'dist/styles/src/mono' },
		{ from: 'components/icon/src/icon.svelte', to: 'dist/components/icon/src' },
		{
			from: 'components/icon-button/src/icon-button.svelte',
			to: 'dist/components/icon-button/src',
		},
		{ from: 'components/nav-link/src/nav-link.svelte', to: 'dist/components/nav-link/src' },
		{ from: 'layouts/shell/src/shell.svelte', to: 'dist/layouts/shell/src' },
		{ from: 'components/error-page/src/client.svelte', to: 'dist/components/error-page/src' },
		{ from: 'components/error-page/src/offer.svelte', to: 'dist/components/error-page/src' },
		{ from: 'components/error-page/src/status.svelte', to: 'dist/components/error-page/src' },
	],
});
