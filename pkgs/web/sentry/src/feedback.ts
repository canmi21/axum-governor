/**
 * Sentry's feedback dialog, in a module nothing imports until a reader asks for it.
 *
 * Its widget is 24KB gzipped, so it is a module of its own and an app reaches it with
 * `await import('@canmi/web/sentry/feedback')` from the control that opens it, never at the top
 * of a file: that keeps it out of every page's entry, and out of the server, where
 * `@sentry/sveltekit` has no `getFeedback`. Named imports, because a namespace import forces every
 * export live and the bundler could no longer split this off. See spec/web/sentry.md.
 */
import { feedbackIntegration, getClient, getFeedback } from '@sentry/sveltekit';

/** What this uses of Sentry's dialog. Its own type is not part of the published surface. */
type Dialog = { appendToDom: () => void; open: () => void };

let form: Dialog | undefined;

/**
 * Show the dialog, registering the integration the first time.
 *
 * Answers whether it opened: not without a client, which an app without a DSN never makes, and
 * not when the dialog itself fails. A caller offers another way to write when it did not. In
 * development it opens, and what is sent is dropped.
 */
export async function openReport(): Promise<boolean> {
	try {
		const client = getClient();
		if (!client) return false;
		// `autoInject: false`, or Sentry floats a button of its own in the corner as well.
		if (!getFeedback()) client.addIntegration(feedbackIntegration({ autoInject: false }));
		const feedback = getFeedback();
		if (!feedback) return false;
		form ??= await feedback.createForm();
		form.appendToDom();
		form.open();
		return true;
	} catch {
		return false;
	}
}
