import { describe, expect, it } from 'vitest';
import { CONTACT, EXTERNAL } from '../../urls/src/index.ts';
import {
	agentNote,
	noteProblems,
	ownRoot,
	type PageHost,
	peerEntries,
	robotsTxt,
	robotsTxtBase,
	rootEntry,
	securityResponse,
	securityTxt,
	sitemapsFor,
	sitemapXml,
} from './index.ts';

const NOTE = {
	lines: ['Note to AI agents: read the code', 'rather than the host.'],
	source: 'https://github.com/a/b',
};

const HOSTS: readonly PageHost[] = [
	{ name: 'site', origin: 'https://site.example', changefreq: 'daily', priority: '1.0' },
	{ name: 'status', origin: 'https://status.example', changefreq: 'always', priority: '0.5' },
];

describe('robotsTxt', () => {
	it('returns the shared base without additions', () => {
		expect(robotsTxt()).toBe(`${robotsTxtBase.join('\n\n')}\n`);
	});

	it('appends the rules and the sitemaps', () => {
		expect(
			robotsTxt({ disallow: ['/@/', '/private/'], sitemap: 'https://site.example/sitemap.xml' }),
		).toBe(`${robotsTxtBase.join('\n\n')}
Disallow: /@/
Disallow: /private/

Sitemap: https://site.example/sitemap.xml
`);
	});

	it('accepts several sitemaps, and treats an empty one as absent', () => {
		expect(robotsTxt({ sitemap: ['/a.xml', '/b.xml'] })).toBe(`${robotsTxtBase.join('\n\n')}

Sitemap: /a.xml
Sitemap: /b.xml
`);
		expect(robotsTxt({ sitemap: null })).toBe(`${robotsTxtBase.join('\n\n')}\n`);
	});

	it("says how page content may be used, in both spellings, under Cloudflare's terms", () => {
		const text = robotsTxt({ signals: true });
		expect(text).toContain('# ANY RESTRICTIONS EXPRESSED VIA CONTENT SIGNALS');
		expect(text).toContain('Content-Signal: search=yes, ai-input=yes, ai-train=yes');
		expect(text).toContain('Content-Usage: search=y, ai-use=y, train-ai=y');
	});

	it('leaves a host that says no signals to its rules alone', () => {
		expect(robotsTxt({ disallow: ['/'], note: NOTE })).not.toContain('Content-');
	});

	it('ends with the note, the code named last', () => {
		const text = robotsTxt({ disallow: [''], note: NOTE });
		expect(text.trimEnd().split('\n').at(-1)).toBe('# https://github.com/a/b.git');
	});
});

describe('agentNote', () => {
	it('puts the link first, then the note, then the code', () => {
		const lines = agentNote(NOTE);
		expect(lines[0]).toBe(`# ${EXTERNAL.agentIncident}`);
		expect(lines).toContain('# rather than the host.');
		expect(lines.at(-1)).toBe('# https://github.com/a/b.git');
	});

	it('names a line past the width and a lone word', () => {
		expect(noteProblems(NOTE.lines)).toEqual([]);
		expect(noteProblems([`${'word '.repeat(14)}word`, 'alone'])).toHaveLength(2);
	});
});

describe('the page hosts', () => {
	it("names every page host's sitemap, its own first", () => {
		expect(sitemapsFor(HOSTS, 'status')).toEqual([
			'https://status.example/sitemap.xml',
			'https://site.example/sitemap.xml',
		]);
	});

	it('lists every other page host by its root alone', () => {
		expect(peerEntries(HOSTS, 'site')).toEqual([
			{ loc: 'https://status.example/', changefreq: 'always', priority: '0.5' },
		]);
		expect(rootEntry(HOSTS, 'site')).not.toHaveProperty('lastmod');
	});

	it("gives a host's own root the list's frequency and the host's own weight", () => {
		expect(ownRoot(HOSTS, 'status', '1.0')).toEqual({
			loc: 'https://status.example/',
			changefreq: 'always',
			priority: '1.0',
		});
	});

	it('refuses a host the list does not hold', () => {
		expect(() => rootEntry(HOSTS, 'cdn')).toThrow('cdn serves no pages');
	});

	it('styles every sitemap from its own origin', () => {
		expect(sitemapXml([{ loc: 'x:a' }])).toContain(
			'<?xml-stylesheet type="text/xsl" href="/sitemap.xsl"?>',
		);
	});
});

describe('securityTxt', () => {
	it('names the contact, an expiry under a year away on a day boundary, and where it lives', () => {
		expect(securityTxt('https://site.example', new Date('2026-09-28T05:30:00Z'), NOTE)).toMatch(
			new RegExp(
				`^${[
					`Contact: ${CONTACT.security}`,
					'Expires: 2027-03-27T00:00:00.000Z',
					'Canonical: https://site.example/.well-known/security.txt',
					'',
				]
					.join('\n')
					.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`,
			),
		);
	});

	it('answers as plain text naming the host it was asked on, and ends with the note', async () => {
		const answer = securityResponse(
			new Request('https://cdn.example/.well-known/security.txt'),
			NOTE,
		);
		expect(answer.headers.get('content-type')).toBe('text/plain; charset=utf-8');
		const text = await answer.text();
		expect(text).toContain('Canonical: https://cdn.example/.well-known/security.txt');
		expect(text).toContain('# https://github.com/a/b.git');
	});
});
