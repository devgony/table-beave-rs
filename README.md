<div align="center">
  <img src="images/beave-rs-icon.png" alt="A beaver gnawing a data table" width="120" />
  <h1><img src="images/wordmark.svg" alt="TableBeave-rs" height="56" /></h1>
  <p>Cute beavers gnaw your Unicode &amp; ASCII box tables into Markdown pipe tables.</p>
  <p>
    <a href="https://table.beave.workers.dev"><strong>🔗 Live demo</strong></a>
  </p>
</div>

---

Lately, LLM outputs love to use Unicode tables — but these render badly in Markdown-like tools such as Obsidian and Notion, which is a real pain. Our cute beavers are here to fix that.

Paste a **Unicode/ASCII box table** like `+---+` or `┌───┐`, and it converts into a **Markdown pipe table** (`| a | b |`) that works right away in GitHub, Notion, and Obsidian. The parser is written in Rust, compiled to WebAssembly, and **runs entirely in the browser**. There is no server.

```text
┌────────────┬───────────────┐                | Animal     | Role          |
│ Animal     │ Role          │                | ---        | ---           |
├────────────┼───────────────┤   ──gnaw──▶    | Box beaver | Parser mascot |
│ Box beaver │ Parser mascot │                | Data otter | Reviewer      |
│ Data otter │ Reviewer      │
└────────────┴───────────────┘
```

## Features

- **Both Unicode and ASCII boxes** — `┌─┬─┐ │ ├─┼─┤ └─┴─┘` (light/heavy/double) and `+---+ | =` handled by a single parser
- **Whole-document conversion** — headings (`# ...`) and prose are kept verbatim; only tables are converted, table by table
- **Multi-line cells** — cells spanning several lines are joined with `<br>`
- **Header option** — use the first row as the header, or auto-generate `Column 1, Column 2 …` when there isn't one
- **Live preview** — the generated Markdown is rendered to an HTML table in real time
- **Clipboard copy / column & row counts / warning messages**

## How parsing works — a hand-written parser, no external library

> Table parsing uses **no external crate whatsoever.** The hand-written parser in `src/parser.rs` does all of it.
> The `pulldown-cmark` dependency is **unrelated to table parsing** — it is used only to _render the generated Markdown to preview HTML_.

Box tables come in many shapes (corner glyphs, line weights, delimiter types, indentation, multi-line cells), so a generic parser doesn't capture them cleanly. Instead, a small parser was written by hand that classifies and groups the input line by line. The flow:

1. **Segment splitting** — `split_segments`
   The document is split into _table blocks_ (runs of consecutive table lines) and _passthrough lines_ (headings, prose, blank lines — kept as-is). This way a document mixing headings and several tables converts only the tables while preserving everything else.

2. **Line classification** — two kinds of lines, decided char by char
   - `is_horizontal_rule`: detects separator lines. Recognizes ASCII (`+ - = :`) and Unicode box-drawing characters (code points `U+2500‒U+257F`) together, requiring "a rule char + a corner/box char" to count as a separator.
   - `detect_cell_delimiter`: detects content lines. A line counts as a cell line if it starts and ends with one of `|`, `│` (U+2502), `┃` (U+2503), `║` (U+2551) and contains at least two of them → so not only ASCII pipes but also **Unicode vertical bars (light/heavy/double)** are supported.

3. **Row grouping** — `collect_row_groups`
   Cell lines between separators are grouped into one "logical row." With separators present, everything between two rules is one row (multi-line cells supported); with no separators at all, each cell line is its own row.

4. **Cell cleanup** — `collapse_group`
   Each row group is split on the delimiter, trimmed per cell, and multi-line cell fragments are joined with `<br>`. Pipes (`|`) inside a cell are escaped as `\|`.

5. **Markdown output** — `render_markdown_table`
   Emits a GitHub-style pipe table (header row + `---` separator row + body rows). When the "first row is header" option is off, it synthesizes `Column N` headers.

6. **Tidy-up** — `tidy_blank_lines`
   Strips leading/trailing blank lines and collapses runs of blanks into one, keeping the output clean.

> **Unicode handling:** box-drawing detection uses the `U+2500‒U+257F` range, and vertical delimiters accept light/heavy/double bars. Because it iterates over Rust's native `char`, multi-byte UTF-8 such as Korean, circled numbers (①②③), and middots (·) is handled correctly. (Tests covering this live in `src/parser.rs`.)

## How a Rust parser runs on the web — Leptos + WebAssembly

This project has **no server.** The Rust code is compiled to WebAssembly and executed directly in the browser.

```text
src/*.rs ──(cargo, target=wasm32)──▶ .wasm
        ──(wasm-bindgen: generate JS bindings)──▶ glue.js
        ──(wasm-opt -Oz: size optimization)──▶ dist/
              │
              ▼
        index.html loads the wasm via init()
              │
              ▼
        main() → app::mount() → mounts the Leptos component tree into <body>
              │
              ▼
        on every keystroke, parse_ascii_table() runs in the browser (reactive)
```

- **Conditional compilation** (`src/main.rs`)
  The Leptos app is mounted only under `cfg(target_arch = "wasm32")`. The native build just prints a hint, so the `parser` module can be unit-tested natively with `cargo test`. In other words, **the parser logic is shared between the wasm and native builds.**

- **Reactive UI** (`src/app.rs`, Leptos 0.8 CSR)
  An input `signal` drives a `Memo` that re-runs `parse_ascii_table` whenever the input changes, and the output textarea, preview, and status indicators update automatically. Clipboard copy is done through `web-sys`.

- **Build pipeline** (Trunk)
  Reading the `<link data-trunk rel="rust" data-wasm-opt="z" />` directive in `index.html`, Trunk ① compiles to `wasm32-unknown-unknown` → ② generates JS bindings with `wasm-bindgen` → ③ optimizes size with `wasm-opt -Oz` → ④ bundles the hashed wasm/js/css into `dist/`.

- **Deployment** (Cloudflare Workers static assets)
  `wrangler.jsonc` serves `dist/` as static assets (no `main` server code, with SPA fallback). `make deploy` runs a release build and then deploys.

## Dependencies

| Crate                 | Purpose                                  | Notes                                           |
| --------------------- | ---------------------------------------- | ----------------------------------------------- |
| `pulldown-cmark` 0.13 | Render generated Markdown → preview HTML | Unrelated to table parsing. `html` feature only |
| `leptos` 0.8 (`csr`)  | Reactive UI components                   | `wasm32` target only                            |
| `web-sys` 0.3         | Browser APIs such as the clipboard       | `wasm32` target only                            |

## Develop / Build / Deploy

Prerequisites: Rust (+ the `wasm32-unknown-unknown` target), [Trunk](https://trunkrs.dev), Node (for `npx`, used by wrangler).

```bash
make dev      # local dev server (live reload)   →  trunk serve
make build    # optimized production bundle       →  trunk build --release  (dist/)
make test     # parser test suite                 →  cargo test
make deploy   # build, then deploy to Cloudflare  →  build + wrangler deploy
make help     # list available targets
```

## Project layout

```text
table-beave-rs/
├── index.html         # Trunk entry (rust/css directives) + Cloudflare Web Analytics beacon
├── style.css          # UI styles
├── src/
│   ├── main.rs        # entry point. mounts Leptos on wasm / prints a hint on native
│   ├── app.rs         # Leptos CSR UI component (reactive state, clipboard)
│   └── parser.rs      # box-table parser (hand-written) + preview renderer + tests
├── images/            # icon and other static assets
├── Cargo.toml         # dependencies / release profile (opt-level=z, lto)
├── Trunk output → dist/ # build artifacts (git-ignored)
├── wrangler.jsonc     # Cloudflare Workers static-assets config
└── Makefile           # dev / build / test / deploy targets
```
