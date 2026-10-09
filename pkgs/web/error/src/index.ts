/**
 * What an error page is, as data: which side failed, and the protocol's name for a status. The
 * page drawn from it and its words are each app's own. See spec/web/error.md.
 */

/** Which side failed: the server answering, or the browser after the page was served. */
export type Origin = 'server' | 'client';

/**
 * The field an app's `App.Error` carries, **absent when nothing failed**: an `error()` keeps its
 * own body, so only an unexpected error is stamped. See spec/web/error.md, "Only a failure is
 * stamped".
 */
export interface Stamped {
	origin?: Origin;
}

/**
 * The function `handleErrorWithSentry` is given on one side: an unknown error, thrown by code
 * rather than by `error()` or SvelteKit itself, is stamped as that side's; the rest keep the
 * status and message they came with.
 */
export function stamp(side: Origin): (input: { kind: string }) => Stamped | undefined {
	return ({ kind }) => (kind === 'unknown' ? { origin: side } : undefined);
}

/**
 * Which of the two pages an error is: `client` where the browser broke, which has no status to
 * show since nothing answered, and `status` for everything else, a 404 whichever side found it.
 */
export function pageOf(error: Stamped | null | undefined): 'client' | 'status' {
	return error?.origin === 'client' ? 'client' : 'status';
}

/**
 * The protocol's own names for the statuses a page shows, in English whatever the page answers in:
 * a reader who meets `404` meets `Not Found` with it everywhere else.
 */
export const STATUS_TEXT: Readonly<Record<number, string>> = {
	400: 'Bad Request',
	401: 'Unauthorized',
	403: 'Forbidden',
	404: 'Not Found',
	405: 'Method Not Allowed',
	410: 'Gone',
	429: 'Too Many Requests',
	500: 'Internal Server Error',
	502: 'Bad Gateway',
	503: 'Service Unavailable',
	504: 'Gateway Timeout',
};

/** `status`'s name, or `Error` for one the table does not hold. */
export const statusText = (status: number): string => STATUS_TEXT[status] ?? 'Error';

/**
 * The title of the page with no status, in the same Title Case the protocol's names are: the
 * failure the protocol has no name for.
 */
export const CLIENT_TITLE = 'Unexpected Client Behavior';
