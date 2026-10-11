import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { CONTRACT } from '../../contract/src/index.ts';

/** Every style is a stylesheet beside this test; what a style keeps under it is in a directory. */
const STYLES = import.meta.dirname;

/** The names a rule declares, from its selector's opening brace to its close. */
function declared(css: string, selector: RegExp): string[] {
	const start = css.search(selector);
	const body = css.slice(css.indexOf('{', start) + 1, css.indexOf('\n}', start));
	return [...body.matchAll(/^\t(--[a-z0-9-]+):/gm)].map(([, name]) => name as string);
}

describe.each(readdirSync(STYLES).filter((file) => file.endsWith('.css')))('%s', (file) => {
	const css = readFileSync(join(STYLES, file), 'utf8');

	it("gives every name in the contract a value, on the root and on either theme's mark", () => {
		const names = declared(css, /^:root,\n\[data-theme='light'\],\n\[data-theme='dark'\] \{/m);
		for (const name of CONTRACT.flatMap((group) => group.names)) {
			expect(names).toContain(name);
		}
	});

	it("puts every name it declares under the contract's groups, or its own parameters", () => {
		const groups = CONTRACT.map(({ group }) => `--${group}`);
		for (const name of declared(css, /^:root,/m)) {
			expect(groups.some((group) => name === group || name.startsWith(`${group}-`))).toBe(true);
		}
	});
});
