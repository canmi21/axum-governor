import { describe, expect, it } from 'vitest';
import { inlineScriptString } from './inline-script';
import {
	applyTheme,
	currentTheme,
	fillTheme,
	followSystemTheme,
	observeTheme,
	themeCookie,
	themeOf,
	themeScript,
	type Theme,
} from './index';

describe('inlineScriptString', () => {
	it('keeps serialized values inside the inline script element', () => {
		expect(inlineScriptString('</script>\u2028\u2029')).toBe(
			'"\\u003C/script\\u003E\\u2028\\u2029"',
		);
	});
});

describe('themeScript', () => {
	it('keeps the site theme cookie contract', () => {
		expect(themeScript).toContain('\\btheme=(light|dark)\\b');
		expect(themeScript).toContain('document.cookie="theme="+m+');
	});

	it('writes the same cookie the control writes', () => {
		// The script settles the first visit and the control settles every one after it. A
		// difference in path or lifetime here is a preference that expires on one of the two
		// paths and not the other, which nothing would report.
		const [, attributes = ''] = /document\.cookie="theme="\+m\+"([^"]*)"/.exec(themeScript) ?? [];
		expect(attributes).not.toBe('');
		expect(themeCookie('dark')).toBe(`theme=dark${attributes}`);
		expect(themeCookie('light')).toBe(`theme=light${attributes}`);
	});
});

/** A cookie jar holding at most a `theme` cookie. */
function jar(value?: string) {
	return { get: (name: string) => (name === 'theme' ? value : undefined) };
}

describe('themeOf', () => {
	it('reads dark only from a dark cookie', () => {
		expect(themeOf(jar('dark'))).toBe('dark');
		expect(themeOf(jar('light'))).toBe('light');
	});

	it('renders light when the cookie is missing or unrecognized', () => {
		// A first visit, and anything the script would not have written.
		expect(themeOf(jar())).toBe('light');
		expect(themeOf(jar('nord'))).toBe('light');
	});
});

describe('fillTheme', () => {
	const page = '<html class="%theme.class%"><script>%theme.script%</script></html>';

	it('classes a dark render and leaves a light one bare', () => {
		expect(fillTheme(page, 'dark')).toBe(
			`<html class="dark"><script>${themeScript}</script></html>`,
		);
		expect(fillTheme(page, 'light')).toBe(`<html class=""><script>${themeScript}</script></html>`);
	});

	it('fills only the script when the render does not know the theme', () => {
		expect(fillTheme(page)).toBe(
			`<html class="%theme.class%"><script>${themeScript}</script></html>`,
		);
	});
});

describe('reading and painting a theme', () => {
	function root(dark: boolean) {
		const classes = new Set(dark ? ['dark'] : []);
		return {
			classList: {
				contains: (name: string) => classes.has(name),
				toggle: (name: string, on: boolean) => (on ? classes.add(name) : classes.delete(name)),
			},
		} as unknown as HTMLElement;
	}

	it('reads what is painted rather than what was stored', () => {
		// On a first visit there is no cookie yet and the class is already correct, which is
		// exactly when the two would disagree.
		expect(currentTheme(root(true))).toBe('dark');
		expect(currentTheme(root(false))).toBe('light');
	});

	it('paints either theme, from either theme', () => {
		const element = root(false);
		applyTheme('dark', element);
		expect(currentTheme(element)).toBe('dark');
		applyTheme('dark', element);
		expect(currentTheme(element)).toBe('dark');
		applyTheme('light', element);
		expect(currentTheme(element)).toBe('light');
	});
});

describe('followSystemTheme', () => {
	it('applies the current mode and follows changes', () => {
		const modes: boolean[] = [];
		let listener: (() => void) | undefined;
		let removed: (() => void) | undefined;
		const root = {
			classList: { toggle: (_name: string, enabled: boolean) => modes.push(enabled) },
		} as unknown as HTMLElement;
		const media = {
			matches: false,
			addEventListener: (_type: string, callback: () => void) => {
				listener = callback;
			},
			removeEventListener: (_type: string, callback: () => void) => {
				removed = callback;
			},
		} as unknown as MediaQueryList;

		const stop = followSystemTheme(root, media);
		(media as unknown as { matches: boolean }).matches = true;
		listener?.();
		stop();

		expect(modes).toEqual([false, true]);
		expect(removed).toBe(listener);
	});
});

describe('observeTheme', () => {
	function watched(dark: boolean) {
		const classes = new Set(dark ? ['dark'] : []);
		const root = {
			classList: {
				contains: (name: string) => classes.has(name),
				toggle: (name: string, on: boolean) => (on ? classes.add(name) : classes.delete(name)),
			},
		} as unknown as HTMLElement;
		return { root, classes };
	}

	it('reports a change once, and says nothing about the mutations that are not one', () => {
		const { root, classes } = watched(false);
		const seen: Theme[] = [];
		let react!: () => void;
		let disconnected = false;
		const observer = {
			observe: () => {},
			disconnect: () => (disconnected = true),
		} as unknown as MutationObserver;

		const stop = observeTheme(
			(theme) => seen.push(theme),
			root,
			(callback) => {
				react = callback;
				return observer;
			},
		);

		// The palette names ride on the same attribute, so a mutation is not itself an answer.
		classes.add('nord');
		react();
		applyTheme('dark', root);
		react();
		react();

		expect(seen).toEqual(['dark']);
		stop();
		expect(disconnected).toBe(true);
	});
});
