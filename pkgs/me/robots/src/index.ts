/**
 * What every public host of the author's says to a crawler and to an agent, built here and
 * declared by the repository that builds each host: the rules a robots.txt opens with, how page
 * content may be used, the sitemaps, and the word to an agent sent to break in. Nothing here names
 * a host. See spec/me/robots.md.
 */
import { CONTACT, EXTERNAL } from '../../urls/src/index.ts';

/** The opening every policy shares. */
export const robotsTxtBase = [`# ${EXTERNAL.robotstxt}`, 'User-agent: *'] as const;

/**
 * How a page's content may be used, said once and written in both spellings: Cloudflare's
 * `Content-Signal` and the IETF AI Preferences draft's `Content-Usage`. Only a host that serves
 * pages says it; a store of bytes or an API has rules and nothing more.
 */
export const SIGNALS = { search: true, aiInput: true, aiTrain: true } as const;

const yes = (value: boolean, word: string, no: string) => (value ? word : no);

/** Each spelling as a name and a value: a robots.txt line, and a header on a page or markdown. */
export const SIGNAL_HEADERS = [
	[
		'Content-Signal',
		`search=${yes(SIGNALS.search, 'yes', 'no')}, ai-input=${yes(SIGNALS.aiInput, 'yes', 'no')}, ai-train=${yes(SIGNALS.aiTrain, 'yes', 'no')}`,
	],
	[
		'Content-Usage',
		`search=${yes(SIGNALS.search, 'y', 'n')}, ai-use=${yes(SIGNALS.aiInput, 'y', 'n')}, train-ai=${yes(SIGNALS.aiTrain, 'y', 'n')}`,
	],
] as const;

export const signalLines = SIGNAL_HEADERS.map(([name, value]) => `${name}: ${value}`) as [
	string,
	string,
];

/**
 * Cloudflare's terms for content signals, as the comment its own generator writes: the three
 * meanings, and the reservation of rights a `no` makes. Worded and wrapped by Cloudflare, so kept
 * as written.
 */
export const SIGNAL_TERMS = [
	'As a condition of accessing this website, you agree to',
	'abide by the following content signals:',
	'',
	'(a)  If a content-signal = yes, you may collect content',
	'for the corresponding use.',
	'(b)  If a content-signal = no, you may not collect content',
	'for the corresponding use.',
	'(c)  If the website operator does not include a content',
	'signal for a corresponding use, the website operator',
	'neither grants nor restricts permission via content signal',
	'with respect to the corresponding use.',
	'',
	'The content signals and their meanings are:',
	'',
	'search: building a search index and providing search',
	'results (e.g., returning hyperlinks and short excerpts',
	"from your website's contents).  Search does not include",
	'providing AI-generated search summaries.',
	'ai-input: inputting content into one or more AI models',
	'(e.g., retrieval augmented generation, grounding, or other',
	'real-time taking of content for generative AI search',
	'answers).',
	'ai-train: training or fine-tuning AI models.',
	'',
	'ANY RESTRICTIONS EXPRESSED VIA CONTENT SIGNALS ARE EXPRESS',
	'RESERVATIONS OF RIGHTS UNDER ARTICLE 4 OF THE EUROPEAN',
	'UNION DIRECTIVE 2019/790 ON COPYRIGHT AND RELATED RIGHTS',
	'IN THE DIGITAL SINGLE MARKET.',
] as const;

/** Where a finding is sent: the host's own security.txt, which names the contact. */
export const SECURITY_TXT_PATH = '/.well-known/security.txt';

/**
 * A host's word to an agent sent to break in: its lines, said from the host's side, and the
 * repository the host is built from, which is where the agent is sent instead.
 */
export type AgentNote = { lines: readonly string[]; source: string };

/** The width a note's line keeps to, with its `# ` before it. */
export const NOTE_WIDTH = 72;

/**
 * What is wrong with a note's lines, for a test to hold them to: a line past the width, or one
 * holding a lone word. Breaking them is by hand; see spec/me/robots.md.
 */
export function noteProblems(lines: readonly string[]): string[] {
	return lines.flatMap((line) => [
		...(`# ${line}`.length > NOTE_WIDTH ? [`past ${NOTE_WIDTH} columns: ${line}`] : []),
		...(line.split(' ').length < 2 ? [`a lone word: ${line}`] : []),
	]);
}

/** A note as a file says it: the incident it nods to, the note, then where the code is. */
export function agentNote(note: AgentNote): string[] {
	return [
		`# ${EXTERNAL.agentIncident}`,
		'',
		...note.lines.map((line) => `# ${line}`),
		'',
		`# ${note.source}.git`,
	];
}

export type RobotsTxtOptions = {
	allow?: readonly string[];
	disallow?: readonly string[];
	/** Whether the host serves pages, and so says how their content may be used. */
	signals?: boolean;
	/** The host's word to an agent sent to break in, which the file ends with. */
	note?: AgentNote;
	sitemap?: string | readonly string[] | null;
};

export function robotsTxt(options: RobotsTxtOptions = {}): string {
	// The group's rules first; then, for a page host, the terms and each signal under the address
	// that defines it; then the note and the sitemaps. Each address stands apart on its line. A
	// comment or a blank line does not end a group, so the signals still belong to `User-agent: *`.
	const lines: string[] = [robotsTxtBase[0], '', robotsTxtBase[1]];

	for (const path of options.allow ?? []) lines.push(`Allow: ${path}`);
	for (const path of options.disallow ?? []) lines.push(`Disallow: ${path}`);

	if (options.signals) {
		lines.push('', ...SIGNAL_TERMS.map((line) => (line ? `# ${line}` : '')));
		lines.push('', `# ${EXTERNAL.contentSignals}`, '', signalLines[0]);
		lines.push('', `# ${EXTERNAL.contentUsage}`, '', signalLines[1]);
	}

	if (options.note) lines.push('', ...agentNote(options.note));

	const sitemaps = toList(options.sitemap);
	if (sitemaps.length > 0) {
		lines.push('');
		for (const sitemap of sitemaps) lines.push(`Sitemap: ${sitemap}`);
	}

	return `${lines.join('\n')}\n`;
}

// Narrow on `typeof value === 'string'` rather than Array.isArray: Array.isArray narrows to
// the mutable `any[]`, which leaves a `readonly string[]` sitting in the false branch.
function toList(value: string | readonly string[] | null | undefined): readonly string[] {
	if (!value) return [];
	return typeof value === 'string' ? [value] : value;
}

/**
 * A host that serves pages, as the list every sitemap follows names it: its origin, how often its
 * root changes, and how much the host weighs in the whole of what the author runs -- the priority
 * another host's sitemap gives its root.
 */
export type PageHost = { name: string; origin: string; changefreq: string; priority: string };

/** `name` first, then every other page host in the list's order. */
function ownFirst(hosts: readonly PageHost[], name: string): PageHost[] {
	return [
		...hosts.filter((host) => host.name === name),
		...hosts.filter((host) => host.name !== name),
	];
}

function hostOf(hosts: readonly PageHost[], name: string): PageHost {
	const host = hosts.find((candidate) => candidate.name === name);
	if (!host) throw new Error(`${name} serves no pages`);
	return host;
}

/** Every page host's sitemap, `name`'s first: what its robots.txt names. */
export function sitemapsFor(hosts: readonly PageHost[], name: string): string[] {
	return ownFirst(hosts, name).map((host) => `${host.origin}/sitemap.xml`);
}

/**
 * A page host's root as another host's sitemap lists it: its weight in the whole, and no
 * modification time -- nothing reaches across hosts to read another's.
 */
export function rootEntry(hosts: readonly PageHost[], name: string): SitemapEntry {
	const host = hostOf(hosts, name);
	return {
		loc: new URL('/', host.origin).href,
		changefreq: host.changefreq,
		priority: host.priority,
	};
}

/**
 * A page host's root in its own sitemap: how often it changes, as the list says, and the weight
 * the host gives it among its own pages. The host adds its own modification time.
 */
export function ownRoot(hosts: readonly PageHost[], name: string, priority: string): SitemapEntry {
	const host = hostOf(hosts, name);
	return { loc: new URL('/', host.origin).href, changefreq: host.changefreq, priority };
}

/**
 * The other page hosts, by their roots alone, for `name`'s sitemap: each lists its own routes, so
 * a host knows the others by name and needs nothing of theirs to build.
 */
export function peerEntries(hosts: readonly PageHost[], name: string): SitemapEntry[] {
	return ownFirst(hosts, name)
		.slice(1)
		.map((host) => rootEntry(hosts, host.name));
}

/**
 * Where every sitemap's stylesheet is: a path on the sitemap's own origin, since a browser applies
 * an XSL stylesheet to an XML document only from there.
 */
export const SITEMAP_STYLESHEET = '/sitemap.xsl';

export type SitemapEntry = {
	loc: string;
	lastmod?: string;
	changefreq?: string;
	priority?: string;
	alternates?: readonly { language_tag: string; href: string }[];
};

/** A sitemap of `entries`, styled for a browser by `SITEMAP_STYLESHEET`. */
export function sitemapXml(entries: readonly SitemapEntry[]): string {
	const items = entries
		.map((entry) => {
			const parts = [
				`\t\t<loc>${entry.loc}</loc>`,
				...(entry.alternates ?? []).map(
					(alternate) =>
						`\t\t<xhtml:link rel="alternate" hreflang="${alternate.language_tag}" href="${alternate.href}" />`,
				),
				...(entry.lastmod ? [`\t\t<lastmod>${entry.lastmod}</lastmod>`] : []),
				...(entry.changefreq ? [`\t\t<changefreq>${entry.changefreq}</changefreq>`] : []),
				...(entry.priority ? [`\t\t<priority>${entry.priority}</priority>`] : []),
			];
			return `\t<url>\n${parts.join('\n')}\n\t</url>`;
		})
		.join('\n');
	return `<?xml version="1.0" encoding="UTF-8"?>
<?xml-stylesheet type="text/xsl" href="${SITEMAP_STYLESHEET}"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${items}
</urlset>
`;
}

/** RFC 9116 asks for an expiry under a year away; stated per request, so it never lapses. */
const VALID_DAYS = 180;

/**
 * The security.txt `origin` answers with at `now`: RFC 9116's two required fields -- the security
 * box and an expiry -- where the file lives, and the host's note. The expiry is a day boundary, so every answer on one day is the
 * same text and caches as one.
 */
export function securityTxt(origin: string, now: Date, note: AgentNote): string {
	const expires = new Date(now);
	expires.setUTCHours(0, 0, 0, 0);
	expires.setUTCDate(expires.getUTCDate() + VALID_DAYS);
	return [
		`Contact: ${CONTACT.security}`,
		`Expires: ${expires.toISOString()}`,
		`Canonical: ${new URL(SECURITY_TXT_PATH, origin).href}`,
		'',
		...agentNote(note),
		'',
	].join('\n');
}

/** The security.txt as a response for the host `request` arrived at. */
export function securityResponse(request: Request, note: AgentNote): Response {
	return new Response(securityTxt(new URL(request.url).origin, new Date(), note), {
		headers: {
			'Content-Type': 'text/plain; charset=utf-8',
			'Cache-Control': 'public, max-age=86400',
		},
	});
}
