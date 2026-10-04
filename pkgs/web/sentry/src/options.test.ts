import { describe, expect, it } from 'vitest';
import { initOptions } from './options';

describe('initOptions', () => {
	it('sends through the default transport in production', () => {
		expect(initOptions('dsn', false)).toEqual({ dsn: 'dsn', environment: 'production' });
	});

	it('stays enabled in development and sends nothing', async () => {
		const options = initOptions('dsn', true);
		expect(options).not.toHaveProperty('enabled');
		expect(options.environment).toBe('development');
		const transport = options.transport?.();
		expect(transport).toBeDefined();
		await expect(transport?.send()).resolves.toEqual({});
		await expect(transport?.flush()).resolves.toBe(true);
	});
});
