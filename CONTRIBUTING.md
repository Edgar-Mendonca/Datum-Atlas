# Contributing

Ideas, bug reports and data corrections are welcome as [issues](https://github.com/Edgar-Mendonca/Datum-Atlas/issues/new). Help › Roadmap in the app (and `docs/site.json`) shows what is planned.

## Data: open sources only

Every value in `data/` names its source in `data/sources.json`. A value may come only from:
- government handbooks and data (public domain);
- open datasets;
- a producer's or supplier's own datasheet that is free to read, cited value by value with its link;
- our own compilation of typical textbook ranges, labelled as ours.

Never copy, or check against, a proprietary or licensed materials database or an aggregator site. Never name one in the app or the docs. A correction needs a published source we can cite. Missing values are gaps, never zero. `cargo test` checks every value, its source and the source's kind.

## How the code is laid out

- `crates/core`: the engine in pure Rust (materials, units, chart geometry, selection, goals, files). All logic lives here, with tests.
- `src-tauri`: the desktop shell (commands, files, updater).
- `ui/src`: the window (SolidJS + TypeScript). It shows what the engine computes and sends edits back as operations.
- `docs/MEMORY.md`: architecture, decisions, roadmap and a log of every session. Read it first.

## Before you send a change

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace            # engine and data tests, including the hardening tests
npm run check:ui                  # type-check the window
npm run test:ui                   # UI tests (Playwright, with the real engine)
cargo deny check                  # licences, advisories, sources (cargo install cargo-deny)
```

Keep code compact and in the style around it. A new engine feature comes with a test, and a new window feature with a UI test where it can.

By contributing you agree your work is released under the MIT licence.
