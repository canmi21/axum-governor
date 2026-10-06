# Light and dark

## One cookie, read the same way everywhere

**`theme` is `light` or `dark`, and `@canmi/kit/theme` is the only code that reads or writes it**:
`themeScript`, the inline script that settles it before the first frame, `themeOf` and `fillTheme`,
the server's reading of it, and `themeCookie` and `applyTheme`, the control's writing of it -- the cookie and the class. With no
cookie the script takes the system's preference and writes it, so the next request's server already
knows. `palette` is a second cookie that only the script reads: `nord` or `contrast`, added as a class.
Nothing writes it and no stylesheet answers either class yet, so it paints nothing -- a leftover the
script still carries; see web's `spec/styling/palettes.md`. The palettes this package ships, `concrete.css` and `mono.css`, are chosen by which
file an app imports.

**An app wires the script into its `app.html`, and the server's reading into its hooks where its
render is its own**, so its first byte already carries the class. **An app whose render is kept and
shared between readers runs the script alone**: a render that differed by cookie could not be
shared, and the script sets the class before anything is painted, so no reader sees the other theme
first. `observeTheme` is how a component follows a change made elsewhere on the page.

## Nord is dark only, and chosen by import

**`palettes/nord.css` is the panel's and the console's one theme**: `color-scheme: dark` on `:root`,
with no `.dark` selector and no light twin yet, chosen by importing it as `mono.css` and
`concrete.css` are, never by the `palette` class. It declares the sixteen Nord colors under their
own names and the semantic tokens the two read -- `--color-ground` through `--color-selection`, the
status tones `good`, `warn`, `danger`, `busy` and `note` among them -- so a surface reads a meaning,
never a number. infra's panel kept these in its own stylesheet until the console needed the same
ones; they live here so the two cannot drift.

**`--color-series-1` to `-8` are a palette of their own, for charts.** Nord's colors cannot be one:
its four frost blues sit too close for the dataviz validator's normal-vision floor, and half of
them are the status tones, which a chart never reuses as a series. The eight are validated on
`--color-surface` -- every check passing, the worst adjacent pair 29.5 apart for normal vision and
14.8 for any color-vision deficiency -- and kept at least 9.9 OKLab ΔE (×100) from every status
token, in a fixed order a chart takes from the first.
