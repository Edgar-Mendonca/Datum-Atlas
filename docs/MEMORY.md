# Datum Atlas: project memory

> **Paste this file at the start of each new session**, or just open the repo: `CLAUDE.md` tells Claude to read it.
> It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Atlas: a Windows desktop app for materials selection and property charts (the Ashby method): find the right material, understand why (performance indices, trade-offs), compare candidates, document the decision |
| **Family** | Third Datum app after Datum Plan (`Edgar-Mendonca/Datum-Plan`) and Datum Plot (`Edgar-Mendonca/Datum-Plot`). Design and workflow: [Datum-Plan/docs/FAMILY.md](https://github.com/Edgar-Mendonca/Datum-Plan/blob/HEAD/docs/FAMILY.md). Brief: [Datum-Plan/docs/briefs/DATUM-ATLAS.md](https://github.com/Edgar-Mendonca/Datum-Plan/blob/HEAD/docs/briefs/DATUM-ATLAS.md). Plan's code is the reference implementation: copy, don't reinvent. |
| **Current version** | **0.2.1** (released 2026-10-29: https://github.com/Edgar-Mendonca/Datum-Atlas/releases/tag/v0.2.1). Plans and answers: [PLAN-0.1.md](PLAN-0.1.md), [PLAN-0.1.1.md](PLAN-0.1.1.md), [PLAN-0.2.0.md](PLAN-0.2.0.md) |
| **Installer** | NSIS `.exe`, per-user: **2.9 MB** (0.2.0; 2.2 MB in 0.1.0); UI bundle 182 kB JS |
| **Repo** | `Edgar-Mendonca/Datum-Atlas` · working branch `claude/bold-volta-90lu2c` |
| **Identity** | Accent **Violet** (`--blue #6C4AB6`, `--blue2 #5A3C9C`, `--sky #C9B8EE`, `--pale #F0EBFA`, `--sel #E5DCF6`, `--sel2 #CDBDEE`; dark `--blue #A58BEA`, File sidebar dark `#3A2766`) · files `.datlas` (projects), `.datdb` (user databases) · identifier `com.edgarmendonca.datumatlas` · installer `DatumAtlas-x.y.z-setup.exe` · logo `docs/brand/datum-atlas-icon.svg` (violet log–log axes, family bubbles, dotted index line, amber `#F2B233` diamond) |
| **Stack** | Rust engine (`crates/core`) + Rust shell (`src-tauri`), Tauri 2, WebView2, SolidJS + Vite UI in strict TypeScript, plain CSS tokens, IBM Plex. Own SVG chart engine (geometry in Rust). Chart.js later for bar/radar, GridCraft later for expressions. **No Python.** |
| **Code size** | About 7,800 lines: core 3,180 (35 tests, incl. hardening), shell 790, UI 3,440 TSX/TS + CSS, UI tests 415 (72 checks); data: 252 materials, 36 sources, 73 values checked |

---

## 1. Architecture

```mermaid
flowchart LR
  subgraph UI["Window (WebView2) · ui/src/*.tsx → Vite → ui/dist"]
    Store[store.ts: out · db · view · cur · chart · act]
    Chart[chart.tsx: ChartSvg · ChartView · ChartBar · Legend · export]
    Browse[browse.tsx: tree · table · Datasheet]
    Stages[stages.tsx: StagesPage · StagePanel · ResultsPage]
    Mat[material.tsx: Add Material]
    Help[help.tsx: HELP · ? button · helpCard]
    BS[ribbon · backstage · print · pages (Setup · Sources · goalCard) · dialogs · roadmap]
  end
  subgraph Shell["src-tauri (Rust)"]
    Cmd[main.rs: state · apply · history · new_project · templates · database<br/>save_material · delete_material · goal_list · open · save · save_file · copy_text · copy_png<br/>options · recovery · recent · open_url · updates]
    Upd[update.rs: REPO = Edgar-Mendonca/Datum-Atlas]
  end
  subgraph Core["crates/core (pure Rust, 16 tests)"]
    Model[model.rs: Material · Val · Source · PROPS · Project · Chart · ChartSel · Stages]
    Db[db.rs: embedded data · Db · derive · tree · search · check · check_user · props_out]
    Goals[goals.rs: GOALS · apply]
    Geo[geometry.rs: ellipses · convex hull · rounded hull · clusters]
    Ch[chart.rs: bubbles · hulls per family group · extents · missing]
    Sel[select.rs: limits · chart · tree stages · reasons · ranking · line values]
    Doc[doc.rs: Op · undo · .datlas]
    Data[(data/*.json)]
  end
  UI -- "invoke(cmd, args)" --> Cmd --> Doc & Ch & Sel & Db & Goals
  Ch --> Geo
  Db --> Data
```

**Rules:**
- The engine computes every value: ranges, bubbles and envelopes (in axis space: log10 on log axes), pass/fail with reasons, rankings and the index value at each selection line. The window maps engine geometry to pixels (zoom/pan window), draws ticks and labels, and sorts/filters tables for display.
- After every change the shell returns `Out` = project + every chart laid out + results + `dbRev`; the window reloads the database (`database`) when `dbRev` changes.
- Every edit is an `Op` (AddChart, SetChart, DelChart, Select, SetStages, Name, Notes, Ui, Batch, Goal) with undo; `Goal` is turned into a `Batch` by `goals::apply` in the shell and the bridge (it needs the database); the selection line is moved in the window while dragging and sent as one `Select` on release.
- Built-in data: `data/*.json` (one per family; values `[low, high, source]`, flags `[bool, source]`), `data/sources.json`; embedded with `include_str!`; `db::check` runs in the tests. User materials: `My materials.datdb` in the app data folder (ids `u-…`, sources `u-s-…`), checked by `db::check_user` (name, family, density, one more property, a source for each value).
- Envelopes: per family (or class), split into groups of nearby bubbles (`geometry::clusters`, gap 5 % of the axes), each the convex hull of its ellipses grown by a rounded margin; only the biggest group is labelled, at its top edge.
- Limits on ranges pass on the mean (geometric for log properties) or, with rule "all", on the whole range; missing values fail ("no data for …") or are kept and marked.
- Index of a line of slope n on log–log axes Y vs X: Y^(1/n)/X (n > 0), Y^(1/|n|)·X (n < 0); ranked descending when "above passes".
- Design goals (`goals.rs`, 22): a goal sets the chart's axes, name and `goal`, places the line of its slope at the 80th percentile of `cy − n·cx` through the median x (the top fifth pass), replaces the limits marked `goal: true` with its own, and sets `rank_chart`. Empty id = Explore (no line, goal limits removed).
- Derived properties (`db::derive`, source `derived`): cv = price·ρ, co2v, eev, es = E/ρ, ss = σy/ρ, diff = λ/(ρcp), shock = σts/(Eα); products low·low/high·high, quotients low/high, high/low.
- Units: stored SI; `db::props_out` gives each property `imperial: [unit, k, c]`; the window converts with `conv`/`toShown`/`toStored`/`vtext` (store.ts) and computes ticks in shown units.
- Checked values: `Material.checked` lists the properties checked against a published source; `db::check` refuses a checked value that is missing or cites a `textbook-typical` or `estimate` source.
- Clipboard text goes through Rust (`copy_text`); Copy Picture sends a PNG as base64 (`copy_png`); chart export is SVG text or a PNG drawn from it, saved by `save_file`.

### Files
| Path | What it does |
| --- | --- |
| `crates/core/src/model.rs` | `Val` (untagged arrays), `Material` (with `checked`), `Source`, `Database`, `PROPS` (34 properties + 7 derived: key, name, symbol, group, unit, log default), `FAMILIES`, `RATINGS`; `.datlas` `Project` format 1 (charts, stages, notes, ui), `Chart` (with `goal`, `kind` bubble/bar), `ChartSel` (Line/Box), `Stages`, `Limit` (with `goal`), `chart_name`. |
| `crates/core/src/units.rs` | Imperial factors per stored unit, `to`/`from` (exact inverses), `fmt` (3 significant figures), `range`. |
| `crates/core/src/db.rs` | `builtin()` from the embedded files, `Db` (built-in + user + derived), `derive`, `tree`, `search`, `check` (incl. checked values), `check_user`, `props_out`. |
| `crates/core/src/goals.rs` | `Goal` (group, name, index, axes, slope, above, function, constraint, objective, free variable, why, limits, `also` for a second chart), `GOALS` (25), `get`, `place`, `apply`. |
| `crates/core/src/geometry.rs` | `t`/`span`/`mean` in axis space, `ellipse`, `convex`, rounded `hull`, `inside`, `clusters`. |
| `crates/core/src/chart.rs` | `build(db, chart)` → `ChartOut` (axes with limits, bubbles, hulls with members and labels, missing with reasons; for bar charts `bands` per family, sorted). |
| `crates/core/src/select.rs` | `run(db, project)` → `Results` (stage counts, rows with reasons, index, rank, × best, `lines` with index name and value; `index_name` writes E½/ρ, σy⅔/ρ). |
| `crates/core/src/doc.rs` | `Op`, `Doc` (undo 200, open/save). |
| `crates/core/src/templates.rs` | Design problems built from goals: blank, beam, tie, panel, spring, hot, tough. |
| `crates/core/src/hardening.rs` | FAMILY §11 tests: failed edits change nothing (incl. a failing `Batch` and goal), random projects and random edits never panic and keep their invariants (finite geometry, counts add up, every failure has a reason), random user databases, damaged `.datlas`/`.datdb` files (cut, garbled, deep nesting), a damaged `.datdb` is an error never empty, atomic save, speed gate (1,000 materials × 20 charts: ~200 ms release, ~1.2 s debug). |
| `crates/core/src/tests.rs` | Data checks, bad-data detection, unit round trips, bubbles in log space, hulls contain their bubbles (6 axis pairs × family/class), missing values, index lines for slopes 1/2/3 above/below, limits (missing, flag, whole range, flags), combined stages, edits/undo/round trip/templates, user materials, tree and search; derived ranges; every goal (line, ranking by its hand-computed index, limits, Explore); index names; bar charts; checked values cite linked datasheets. |
| `crates/core/examples/bridge.rs` | The engine over stdin/stdout JSON lines for the UI tests. |
| `data/*.json`, `data/sources.json` | The built-in database (252 materials, 36 sources), `status: draft`; 20 materials have `checked` lists (73 values); durability ratings on every material. |
| `src-tauri/src/main.rs` | Commands in the diagram; options (theme, autosave, beta, openLast, labels, guides, dpi, units, paper, landscape, company, logo); user database file; recovery; recent files; `open_url` only for this repo. |
| `src-tauri/src/update.rs` | Datum Plan's updater with `REPO = Edgar-Mendonca/Datum-Atlas`. |
| `ui/index.html` | Icon sprite (Plan/Plot icons recoloured violet + atlas, bar, box, compare, datasheet, db, eco, eye, hull, index, material, pan, poly, process, radar, swap, tree, units). |
| `ui/src/api.ts` | Types, `fmt`/`range`/`valText`/`mid`, `FAMILY_COLORS` (Okabe-Ito). |
| `ui/src/store.ts` | Signals and actions (see diagram); `goals`, `setGoal`; units (`imperial`, `conv`, `toShown`, `toStored`, `unitOf`, `vtext`, `setUnits`); `show(view)` also switches the ribbon tab; `failed()` for greying. |
| `ui/src/chart.tsx` | `ticks`, `ChartSvg` (screen, print and export), `Legend`, `ChartView` (zoom/pan window per chart, hover, click, drag line/box, Line and Box tools), `ChartBar` (goal list, Why, bubbles/bars, envelopes, zoom), `ChartTools` (axes, Pan/Line/Box, slope), bar rendering, label `nudge`, `chartSvgText`, `exportChart`, `copyChart`. |
| `ui/src/browse.tsx` | `Tree`, `Browse` (grid engine `selection.ts`, sort, columns menu, Ctrl+C via Rust), `Datasheet` (groups, numbered references). |
| `ui/src/stages.tsx` | `StagesPage` (limits, chart selections, family tree), `StagePanel`, `ResultsPage`, `addLimit`. |
| `ui/src/material.tsx` | Add/Edit Material window with the "What you need" panel; `deleteMaterial`. |
| `crates/core/src/formula.rs` | Formulas of properties on ranges: `parse` (with names resolved), `key_of`, `refs`, `eval` (interval arithmetic), `show`. |
| `ui/src/formula.tsx` | The formula-properties window (`editFormulas(axis?)`). |
| `crates/core/src/export.rs` | Results as a table: `results`, `csv`, `xlsx`. |
| `crates/core/src/compare.rs` | Compare and radar: `build`, `RADAR`, `MAX`. |
| `ui/src/compare.tsx`, `ui/src/report.tsx` | The Compare page with the radar, and the selection report (screen and print). |
| `crates/core/src/sheet.rs` | Spreadsheets in and out of a database: `read`, `from_text`, `guess`, `range`, `to_database`, `of_database`, `bytes`. |
| `ui/src/importer.tsx` | The import window (`importMaterials`) and the Databases section. |
| `ui/src/help.tsx` | `HELP` (26 topics), the `?` button, `helpCard`. Help › What Is This Tool? lists every topic. |
| `ui/src/pages.tsx` | Setup (requirements, goal list, design problems), `goalCard` (Why), Sources page (kinds, licences, cites), Data (your materials, families, sources), `Later` pages for Compare, Index builder, Report; `MiniChart`. |
| `ui/src/print.tsx` | Print page: chart pages (vector, legend, caption), datasheet and results as flowing pages; header/footer codes. |
| `ui/src/features.ts`, `roadmap.tsx` | The feature catalogue (A data, B charts, C selection, D compare/report, E eco, F process, G help) and `soon(id)` cards, Roadmap page; past releases plus `RELEASES`/`PLANS` read from `docs/site.json`. |
| `ui/src/ribbon.tsx`, `backstage.tsx`, `dialogs.tsx`, `cmds.ts`, `ui.tsx`, `selection.ts`, `cells.ts`, `fmt.ts`, `main.tsx`, `styles.css` | Family patterns from Plan and Plot, adapted. |
| `docs/site.json` | The public summary (FAMILY §10): tagline, audience, about (Overview paragraphs), three steps, highlights, screenshots, roadmap (Next/Soon/Later/Goal). Read by the family site, the Datum network and Help › Roadmap. `site_json_is_current` checks its fields, about and steps, files and that its roadmap starts after the current version. |
| `tests_ui/run.cjs` | Playwright against `ui/dist` with the real engine (bridge): 72 checks (goals, units, bars, Tell Me materials, sources, ? help, Home tiles, checked ✓, chart, hulls, hover, click on the line, datasheet, drag changes the pass count, limits and results, browse and TSV, Add Material, ribbon fit at 1280/1600, planned cards, print of every view, glossary, Options, no script errors). `SHOTS=dir` saves screenshots. |

### Update flow (same as Plan)
On start and on "Check again": GET `/repos/Edgar-Mendonca/Datum-Atlas/releases/latest` (or `/releases` with beta) → badge + Home card with the CHANGELOG section → Update now downloads `DatumAtlas-x.y.z-setup.exe`, checks `SHA256SUMS.txt`, runs it with `/S`, restarts. Install only from this repo's release URLs.

### Release flow (same as Plan)
Bump `src-tauri/Cargo.toml` + `crates/core/Cargo.toml` → CHANGELOG `## x.y.z — date` → update this file **and `docs/site.json`** (the roadmap in both) → commit + push → `release.yml` (tag push, or workflow_dispatch when tags are blocked) → GitHub Release with setup.exe + SHA256SUMS.txt → users click Update now.

### How we work (from Datum Plan, FAMILY §11)
1. **The user reviews the last release** in the app and sends screenshots with bugs and wishes.
2. **Diagnose first.** Reproduce the problem in the test harness or the real app before changing code. Small fixes go straight to a release.
3. **Bigger asks get a plan file** (`docs/PLAN-x.y.md`): feedback, diagnosis, proposal, size, questions. Logged here and pushed. Build only after the user answers.
4. **Build in small steps**, engine first (`crates/core`, with a unit test), then the UI. Compact code in the surrounding style.
5. **Check in the real app** under Xvfb, with screenshots of each change. Fix what looks wrong.
6. **Add a UI test** for every reported bug and new feature (`tests_ui/run.cjs`). Make it fail on the old code where possible.
5b. Real app: `scratchpad/app.sh` (Xvfb :99 at 1600×1000, `target/debug/datum-atlas`, kill with `pkill -x`), drive with `xdotool`, screenshot with `import -window root`.
7. **Run the checks:** clippy `-D warnings`, `cargo test --workspace` (with the data checks), `npm run check:ui`, `npm run test:ui`.
8. **Release:** bump, CHANGELOG (becomes the in-app "What's new"), update this file, commit, push, run `release.yml`, watch CI + Release, confirm the installer and its size.
9. **Report to the user** in plain language: what changed, what was checked, what is not done, what to check on Windows.

---

## 2. Decisions

| Date | Decision | Why |
| --- | --- | --- |
| 2026-10-24 | Violet accent, `.datlas` + `.datdb`, `com.edgarmendonca.datumatlas`, releases and updater from `Edgar-Mendonca/Datum-Atlas` | The user's instruction, per FAMILY §1 and §3. (Name "Datum Atlas" to confirm, PLAN Q1.) |
| 2026-10-24 | No Python inside the app | The user's instruction and FAMILY §2: install size, start time, a second runtime. |
| 2026-10-24 | Built-in data compiled by us from public sources, a reference for every value; never proprietary databases | The user's instruction and brief §3.5. |
| 2026-10-24 | Plan, memory and questions before any feature code; build 0.1.0 after the answers | The user's instruction and FAMILY §12. |
| 2026-10-24 | Own SVG chart engine; Datum Plot's matplotlib renderer is not reused, but its graph-view patterns are (log/linear mapping, hover, crosshair, double-click zones, docked Format panel, export dialog, Copy Picture through Rust) | Plot draws with Python, which Atlas does not have (PLAN §3). |
| 2026-10-24 | Rust computes chart geometry (bubbles, hulls); the window owns the view transform, ticks and labels | Pan and zoom need no engine call per frame; values still come only from the engine. |
| 2026-10-24 | Dependencies arrive with the release that needs them: Chart.js (radar/bars), GridCraft (expressions), calamine/rust_xlsxwriter (import/export) | Keep 0.1.0 small and quick to build. |
| 2026-10-24 | Database as reviewed JSON in `data/`, embedded with `include_str!` and validated by `cargo test`; each material has a `status` (draft/checked) | Readable diffs for every value and its source; honest drafts that tighten over releases. |
| 0.1.0 | Answers (PLAN-0.1 §10): name Datum Atlas; engineers and students; 100 materials; users add their own with a window explaining what is needed; eco audit and process selection before 1.0; earlier "material atlas" repo not used; references and sources as proposed; print in 0.1.0; whole UI laid out; brief's tabs incl. Home; Okabe-Ito families; classic look; °C and USD/kg; new violet logo; each app's memory in its own repo | The user's answers, 2026-10-24. |
| 0.1.0 | Basic selection (limits, line, box, families, results) moved into 0.1.0; index builder and expressions become 0.2 | The user: "start with the basics: how the property charts are done, selection and so on". |
| 0.1.0 | Follow the established selection tool's workflow, never its data, names, wording or artwork | The user suggested copying it; the brief forbids proprietary data and naming other products. Functional workflow is fine. |
| 0.1.0 | Elastomers and foams are top-level families (own colour and envelope) | They sit apart on every chart. |
| 0.1.0 | Family envelopes are split into groups of nearby bubbles | One convex hull around a spread-out family (natural materials, foams) covered half the chart. |
| 0.1.0 | Click on a bubble shows a short datasheet beside the chart; double-click opens the full one | Keeps the chart in view while exploring; the brief's "click opens the datasheet". |
| 0.1.0 | UI tests run the real engine through `examples/bridge.rs` instead of a static fixture | Dragging the line, limits and Add Material need real answers. |
| 0.1.0 | Ceramic, glass and stone strengths: tensile column holds the flexural strength (noted on the datasheet); compressive strength added as its own property | Ceramics have no yield strength; the tensile test is rarely done. |
| 0.1.1 | Answers to PLAN-0.1.1 §7: build §3; 22 goals (the screenshot showed 14, so 8 extras); goals set the line **and** suggest limits; a goal changes the open chart; check data now, metals first; no teaching mode | The user's answers, 2026-10-25. |
| 0.1.1 | Goal limits are marked (`Limit.goal`) and replaced by the next goal; the user's own limits are never touched | A goal must be easy to change without losing the user's work. |
| 0.1.1 | **? help beside the tools in every Datum app**: a click opens a card saying what the tool is for and how to use it, from one `HELP` table | The user, instead of a teaching mode: “we should do it in all of our applications”. |
| 0.1.1 | Data check cites only primary publishers (producers, suppliers' own datasheets, universities, government), never aggregator sites (MatWeb, MakeItFrom, Total Materia, Wikipedia); a value is marked checked only if the datasheet's figure falls in our range, else the range is widened to include it | Open sources only, and honest ✓ marks. |
| 0.1.1 | The check used the figures quoted by web search from each datasheet (the build machine cannot open web pages); a second pass that opens each document is planned | Recorded so nobody mistakes it for a table-level check. |
| 0.1.1 | `docs/site.json` (about, steps, highlights, roadmap) is updated in every release, together with the roadmap here; public text never names or compares with a commercial product (hub FAMILY §13) | The user, 2026-10-28: the family site and the Datum network build from it. |
| 0.2.0 | **Data for the 250 materials: open, public and free sources only** (government, open datasets, producers' free datasheets cited value by value, our labelled compilation). Never a proprietary or licensed database or aggregator, not even to check a value. Enforced by the source kinds in `db::check`. | The user, 2026-10-28, before the big update. |
| 0.2.0 | **One big update instead of small splits**: 0.1.2 and the old 0.2/0.3/0.3.1 become one release, 0.2.0 ([PLAN-0.2.0.md](PLAN-0.2.0.md)); eco audit and process selection become 0.3.0. Hardening is still built first. Fixes between big updates still go out as small releases. | The user, 2026-10-28: Atlas is far behind the other Datum apps; "we are capable of doing a big build". |
| 0.1.2 | Hardening (FAMILY §11) ships with the new features, built first | The user's answer to PLAN-0.1.2 Q8. |
| 0.1.1 | Derived properties are computed by the engine at load, not stored | One source of truth; they follow corrections automatically. |
| 0.1.0 | Logo: violet log–log axes, a dotted selection line between two family envelopes (the upper one with a violet bubble), the amber diamond at the line's end | The user asked for a better violet logo; it keeps FAMILY §5. |
| 2026-10-29 | Windows publisher "Datum" for every Datum app (hub FAMILY §9); copyright stays "© 2026 Edgar Mendonca" | One family name in Windows Apps and the installer. |

---

## 3. Roadmap

✅ done · 🔜 next · ⬜ planned. Small releases; every one goes through the in-app updater. Details in [PLAN-0.1.md](PLAN-0.1.md) §7.

| Version | Theme | Contents | Status |
| --- | --- | --- | --- |
| — | Plan | CLAUDE.md, this file, PLAN-0.1 with questions | ✅ |
| **0.1.0** | Data, charts, selection | Shell, updater, File page, Violet design and logo; 100 referenced materials; Browse; Datasheet; property charts (bubbles, family envelopes, hover, click, zoom, pan, guides, legend); limits, line, box and family stages; Results with reasons and ranking; Add Material + user database; `.datlas`; print; SVG/PNG export; whole UI laid out | ✅ |
| **0.1.1** | Goals, units, search ([PLAN-0.1.1.md](PLAN-0.1.1.md)) | 22 design goals (axes, line, suggested limits, Why card); derived properties; Home tiles and Sources page; SI/imperial; Tell Me finds goals and materials; bar charts; Copy Picture; label nudging; ? help; 51 metal values checked against datasheets | ✅ |
| **0.2.0** | The big update ([PLAN-0.2.0.md](PLAN-0.2.0.md)) | Hardening (safe writes, damaged files set aside, fuzz tests, cargo deny, Windows/macOS CI); several lines, bands, polygons, two-chart goals, keyboard, weighted ranking; 25 goals + 47-index library and builder; expression axes and formula properties; charts side by side, best labels, label placement, styles; compare, radar, selection report, results to xlsx/CSV; 252 materials from open sources, durability ratings, 73 values checked; several databases, spreadsheet/CSV import and export; roadmap read from site.json | ✅ |
| **0.2.1** | Publisher | Windows publisher "Datum" for the whole family (hub FAMILY §9); copyright unchanged | ✅ |
| 0.3.0 | Eco and process | Eco audit (energy and CO₂ across a product's life, greener substitutes); process selection | 🔜 |
| 1.0.0 | Stable | ~1,000 materials, user guide, signed installer | ⬜ |

---

## 4. Session log

### 2026-10-24: Session 1 (plan)
- The user started Datum Atlas as the third Datum app: Violet accent, `.datlas`/`.datdb`, `com.edgarmendonca.datumatlas`, this repo for releases, no Python, data from public sources with a reference for every value. Asked for CLAUDE.md, this file, `docs/PLAN-0.1.md` and the brief's questions before building.
- Read Datum Plan (read-only): the brief, FAMILY.md, CLAUDE.md, MEMORY.md, CHANGELOG.md, the reference code (styles, icon sprite, ribbon, backstage, print, selection/cells/fmt, update.rs, release and CI workflows, UI tests) and the draft logo.
- Read Datum Plot (read-only): CLAUDE.md, MEMORY.md, PLAN-0.1, `graph.tsx`, `format.tsx`, `print.tsx`, `py/datum/graph.py`. Finding: Plot's graphs are drawn by matplotlib in Python, so only its window-side patterns carry over (PLAN §3).
- Wrote CLAUDE.md, this file and [PLAN-0.1.md](PLAN-0.1.md) (16 questions). Copied the draft logo. No code yet. **Waiting for the answers.**
- Noticed in passing: Datum Plan's CHANGELOG.md has lost the `## x.y.z —` headings of 0.3.7 to 0.3.3 (only " 2026-10-24" remains), so their notes cannot be found by version.

---

### 2026-10-24/25: Session 2 (answers, 0.1.0)
- Answers recorded in [PLAN-0.1.md](PLAN-0.1.md) §10 (see decisions). Built 0.1.0:
  - Engine: model, units, db (embedded data + checks), geometry (ellipses, rounded hulls, clusters), chart, select (stages, reasons, ranking, line values), doc, templates; 12 tests.
  - Data: 100 materials (31 metals, 23 polymers, 8 elastomers, 15 ceramics and glasses, 7 composites, 6 foams, 10 natural), 13 properties each where known, 9 sources; all drafts.
  - Shell: commands, user database `My materials.datdb`, options, recovery, updater for this repo.
  - Window: chart view, Browse, Datasheet, Stages, Results, Add Material, Setup, Data, laid-out pages (Compare, Index, Report), ribbon with every planned command, File page, Print, Options, Help.
  - New violet logo and Windows icons; README with screenshots; CHANGELOG.
- Released 0.1.0 with `release.yml` (workflow_dispatch): CI and Release green; installer 2.2 MB.
- Checked: Playwright with the real engine (30 checks); the real app under Xvfb (Home, "Hot, stiff and light", hover with the failure reason, dragging the line: chart stage 24 → 57).
- Fixed while testing: a click on a bubble under the selection line was swallowed by the line; one convex envelope per family covered half the chart (now groups); ρ shown as "P" in uppercase mono headings; symbols with underscores (K_Ic).

### 2026-10-25: Session 3 (review of 0.1.0, plan for 0.1.1)
- The user reviewed 0.1.0 (3 screenshots).
  - Wants the **design goal drop-down** from their earlier attempt (16 goals in groups: minimum mass, cost, carbon; elastic design; damage tolerance; thermal).
  - Wants Home tiles **Material Library → Add a material → Sources → Roadmap**.
  - Print is fine.
  - Asked for an updated memory and a list of what can come in 0.1.1 and after, so they can choose.
- Diagnosis: 6 goals exist only as templates making new selections; the cost and carbon goals need per-volume axes (Cₘ·ρ, CO₂·ρ), which 0.1.0 lacks.
- Wrote [PLAN-0.1.1.md](PLAN-0.1.1.md): the proposal, a menu of ~40 items (A–G with IDs), size, 6 questions. No code changed. **Waiting for the answers.**

### 2026-10-25: Session 4 (answers, 0.1.1)
- Answers recorded in [PLAN-0.1.1.md](PLAN-0.1.1.md) §7 (see decisions). Built 0.1.1:
  - Engine: `goals.rs` (22 goals, `apply`), `Op::Goal`, derived properties, imperial units for the new units, bar charts (`bands`), index names with ½ ⅓ ⅔ ², templates from goals, checked-value rule; 16 tests.
  - Window: goal list (with groups) and Why card, two-row chart toolbar, Setup goal list, Home tiles, Sources page, units everywhere (axes, ticks, tables, datasheet, limits, Add Material), bar charts, Copy Picture, label nudging, Tell Me finds goals and materials, `?` help on the tools (`help.tsx`) and Help › What Is This Tool?.
  - Data: 16 datasheet sources; 51 values in 15 metals checked (✓); ranges widened where a datasheet fell outside (4340 elongation, A356 sy/ts/elongation, C26000 ρ and λ, C70600 σy, 5083 λ, 17-4PH ts).
- Checked: clippy, 16 engine tests, `check:ui`, 41 UI checks (all pass); the real app under Xvfb (Home tiles, “Light, stiff beam”: line E½/ρ, 12 of 100 pass, ranking by the index).
- Fixed while testing: the bar chart used the bubble chart's zoom window (now one window per kind and axes); the goal list crashed the UI tests until the bridge learnt `goal_list`; the toolbar wrapped at 1440 px (now two rows).
- Not done: metals still draft: 1020, 1080, 304, 316, 430, grey iron, 1100, AZ91, Inconel 625, CoCr F75, zinc AG40A, lead, tin, tungsten (no primary datasheet found, or the datasheet disagreed with our range in a way that needs a closer look). Help `?` for Datum Plan and Datum Plot is suggested as a task in their own repos.

### 2026-10-28: Session 5 (site.json)
- Added `docs/site.json` (hub FAMILY §10) from the user's draft, checked against the roadmap and CHANGELOG. Corrected: not every value cites a public source (our compilation and price estimates are our own), so it now says every value names its source; datasheets and results are printed, not exported; bar charts, cost per volume and Copy Picture added; versions written with three numbers (also in the roadmap here).
- New test `site_json_is_current`; CLAUDE.md and the release flow now include site.json. 17 engine tests pass.
- README, CHANGELOG 0.1.0 and the app said "Excel"; reworded to "spreadsheet" at the user's request (FAMILY §13).

### 2026-10-28: Session 6 (wording, plan for 0.1.2)
- Reworded "Excel" to "spreadsheet" in the README, CHANGELOG 0.1.0 and the app (Material Library tile, Browse copy toast, ? help, shortcuts, ribbon Import, Data page, roadmap features), and removed other product names from app text (Copy Picture and SVG tiles, the copy toast, the ribbon tooltip), per hub FAMILY §13. Code comments are unchanged.
- Wrote [PLAN-0.1.2.md](PLAN-0.1.2.md):
  - where we are;
  - proposal: 0.1.2 lines and results, 0.1.3 charts side by side, 0.2.0/0.2.1/0.2.2 indices;
  - size;
  - 7 questions.
- Diagnosis: several lines on one chart share its axes, so "stiff and strong" needs a second chart. Proposed goals that add it.
- No feature code changed. **Waiting for the answers.**

### 2026-10-28: Session 7 (site Overview)
- Added `about` and `steps` to `docs/site.json` (hub FAMILY §10: the Overview of /apps/atlas) from the user's draft, checked against the app. Corrected:
  - not every value cites a public source, so it now says every value "names its source";
  - leaving whole families out added to the ways of narrowing the field;
  - Compare is not built yet (0.3.0), so step 3 says "see the materials that pass, ranked";
  - results print but do not export, so it says print the chart, datasheets and results, or export the chart.
- README links to the family page. The test checks about (1–2 paragraphs) and steps (3).
- Noticed: hub FAMILY §11 now asks every app for a hardening standard (`hardening` tests, `deny.toml` with `cargo deny`, Windows/macOS CI, SECURITY.md, CONTRIBUTING.md). Atlas has none of it yet; to plan with 0.1.2.

### 2026-10-28: Session 8 (hardening in the plan)
- Added hardening (hub FAMILY §11) to [PLAN-0.1.2.md](PLAN-0.1.2.md):
  - what Atlas has and lacks;
  - the work (§3 item 8, done first);
  - its size (~1,200 lines in all);
  - Q8: in 0.1.2 or alone first.
- Found: a damaged `My materials.datdb` is read as empty with `.ok()`, and the next Add Material writes over it, so the user's materials are lost. The fix is the first hardening item.
- Found: every shell command uses `lock().unwrap()`, so one panic makes every later command fail.
- `site.json` roadmap: 0.1.2 lists the hardening.

### 2026-10-28: Session 9 (answer Q8)
- The user: hardening goes into 0.1.2 together with the new features (PLAN-0.1.2 §7). Questions 1–7 are still open.

### 2026-10-28: Session 10 (the big update)
- The user asked for one big update instead of small splits, because Atlas is behind the other apps (Plan 0.4.1, Plot 0.2.20).
- Wrote [PLAN-0.2.0.md](PLAN-0.2.0.md):
  - 28 items in 7 groups (sturdy, selection, indices and formulas, charts, compare and report, data, the app);
  - the order of work;
  - size: ~6–7k lines plus 150 materials, over several sessions;
  - 4 questions.
- Checked what can be copied from Datum Plan: `formula.rs`, `hardening.rs`, calamine and rust_xlsxwriter, `deny.toml`, SECURITY.md and CONTRIBUTING.md.
- PLAN-0.1.2 is marked as replaced. The roadmap is now 0.2.0 (big), 0.3.0 (eco and process), 1.0.0. `site.json` waits for the go.

### 2026-10-28: Session 11 (0.2.0 stage A: hardening)
- **Engine:**
  - `db::read_user`: a damaged file is an error, never empty; unusable values are dropped and listed.
  - `doc::write_atomic`: saves go through a temporary file and a rename.
  - `hardening.rs`: 8 tests.
  - `db::check` refuses any built-in source whose kind is not open, public or free (`OPEN_KINDS`), or that names a proprietary database or aggregator (`BARRED`).
  - Unknown goal ids are now an error (before: silently treated as Explore).
- **Shell:**
  - poisoned-lock recovery (`St::lock`);
  - a damaged `My materials.datdb` is renamed `My materials (damaged <date>).datdb` and the window shows a dialog;
  - a file that cannot be read locks your own materials against writing;
  - every file write is atomic.
  - Checked in the real app under Xvfb with a damaged file.
- **Repo:**
  - `deny.toml` (BSL-1.0 allowed for the clipboard crates), checked locally: advisories, bans, licences and sources ok;
  - CI jobs `deny` and `platforms` (Windows, macOS);
  - `SECURITY.md`, `CONTRIBUTING.md` (with the open-sources-only data rule).
- All checks pass (25 engine tests, UI tests).

### 2026-10-28: Session 11, continued (0.2.0 stage B: selection)
- **Engine:**
  - `Chart.sels` is a list of selections (`Line`, `Box`, new `Poly`); a material passes a chart when it passes all of them, so facing lines make a band. Old files' `select` moves into `sels` and is never written.
  - `Stages.rank_line`, `ranking` ("line" or "weighted") and `weights` (property, weight, higher better). Weighted scores scale each property so the best passing material scores 1 (a ratio for log properties, min–max for linear ones), then average by weight.
  - `Op::Select { sels }`, `Op::PassThrough` (line through a typed index value, via `select::y_for`), `geometry::in_polygon`.
  - Three two-constraint goals (25 in all): stiff and strong beam, stiff and strong panel, stiff and damage-tolerant tie. Each adds its second chart, marked `<id>/2`; a new goal removes the old second chart.
- **Window:**
  - every selection drawn, with its own handle and index label;
  - Shape tool (click corners; Enter or double-click closes; Esc cancels);
  - + Line, Pass through…, and selection chips (edit, remove, "rank by it", "ranks");
  - arrow keys and Delete on the focused chart;
  - a Ranking section in Stages (line or weighted);
  - ? help topics `sels` and `ranking`;
  - the second chart in the goal's Why card; printed captions for every line.
- 28 engine tests; 9 new UI checks, all passing.

### 2026-10-28: Session 11, continued (0.2.0 stage C: indices and formulas)
- **Engine:**
  - `formula.rs`: a parser on Datum Plan's pattern (+ − * / ^, brackets, sqrt ln log10 exp abs min max, properties in brackets by key, name or symbol), interval evaluation (a missing input, a division by a range holding zero or a root of a negative gives a gap), `show`.
  - `Project.formulas` (`Formula { name, expr, unit, log }`, key `f:<name>`); `Op::SetFormulas` checks names and syntax in order; `normalize` drops formulas that do not parse.
  - `Db::with_formulas` works them into every material (the shell and bridge do it before laying out charts and results); `Db::info` and `pname` give names and units of built-in and formula keys.
  - Limits, weights and index names work on formulas.
  - Index library: 22 more entries in the goal table (`library: true`, 47 in all), some with the formula properties they need (`formulas`), added on apply.
  - Waiting for data: `fatigue-light` (fatigue strength) and `conductor-light` (resistivity); the goal test lists them.
- **Window:**
  - `formula.tsx` (the formula window; also Chart › Expression Axis and Data › Formula Property);
  - the Index page (builder: function → constraint → objective; library with search; Plot on a new chart / Use the open chart);
  - formula properties in every property list (`chartProps`, `formulaProps`, `propOf`);
  - `selected` on options so a list shows a value whose option arrives late;
  - help topics `formula` and `index`.
- 30 engine tests and the new UI checks pass.

### 2026-10-28: Session 11, continued (0.2.0 stage D+E: charts, compare, report)
- **Engine:**
  - `export.rs` turns the results into a table (rank, index, × best, passes, the typical value of each property the selection uses, the reasons) and writes CSV (with a byte-order mark, quoted) or .xlsx (`rust_xlsxwriter` 0.101, as in Datum Plan).
  - `compare.rs`: up to 6 materials (`Project.compare`, `Op::Compare`); per property the highest and lowest, and the spread; radar positions 0–1 between the database's lowest and highest (log scale for log properties). `Out.compare`.
  - `Chart.best`, `style` (classic, publication, mono) and `moved` (label offsets).
  - Shell command `export_results(fmt)`.
- **Window:**
  - `compare.tsx` (table and `Radar`);
  - `report.tsx` (the selection report on screen and as a printed flowing document; parts in `ui.report`);
  - Best 5 labels with ranks, label placement around bubbles with leader lines and dragging (double-click resets);
  - styles;
  - charts side by side (1, 2 or 4, a stable page holding the open chart; hover highlights on all; fit and zoom act on the open chart);
  - "+ compare" in Results, Compare on the card beside the chart;
  - To a spreadsheet and CSV buttons; ribbon Compare, Radar and Report are live.
- 32 engine tests; UI checks for each of these pass.

### 2026-10-28: Session 11, continued (0.2.0 stage F, data)
- **252 materials** (+152): 105 metals, 53 polymers, 16 elastomers, 30 ceramics and glasses, 15 composites, 12 foams, 21 natural materials.
  - New metal classes: tool steels, precious and refractory metals.
  - New polymer classes: filled and bio-based.
  - Also thermoplastic elastomers, carbon, wood products, and other composites.
- **Sources (open rule):**
  - MIL-HDBK-5 for aerospace alloys, MIL-HDBK-17 for laminates, the Wood Handbook for woods, NIST for some ceramics, NPL tables for pure metals, USGS for pure-metal prices, ICE for CO₂ and energy;
  - otherwise our labelled compilation of typical ranges. All draft.
- Fatigue strength and electrical resistivity added to the metals: the "Light part under cyclic load" and "Light electrical conductor" indices now have data (waiting list empty).
- Durability ratings for every material: fresh water, weak acids, weak alkalis, UV, and fire resistance (renamed from flammability, so Excellent means it does not burn). Our compilation; class defaults with per-material overrides.
- Expansion left out (a gap, value in the notes) where it is near zero or negative and cannot sit on a log axis: glass-ceramic, high-modulus CFRP, carbon-carbon, CF/PEEK.
- The data was appended by a one-off script (scratchpad), so the 0.1.1 datasheet checks stay.
- Fixed: after saving your own material the datasheet opened only if the database reloaded within 150 ms; it now waits for the reload.
- The speed test builds exactly 1,000 materials (~200 ms release).

### 2026-10-28: Session 11, continued (0.2.0 stage F, databases and import)
- **Engine:**
  - `sheet.rs`:
    - reads CSV (the separator is guessed; quotes) and the first sheet of a workbook (calamine 0.37, as in Plan);
    - guesses each column from its heading (key, name or symbol; an imperial unit in brackets is converted);
    - reads one value or a range per cell, with blanks as gaps;
    - `to_database` (needs a name column and a source; cleaned like a `.datdb`);
    - `of_database` writes a database out so it reads back.
  - `Db::with_others`: other databases with prefixed ids and sources, and `Material.origin`.
  - Tests: import, round trip, other databases; damaged CSV and .xlsx never panic.
- **Shell:**
  - `databases`, `open_database`, `close_database` (the list is kept in Options `databases`, preserved when the window saves Options; an unreadable one is closed and reported at start);
  - `export_database` (.datdb, .xlsx, .csv);
  - `import_preview`, `import_materials` (fresh `u-` ids and sources).
- **Window:**
  - `importer.tsx`: the import window with a property per column and imperial choices, and the Databases section on the Data page;
  - the datasheet says which database a material comes from;
  - help topics `import` and `databases`.
- The bridge serves the same commands with fixtures `tests_ui/import.csv` and `team.datdb`.
- 35 engine tests and all UI checks pass; cargo deny ok.

### 2026-10-28: Session 11, continued (0.2.0 stage F: more values checked)
- 22 more values checked (73 in all) against producers' and suppliers' own datasheets and NIST:
  - 316 (Rolled Alloys); 430 (Combined Metals, ATI);
  - aluminium 1100 (NIST "Properties of Wrought Aluminum Alloys", Alloys International);
  - zinc die-casting alloy 3 (Xometry);
  - alloy 625 (ATI, voestalpine);
  - PC and PA66 density (Ensinger); PEEK density (Sterling Plastics).
  - The script asserts each datasheet figure lies in our range before marking it.
- Widened: 1100 λ to 231 (NIST), 1100 tensile to 89, 625 tensile to 827.
- Still draft (no primary source found by search): 1020, 1080, 304, grey iron, AZ91, CoCr F75 (only additive-manufacturing data found), lead, tin, tungsten; most polymers.
- Source titles made generic where they carried brand names ("Nickel alloy 718", "Steel 1.6582", "Zinc die-casting alloy 3"); the links stay as published.
- The checked-value test accepts datasheet, government and handbook sources with a link (≥ 70 values).

### 2026-10-28: Session 11, continued (0.2.0 stage G and release)
- Help › Roadmap and the in-app release list read `docs/site.json` (`features.ts` imports it), so the site and the app say the same.
- Ribbon and File stubs wired: Best 5 Labels, Style, Import, Save My Materials, Open Database.
- Getting Started and the Glossary cover bands, shapes, weighted ranking, the index library, formula properties, radar, databases and durability ratings.
- site.json: about, steps and highlights rewritten for 0.2.0 and checked against the app; roadmap now starts at 0.3.0 (Next).
- README features 0.2.0 and Coming next; CHANGELOG 0.2.0; versions 0.2.0.
- Checks: clippy, cargo test, check:ui, test:ui (72 checks), cargo deny, the real app under Xvfb.
- Released 0.2.0 through `release.yml`.

### 2026-10-29: Session 12 (0.2.1: publisher)
- `bundle.publisher` is now "Datum" (hub FAMILY §9, the whole family); `copyright` stays "© 2026 Edgar Mendonca". No other place in the repo writes the publisher name; this repo has no FAMILY.md copy.
- Version 0.2.1, CHANGELOG, `VERSION_NOW`; released through `release.yml`.

---

## 5. Known limits and next steps

**Next:** plan 0.3.0 (eco audit and process selection) with a `docs/PLAN-0.3.0.md` and questions first.

**Not yet checked on a real Windows PC:** printing/Save as PDF in WebView2, PNG export fonts (SVG drawn as an image may fall back from IBM Plex), the user database path, the updater install.

**Known limits:**
- Built-in values are drafts with document-level references, except 73 values checked against datasheets (from search-quoted figures; a pass that opens each document is planned). Still drafts with no primary source found: 1020, 1080, 304, grey iron, AZ91, CoCr F75, lead, tin, tungsten, most polymers.
- Envelopes are convex per group, not concave.
- Copy Picture is not yet tried on a real Windows clipboard.
- Import reads the first sheet of a workbook only; the radar places log properties on a log scale, which can surprise.
- Windows SmartScreen warns on the first install (unsigned).

**Open risks:**
- Referencing every value well takes time; most values are still drafts with document-level references.
- Prices and CO₂ values change and vary by source; they carry a year and basis.
