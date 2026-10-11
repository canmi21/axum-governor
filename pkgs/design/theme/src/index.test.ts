import { describe, expect, it } from 'vitest';
import { inlineScriptString } from './inline-script.ts';
import { applyTheme, currentTheme, fillTheme, themeCookie, themeScript } from './index.ts';

describe('inlineScriptString', () => {
	it('keeps serialized values inside the inline script element', () => {
		expect(inlineScriptString('</script>\u2028\u2029')).toBe(
			'"\\u003C/script\\u003E\\u2028\\u2029"',
		);
	});
});

describe('themeScript', () => {
	it('reads the theme cookie and marks the root', () => {
		expect(themeScript).toContain('\\btheme=(light|dark)\\b');
		expect(themeScript).toContain('setAttribute("data-theme",m)');
		expect(themeScript).not.toContain('classList');
	});

	it('writes the same cookie the control writes', () => {
		const [, attributes = ''] = /document\.cookie="theme="\+m\+"([^"]*)"/.exec(themeScript) ?? [];
		expect(attributes).not.toBe('');
		expect(themeCookie('dark')).toBe(`theme=dark${attributes}`);
		expect(themeCookie('light')).toBe(`theme=light${attributes}`);
	});
});

describe('fillTheme', () => {
	it('fills the script placeholder', () => {
		expect(fillTheme('<script>%theme.script%</script>')).toBe(`<script>${themeScript}</script>`);
	});
});

/** An element holding the attributes it was given, outside any document. */
function element(theme?: string) {
	const attributes = new Map<string, string>(theme ? [['data-theme', theme]] : []);
	return {
		getAttribute: (name: string) => attributes.get(name) ?? null,
		setAttribute: (name: string, value: string) => void attributes.set(name, value),
	} as unknown as Element;
}

describe('currentTheme and applyTheme', () => {
	it('reads dark only from a dark mark', () => {
		expect(currentTheme(element('dark'))).toBe('dark');
		expect(currentTheme(element('light'))).toBe('light');
		expect(currentTheme(element())).toBe('light');
	});

	it('moves the mark, which is all a theme is', () => {
		const root = element('light');
		applyTheme('dark', root);
		expect(currentTheme(root)).toBe('dark');
		applyTheme('light', root);
		expect(currentTheme(root)).toBe('light');
	});
});
