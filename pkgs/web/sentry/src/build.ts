import type { sentrySvelteKit } from '@sentry/sveltekit/vite';

/** What `sentrySvelteKit` takes. Its own name for this is not part of the published surface. */
export type SentryPluginOptions = NonNullable<Parameters<typeof sentrySvelteKit>[0]>;

/** The environment a build reads, passed in so a test can hand it one. */
export type BuildEnv = Readonly<Record<string, string | undefined>>;

/** The Sentry organization every app's project lives in. */
export const SENTRY_ORG = 'canmi';

export interface UploadPolicy {
	/**
	 * Throw when CI builds without `SENTRY_AUTH_TOKEN`, for an app whose CI build is the one
	 * deployed. The site sets it; the status page, built on Vercel, does not.
	 */
	requireTokenInCi?: boolean;
}

/**
 * Whether this build sends its source maps to Sentry: only when `SENTRY_AUTH_TOKEN` is set.
 *
 * `SENTRY_SKIP_UPLOAD`, set in `mise.toml`, turns it off whatever the token says -- see
 * spec/architecture/data.md, "A CI build compiles the site, and no longer compiles the corpus".
 * Any non-empty value skips, so `SENTRY_SKIP_UPLOAD= pnpm run build` is how one local build
 * uploads after all; `0` and `false` skip too, since this is a switch and parses no words.
 */
export function uploadsSourceMaps(env: BuildEnv, policy: UploadPolicy = {}): boolean {
	if (env.SENTRY_SKIP_UPLOAD) return false;
	const token = env.SENTRY_AUTH_TOKEN;
	if (!token && env.CI && policy.requireTokenInCi) {
		throw new Error(
			'SENTRY_AUTH_TOKEN is unset in CI. Add it as an encrypted build variable, or the ' +
				'deployed worker will report every error without a usable stack trace.',
		);
	}
	return Boolean(token);
}

export interface PluginRequest {
	/** The project's slug in {@link SENTRY_ORG}. */
	project: string;
	/** The answer of {@link uploadsSourceMaps}, asked once by the caller. */
	upload: boolean;
	env: BuildEnv;
	/** Globs of the maps the adapter wrote, deleted once they are uploaded. */
	mapsToDelete: string[];
}

/**
 * The options every app hands `sentrySvelteKit`.
 *
 * The skip drives `autoUploadSourceMaps` rather than only withholding the token, because the
 * plugin reads `SENTRY_AUTH_TOKEN` from the environment itself. Maps are deleted after the upload,
 * so the deployed output carries none.
 */
export function pluginOptions({
	project,
	upload,
	env,
	mapsToDelete,
}: PluginRequest): SentryPluginOptions {
	return {
		org: SENTRY_ORG,
		project,
		autoUploadSourceMaps: upload,
		authToken: upload ? env.SENTRY_AUTH_TOKEN : undefined,
		telemetry: false,
		sourcemaps: { filesToDeleteAfterUpload: mapsToDelete },
	};
}

/**
 * Vite's `build.sourcemap` for a build that does or does not upload.
 *
 * `hidden` emits maps without the `sourceMappingURL` comment, so no browser asks for a file that
 * was deleted. A build that skips emits none, since deletion only follows an upload and a map
 * left behind would ship the app's source as a static asset.
 */
export function sourcemapSetting(upload: boolean): 'hidden' | false {
	return upload ? 'hidden' : false;
}
