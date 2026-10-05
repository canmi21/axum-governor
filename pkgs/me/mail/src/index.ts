/**
 * The domain every box is at. A box is an address a site answers on, forwarded by Cloudflare's
 * Email Routing, so the mailbox behind it can change without it; the author's own address is a
 * person's and is `@canmi/me/identity`'s, never one of these. See spec/me/mail.md.
 */
export const MAIL_DOMAIN = 'canmi.net';

/** Every box, by name, with what it is for. A box is added here before anything names it. */
export const BOXES = {
	security: "a vulnerability report, the address every host's security.txt names",
	support: 'reader mail, and the address a page offers when its own way to write has failed',
} as const;

export type Box = keyof typeof BOXES;

/** The address `box` is at, spelled from the one domain. */
export function addressOf(box: Box): string {
	return `${box}@${MAIL_DOMAIN}`;
}

/** The `mailto:` link to `box`. */
export function mailtoOf(box: Box): string {
	return `mailto:${addressOf(box)}`;
}
