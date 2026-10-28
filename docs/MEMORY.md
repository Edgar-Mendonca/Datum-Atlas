# Datum Atlas: project memory

> **Paste this file at the start of each new session**, or just open the repo: `CLAUDE.md` tells Claude to read it.
> It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Atlas: a Windows desktop app for materials selection and property charts (the Ashby method): find the right material, understand why (performance indices, trade-offs), compare candidates, document the decision |
| **Family** | Third Datum app after Datum Plan (`Edgar-Mendonca/Datum-Plan`) and Datum Plot (`Edgar-Mendonca/Datum-Plot`). Design and workflow: [Datum-Plan/docs/FAMILY.md](https://github.com/Edgar-Mendonca/Datum-Plan/blob/HEAD/docs/FAMILY.md). Brief: [Datum-Plan/docs/briefs/DATUM-ATLAS.md](https://github.com/Edgar-Mendonca/Datum-Plan/blob/HEAD/docs/briefs/DATUM-ATLAS.md). Plan's code is the reference implementation: copy, don't reinvent. |
| **Current version** | **0.1.1** (released 2026-10-25: https://github.com/Edgar-Mendonca/Datum-Atlas/releases/tag/v0.1.1). Plans and answers: [PLAN-0.1.md](PLAN-0.1.md), [PLAN-0.1.1.md](PLAN-0.1.1.md) |
| **Installer** | NSIS `.exe`, per-user: **2.2 MB** (0.1.0); UI bundle 182 kB JS |
| **Repo** | `Edgar-Mendonca/Datum-Atlas` · working branch `claude/bold-volta-90lu2c` |
| **Identity** | Accent **Violet** (`--blue #6C4AB6`, `--blue2 #5A3C9C`, `--sky #C9B8EE`, `--pale #F0EBFA`, `--sel #E5DCF6`, `--sel2 #CDBDEE`; dark `--blue #A58BEA`, File sidebar dark `#3A2766`) · files `.datlas` (projects), `.datdb` (user databases) · identifier `com.edgarmendonca.datumatlas` · installer `DatumAtlas-x.y.z-setup.exe` · logo `docs/brand/datum-atlas-icon.svg` (violet log–log axes, family bubbles, dotted index line, amber `#F2B233` diamond) |
| **Stack** | Rust engine (`crates/core`) + Rust shell (`src-tauri`), Tauri 2, WebView2, SolidJS + Vite UI in strict TypeScript, plain CSS tokens, IBM Plex. Own SVG chart engine (geometry in Rust). Chart.js later for bar/radar, GridCraft later for expressions. **No Python.** |
| **Code size** | About 5,400 lines: core 1,690 (17 tests), shell 630, UI 2,820 TSX/TS + CSS, UI tests 250 (41 checks); data: 100 materials, 25 sources, 51 values checked |

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
| `crates/core/src/goals.rs` | `Goal` (group, name, index, axes, slope, above, function, constraint, objective, free variable, why, limits), `GOALS` (22), `get`, `apply`. |
| `crates/core/src/geometry.rs` | `t`/`span`/`mean` in axis space, `ellipse`, `convex`, rounded `hull`, `inside`, `clusters`. |
| `crates/core/src/chart.rs` | `build(db, chart)` → `ChartOut` (axes with limits, bubbles, hulls with members and labels, missing with reasons; for bar charts `bands` per family, sorted). |
| `crates/core/src/select.rs` | `run(db, project)` → `Results` (stage counts, rows with reasons, index, rank, × best, `lines` with index name and value; `index_name` writes E½/ρ, σy⅔/ρ). |
| `crates/core/src/doc.rs` | `Op`, `Doc` (undo 200, open/save). |
| `crates/core/src/templates.rs` | Design problems built from goals: blank, beam, tie, panel, spring, hot, tough. |
| `crates/core/src/tests.rs` | Data checks, bad-data detection, unit round trips, bubbles in log space, hulls contain their bubbles (6 axis pairs × family/class), missing values, index lines for slopes 1/2/3 above/below, limits (missing, flag, whole range, flags), combined stages, edits/undo/round trip/templates, user materials, tree and search; derived ranges; every goal (line, ranking by its hand-computed index, limits, Explore); index names; bar charts; checked values cite linked datasheets. |
| `crates/core/examples/bridge.rs` | The engine over stdin/stdout JSON lines for the UI tests. |
| `data/*.json`, `data/sources.json` | The built-in database (100 materials, 25 sources: 9 general + 16 datasheets), `status: draft`; 15 metals have `checked` lists (51 values). |
| `src-tauri/src/main.rs` | Commands in the diagram; options (theme, autosave, beta, openLast, labels, guides, dpi, units, paper, landscape, company, logo); user database file; recovery; recent files; `open_url` only for this repo. |
| `src-tauri/src/update.rs` | Datum Plan's updater with `REPO = Edgar-Mendonca/Datum-Atlas`. |
| `ui/index.html` | Icon sprite (Plan/Plot icons recoloured violet + atlas, bar, box, compare, datasheet, db, eco, eye, hull, index, material, pan, poly, process, radar, swap, tree, units). |
| `ui/src/api.ts` | Types, `fmt`/`range`/`valText`/`mid`, `FAMILY_COLORS` (Okabe-Ito). |
| `ui/src/store.ts` | Signals and actions (see diagram); `goals`, `setGoal`; units (`imperial`, `conv`, `toShown`, `toStored`, `unitOf`, `vtext`, `setUnits`); `show(view)` also switches the ribbon tab; `failed()` for greying. |
| `ui/src/chart.tsx` | `ticks`, `ChartSvg` (screen, print and export), `Legend`, `ChartView` (zoom/pan window per chart, hover, click, drag line/box, Line and Box tools), `ChartBar` (goal list, Why, bubbles/bars, envelopes, zoom), `ChartTools` (axes, Pan/Line/Box, slope), bar rendering, label `nudge`, `chartSvgText`, `exportChart`, `copyChart`. |
| `ui/src/browse.tsx` | `Tree`, `Browse` (grid engine `selection.ts`, sort, columns menu, Ctrl+C via Rust), `Datasheet` (groups, numbered references). |
| `ui/src/stages.tsx` | `StagesPage` (limits, chart selections, family tree), `StagePanel`, `ResultsPage`, `addLimit`. |
| `ui/src/material.tsx` | Add/Edit Material window with the "What you need" panel; `deleteMaterial`. |
| `ui/src/help.tsx` | `HELP` (18 topics), the `?` button, `helpCard`. Help › What Is This Tool? lists every topic. |
| `ui/src/pages.tsx` | Setup (requirements, goal list, design problems), `goalCard` (Why), Sources page (kinds, licences, cites), Data (your materials, families, sources), `Later` pages for Compare, Index builder, Report; `MiniChart`. |
| `ui/src/print.tsx` | Print page: chart pages (vector, legend, caption), datasheet and results as flowing pages; header/footer codes. |
| `ui/src/features.ts`, `roadmap.tsx` | The feature catalogue (A data, B charts, C selection, D compare/report, E eco, F process, G help) and `soon(id)` cards, Roadmap page. |
| `ui/src/ribbon.tsx`, `backstage.tsx`, `dialogs.tsx`, `cmds.ts`, `ui.tsx`, `selection.ts`, `cells.ts`, `fmt.ts`, `main.tsx`, `styles.css` | Family patterns from Plan and Plot, adapted. |
| `docs/site.json` | The public summary (FAMILY §10): tagline, audience, highlights, screenshots, roadmap (Next/Soon/Later/Goal). Read by the family site, the Datum network and Help › Roadmap. `site_json_is_current` checks its fields, files and that its roadmap starts after the current version. |
| `tests_ui/run.cjs` | Playwright against `ui/dist` with the real engine (bridge): 41 checks (goals, units, bars, Tell Me materials, sources, ? help, Home tiles, checked ✓, chart, hulls, hover, click on the line, datasheet, drag changes the pass count, limits and results, browse and TSV, Add Material, ribbon fit at 1280/1600, planned cards, print of every view, glossary, Options, no script errors). `SHOTS=dir` saves screenshots. |

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
| 0.1.1 | `docs/site.json` is updated in every release, together with the roadmap here; public text never names or compares with a commercial product (hub FAMILY §13) | The user, 2026-10-28: the family site and the Datum network build from it. |
| 0.1.1 | Derived properties are computed by the engine at load, not stored | One source of truth; they follow corrections automatically. |
| 0.1.0 | Logo: violet log–log axes, a dotted selection line between two family envelopes (the upper one with a violet bubble), the amber diamond at the line's end | The user asked for a better violet logo; it keeps FAMILY §5. |

---

## 3. Roadmap

✅ done · 🔜 next · ⬜ planned. Small releases; every one goes through the in-app updater. Details in [PLAN-0.1.md](PLAN-0.1.md) §7.

| Version | Theme | Contents | Status |
| --- | --- | --- | --- |
| — | Plan | CLAUDE.md, this file, PLAN-0.1 with questions | ✅ |
| **0.1.0** | Data, charts, selection | Shell, updater, File page, Violet design and logo; 100 referenced materials; Browse; Datasheet; property charts (bubbles, family envelopes, hover, click, zoom, pan, guides, legend); limits, line, box and family stages; Results with reasons and ranking; Add Material + user database; `.datlas`; print; SVG/PNG export; whole UI laid out | ✅ |
| **0.1.1** | Goals, units, search ([PLAN-0.1.1.md](PLAN-0.1.1.md)) | 22 design goals (axes, line, suggested limits, Why card); derived properties; Home tiles and Sources page; SI/imperial; Tell Me finds goals and materials; bar charts; Copy Picture; label nudging; ? help; 51 metal values checked against datasheets | ✅ |
| 0.1.2 | Lines and charts ([PLAN-0.1.2.md](PLAN-0.1.2.md)) | Several selection lines per chart, results to a spreadsheet, keyboard, more values checked (rest of the metals, then polymers). Proposed: also two-constraint goals, best-material labels, one roadmap from `site.json`; charts side by side moved to 0.1.3. Waiting for the user's choices. | 🔜 |
| 0.2.0 | Indices | Index builder (~40 indices); expression axes; formula properties (GridCraft) | ⬜ |
| 0.3.0 | Compare and report | Compare + radar; selection report; ~250 materials, more values checked | ⬜ |
| 0.3.1 | Own data | Several databases, spreadsheet/CSV import with mapping; label placement; style presets; polygon stage | ⬜ |
| 0.4.0 | Eco audit | Energy and CO₂ across a product's life | ⬜ |
| 0.5.0 | Process selection | Manufacturing processes alongside materials | ⬜ |
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

---

## 5. Known limits and next steps

**Next:** the user answers [PLAN-0.1.2.md](PLAN-0.1.2.md) §6 and reviews 0.1.1 on Windows; then build 0.1.2.

**Not yet checked on a real Windows PC:** printing/Save as PDF in WebView2, PNG export fonts (SVG drawn as an image may fall back from IBM Plex), the user database path, the updater install.

**Known limits:**
- Built-in values are drafts with document-level references, except 51 metal values checked against datasheets (from search-quoted figures; a pass that opens each document is planned). Polymers, ceramics, composites, foams and natural materials are next, one family per release.
- Hull labels can overlap where families meet (label placement in 0.3.1). Envelopes are convex per group, not concave.
- Copy Picture is not yet tried on a real Windows clipboard.
- One selection line per chart; goals with two constraints (e.g. stiff **and** strong) need two charts.
- Windows SmartScreen warns on the first install (unsigned).

**Open risks:**
- Referencing every value well takes time; most values are still drafts with document-level references.
- Prices and CO₂ values change and vary by source; they carry a year and basis.
