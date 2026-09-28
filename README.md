# md2html

Convert Markdown to a self-contained, print-ready HTML document.

One binary, no runtime dependencies, no network fetches in the output — the
stylesheet is inlined, so the resulting `.html` is a single file you can email,
open offline, or hand to a browser's **Print / Save as PDF**.

## Run it

```sh
nix run github:rbritton3468/md2html -- notes.md -o notes.html
nix shell github:rbritton3468/md2html      # puts md2html on $PATH
```

From a local checkout:

```sh
nix run . -- notes.md -o notes.html
```

## Usage

```
md2html [OPTIONS] [FILE]

  FILE                  Markdown input; omitted or '-' reads stdin

  -o, --output <FILE>   Write to FILE instead of stdout
  -t, --title <TEXT>    Document title (default: first level-1 heading, else
                        the input file name, else 'Document')
  -f, --fragment        Emit the rendered body only: no wrapper, no styles
      --css <FILE>      Use FILE as the stylesheet instead of the built-in one
      --no-css          Standalone document with no stylesheet
      --dump-css        Print the built-in stylesheet and exit
  -h, --help            Show this help
  -V, --version         Show version
```

```sh
md2html README.md -o readme.html          # standalone, styled, print-ready
md2html README.md | wc -c                 # to stdout
pandoc-ish | md2html - -t "Report"        # from stdin, explicit title
md2html -f snippet.md                     # fragment to embed elsewhere
md2html --dump-css > mine.css             # start from the built-in sheet
md2html notes.md --css mine.css -o out.html
```

## Printing

The built-in stylesheet has a real `@media print` block, so printing from the
browser gives you:

- 18mm × 16mm page margins via `@page`
- headings that never end up stranded at the bottom of a page
- code blocks, tables, blockquotes and images that don't split across pages
- long code lines wrapped instead of clipped — printers can't scroll
- table headers repeated on every page
- link destinations spelled out as `<https://…>` after each external link
- black-on-white ink-friendly colors, overriding the screen/dark-mode palette

Screen rendering is a centered ~46rem column that follows
`prefers-color-scheme`.

Swap the whole thing with `--css` if you'd rather use your own.

## Markdown support

CommonMark via [`pulldown-cmark`](https://github.com/raphlinus/pulldown-cmark),
plus tables, footnotes, strikethrough, task lists, smart punctuation, and
heading attributes.

## Development

```sh
nix develop          # cargo, rustc, clippy, rustfmt, rust-analyzer
cargo test
nix flake check      # build + tests + clippy + rustfmt + a render smoke test
nix fmt              # format the Nix
```

The package derivation lives inline in `flake.nix`; `Cargo.lock` is committed
and consumed via `cargoLock.lockFile`, so there is no vendor hash to refresh
when dependencies change.

## Use it from another flake

```nix
{
  inputs.md2html.url = "github:rbritton3468/md2html";

  # then, in a module:
  # home.packages = [ inputs.md2html.packages.${pkgs.system}.default ];
}
```

## License

MIT
