import { afterEach, expect, it, vi } from 'vitest';

const sentry = vi.hoisted(() => ({
	getClient: vi.fn(),
	getFeedback: vi.fn(),
	feedbackIntegration: vi.fn(() => ({ name: 'Feedback' })),
}));
vi.mock('@sentry/sveltekit', () => sentry);

afterEach(() => {
	vi.resetModules();
	vi.clearAllMocks();
});

it('does not open without a client', async () => {
	sentry.getClient.mockReturnValue(undefined);
	const { openReport } = await import('./feedback.ts');
	expect(await openReport()).toBe(false);
});

it('registers the dialog once and opens it', async () => {
	const form = { appendToDom: vi.fn(), open: vi.fn() };
	const feedback = { createForm: vi.fn(async () => form) };
	const client = { addIntegration: vi.fn() };
	sentry.getClient.mockReturnValue(client);
	sentry.getFeedback.mockReturnValueOnce(undefined).mockReturnValue(feedback);
	const { openReport } = await import('./feedback.ts');
	expect(await openReport()).toBe(true);
	expect(await openReport()).toBe(true);
	expect(client.addIntegration).toHaveBeenCalledOnce();
	expect(feedback.createForm).toHaveBeenCalledOnce();
	expect(form.open).toHaveBeenCalledTimes(2);
});

it('answers that it did not open when the dialog fails', async () => {
	sentry.getClient.mockReturnValue({ addIntegration: vi.fn() });
	sentry.getFeedback.mockReturnValue({
		createForm: vi.fn(async () => {
			throw new Error('no dialog');
		}),
	});
	const { openReport } = await import('./feedback.ts');
	expect(await openReport()).toBe(false);
});
