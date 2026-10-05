# Mail

## `@canmi/me/mail` is where every address a site answers on is spelled

**A box is an address on the site's domain, and every one is named here**: `MAIL_DOMAIN` and the
`BOXES` table, each box with what it is for. Anything that needs an address asks for it by name --
`addressOf('support')`, `mailtoOf('security')` -- so no app, article or file writes one out, and
moving the domain is one edit.

- **A box is not a person.** Cloudflare's Email Routing forwards each to whatever mailbox reads
  it, so the mailbox behind it changes without the address. The author's own address is a
  person's, and is `@canmi/me/identity`'s.
- **A box is added here before anything names it.** `security` is the one every host's
  security.txt names, through `CONTACT` in `@canmi/me/urls`; `support` is reader mail, and the
  address a page offers when its own way to write has failed.
- **The corpus names a box the same way.** An article's `:link[support]` compiles to the box's
  address, read from this table; web's `libs/compile` reserves `author` for the author's own.
