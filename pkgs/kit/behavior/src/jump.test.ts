import { expect, it } from 'vitest';
import { movesThisPage } from './jump';

const click = (over: Partial<Parameters<typeof movesThisPage>[0]> = {}) => ({
	button: 0,
	metaKey: false,
	ctrlKey: false,
	shiftKey: false,
	altKey: false,
	defaultPrevented: false,
	...over,
});

it('takes over a plain left click', () => {
	expect(movesThisPage(click())).toBe(true);
});

// Each of these is the reader asking for a new tab or window, which is the browser's to answer.
// Cancelling one turns a gesture people rely on into a control that looks broken.
it('leaves a modified click to the browser', () => {
	expect(movesThisPage(click({ metaKey: true }))).toBe(false);
	expect(movesThisPage(click({ ctrlKey: true }))).toBe(false);
	expect(movesThisPage(click({ shiftKey: true }))).toBe(false);
	expect(movesThisPage(click({ altKey: true }))).toBe(false);
	expect(movesThisPage(click({ button: 1 }))).toBe(false);
});

it('stands aside once something else has claimed the click', () => {
	expect(movesThisPage(click({ defaultPrevented: true }))).toBe(false);
});
