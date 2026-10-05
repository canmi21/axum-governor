/**
 * What a "report a problem" control calls: Sentry's dialog when it opens, and the reader's mail
 * client at `mailto` when it does not -- no client, a dialog that fails, or the dialog's own chunk
 * not loading, as for a tab opened before a deploy. Imports nothing of Sentry itself, so a page
 * may import this at the top and on the server; the dialog is fetched on the press. See
 * spec/web/sentry.md.
 */
export async function report(mailto: string): Promise<void> {
	try {
		const { openReport } = await import('./feedback.ts');
		if (await openReport()) return;
	} catch {
		// Fall through to mail, as when the dialog did not open.
	}
	window.location.href = mailto;
}
