import { afterEach, beforeEach, expect, it, vi } from 'vitest';

const loads = vi.hoisted(() => ({ count: 0 }));

const CASES: { name: string; owner: object; key: string }[] = [
	{ name: 'Array.prototype.toSorted', owner: Array.prototype, key: 'toSorted' },
	{ name: 'URL.canParse', owner: URL, key: 'canParse' },
];

const native = new Map(CASES.map((c) => [c.name, Object.getOwnPropertyDescriptor(c.owner, c.key)]));

beforeEach(() => {
	loads.count = 0;
	vi.resetModules();
	vi.doMock('core-js/stable', () => {
		loads.count += 1;
		return {};
	});
});

afterEach(() => {
	for (const c of CASES) {
		const descriptor = native.get(c.name);
		if (descriptor) Object.defineProperty(c.owner, c.key, descriptor);
		else Reflect.deleteProperty(c.owner, c.key);
	}
});

async function run(): Promise<number> {
	const { prepareBrowserRuntime } = await import('./index');
	await prepareBrowserRuntime();
	return loads.count;
}

it('loads nothing when every canary is present', async () => {
	expect(await run()).toBe(0);
});

for (const c of CASES) {
	it(`loads core-js when ${c.name} is missing`, async () => {
		Object.defineProperty(c.owner, c.key, { configurable: true, writable: true, value: undefined });
		expect(await run()).toBe(1);
	});

	it(`loads nothing when ${c.name} is present`, async () => {
		expect(typeof (c.owner as Record<string, unknown>)[c.key]).toBe('function');
		expect(await run()).toBe(0);
	});
}
