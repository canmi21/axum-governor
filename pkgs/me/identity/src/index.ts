import data from '../author.json' with { type: 'json' };

/**
 * The author, declared once for every app. The values are a JSON file so that Rust and the
 * scripts outside the Vite graph read the same file. See spec/architecture/identity.md.
 */
export interface Author {
	/** The name a page signs with, and the one the author goes by. */
	name: string;
	/**
	 * Said only where the author is introduced: the home page, its card and its agent view once, and
	 * the structured data. Not translated. See spec/architecture/identity.md, "One name, said
	 * plainly".
	 */
	fullName: string;
	role: string;
	/** A person's address, and the one the feed names as its author. */
	email: string;
	// Handles, not URLs: where each is reached is platform/libs/sdk'.
	/** The author's own account. */
	telegram: string;
	/** The group the author runs, which the row of links points at; the handle is theirs too. */
	telegramGroup: string;
	twitter?: string;
	github: string;
	/** GitHub's numeric id, which addresses the avatar and survives a renamed handle. */
	githubId: number;
	fediverse: string;
	/** A domain the owner proves on Bluesky; the site's domain is that proof. */
	bluesky: string;
}

export const author: Readonly<Author> = data;

/** The author named in full anywhere but the home page: the name, then the address. */
export const mailbox = `${author.name} <${author.email}>`;
