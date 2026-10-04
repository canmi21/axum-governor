import { PUBLIC_LANGUAGE, type LocaleCode } from './index.ts';

/**
 * Formatting the site does the same way everywhere, gathered because it was not.
 *
 * Each of these was written out two to five times, and the copies had begun to disagree in the
 * one way a reader can see: the same magnitude was abbreviated `1.5k` on one widget and `1.5K` on
 * another. Nothing was wrong with either; what was wrong was that the page had two answers.
 */

/**
 * The tag `Intl` should be given for a view. The source view has no language of its own, so it
 * borrows English for the numbers around its untranslated words. Derived from `PUBLIC_LANGUAGE`
 * rather than restated: the licence pages carried their own copy, four times, spelling `tw` out
 * as a special case -- it produced identical output, which is why nobody noticed the duplication.
 */
export function intlLocale(locale: LocaleCode): string {
	return locale === 'mw' ? PUBLIC_LANGUAGE.en : PUBLIC_LANGUAGE[locale];
}

/**
 * A date as the site writes it: `Apr 13, 2026`.
 *
 * **UTC, always.** The day shown has to match the date in the article's frontmatter, and a page
 * prerendered west of Greenwich would otherwise render the day before. That reason lived beside
 * one of the three copies of this and not the other two.
 *
 * English, also always. These are timestamps on cards and article headers rather than prose, and
 * a translated month name beside an untranslated title reads as a mistake rather than a courtesy.
 */
export function shortDate(value: string | number | Date): string {
	return new Intl.DateTimeFormat('en-US', {
		month: 'short',
		day: 'numeric',
		year: 'numeric',
		timeZone: 'UTC',
	}).format(new Date(value));
}

/**
 * A count shortened to fit a stat row or a chart axis: `950`, `1.5k`, `16k`, `2.3M`. Lowercase
 * `k` and uppercase `M`, which is what SI writes rather than a house style to remember -- the two
 * copies this replaces had picked opposite conventions. One decimal below ten thousand and none
 * above it: past that the tenth is noise, and the shorter label is worth more on an axis.
 */
export function compactCount(value: number): string {
	if (value >= 1_000_000) return `${trim(value / 1_000_000)}M`;
	if (value >= 10_000) return `${Math.round(value / 1_000)}k`;
	if (value >= 1_000) return `${trim(value / 1_000)}k`;
	return value.toString();
}

/** One decimal, and none at all when it would be a zero. */
function trim(value: number): string {
	return value.toFixed(1).replace(/\.0$/, '');
}
