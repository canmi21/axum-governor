import { expect, it } from 'vitest';
import { compactCount, intlLocale, shortDate } from './format';

it('writes a date in UTC, whatever the machine rendering it thinks', () => {
	// The instant is the 13th in UTC and the 12th anywhere west of Greenwich. Frontmatter says
	// the 13th, so the page has to say the 13th.
	expect(shortDate('2026-04-13T02:00:00.000Z')).toBe('Apr 13, 2026');
});

it('shortens a count with the casing SI uses', () => {
	expect(compactCount(950)).toBe('950');
	expect(compactCount(1_000)).toBe('1k');
	expect(compactCount(1_500)).toBe('1.5k');
	expect(compactCount(15_500)).toBe('16k');
	expect(compactCount(2_300_000)).toBe('2.3M');
});

it('gives the source view a language to group numbers by', () => {
	// `mw` is the article's own words and names no language of its own.
	expect(intlLocale('mw')).toBe('en-US');
	expect(intlLocale('tw')).toBe('zh-TW');
	expect(intlLocale('zh')).toBe('zh-CN');
});
