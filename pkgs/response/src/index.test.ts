import { describe, expect, it } from 'vitest';
import CODES from '../codes.json' with { type: 'json' };
import FIXTURES from './fixtures.json' with { type: 'json' };
import { errorBody, failure, success, unwrap } from './index.ts';

describe('the catalogue', () => {
	it('names every code in lowercase with underscores', () => {
		for (const code of Object.keys(CODES)) expect(code).toMatch(/^[a-z]+(_[a-z]+)*$/);
	});

	// See spec/architecture/services.md, "Every answer is one envelope", for how a message reads.
	it('gives every code one line of English that opens with a capital and ends without a stop', () => {
		for (const [code, message] of Object.entries(CODES)) {
			expect(message, code).toMatch(/^[A-Z][\x20-\x7e]*[^.\s]$/);
		}
	});
});

describe('the envelope', () => {
	it('is the shape both languages read', () => {
		expect(unwrap(FIXTURES.success, 'fixture')).toEqual({ count: 3 });
		expect(errorBody('no_such_route')).toEqual(FIXTURES.failure);
		expect(() => unwrap(FIXTURES.failure, 'fixture')).toThrow(
			'fixture answered no_such_route: No route answers this path',
		);
	});

	it('answers a failure with its status, uncached, and a moment may say more', async () => {
		const answer = failure(409, 'invalid_port', { message: 'Port 23440 is held by geo' });
		expect(answer.status).toBe(409);
		expect(answer.headers.get('cache-control')).toBe('no-store');
		expect(await answer.json()).toEqual({
			status: 'error',
			code: 'invalid_port',
			message: 'Port 23440 is held by geo',
		});
	});

	it('answers a success with its data', async () => {
		expect(await success([1, 2], { status: 201 }).json()).toEqual({
			status: 'success',
			data: [1, 2],
		});
	});
});
