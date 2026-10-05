import type { MiddlewareHandler } from 'hono';

/**
 * `X-Powered-By: Hono` on every answer, which is all Wappalyzer's Hono fingerprint reads -- and on
 * a page's own response alone, so an API answered in a browser is named too. Hono's own
 * `poweredBy` sets the header in place, which throws on an answer passed through from a `fetch`,
 * whose headers are immutable; that one is copied first. See spec/web/disclose.md.
 */
export function poweredBy(): MiddlewareHandler {
	return async (c, next) => {
		await next();
		try {
			c.res.headers.set('X-Powered-By', 'Hono');
		} catch {
			const headers = new Headers(c.res.headers);
			headers.set('X-Powered-By', 'Hono');
			c.res = new Response(c.res.body, {
				status: c.res.status,
				statusText: c.res.statusText,
				headers,
			});
		}
	};
}
