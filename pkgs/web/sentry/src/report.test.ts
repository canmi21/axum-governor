import { afterEach, expect, it, vi } from 'vitest';

const feedback = vi.hoisted(() => ({ openReport: vi.fn() }));
vi.mock('./feedback.ts', () => feedback);

afterEach(() => {
	vi.unstubAllGlobals();
	vi.clearAllMocks();
});

it('leaves the page alone when the dialog opened', async () => {
	vi.stubGlobal('window', { location: { href: 'https://example.com/' } });
	feedback.openReport.mockResolvedValue(true);
	const { report } = await import('./report.ts');
	await report('mailto:support@example.com');
	expect(window.location.href).toBe('https://example.com/');
});

it('opens mail when the dialog did not open, or failed', async () => {
	const { report } = await import('./report.ts');
	for (const outcome of [() => Promise.resolve(false), () => Promise.reject(new Error('chunk'))]) {
		vi.stubGlobal('window', { location: { href: 'https://example.com/' } });
		feedback.openReport.mockImplementation(outcome);
		await report('mailto:support@example.com');
		expect(window.location.href).toBe('mailto:support@example.com');
	}
});
