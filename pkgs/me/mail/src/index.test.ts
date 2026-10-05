import { expect, it } from 'vitest';
import { addressOf, BOXES, MAIL_DOMAIN, mailtoOf } from './index.ts';

it('spells every box from the one domain', () => {
	for (const box of Object.keys(BOXES) as (keyof typeof BOXES)[]) {
		expect(addressOf(box)).toBe(`${box}@${MAIL_DOMAIN}`);
		expect(mailtoOf(box)).toBe(`mailto:${box}@${MAIL_DOMAIN}`);
	}
});

it('keeps the box names a security.txt and an error page rely on', () => {
	expect(addressOf('security')).toBe('security@canmi.net');
	expect(addressOf('support')).toBe('support@canmi.net');
});
