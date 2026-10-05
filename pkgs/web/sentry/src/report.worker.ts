/**
 * `report` on a Worker, where nothing presses a control: what `workerd` resolves
 * `@canmi/web/sentry/report` to, so a Worker's bundle never reaches the dialog, whose imports
 * `@sentry/sveltekit` does not have there. See spec/web/sentry.md.
 */
export async function report(_mailto: string): Promise<void> {}
