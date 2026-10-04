import { describe, expect, it } from 'vitest';
import { edgeIntent, type EdgeWatch } from './edge';

const LEFT: EdgeWatch = { side: 'left', within: 8, release: 24 };
const RIGHT: EdgeWatch = { side: 'right', within: 8, release: 24 };
const WIDTH = 1000;

describe('a panel at an edge', () => {
	it('comes out only once the pointer reaches its edge', () => {
		expect(edgeIntent(8, WIDTH, LEFT, false)).toBe('reveal');
		expect(edgeIntent(9, WIDTH, LEFT, false)).toBeUndefined();
		expect(edgeIntent(992, WIDTH, RIGHT, false)).toBe('reveal');
		expect(edgeIntent(991, WIDTH, RIGHT, false)).toBeUndefined();
	});

	it('goes back only once the pointer is clear of its inner side by the margin', () => {
		const sidebar = { left: 8, right: 200 };
		expect(edgeIntent(224, WIDTH, LEFT, true, sidebar)).toBeUndefined();
		expect(edgeIntent(225, WIDTH, LEFT, true, sidebar)).toBe('conceal');
		const drawer = { left: 640, right: 992 };
		expect(edgeIntent(616, WIDTH, RIGHT, true, drawer)).toBeUndefined();
		expect(edgeIntent(615, WIDTH, RIGHT, true, drawer)).toBe('conceal');
	});

	it('does not go back while it has not been measured', () => {
		expect(edgeIntent(500, WIDTH, LEFT, true)).toBeUndefined();
	});
});
