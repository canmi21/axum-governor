import { Hono } from 'hono';
import { expect, it } from 'vitest';
import { poweredBy } from './hono.ts';

it('names Hono on an answer it made', async () => {
	const app = new Hono().use(poweredBy()).get('/', (c) => c.text('ok'));
	const res = await app.request('/');
	expect(res.headers.get('X-Powered-By')).toBe('Hono');
	expect(await res.text()).toBe('ok');
});

it('names Hono on an answer passed through with immutable headers', async () => {
	const passed = Response.redirect('https://example.com/', 302);
	expect(() => passed.headers.set('X', '1')).toThrow();
	const app = new Hono().use(poweredBy()).get('/', () => passed);
	const res = await app.request('/');
	expect(res.status).toBe(302);
	expect(res.headers.get('Location')).toBe('https://example.com/');
	expect(res.headers.get('X-Powered-By')).toBe('Hono');
});
