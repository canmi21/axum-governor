# Robots

## `@canmi/me/robots` is what every public host says, and each repository says it for its own

**Every public host of the author's answers `robots.txt` and `/.well-known/security.txt` built
here, declared by the repository that builds the host.** This package holds what every host shares
and names no host: the opening of a robots.txt, how page content may be used, Cloudflare's terms,
the sitemap, security.txt and the layout of the word to an agent. A repository declares its own
hosts -- each one's rules, its note, and the repository it is built from -- and nothing else does,
so a host's file names the code that is actually behind it.

The rule itself is the workspace's `spec/robots.md`; this file is how the package keeps it.

## Content signals are for pages, and say yes to all three

**A host that serves pages says how their content may be used; a store of bytes or an API does
not.** A signal is a statement about content -- whether it may be indexed, quoted into an answer,
trained on -- and an object store or an API has rules about fetching and nothing else to say. The
signals are said once, as `SIGNALS`, and written in both spellings, since crawlers read one or the
other:

```
Content-Signal: search=yes, ai-input=yes, ai-train=yes
Content-Usage: search=y, ai-use=y, train-ai=y
```

- `Content-Signal` is Cloudflare's, from contentsignals.org: `search` is an index and results with
  links and short excerpts, AI summaries excluded; `ai-input` is content fed to a model as it
  answers -- retrieval, grounding, generative search; `ai-train` is training or fine-tuning.
- `Content-Usage` is the IETF AI Preferences working group's draft, the standards-track form of
  the same idea: `search`, `ai-use` and `train-ai`, each `y` or `n`, in robots.txt or as an HTTP
  header. An unstated one is unknown rather than either answer.

**All three are yes.** The author wants to be found, quoted in answers, and known to models, and a
no on any of them would work against the rest.

**A page host's file reads in one order**: the robots reference, the group's rules, Cloudflare's
terms as a comment -- the three meanings, and the EU reservation of rights a `no` would make --
then each spelling under the address that defines it, then the note, then the sitemaps. A comment
or a blank line does not end a group, so the signals stay `User-agent: *`'s. The terms are
Cloudflare's wording, kept as written.

**The policy is the author's, not the edge's.** Cloudflare can write content signals into a zone's
`robots.txt` itself; that setting stays off, since a line the edge adds is one nobody can grep
for.

## Every page host names every other

**A page host's robots.txt names every page host's sitemap, its own first**, and its sitemap lists
every other page host by its root alone: each host lists its own routes, so none needs another's
data to build, only the list of who the others are. The list is the declaring repository's, as
`PageHost`s in the order every sitemap follows, each with how often its root changes and how much
the host weighs in the whole of what the author runs -- the priority another host's sitemap gives
its root. **Within its own sitemap a host weighs its own pages on its own scale.** A root's
modification time is said by its own sitemap alone; reaching across hosts for it would be a
dependency for one line.

**Every sitemap is styled from its own origin**, at `/sitemap.xsl`, because a browser applies an
XSL stylesheet to an XML document from nowhere else; each host answers that path itself. Chrome
stops applying XSLT on 17 November 2026, version 158, and a sitemap then shows as plain XML there;
crawlers never read the stylesheet, so nothing a sitemap is for depends on it.

## A word to an agent sent to break in

**Every robots.txt and security.txt ends with a word to an agent told to find vulnerabilities**:
the code is open source, so read it rather than attack the host, and send a real finding to the
security contact. It is a language firewall -- an agent that reads instructions can be talked out
of an attack as well as into one -- and a nod to the note Hugging Face put in its `security.txt`
after agents broke into it.

- **Each host says it in its own words**, from its side -- a CDN serves bytes, an alias layer only
  redirects -- in each of the two files, and the repository declaring the hosts holds a test that
  no two of its notes are the same.
- **Link first**: the account of the incident it nods to, then the note, laid out by sentence,
  then the host's repository as a `.git` address. A robots.txt names no address of its own; it
  sends a finding to `SECURITY_TXT_PATH`, where the contact is published.
- **A note is broken into lines by hand, and stored as its lines.** A line ends where the sentence
  pauses -- a full stop, a comma, a semicolon -- the lengths run close, within `NOTE_WIDTH` columns,
  and no line holds a lone word or the first words of a sentence it does not finish.
  `noteProblems` holds a note to the two of these a test can check -- the width and a lone word --
  and the rest is held by hand; breaking by rule either filled lines to a width or left them
  ragged, and a handful of short notes are cheaper set once.

## security.txt is RFC 9116's two fields and the note

`Contact` is the `security` box, `CONTACT.security` in `@canmi/me/urls`, the same on every host;
`Expires` is 180 days ahead on a day boundary, stated per request so it never lapses, and every
answer on one day is the same text and caches as one; `Canonical` is the host's own path.
