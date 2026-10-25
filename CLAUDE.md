# Datum Atlas: notes for Claude

Read `docs/MEMORY.md` first. It is the project's memory: architecture, decisions, roadmap, and a log of every session.
Datum Atlas is the third app of the Datum family (after Datum Plan and Datum Plot). Follow `docs/FAMILY.md` and `docs/briefs/DATUM-ATLAS.md` in `Edgar-Mendonca/Datum-Plan` exactly (add that repo read-only). Datum Plan's code is the reference implementation: copy from it rather than reinventing anything. Datum Plot (`Edgar-Mendonca/Datum-Plot`) is checked for chart and graph-view patterns, but its renderer is Python and is not reused.

At the end of every session that changes anything, update `docs/MEMORY.md`:
- tick the roadmap
- add a session-log entry
- record any new decisions
- refresh the architecture section if files moved

User preferences:
- Rust and Tauri 2, small and efficient code, Windows first. **No Python inside the app.**
- Ship in small numbered releases (0.1.0, 0.1.1, …), always three numbers. Every release appears in the in-app updater.
- The family look with the **Violet** accent (FAMILY §3): MS Office ribbon, Cairn-style sidebar, IBM Plex, square controls, light/dark/auto themes.
- Keep code compact and match the surrounding style. Engine logic (units, ranges, geometry, hulls, selection, indices) belongs in `crates/core` (pure Rust, unit-tested), not in the UI. The window only maps engine geometry to the screen.
- The UI is SolidJS + Vite in `ui/src/*.tsx` (built to `ui/dist`). State lives in `ui/src/store.ts`.
- Every tool gets a small **?** beside it that opens a card saying what it is for and how to use it (`ui/src/help.tsx`, one `HELP` table). No separate teaching mode. The user wants this in all Datum apps.
- **Data:** every value is compiled by us from public sources, with a reference in `data/sources.json`. Never copy Granta/CES, MatWeb or any other proprietary database. Missing values are gaps, never zero.
- In user-facing text (README, app), Atlas stands on its own: never name other products or use their trademarks, data or wording.
- Big asks get a `docs/PLAN-x.y.md` with questions first; build after the answers. Report in plain language: what changed, what was checked, what is not done.

Identity: "Datum Atlas", `.datlas` (projects) and `.datdb` (user databases), `com.edgarmendonca.datumatlas`, `REPO = "Edgar-Mendonca/Datum-Atlas"` in `src-tauri/src/update.rs`, installer `DatumAtlas-x.y.z-setup.exe`.

Checks before pushing:
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (includes the database checks)
- `npm run check:ui`
- UI tests: `npm run test:ui` (Playwright against `ui/dist` with the `dump` fixture)

Releasing:
1. Bump the version in `src-tauri/Cargo.toml` and `crates/core/Cargo.toml`.
2. Add a CHANGELOG section (`## x.y.z — date`, a one-line summary, bold sections in plain language).
3. Push the tag `vX.Y.Z`, or run `release.yml` with workflow_dispatch when tag pushes are blocked. The workflow creates the tag and publishes the installer and `SHA256SUMS.txt`.
