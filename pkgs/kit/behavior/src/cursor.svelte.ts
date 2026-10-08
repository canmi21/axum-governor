/**
 * Which row of a list the keyboard is on, while focus stays in the field above it. See
 * spec/kit/behavior.md.
 *
 * The field keeps focus and the row is only pointed at, which is the WAI-ARIA combobox pattern --
 * https://www.w3.org/WAI/ARIA/apg/patterns/combobox/ -- so `combobox` hands out the attributes
 * that let a screen reader follow the row the eye is following.
 */

/** The attributes the field, the list and each row carry, ready to spread. */
export type ComboboxAttributes = {
	input: {
		role: 'combobox';
		'aria-controls': string;
		'aria-expanded': boolean;
		'aria-autocomplete': 'list';
		'aria-activedescendant': string | undefined;
	};
	list: { role: 'listbox'; id: string };
	option: (index: number) => { role: 'option'; id: string; 'aria-selected': boolean };
};

export class ListCursor {
	/** The row the keyboard is on. */
	active = $state(0);
	#count = $state(0);

	/** How many rows there are; told by the list, and holding `active` inside them. */
	get count(): number {
		return this.#count;
	}

	set count(count: number) {
		this.#count = Math.max(0, count);
		if (this.active >= this.#count) this.active = Math.max(0, this.#count - 1);
	}

	/**
	 * Answer a keydown in the field: the arrows move and wrap, Home and End go to the ends, Enter
	 * chooses. Returns whether the key was this list's, and cancels it then, so the caret stays
	 * where it was.
	 *
	 * A key that is part of an IME composition is the reader still writing -- Enter there commits
	 * pinyin rather than choosing a row -- so it is left alone.
	 */
	key(event: KeyboardEvent, onChoose: (index: number) => void): boolean {
		const count = this.#count;
		if (count === 0 || event.isComposing) return false;
		switch (event.key) {
			case 'ArrowDown':
				this.active = (this.active + 1) % count;
				break;
			case 'ArrowUp':
				this.active = (this.active - 1 + count) % count;
				break;
			case 'Home':
				this.active = 0;
				break;
			case 'End':
				this.active = count - 1;
				break;
			case 'Enter':
				onChoose(this.active);
				break;
			default:
				return false;
		}
		event.preventDefault();
		return true;
	}

	/** The pointer is over a row. */
	point(index: number): void {
		if (index >= 0 && index < this.#count) this.active = index;
	}

	/**
	 * The attributes for the field, the list and each row, read from where the cursor is now.
	 *
	 * The list counts as expanded while it has rows, and the field names the active row as its
	 * active descendant then -- the row the eye is on is the row a screen reader announces.
	 */
	combobox(ids: { list: string; option: (index: number) => string }): ComboboxAttributes {
		const open = this.#count > 0;
		const active = this.active;
		return {
			input: {
				role: 'combobox',
				'aria-controls': ids.list,
				'aria-expanded': open,
				'aria-autocomplete': 'list',
				'aria-activedescendant': open ? ids.option(active) : undefined,
			},
			list: { role: 'listbox', id: ids.list },
			option: (index) => ({
				role: 'option',
				id: ids.option(index),
				'aria-selected': index === active,
			}),
		};
	}
}

/**
 * Scroll the `index`th row of `list` into view, moving as little as it can.
 *
 * Rows are found by their `option` role rather than as children, so a list that groups its rows
 * under headings still counts them as the cursor does.
 */
export function reveal(list: HTMLElement, index: number): void {
	list.querySelectorAll('[role="option"]')[index]?.scrollIntoView({ block: 'nearest' });
}
