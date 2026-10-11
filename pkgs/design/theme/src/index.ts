/**
 * The page's theme, marked as `data-theme` on the document's root: a script settles it before the
 * first frame from the reader's cookie or their system, and a control moves it after. Any element
 * below may carry a mark of its own -- see lib's spec/design/styles.md, "A theme is a mark, and any
 * element may carry one".
 */
import { inlineScriptString } from './inline-script.ts';

export type Theme = 'light' | 'dark';

const SYSTEM_DARK_QUERY = '(prefers-color-scheme:dark)';

/** How the cookie is written, by the script and by a control alike, so the two never disagree. */
const COOKIE_ATTRIBUTES = ';path=/;max-age=31536000;SameSite=Lax';

export const themeScript = `(function(){var c=document.cookie.match(/\\btheme=(light|dark)\\b/);var m=c?c[1]:window.matchMedia(${inlineScriptString(SYSTEM_DARK_QUERY)}).matches?"dark":"light";document.documentElement.setAttribute("data-theme",m);if(!c)document.cookie="theme="+m+${inlineScriptString(COOKIE_ATTRIBUTES)}})()`;

/** Fill an `app.html`'s `%theme.script%` with the script above. */
export function fillTheme(html: string): string {
	return html.replace('%theme.script%', themeScript);
}

/** The cookie a control writes when the reader picks a theme. */
export function themeCookie(theme: Theme): string {
	return `theme=${theme}${COOKIE_ATTRIBUTES}`;
}

/** The theme on screen, read off the root's mark, which the script sets whether a cookie was. */
export function currentTheme(root: Element = document.documentElement): Theme {
	return root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
}

/**
 * Move the mark with every transition switched off, then on again: a theme repaints in one frame,
 * and anything easing a color would ease this one too. The layout read between the writes is what
 * commits the new values under the rule rather than after it.
 */
function withoutTransitions(change: () => void, root: Element): void {
	const owner = root.ownerDocument as Document | undefined;
	if (!owner?.head) {
		change();
		return;
	}
	const suppress = owner.createElement('style');
	suppress.textContent = '*,*::before,*::after{transition:none!important;animation:none!important}';
	owner.head.appendChild(suppress);
	change();
	void (root as HTMLElement).offsetHeight;
	suppress.remove();
}

/** Paint a theme: the mark is the whole of it, every color a token beneath it. */
export function applyTheme(theme: Theme, root: Element = document.documentElement): void {
	withoutTransitions(() => root.setAttribute('data-theme', theme), root);
}
