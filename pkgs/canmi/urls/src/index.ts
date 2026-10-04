/**
 * The author's own: their site, their identity's addresses, the world's addresses anyone could
 * use, and the functions every program here reads a request with. Depends on nothing else of
 * theirs, so every layer above may read it. See spec/architecture/layers.md, "Addresses are split
 * by who owns the name".
 */

/** Stated by a build for a runtime with no environment to read: a worker, a page. */
declare const STATED_PORT_OFFSET: number | undefined;

/**
 * How far the sandbox shifts every pinned port, and 0 everywhere else. Read from the environment
 * where there is one, and stated by the build where there is not; production states nothing and
 * gets 0. See spec/architecture/modes.md, "Every port is shifted by one hundred".
 */
export const PORT_OFFSET: number =
	typeof STATED_PORT_OFFSET === 'number'
		? STATED_PORT_OFFSET
		: Number(
				(globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env
					?.LATTICE_PORT_OFFSET || 0,
			);

/** The author's site. */
export const SITE = 'https://canmi.net';

/** The port the site's development server answers on, before the sandbox shifts it. */
export const SITE_PORT = 26511;

/**
 * The GitHub account everything here is published under.
 *
 * An identity rather than an address, which is why it sits beside the map instead of inside it:
 * the map is mirrored into Rust as URL constants, and this is the one string in it that is not
 * a URL. The CDN's release proxy fetches from this account and no other -- there is no place in
 * its URL to name a different one. See spec/architecture/delivery.md.
 */
export const GITHUB_OWNER = 'canmi21';

/**
 * Where everything here that is not a dependency comes from. Named at the top of the licence
 * routes, which have to state the terms of the code around the credits as well as the credits
 * themselves -- so it is a published fact, not a convenience, and belongs beside the other URLs
 * rather than written into a route.
 */
export const SOURCE = `https://github.com/${GITHUB_OWNER}/lattice`;

/**
 * Where a vulnerability is reported, published in every zone's security.txt. An address on the
 * site's domain that Cloudflare forwards, so the mailbox behind it can change without it.
 */
export const CONTACT = {
	security: 'mailto:security@canmi.net',
} as const;

/** The world's addresses: services somebody else runs, which anything here may reach. */
export const EXTERNAL = {
	github: {
		web: 'https://github.com',
		api: 'https://api.github.com',
		raw: 'https://raw.githubusercontent.com',
		avatars: 'https://avatars.githubusercontent.com',
		cdn: 'https://cdn.jsdelivr.net/gh',
	},
	google: {
		sourcePreferences: 'https://www.google.com/preferences/source',
	},
	// Where the dependencies come from, named on the licence page. Keyed by purl type, which
	// is what the record uses, so the page looks a registry up rather than mapping names.
	registries: {
		npm: 'https://www.npmjs.com',
		cargo: 'https://crates.io',
		// The sparse index the embed collector reads crate metadata from.
		cargoIndex: 'https://index.crates.io',
	},
	// The canonical page for a licence, joined with `/{id}.html`. SPDX rather than any of the
	// stewards' own sites, because the whole licence record is keyed by SPDX identifier and
	// this is the one address that exists for every one of them.
	spdx: 'https://spdx.org/licenses',
	robotstxt: 'https://www.robotstxt.org/robotstxt.html',
	// Where each content-signal spelling is defined, which a robots.txt names above its line:
	// Cloudflare's, and the IETF AI Preferences draft that defines the `Content-Usage` rule.
	contentSignals: 'https://contentsignals.org',
	contentUsage: 'https://datatracker.ietf.org/doc/draft-ietf-aipref-attach/',
	// The agents that broke into Hugging Face, which a robots.txt here tips its hat to. See
	// spec/architecture/robots.md, "A word to an agent sent to break in".
	agentIncident: 'https://openai.com/index/hugging-face-incident-and-the-road-ahead/',
	// A Sentry DSN only permits *sending* events to one project -- it grants no read
	// access -- and the browser SDK compiles it into the bundle, where anyone can read it
	// out of devtools. It is therefore public by construction, and declaring it here is
	// honest about that rather than pretending a secret store could hide it.
	//
	// The API worker's DSN is a different project that never reaches a browser, so it
	// stays a wrangler secret. Each is treated according to whether it is exposed.
	sentry: {
		site: 'https://a7f2f790ed2fa4f8e0c4310d26d9c39f@o4511131162116096.ingest.us.sentry.io/4511380121976832',
		// Possibly absent: an app without a DSN sends and uploads nothing. See
		// spec/architecture/probe.md, "Errors go to Sentry".
		status:
			'https://0c9dd7de9a89dddc79dbdc2252e1c940@o4511131162116096.ingest.us.sentry.io/4512173650542592' as
				| string
				| undefined,
	},
	// Named as the feed's generator. Nothing fetches it, but it is emitted into published
	// output, so it belongs with the other URLs rather than inline in a route.
	feedsmith: 'https://feedsmith.dev',
	// Where changed URLs are announced. The shared endpoint rather than one engine's own:
	// participants agree to forward what they receive, so submitting here reaches all of
	// them and picking one would be choosing which of them to tell. See spec/indexing.md.
	indexnow: 'https://api.indexnow.org/IndexNow',
	// Bases for social profile links. Handles are canmi/identity's; these are only where a
	// handle is reachable.
	//
	// `twitter.com` rather than `x.com`, on both. The service renamed itself and kept the
	// old host as a permanent redirect, which it will go on keeping -- too much of the web
	// points at it to drop. So the choice is between a name its owner picked and the name
	// everybody uses, at the cost of one redirect nobody waits on. See spec/twitter.md.
	social: {
		telegram: 'https://t.me',
		twitter: 'https://twitter.com',
		twitterIntent: 'https://twitter.com/intent/follow',
		fediverse: 'https://nya.one',
		bluesky: 'https://bsky.app/profile',
	},
	// Companion sites the cargo widget links a crate to, beside the registry above. Keyed by
	// what each serves, joined with `/{crate}` (docs) and `/crates/{crate}` (lib).
	rust: {
		docs: 'https://docs.rs',
		lib: 'https://lib.rs',
	},
	// Webring gateways the homepage footer links into. Whole navigation URLs rather than
	// bases: the path and query are the gateway's interface, not something assembled here.
	webring: {
		travellings: 'https://www.travellings.cn/go.html',
		moe: 'https://travel.moe/go?travel=on',
	},
	// Registration directory behind the homepage badge, joined with `?keyword={id}`.
	icpmoe: 'https://icp.gov.moe',
	// Analytics loader fetched by the browser. The website id rides on the script tag: it is
	// an identity, not an address.
	umami: 'https://cloud.umami.is/script.js',
	// Where the two analytics clients report to, which is not where either comes from.
	// Neither host is written in this repository's own code -- umami's is a constant inside
	// the script it downloads, and OpenPanel's is the default baked into `@openpanel/sdk` --
	// so both are recorded here from having been read out of them. They are declared to be
	// resolved early rather than to be fetched. See spec/analytics.md.
	umamiGateway: 'https://gateway.umami.is',
	openpanel: 'https://api.openpanel.dev',
	// Hosts the Latin webfont stylesheet resolves through; preconnected before it is fetched.
	googleFonts: {
		css: 'https://fonts.googleapis.com',
		static: 'https://fonts.gstatic.com',
	},
} as const;

/**
 * The address `local` binds to, and is therefore reached at.
 *
 * A literal rather than `localhost`, because binding is a separate question from addressing:
 * this server binds one address on purpose so it stays off the
 * network, and `localhost` resolves to `::1` first on macOS, which nothing listens on.
 *
 * A bare hostname rather than a URL, because Vite's `server.host` takes the host alone while
 * the Tauri dev URL wants an origin from `loopbackUrl`.
 */
export const LOOPBACK_HOST = '127.0.0.1';

/**
 * The hostnames that mean this machine.
 *
 * `[::1]` in brackets is the form `URL.hostname` normalises every IPv6 loopback spelling to,
 * and a `Host` header brackets it too; the bare form covers a caller that takes a host apart
 * itself. Without both, a request arriving over IPv6 -- the site binds `::` -- read as production.
 *
 * `LOOPBACK_HOST` covers IPv4: the same address, not the same job as recognising one.
 */
const DEV_HOSTS: ReadonlySet<string> = new Set(['localhost', LOOPBACK_HOST, '[::1]', '::1']);

export function isDevHost(hostname: string): boolean {
	return DEV_HOSTS.has(hostname);
}

export function loopbackUrl(port: number): string {
	const url = new URL(`http://${LOOPBACK_HOST}`);
	url.port = String(port);
	return url.origin;
}

/**
 * The full stops a CJK keyboard types for `.` -- ideographic, fullwidth and halfwidth -- the three
 * IDNA already reads as a dot in a hostname, each as written and as a path carries it, encoded.
 */
const FULL_STOPS = /[。．｡]|%E3%80%82|%EF%BC%8E|%EF%BD%A1/gi;

/**
 * A request path in its one spelling: every CJK full stop a dot, every backslash a slash, every run
 * of slashes one, and no trailing slash -- so the root alone is `''`, the bare host. See
 * spec/architecture/delivery.md, "Every address has one spelling".
 */
export function normalizePath(path: string): string {
	return path
		.replace(FULL_STOPS, '.')
		.replaceAll('\\', '/')
		.replace(/\/{2,}/g, '/')
		.replace(/\/+$/, '');
}

/** Where a request belongs and how it is sent there. */
export type Normalized = { location: string; status: 301 | 308 };

/**
 * Where a request for `url` belongs, or `undefined` when it is already there. The bare host goes
 * on the wire as `/`, so `/` -- alone or with a query -- is already there, and sending it anywhere
 * would come back as `/` and loop. Any other spelling of the bare host goes to the host itself,
 * no slash, by a 301; any other path to its spelling with the query kept, by a 308.
 */
export function normalizedLocation(url: URL): Normalized | undefined {
	if (url.pathname === '/') return undefined;
	const path = normalizePath(url.pathname);
	if (path === url.pathname) return undefined;
	if (path === '' && url.search === '') return { location: url.origin, status: 301 };
	return { location: `${path || '/'}${url.search}`, status: 308 };
}
