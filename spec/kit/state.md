# What a page remembers: records a project declares, one mechanism

`@canmi/kit/behavior/state` is the mechanism, and holds no records. **A project declares each of
its records as data** -- `record(key, steps)` -- and pairs it with a storage area at every call
site: the site's `reader` in `localStorage["state"]`, what is true of the person, and `tab` in
`sessionStorage["state"]`, what is true of this sitting. A fact that would otherwise arrive as a
loose key of its own goes in one of a project's records, which is how a reader's storage is kept
from becoming a scatter of names nothing owns and nothing can move together. Kit modules that keep
a fact -- resize, progress, brevity -- take the record as an argument and never name one of their
own. There is no eviction, no staleness and no serialization beyond `JSON`, because none of these
facts expire and all of them are small. Decided with the author on 2026-10-10: the steps are each
project's, and how a step runs is the kit's.

**Keys are flat and dotted.** `support.preferred`, not a `support` object with a `preferred` inside
it. Nesting buys grouping the dot already expresses, and costs every reader and writer a walk down a
path that may not exist yet.

**A migration is data, and the kit executes it.** A record's steps are a list, and `steps[0]`
takes version 1 to 2, so the version is one more than the step count and stands from the first
write -- a record written without a version could never be migrated, because the code migrating
it would have no way to know what it is looking at. A step maps each key it changes to a function
from that key's old value to its new one. The function runs only where the key is present.
Returning `undefined` drops the key, and so does throwing, because a step has nowhere to report
to. Nothing about how a step runs is a project's to write.

**A record from a later version is left alone rather than reset.** That is a reader whose other
device runs a newer build, and the keys this build understands are still readable inside it. A
record that is not an object, or carries no usable version, is replaced, because nothing in it can be
placed.

**Records that share a key name in different storage areas are separate records**: each has its
own steps and version, and a step written for one never runs against another.

**A collection is the exception to flat and dotted.** `video.at` is one key holding a map from
clip reference to position, because its keys are not names a site chooses: they are whatever its
articles refer to. The rule is about names, not about depth, and one fact whose shape is a map is
not a group of facts that wanted a prefix. It also makes the collection readable and clearable in
one go, where a scatter of `video.at.<reference>` would not be.

**The collection is not capped, and that is a decision rather than an omission.** An entry carries
a position and a still of about a kilobyte, so a thousand clips is about a megabyte -- a
single-digit fraction of a quota measured in megabytes -- and a tab is short: the number of
distinct clips a reader opens in one sitting is nowhere near a thousand.

**The store is passed in rather than reached for**, so the tests hand over a plain object and no
global is installed to reach this. The record and the store are named separately at every call
site for the same reason: in production they always pair, and a test is the place where they do
not.

**A fallback names the kind of thing wanted, and `undefined` is not a kind.** The container compares
what it found against the kind it was asked for, so a read whose fallback is `undefined` discards
every stored value; ask with `{}` for an object, and let the caller's own check refuse what is not
its shape.
