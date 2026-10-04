import { describe, expect, it } from 'vitest';
import { pluginOptions, SENTRY_ORG, sourcemapSetting, uploadsSourceMaps } from './build';

describe('uploadsSourceMaps', () => {
	it('uploads only when the token is set', () => {
		expect(uploadsSourceMaps({ SENTRY_AUTH_TOKEN: 't' })).toBe(true);
		expect(uploadsSourceMaps({})).toBe(false);
		expect(uploadsSourceMaps({ SENTRY_AUTH_TOKEN: '' })).toBe(false);
	});

	it('skips on any non-empty SENTRY_SKIP_UPLOAD, whatever the token says', () => {
		expect(uploadsSourceMaps({ SENTRY_AUTH_TOKEN: 't', SENTRY_SKIP_UPLOAD: '1' })).toBe(false);
		expect(uploadsSourceMaps({ SENTRY_AUTH_TOKEN: 't', SENTRY_SKIP_UPLOAD: '0' })).toBe(false);
		expect(uploadsSourceMaps({ SENTRY_AUTH_TOKEN: 't', SENTRY_SKIP_UPLOAD: '' })).toBe(true);
	});

	it('skips silently in CI without a token unless the app requires one', () => {
		expect(uploadsSourceMaps({ CI: '1' })).toBe(false);
		expect(() => uploadsSourceMaps({ CI: '1' }, { requireTokenInCi: true })).toThrow(
			/SENTRY_AUTH_TOKEN is unset in CI/,
		);
		expect(uploadsSourceMaps({ CI: '1', SENTRY_AUTH_TOKEN: 't' }, { requireTokenInCi: true })).toBe(
			true,
		);
	});

	it('lets a required token be skipped on purpose', () => {
		expect(
			uploadsSourceMaps({ CI: '1', SENTRY_SKIP_UPLOAD: '1' }, { requireTokenInCi: true }),
		).toBe(false);
	});
});

describe('pluginOptions', () => {
	const env = { SENTRY_AUTH_TOKEN: 'secret' };
	const maps = ['.svelte-kit/cloudflare/**/*.map'];

	it('passes the token and deletes the maps when uploading', () => {
		expect(pluginOptions({ project: 'p', upload: true, env, mapsToDelete: maps })).toEqual({
			org: SENTRY_ORG,
			project: 'p',
			autoUploadSourceMaps: true,
			authToken: 'secret',
			telemetry: false,
			sourcemaps: { filesToDeleteAfterUpload: maps },
		});
	});

	it('withholds the token when not uploading', () => {
		const options = pluginOptions({ project: 'p', upload: false, env, mapsToDelete: maps });
		expect(options.autoUploadSourceMaps).toBe(false);
		expect(options.authToken).toBeUndefined();
	});

	it('emits hidden maps only for a build that uploads', () => {
		expect(sourcemapSetting(true)).toBe('hidden');
		expect(sourcemapSetting(false)).toBe(false);
	});
});
