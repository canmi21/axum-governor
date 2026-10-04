import { init } from '@sentry/sveltekit';
import { initOptions, type SentryApp } from './options.ts';

export type { SentryApp } from './options.ts';

/** Initialize the browser SDK, or do nothing when the app has no DSN. */
export function initClient({ dsn, dev }: SentryApp): void {
	if (!dsn) return;
	init(initOptions(dsn, dev));
}
