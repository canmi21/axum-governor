/**
 * A key held with the command key, bound on the whole window. See spec/kit/behavior.md.
 *
 * Meta or Control, either accepted, because a site is read on both kinds of machine and neither
 * audience should have to learn the other's key. Captured on the window so it works wherever the
 * reader is, and stood down inside a text field so it cannot steal a keystroke someone meant for
 * what they were writing.
 */

export type Modifiers = {
	/** Shift held as well. Unset means Shift must not be held, so `⌘⇧K` stays free for another binding. */
	shift?: boolean;
	/** Alt (Option) held as well, on the same terms as `shift`. */
	alt?: boolean;
};

/**
 * Whether `event` is `key` with the command key and exactly the modifiers asked for.
 *
 * `key` is absent when the keydown did not come from the browser: an extension, a password
 * manager's autofill or an automation harness dispatching a plain Event under the `keydown` name.
 * A string method on nothing threw for a live reader mid-sentence, so absent is simply no match.
 * With Alt asked for, the physical key is accepted too, since Option on a Mac rewrites `key` --
 * Option-K reports `˚`.
 */
export function matches(event: KeyboardEvent, key: string, options: Modifiers = {}): boolean {
	if (!(event.metaKey || event.ctrlKey)) return false;
	if (event.shiftKey !== (options.shift ?? false)) return false;
	if (event.altKey !== (options.alt ?? false)) return false;
	const wanted = key.toLowerCase();
	if (event.key?.toLowerCase() === wanted) return true;
	return event.altKey && event.code === `Key${wanted.toUpperCase()}`;
}

/** Whether focus is somewhere a keystroke is text: a field, a text area, a select, or editable. */
function inField(target: EventTarget | null): boolean {
	const element = target as Partial<HTMLElement> | null;
	if (!element) return false;
	if (element.isContentEditable) return true;
	const tag = element.tagName?.toUpperCase();
	return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
}

/**
 * Run `run` when `key` is pressed with the command key, anywhere in the window.
 *
 * Returns the function that unbinds it, so `$effect(() => shortcut('k', toggle))` binds it for as
 * long as the component lives. Inside a text field it stands down unless `inFields` is set -- a
 * palette whose own field should close on the same key that opened it is the case for setting it.
 * `target` is the window unless a test hands over something else.
 */
export function shortcut(
	key: string,
	run: (event: KeyboardEvent) => void,
	options: Modifiers & {
		inFields?: boolean;
		target?: Pick<EventTarget, 'addEventListener' | 'removeEventListener'>;
	} = {},
): () => void {
	const target = options.target ?? window;
	const listener = (event: Event) => {
		const keyboard = event as KeyboardEvent;
		if (!matches(keyboard, key, options)) return;
		if (!options.inFields && inField(keyboard.target)) return;
		keyboard.preventDefault();
		run(keyboard);
	};
	target.addEventListener('keydown', listener);
	return () => target.removeEventListener('keydown', listener);
}

/**
 * How the binding is written for the reader: `⌘K` on Apple platforms, `Ctrl K` elsewhere.
 *
 * Without a document this is the server, which answers `⌘` -- what an Apple browser hydrates to,
 * so the commonest reader sees no flicker. A document is the test rather than `navigator`, because
 * Node and Workers both have a `navigator` that would describe the server's machine instead.
 */
export function label(key: string, platform?: string): string {
	const named = platform ?? (typeof document === 'undefined' ? 'Mac' : platformOf(navigator));
	const letter = key.toUpperCase();
	return /mac|iphone|ipad|ipod/i.test(named) ? `⌘${letter}` : `Ctrl ${letter}`;
}

/** The platform a browser reports, through Client Hints where it has them. */
function platformOf(browser: Navigator): string {
	const hinted = (browser as Navigator & { userAgentData?: { platform?: string } }).userAgentData;
	return hinted?.platform || browser.platform || '';
}
