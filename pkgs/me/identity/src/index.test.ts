import { describe, expect, it } from 'vitest';
import { author } from './index';

describe('author', () => {
	// The OpenGraph card reads the same file from Rust, and an empty field there draws a gap.
	it('names every field the pages and the card introduce', () => {
		for (const key of ['name', 'fullName', 'role', 'email', 'github', 'fediverse'] as const) {
			expect(author[key], key).toBeTruthy();
		}
		expect(Number.isInteger(author.githubId)).toBe(true);
	});
});
