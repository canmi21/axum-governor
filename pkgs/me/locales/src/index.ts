/**
 * What each language calls itself, keyed by the tag the corpus stores.
 *
 * **Never translated, and never a code.** A reader who cannot read the interface still has to find
 * their own language in a list, so the name reads the same whichever view rendered it -- and a
 * writer looking at eight translations of one paragraph should not be reading `ko-KR` to work out
 * which is which. Only the pair that needs telling apart carries a qualifier: there is one English
 * here and two Chinese, and each is written in its own script.
 *
 * The site's language picker was the first consumer and the desktop client's segment view was
 * the second, which is what moved this out of `apps/site/src/lib/locale/switcher.ts`. That client
 * is archived and the surface replacing it inherits the same need, so the move still stands.
 */
export const LOCALE_TAGS = [
	'en-US',
	'zh-CN',
	'ja-JP',
	'de-DE',
	'ko-KR',
	'fr-FR',
	'es-ES',
	'zh-TW',
] as const;

export type LocaleTag = (typeof LOCALE_TAGS)[number];

export const ENDONYM = {
	'en-US': 'English',
	'zh-CN': '中文 (简体)',
	'zh-TW': '中文 (繁體)',
	'ja-JP': '日本語',
	'ko-KR': '한국어',
	'de-DE': 'Deutsch',
	'fr-FR': 'Français',
	'es-ES': 'Español',
} as const satisfies Record<LocaleTag, string>;

/** The tag's own name, or the tag itself when the corpus grows one this does not know. */
export function endonym(tag: string): string {
	return (ENDONYM as Record<string, string>)[tag] ?? tag;
}

/**
 * The codes the corpus keys a view by, and `mw` for the article's own language.
 *
 * The short internal spelling of the same set [[LOCALE_TAGS]] names in BCP-47. It moved here for
 * the reason those did: a third consumer appeared -- `@monoflake/artifacts`, which types a
 * published view -- and a vocabulary two packages spell out separately is a vocabulary that drifts.
 */
export const LOCALE_CODES = ['mw', 'de', 'en', 'es', 'fr', 'ja', 'ko', 'zh', 'tw'] as const;

export type LocaleCode = (typeof LOCALE_CODES)[number];

/**
 * The language this site's own copy is written in -- not a fallback for a missing article
 * language. It moved here when the API needed to name a locale no article carries a view of,
 * which is the third consumer this table has collected. See lattice's spec/locale/interface.md.
 */
export const SITE_LANGUAGE = 'en-US';

/**
 * What a code is called in public, which is the only place BCP-47 and the corpus's codes meet.
 *
 * `mw` has no entry: the source view's tag is whatever language the article was written in, so
 * it is the article's to say and not this table's. See lattice's spec/locale/addressing.md.
 */
export const PUBLIC_LANGUAGE = {
	de: 'de-DE',
	en: 'en-US',
	es: 'es-ES',
	fr: 'fr-FR',
	ja: 'ja-JP',
	ko: 'ko-KR',
	zh: 'zh-CN',
	tw: 'zh-TW',
} as const satisfies Record<Exclude<LocaleCode, 'mw'>, string>;

const CODE_SET = new Set<string>(LOCALE_CODES);
const LANGUAGE_TAG_SHAPE = /^[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$/;

/**
 * The four below are here because the compiler needs them and a request handler does not.
 *
 * What a tag is belongs to this package; what a response does about one -- the cookie, the
 * `Accept-Language` negotiation, the `Vary` -- stays in the application, which is why that half
 * did not move. The boundary was drawn by the second consumer rather than chosen in advance.
 */
export function localeCode(value: string | null | undefined): LocaleCode | undefined {
	return value != null && CODE_SET.has(value) ? (value as LocaleCode) : undefined;
}

/** The public URL for a selected view; the source view keeps the bare address. */
export function localeUrl(url: string, code: LocaleCode): string {
	return code === 'mw' ? url : `${url}?lang=${code}`;
}

/** A public BCP-47 tag. `mw` is the source article, so its tag is article-owned. */
export function languageTag(code: LocaleCode, sourceLanguage: string): string {
	return code === 'mw' ? sourceLanguage : PUBLIC_LANGUAGE[code];
}

/** Reject malformed source metadata before it reaches `<html lang>` or `og:locale`. */
export function assertLanguageTag(value: unknown, file: string): asserts value is string {
	if (typeof value !== 'string' || !LANGUAGE_TAG_SHAPE.test(value)) {
		throw new Error(`${file}: invalid BCP-47 lang frontmatter ${JSON.stringify(value)}`);
	}
}
