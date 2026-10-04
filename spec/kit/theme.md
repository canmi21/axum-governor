# Light and dark

## One cookie, read the same way everywhere

**`theme` is `light` or `dark`, and `@canmi/kit/theme` is the only code that reads or writes it**:
`themeScript`, the inline script that settles it before the first frame, `themeOf` and `fillTheme`,
the server's reading of it, and `themeCookie` and `applyTheme`, the control's writing of it. With no
cookie the script takes the system's preference and writes it, so the next request's server already
knows. `palette` is a second cookie that only the script reads: `nord` or `contrast`, added as a class. Nothing
here writes it. The palettes this package ships, `concrete.css` and `mono.css`, are chosen by which
file an app imports.

**An app wires the script into its `app.html`, and the server's reading into its hooks where its
render is its own**, so its first byte already carries the class. **An app whose render is kept and
shared between readers runs the script alone**: a render that differed by cookie could not be
shared, and the script sets the class before anything is painted, so no reader sees the other theme
first. `observeTheme` is how a component follows a change made elsewhere on the page.
