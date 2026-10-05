import { afterEach, expect, it, vi } from 'vitest';
import { disclose } from './index.ts';

afterEach(() => vi.unstubAllGlobals());

it('sets each dotted path, making the objects on the way', () => {
	vi.stubGlobal('window', {});
	disclose({ '__algolia.algoliasearch.version': '5.59.0', umami: true });
	expect(window).toEqual({ __algolia: { algoliasearch: { version: '5.59.0' } }, umami: true });
});

it('replaces nothing already there', () => {
	const sentry = { SDK_VERSION: '11.4.0', init: () => {} };
	vi.stubGlobal('window', { Sentry: sentry });
	disclose({ 'Sentry.SDK_VERSION': '0.0.0', 'Sentry.other': 1 });
	expect((window as unknown as { Sentry: typeof sentry }).Sentry).toBe(sentry);
	expect(sentry).toMatchObject({ SDK_VERSION: '11.4.0', other: 1 });
});

it('does nothing on the server', () => {
	vi.stubGlobal('window', undefined);
	expect(() => disclose({ a: 1 })).not.toThrow();
});

it('skips an undefined value', () => {
	vi.stubGlobal('window', {});
	disclose({ 'd3.version': undefined, umami: true });
	expect(window).toEqual({ umami: true });
});
