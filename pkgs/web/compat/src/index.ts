/**
 * Browser features missing from a browser that once crashed a reader; any one absent loads
 * core-js. A new one met in production is one more line here and one in canaries.test.ts.
 *
 * Why a list of met cases and not a complete one, why `stable` and not `es` or `actual` -- see
 * spec/compat.md, "The API floor".
 */
export const CANARIES: { name: string; present: () => boolean }[] = [
	{
		name: 'Array.prototype.toSorted',
		present: () => typeof Array.prototype.toSorted === 'function',
	},
	{ name: 'URL.canParse', present: () => typeof URL.canParse === 'function' },
];

export async function prepareBrowserRuntime(): Promise<void> {
	if (CANARIES.every((canary) => canary.present())) return;
	// core-js ships no declaration for its entry points.
	// @ts-expect-error TS7016
	await import('core-js/stable');
}
