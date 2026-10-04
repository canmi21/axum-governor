import { describe, expect, it } from 'vitest';
import { isDevHost, loopbackUrl, normalizedLocation, normalizePath, SITE, SOURCE } from './index';

it('names the repository the code is published from', () => {
	expect(SOURCE).toMatch(/^https:\/\/github\.com\/\S+\/\S+$/);
});

describe('isDevHost', () => {
	it('matches localhost', () => {
		expect(isDevHost('localhost')).toBe(true);
	});

	it('matches 127.0.0.1', () => {
		expect(isDevHost('127.0.0.1')).toBe(true);
	});

	it('matches the IPv6 loopback as a URL spells it', () => {
		// The site binds `::`, so a request can arrive over IPv6. `URL.hostname` normalises every
		// spelling to the bracketed one, which is what a caller actually hands over.
		expect(isDevHost(new URL('http://[::1]:26512/').hostname)).toBe(true);
		expect(isDevHost(new URL('http://[0:0:0:0:0:0:0:1]:26512/').hostname)).toBe(true);
		expect(isDevHost('::1')).toBe(true);
	});

	it('rejects production hosts', () => {
		expect(isDevHost(new URL(SITE).hostname)).toBe(false);
	});

	it('rejects empty and arbitrary strings', () => {
		expect(isDevHost('')).toBe(false);
		expect(isDevHost('localhost.evil.com')).toBe(false);
	});
});

describe('loopbackUrl', () => {
	it('puts a local tool on its assigned port', () => {
		expect(loopbackUrl(26521)).toBe('http://127.0.0.1:26521');
	});
});

describe('normalizePath', () => {
	it.each([
		['/', ''],
		['//', ''],
		['/a/', '/a'],
		['/a//b///', '/a/b'],
		['/a\\b\\', '/a/b'],
		['/\\', ''],
		['/\\/', ''],
		['/\\\\', ''],
		['/a\\/b', '/a/b'],
		['/robots.txt\\', '/robots.txt'],
		['/robots\u3002txt', '/robots.txt'],
		['/robots%E3%80%82txt', '/robots.txt'],
		['/robots%e3%80%82txt', '/robots.txt'],
		['/robots%EF%BC%8Etxt', '/robots.txt'],
		['/robots%EF%BD%A1txt', '/robots.txt'],
		['/llms\uFF0Etxt\uFF61', '/llms.txt.'],
		['//evil.example/x', '/evil.example/x'],
		['/a/b', '/a/b'],
	])('%s -> %s', (from, to) => {
		expect(normalizePath(from)).toBe(to);
	});

	it('keeps the query, and answers nothing for a path already in its spelling', () => {
		expect(normalizedLocation(new URL('https://canmi.net/a//b/?lang=ja'))).toEqual({
			location: '/a/b?lang=ja',
			status: 308,
		});
		expect(normalizedLocation(new URL('https://canmi.net/a/b?lang=ja'))).toBeUndefined();
	});

	it('reads a CJK full stop in an address as the dot it was meant to be', () => {
		expect(normalizedLocation(new URL('https://canmi.net/robots。txt'))).toEqual({
			location: '/robots.txt',
			status: 308,
		});
	});

	it('leaves / alone or with a query, and sends another spelling of the root to the bare host', () => {
		expect(normalizedLocation(new URL('https://canmi.net'))).toBeUndefined();
		expect(normalizedLocation(new URL('https://canmi.net/'))).toBeUndefined();
		expect(normalizedLocation(new URL('https://canmi.net/?abc='))).toBeUndefined();
		expect(normalizedLocation(new URL('https://canmi.net//'))).toEqual({
			location: 'https://canmi.net',
			status: 301,
		});
		expect(normalizedLocation(new URL('https://canmi.net/\\/'))).toEqual({
			location: 'https://canmi.net',
			status: 301,
		});
		expect(normalizedLocation(new URL('https://canmi.net//?abc='))).toEqual({
			location: '/?abc=',
			status: 308,
		});
	});
});
