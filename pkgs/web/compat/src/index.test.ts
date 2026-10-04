import { expect, it } from 'vitest';
import { prepareBrowserRuntime } from './index';

it('installs non-mutating array sorting before an older browser hydrates', async () => {
	const native = Object.getOwnPropertyDescriptor(Array.prototype, 'toSorted');
	// Simulate a browser that predates the change-array-by-copy methods.
	// eslint-disable-next-line no-extend-native
	Object.defineProperty(Array.prototype, 'toSorted', {
		configurable: true,
		writable: true,
		value: undefined,
	});

	try {
		await prepareBrowserRuntime();
		const input = [3, 1, 2];

		expect(input.toSorted()).toEqual([1, 2, 3]);
		expect(input).toEqual([3, 1, 2]);
	} finally {
		// Restore the shared test runtime for every following test.
		// eslint-disable-next-line no-extend-native
		if (native) Object.defineProperty(Array.prototype, 'toSorted', native);
		else Reflect.deleteProperty(Array.prototype, 'toSorted');
	}
});
