import { expect, it } from 'vitest';
import { withoutParameter } from './index';

const at = (path: string) => new URL(path, 'https://example.com');

it('takes the one parameter and leaves the rest as they arrived', () => {
	expect(withoutParameter(at('/post?a=1&ref=status&b=x%20y#top'), 'ref')).toBe(
		'/post?a=1&b=x%20y#top',
	);
	expect(withoutParameter(at('/?flag&ref=app&q=a+b'), 'ref')).toBe('/?flag&q=a+b');
});

it('drops the question mark with the only parameter', () => {
	expect(withoutParameter(at('/post?ref=status#top'), 'ref')).toBe('/post#top');
	expect(withoutParameter(at('/?ref'), 'ref')).toBe('/');
});

it('takes every copy of the name, and only that name', () => {
	expect(withoutParameter(at('/?ref=a&referrer=b&ref=c'), 'ref')).toBe('/?referrer=b');
	expect(withoutParameter(at('/?%72ef=a&b=1'), 'ref')).toBe('/?b=1');
});

it('changes nothing when the name is absent', () => {
	expect(withoutParameter(at('/post'), 'ref')).toBeUndefined();
	expect(withoutParameter(at('/post?a=1&%E0=2'), 'ref')).toBeUndefined();
});
