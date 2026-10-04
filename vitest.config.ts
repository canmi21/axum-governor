import { defineConfig } from 'vitest/config';

export default defineConfig({
	test: {
		include: ['pkgs/**/*.test.ts'],
		exclude: ['**/node_modules/**', '**/dist/**'],
	},
});
