# assets

Everything here is compiled into `zoners-web` with `include_bytes!` (see `ASSETS`
in `src/bin/zoners-web.rs`) and served under `assets/` with a one year immutable
cache. URLs carry a hash of all assets, so any change busts the cache.

| File              | Source                                        | License |
| ----------------- | --------------------------------------------- | ------- |
| `app.css`         | ours                                          |         |
| `app.js`          | ours                                          |         |
| `favicon.svg`     | ours                                          |         |
| `daisyui.css`     | daisyUI 5.7.40, a subset (see below)          | MIT     |
| `htmx.min.js`     | htmx 2.0.10, `dist/htmx.min.js`               | 0BSD    |
| `fonts/*.woff2`   | Atkinson Hyperlegible Next and Mono           | OFL 1.1 |

## Regenerating `daisyui.css`

There is no Tailwind build. daisyUI publishes each component as plain CSS, and
the page uses five of them. The full `daisyui.css` is 1.1 MB, this subset is
134 kB (10 kB gzipped). Tailwind utility classes are not available, layout is
hand-written in `app.css`.

```sh
V=5.7.40; B=https://cdn.jsdelivr.net/npm/daisyui@$V
FILES="base/properties.css base/reset.css base/rootcolor.css base/scrollbar.css base/svg.css theme/light.css components/button.css components/input.css components/range.css components/table.css components/alert.css"
{ echo "/*! daisyUI $V (MIT), subset vendored for zoners. Files: $FILES. See assets/README.md to regenerate. */"
  for f in $FILES; do curl -sfL "$B/$f"; echo; done; } > assets/daisyui.css
```

To use another component, add its file to `FILES` and rerun. `theme/light.css`
only supplies defaults for tokens that `app.css` does not set. The light and
dark palettes both live in `app.css`.
