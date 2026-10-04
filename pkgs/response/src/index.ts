/**
 * The shape every API here answers in, from TypeScript and from Rust alike: `src/lib.rs` is the
 * other half, and both read `codes.json` and are tested against `src/fixtures.json`. See
 * spec/architecture/services.md, "Every answer is one envelope".
 */
import CODES from '../codes.json' with { type: 'json' };

/** One of ours, lowercase with underscores; not an HTTP status, which the response already has. */
export type Code = keyof typeof CODES;

/**
 * Success carries what the route answers; failure carries a code for a program and a message for
 * a person. What `data` holds is the route's business. See the workspace spec/json.md.
 */
export type ApiResponse<T> =
	| { status: 'success'; data: T }
	| { status: 'error'; code: Code; message: string };

export type ApiSuccess<T> = Extract<ApiResponse<T>, { status: 'success' }>;
export type ApiError = Extract<ApiResponse<never>, { status: 'error' }>;

/** A failure's body, with the code's own message unless the moment has a more exact one. */
export function errorBody(code: Code, message: string = CODES[code]): ApiError {
	return { status: 'error', code, message };
}

/** A failure as a response. Never cached unless the caller says otherwise. */
export function failure(
	status: number,
	code: Code,
	options: { message?: string; headers?: HeadersInit } = {},
): Response {
	return Response.json(errorBody(code, options.message), {
		status,
		headers: { 'Cache-Control': 'no-store', ...options.headers },
	});
}

/** A success as a response. */
export function success<T>(
	data: T,
	options: { status?: number; headers?: HeadersInit } = {},
): Response {
	return Response.json({ status: 'success', data } satisfies ApiResponse<T>, options);
}

/**
 * Read the envelope and hand back what is inside, or throw with the code and the message.
 *
 * The one place an answer is unwrapped. A route's own shape is checked by whoever asked for it, on
 * the value this returns: this function knows `status` and nothing else.
 */
export function unwrap<T>(body: unknown, source: string): T {
	if (typeof body !== 'object' || body === null || !('status' in body)) {
		throw new Error(`${source} answered something that is not an API response`);
	}
	const envelope = body as ApiResponse<T>;
	if (envelope.status === 'error') {
		throw new Error(`${source} answered ${envelope.code}: ${envelope.message}`);
	}
	if (envelope.status !== 'success') {
		throw new Error(`${source} answered an unknown status`);
	}
	return envelope.data;
}
