import { inlineScriptString } from './inline-script.ts';

const SYSTEM_DARK_QUERY = '(prefers-color-scheme:dark)';

export type Theme = 'light' | 'dark';

/**
 * How the theme cookie is written, wherever it is written.
 *
 * The script below settles the first visit and a control settles every one after it, so both
 * write this cookie -- and a control that wrote a shorter life, or a different path, would let
 * the two disagree about a preference the reader only set once. One string, interpolated into
 * the script and returned to the control.
 */
const COOKIE_ATTRIBUTES = ';path=/;max-age=31536000;SameSite=Lax';

export const themeScript = `(function(){var mm=document.cookie.match(/\\btheme=(light|dark)\\b/);var pm=document.cookie.match(/\\bpalette=(nord|contrast)\\b/);var m=mm?mm[1]:window.matchMedia(${inlineScriptString(SYSTEM_DARK_QUERY)}).matches?"dark":"light";var h=document.documentElement;if(m==="dark")h.classList.add("dark");if(pm)h.classList.add(pm[1]);if(!mm)document.cookie="theme="+m+${inlineScriptString(COOKIE_ATTRIBUTES)}})()`;

/**
 * The theme a request asks for, read the way the server reads it: `dark` only when the cookie
 * says so. A first visit has no cookie and renders light; the script above corrects it before
 * the first frame.
 */
export function themeOf(cookies: { get(name: string): string | undefined }): Theme {
	return cookies.get('theme') === 'dark' ? 'dark' : 'light';
}

/**
 * Fill an `app.html`'s theme placeholders: `%theme.script%` always, and `%theme.class%` when the
 * render knows the theme. A render shared between readers passes none -- see
 * spec/styling/palettes.md, "Light and dark are one cookie, read the same way everywhere".
 */
export function fillTheme(html: string, theme?: Theme): string {
	const classed = theme ? html.replace('%theme.class%', theme === 'dark' ? 'dark' : '') : html;
	return classed.replace('%theme.script%', themeScript);
}

/** The cookie a control writes when the reader picks a theme. */
export function themeCookie(theme: Theme): string {
	return `theme=${theme}${COOKIE_ATTRIBUTES}`;
}

/**
 * Which theme is on screen right now, read off the document rather than off the cookie.
 *
 * The class is what the page is actually painted from, and the script above sets it before the
 * first frame whether or not a cookie existed. Reading the cookie instead would answer nothing
 * on a first visit, which is exactly when the two can differ.
 */
export function currentTheme(root: HTMLElement = document.documentElement): Theme {
	return root.classList.contains('dark') ? 'dark' : 'light';
}

/** Paint a theme. Toggling the class is the whole of it; every colour is a token beneath it. */
/**
 * Move the class with every transition in the document switched off, then switch them back on.
 *
 * A theme repaints in one frame, but anything easing a colour for its own reasons eases this one
 * too: measured on an article's link cards, 150ms of greys belonging to neither theme, behind a
 * page that had already finished. **The reflow is the whole trick** -- without a layout read
 * between the writes the browser folds all three into one recalculation, sees only the ends, and
 * transitions anyway.
 */
function withoutTransitions(change: () => void, root: HTMLElement): void {
	// A root outside a document has nothing painting from it, so there is nothing to suppress and
	// the sheet would have nowhere to go. Make the change and return.
	const document_ = root.ownerDocument as Document | undefined;
	if (!document_?.head) {
		change();
		return;
	}
	const suppress = document_.createElement('style');
	suppress.textContent = '*,*::before,*::after{transition:none!important;animation:none!important}';
	// `appendChild` rather than `append`: this library is read by the workers too, and their
	// types give `append` a different signature on the same name.
	document_.head.appendChild(suppress);
	change();
	// Read, and do not remove the read: it is what forces the new values to be committed under
	// the rule above rather than after it.
	void root.offsetHeight;
	suppress.remove();
}

export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
	withoutTransitions(() => root.classList.toggle('dark', theme === 'dark'), root);
}

/**
 * Call back whenever the painted theme changes, and return the unsubscribe.
 *
 * For whatever copied a colour out of the tokens and so cannot repaint on its own -- see
 * spec/styling/blocks.md, "A diagram is drawn in both themes at once, because the palette is
 * inside the SVG". The class is watched rather than `applyTheme` announcing, so `currentTheme`
 * reads it. `create` is injected like `followSystemTheme`'s media query, so the node suite can
 * drive it without a DOM.
 */
export function observeTheme(
	callback: (theme: Theme) => void,
	root: HTMLElement = document.documentElement,
	create: (react: () => void) => MutationObserver = (react) => new MutationObserver(react),
): () => void {
	let painted = currentTheme(root);
	const observer = create(() => {
		const next = currentTheme(root);
		// The class also carries the palette names, so most mutations here settle nothing.
		if (next === painted) return;
		painted = next;
		callback(next);
	});
	observer.observe(root, { attributeFilter: ['class'] });
	return () => observer.disconnect();
}

export function followSystemTheme(
	root: HTMLElement = document.documentElement,
	media: MediaQueryList = window.matchMedia(SYSTEM_DARK_QUERY),
): () => void {
	// The same class, so the same one frame in which nothing may animate.
	const apply = () => withoutTransitions(() => root.classList.toggle('dark', media.matches), root);
	apply();
	media.addEventListener('change', apply);
	return () => media.removeEventListener('change', apply);
}
