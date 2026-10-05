/**
 * `openReport` on a Worker, which has no dialog to open: what `workerd` resolves
 * `@canmi/web/sentry/feedback` to, since `@sentry/sveltekit` has no `getFeedback` there. See
 * spec/web/sentry.md.
 */
export async function openReport(): Promise<boolean> {
	return false;
}
