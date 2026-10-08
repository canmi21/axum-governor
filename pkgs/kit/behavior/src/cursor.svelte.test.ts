import { describe, expect, it, vi } from 'vitest';

// The runes are the Svelte compiler's, which this suite does not run; outside a component a field
// declared with `$state` holds its value the way a plain field does, which is all these read.
vi.hoisted(() => {
	Object.assign(globalThis, { $state: <T>(value: T) => value });
});

import { ListCursor, reveal } from './cursor.svelte';

type Press = Partial<KeyboardEvent> & { prevented?: boolean };

function press(key: string, over: Partial<KeyboardEvent> = {}): KeyboardEvent & Press {
	const event: Press = {
		key,
		isComposing: false,
		...over,
		preventDefault() {
			event.prevented = true;
		},
	};
	return event as KeyboardEvent & Press;
}

function cursor(count: number) {
	const list = new ListCursor();
	list.count = count;
	return list;
}

const ignore = () => {};

describe('a list cursor', () => {
	it('moves with the arrows and wraps at both ends', () => {
		const list = cursor(3);
		list.key(press('ArrowDown'), ignore);
		expect(list.active).toBe(1);
		list.key(press('ArrowDown'), ignore);
		list.key(press('ArrowDown'), ignore);
		expect(list.active).toBe(0);
		list.key(press('ArrowUp'), ignore);
		expect(list.active).toBe(2);
	});

	it('goes to the ends with Home and End', () => {
		const list = cursor(5);
		list.key(press('End'), ignore);
		expect(list.active).toBe(4);
		list.key(press('Home'), ignore);
		expect(list.active).toBe(0);
	});

	it('chooses the active row with Enter, and cancels only the keys it took', () => {
		const list = cursor(3);
		const chosen: number[] = [];
		list.point(2);
		const enter = press('Enter');
		expect(list.key(enter, (index) => chosen.push(index))).toBe(true);
		expect(chosen).toEqual([2]);
		expect(enter.prevented).toBe(true);
		const letter = press('a');
		expect(list.key(letter, ignore)).toBe(false);
		expect(letter.prevented).toBeUndefined();
	});

	// Enter inside an IME composition commits what is being written, not a row.
	it('leaves a composing keystroke to the writer', () => {
		const list = cursor(3);
		const chosen: number[] = [];
		expect(list.key(press('Enter', { isComposing: true }), (index) => chosen.push(index))).toBe(
			false,
		);
		expect(chosen).toEqual([]);
	});

	it('handles nothing with nothing to move through', () => {
		const list = cursor(0);
		expect(list.key(press('ArrowDown'), ignore)).toBe(false);
		expect(list.active).toBe(0);
	});

	it('follows the pointer, inside the rows only', () => {
		const list = cursor(3);
		list.point(1);
		expect(list.active).toBe(1);
		list.point(7);
		expect(list.active).toBe(1);
	});

	it('stays inside the rows when there are fewer of them', () => {
		const list = cursor(5);
		list.point(4);
		list.count = 2;
		expect(list.active).toBe(1);
		list.count = 0;
		expect(list.active).toBe(0);
	});
});

describe('the combobox wiring', () => {
	const ids = { list: 'results', option: (index: number) => `result-${index}` };

	it('points the field at the active row while the list has rows', () => {
		const list = cursor(3);
		list.point(1);
		const wiring = list.combobox(ids);
		expect(wiring.input).toEqual({
			role: 'combobox',
			'aria-controls': 'results',
			'aria-expanded': true,
			'aria-autocomplete': 'list',
			'aria-activedescendant': 'result-1',
		});
		expect(wiring.list).toEqual({ role: 'listbox', id: 'results' });
		expect(wiring.option(1)).toEqual({ role: 'option', id: 'result-1', 'aria-selected': true });
		expect(wiring.option(0)['aria-selected']).toBe(false);
	});

	it('is collapsed, with no active row, while the list is empty', () => {
		const wiring = cursor(0).combobox(ids);
		expect(wiring.input['aria-expanded']).toBe(false);
		expect(wiring.input['aria-activedescendant']).toBeUndefined();
	});
});

describe('revealing a row', () => {
	it('scrolls the row by its role as little as it can', () => {
		const scrolled: unknown[] = [];
		const rows = [0, 1, 2].map((index) => ({
			scrollIntoView: (options: unknown) => scrolled.push([index, options]),
		}));
		const list = {
			querySelectorAll: (selector: string) => (selector === '[role="option"]' ? rows : []),
		} as unknown as HTMLElement;
		reveal(list, 2);
		reveal(list, 9);
		expect(scrolled).toEqual([[2, { block: 'nearest' }]]);
	});
});
