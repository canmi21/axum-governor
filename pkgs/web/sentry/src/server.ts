import { initCloudflareSentryHandle, sentryHandle } from '@sentry/sveltekit';
import type { Handle } from '@sveltejs/kit/hooks';
import { initOptions, type SentryApp } from './options.ts';

export type { SentryApp } from './options.ts';

/**
 * The handles that initialize and instrument the server SDK, first in `sequence`; none without a
 * DSN.
 *
 * `initCloudflareSentryHandle` serves both adapters: the `worker` export condition wraps the
 * request in Cloudflare's context, and the `node` one initializes on the first request.
 */
export function serverHandles({ dsn, dev }: SentryApp): Handle[] {
	if (!dsn) return [];
	return [initCloudflareSentryHandle(initOptions(dsn, dev)), sentryHandle()];
}
