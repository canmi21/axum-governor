import { expect, it } from 'vitest';
import { esbuildTarget } from './build';

it('spells each floor the way esbuild does', () => {
	expect(esbuildTarget(['chrome >= 110', 'safari >= 16.0'])).toEqual(['chrome110', 'safari16.0']);
});

it('refuses a query that is not a floor', () => {
	expect(() => esbuildTarget(['last 2 versions'])).toThrow(/not a floor/);
});
