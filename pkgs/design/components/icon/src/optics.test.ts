import { describe, expect, it, vi } from 'vitest';
import { heightBeside, viewBoxOf } from './optics.ts';

// The table's key, a Svelte component this suite never draws and node cannot load uncompiled.
vi.mock('@tabler/icons-svelte-runes/icons/git-merge', () => ({ default: {} }));

describe('viewBoxOf', () => {
	it('leaves a drawing with no correction as drawn', () => {
		expect(viewBoxOf({})).toBe('0 0 24 24');
	});

	it('moves the drawing by moving the canvas the other way', () => {
		expect(viewBoxOf({ x: 1.5 })).toBe('-1.5 0 24 24');
		expect(viewBoxOf({ y: -1 })).toBe('0 1 24 24');
	});

	it('scales about the center: a larger drawing is a smaller canvas around the same point', () => {
		expect(viewBoxOf({ scale: 1.2 })).toBe('2 2 20 20');
		expect(viewBoxOf({ scale: 0.8 })).toBe('-3 -3 30 30');
	});
});

describe('heightBeside', () => {
	it('stands an icon by a capital or a figure at the capital height, else at the lowercase', () => {
		expect(heightBeside('Deploys', 'before')).toBe('var(--font-sans-cap-height)');
		expect(heightBeside('3 more', 'before')).toBe('var(--font-sans-cap-height)');
		expect(heightBeside('deploys', 'before')).toBe('1ex');
		expect(heightBeside('View all', 'after')).toBe('1ex');
		expect(heightBeside('ALL ', 'after')).toBe('var(--font-sans-cap-height)');
		expect(heightBeside('Ätna', 'before')).toBe('var(--font-sans-cap-height)');
		expect(heightBeside('', 'before')).toBe('1ex');
	});
});
