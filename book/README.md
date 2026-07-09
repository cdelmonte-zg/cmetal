# Documentation site (mdBook)

The published site: <https://cdelmonte-zg.github.io/clings/>. It is built from
`book/src/` with [mdBook](https://rust-lang.github.io/mdBook/) and deployed by
`.github/workflows/pages.yml` on every push to `main` that touches the docs.

## Single source of truth

The project pages are **symlinks** into the canonical files at the repo root, so
they are never duplicated:

- `src/project/roadmap.md` -> `VISION.md`
- `src/guides/contributing.md` -> `CONTRIBUTING.md`

Everything else (getting-started, guides, reference) is written for the site —
the navigable front door the scattered markdown lacked.

## Build locally

```bash
cargo install mdbook           # once
cd book && mdbook serve        # live-reload at http://localhost:3000
# or: mdbook build             # renders to book/book/ (gitignored)
```
