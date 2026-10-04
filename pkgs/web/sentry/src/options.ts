/**
 * What an app says about itself to the SDK, on either side.
 *
 * `dsn` absent means the app has no Sentry project: every helper then does nothing.
 */
export interface SentryApp {
	dsn: string | undefined;
	dev: boolean;
}

/** A transport that accepts every envelope and sends none of them. */
const silent = () => ({
	send: () => Promise.resolve({}),
	flush: () => Promise.resolve(true),
});

/**
 * The init options both sides share.
 *
 * Development initializes the SDK with every integration installed, so capture is exercised, and
 * hands it a transport that sends nothing. `enabled: false` would install no integrations at all.
 * See spec/analytics.md, "Development loads the client and reports nothing".
 */
export function initOptions(dsn: string, dev: boolean) {
	return {
		dsn,
		environment: dev ? 'development' : 'production',
		...(dev ? { transport: silent } : {}),
	};
}
