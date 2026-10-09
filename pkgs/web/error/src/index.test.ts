import { describe, expect, it } from 'vitest';
import { pageOf, stamp, statusText } from './index.ts';

describe('error', () => {
	it('stamps only an unknown error, with the side it happened on', () => {
		expect(stamp('client')({ kind: 'unknown' })).toEqual({ origin: 'client' });
		expect(stamp('server')({ kind: 'unknown' })).toEqual({ origin: 'server' });
		expect(stamp('server')({ kind: 'app' })).toBeUndefined();
	});

	it('draws the page with no status only where the browser broke', () => {
		expect(pageOf({ origin: 'client' })).toBe('client');
		expect(pageOf({ origin: 'server' })).toBe('status');
		expect(pageOf({})).toBe('status');
		expect(pageOf(null)).toBe('status');
	});

	it('names a status by the protocol, and any other as an error', () => {
		expect(statusText(404)).toBe('Not Found');
		expect(statusText(418)).toBe('Error');
	});
});
