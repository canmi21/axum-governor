# What a page remembers: two records and one mechanism

`@canmi/kit/behavior/state` keeps a page's own small facts about its reader in one container per
storage area, edited in place: `reader` in `localStorage["state"]`, what is true of the person, and
`tab` in `sessionStorage["state"]`, what is true of this sitting. A fact that would otherwise arrive
as a loose key of its own goes in one of them, which is how a reader's storage is kept from becoming
a scatter of names nothing owns and nothing can move together. There is no eviction, no staleness
and no serialization beyond `JSON`, because none of these facts expire and all of them are small.

**Keys are flat and dotted.** `support.preferred`, not a `support` object with a `preferred` inside
it. Nesting buys grouping the dot already expresses, and costs every reader and writer a walk down a
path that may not exist yet.

**The version is an integer, and it is there from the first write.** A record's migrations are a
list: the first takes a record at version 1 to version 2, and a step edits in place and may assume
every earlier one has run. A record written without a version could never be migrated, because the
code migrating it would have no way to know what it is looking at.

**A record from a later version is left alone rather than reset.** That is a reader whose other
device runs a newer build, and the keys this build understands are still readable inside it. A
record that is not an object, or carries no usable version, is replaced, because nothing in it can be
placed.

**The two records share everything except the key's area, the version and the migrations**: they
hold different facts and version independently, and a step written for one running against the
other is the failure the mechanism exists to prevent.

**A fallback names the kind of thing wanted, and `undefined` is not a kind.** The container compares
what it found against the kind it was asked for, so a read whose fallback is `undefined` discards
every stored value; ask with `{}` for an object, and let the caller's own check refuse what is not
its shape.
