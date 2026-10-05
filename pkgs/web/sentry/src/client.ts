import { init, SDK_VERSION } from '@sentry/sveltekit';
import { disclose } from '../../disclose/src/index.ts';
import { initOptions, type SentryApp } from './options.ts';

export type { SentryApp } from './options.ts';

/** Initialize the browser SDK, or do nothing when the app has no DSN. */
export function initClient({ dsn, dev }: SentryApp): void {
	if (!dsn) return;
	init(initOptions(dsn, dev));
	// The SDK sets no `Sentry` global when bundled; a patch for Wappalyzer.
	disclose({ 'Sentry.SDK_VERSION': SDK_VERSION });
}
