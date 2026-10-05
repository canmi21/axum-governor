# Issues

Open questions the library has. What is decided and waiting is [todo.md](todo.md), and the direction
is [roadmap.md](roadmap.md); how the three divide the work is the workspace's `spec/planning.md`, and
the questions every project shares -- the coverage target among them -- are the workspace's
`spec/issues.md`.

## The theme script still reads a `palette` cookie that paints nothing

`themeScript` adds `nord` or `contrast` as a class when the cookie holds one, nothing writes the
cookie, and no stylesheet answers either class -- see [kit/theme.md](kit/theme.md). Whether the
script stops reading it, or a palette control and its stylesheets are built to answer it, is not
decided.
