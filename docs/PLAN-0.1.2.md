# Datum Atlas 0.1.2 and after: plan

Status: **waiting for the user's choices** (§6). Written 2026-10-28, after 0.1.1 was released, at the user's request: "plan the next updates on the app based on the roadmap". Nothing is built until the user answers.

This file follows the family pattern: request → where we are → proposal → size → questions.

---

## 1. Request

Plan the next releases from the roadmap (MEMORY §3, `docs/site.json`):

| Version | Theme | Roadmap |
| --- | --- | --- |
| 0.1.2 | Lines and charts | Several selection lines per chart; results to a spreadsheet; keyboard control; more values checked (the rest of the metals, then polymers) |
| 0.2.0 | Indices | Index builder (~40 indices); expression axes; formula properties |
| 0.3.0 | Compare and report | Compare and radar charts; selection report; ~250 materials, more values checked |
| 0.3.1 | Your own data | Several databases; spreadsheet/CSV import with mapping; label placement; style presets; polygon selection |
| 0.4.0 | Eco audit | Energy and CO₂ across a product's life |
| 0.5.0 | Process selection | Manufacturing processes alongside materials |
| 1.0.0 | Stable | ~1,000 materials, user guide, signed installer |

## 2. Where we are (diagnosis)

- **One selection per chart.** `Chart.select` holds one line *or* one box (`Option<ChartSel>`). Results rank by the line of one chart (`rank_chart`).
- **"Several lines on one chart" needs a closer look.** The catalogue's example (C9: "stiffness and strength together") does not fit on one chart. The lines on a chart share its axes: on modulus–density, every line is a stiffness index. Stiffness *and* strength needs two charts (E–ρ and σy–ρ). Atlas already combines selections across charts, but the user has to build the second chart by hand. What one chart with several lines does well:
  - a **band** between two lines (e.g. stiff enough but not over-designed);
  - **two indices on the same axes**: a tie (slope 1) and a beam (slope 2) made from one material.
- **The catalogue also lists for 0.1.2:**
  - charts side by side (B13);
  - labelling the best materials (B14).

  Together with the four roadmap items, that is more than one small release.
- **Results leave the app only by print or Ctrl+C from Browse.** The results table has no export. A real `.xlsx` file needs a Rust crate (`rust_xlsxwriter`); CSV needs nothing new.
- **Keyboard:** the line moves only by dragging or by typing its slope.
- **Data:**
  - 51 values in 15 metals are checked.
  - 14 metals are still drafts: 1020, 1080, 304, 316, 430, grey iron, 1100, AZ91, 625, CoCr F75, zinc AG40A, lead, tin, tungsten.
  - All 23 polymers are still drafts.
  - The 0.1.1 check used datasheet figures quoted by web search: this environment's network policy blocks opening the datasheets themselves (Q5).
- **Housekeeping:**
  - The in-app roadmap (`ui/src/features.ts`) keeps its own release list, and it uses two-number versions ("0.2", "0.3").
  - `docs/site.json` now holds the same roadmap, so the two can drift apart.
- **Not yet confirmed on a real Windows PC:**
  - Copy Picture;
  - printing;
  - the 0.1.0 → 0.1.1 update through the updater.

## 3. Proposal

Keep every release small. Split the 0.1.2 theme in two, and the 0.2 theme in three.

### 0.1.2 "Lines and results" (next)
1. **Several lines on a chart (C9).** A chart keeps a list of selections: lines and at most one box.
   - Each line has its slope and its side.
   - Two lines of the same slope with opposite sides make a band.
   - Results rank by one line, which you choose (marked "ranks" in Stages).
   - Old `.datlas` files open unchanged: a single selection becomes a list of one.
   - Engine first, with tests: a band, two slopes, and ranking by the chosen line.
2. **Goals with two constraints.** New goals that need two charts add the second chart themselves:
   - stiff **and** strong beam;
   - stiff and strong panel;
   - light, stiff and tough tie.

   Selections already combine across charts, so the shortlist is what passes both.
3. **Keyboard for the line (C10):**
   - arrow keys move the selected line (Shift for big steps);
   - Delete removes it;
   - "Pass through…" types the index value the line should pass through.
4. **Label the best (B14).** The five best materials by the index are labelled on the chart (on/off in the toolbar). The labels move apart as the family labels do.
5. **Results to a spreadsheet (D5).** File › Export › Results: the ranked list with the index, "× best", the stages and the reason each failure failed. Saved as `.xlsx` and CSV (Q3).
6. **Data.** Check the 14 remaining metals, then start on the polymers. Same rule as 0.1.1: primary publishers only, ✓ only when the datasheet's figure falls in our range.
7. **One roadmap.**
   - Embed `docs/site.json` in the app, and have Help › Roadmap take its release names, timing and items from it, so the app, the family site and MEMORY show the same plan.
   - Versions in the app become three numbers.

### 0.1.3 "Charts side by side"
- **B13:** two or four charts at once, one selection shared across them. Hovering a material highlights it on every chart.
- Two-constraint goals open their two charts side by side.

### 0.2.0, 0.2.1, 0.2.2 "Indices"
- **0.2.0 Index library and builder (C7, C6):**
  - about 40 standard indices with their derivations;
  - pick function → constraint → objective → free variable, and get the index, its derivation and the chart with the line.
  - The 22 goals become the first entries of the library.
- **0.2.1 Expression axes and formula properties (B8, A13):**
  - an axis such as `[Young's modulus]^0.5/[Density]`;
  - your own derived properties as spreadsheet-style formulas.
  - This needs the family's formula engine. It is its own release because parsing, units and errors need care.
- **0.2.2 Weighted ranking (C11, new):** rank by any property or a weighted mix (e.g. 60 % mass, 40 % cost), with the trade-off explained.

### 0.3.0 to 1.0.0
- No change to the roadmap: compare and report, own data, eco audit, process selection, stable.
- Data checking continues one family per release.
- Each of these gets its own plan file before it is built.

## 4. Order of work in 0.1.2

1. Engine (`crates/core`):
   - selections as a list (`model.rs`, with old files loading);
   - `select.rs` with bands and the ranking line;
   - two-chart goals (`goals.rs`);
   - results to `.xlsx`/CSV.
   - Each comes with tests.
2. Window:
   - lines list in the toolbar and Stages;
   - keyboard;
   - best-material labels;
   - the Export › Results tile;
   - the Roadmap page reading `site.json`;
   - `?` help for every new tool.
3. Data check (metals, then polymers) with `checked` lists and sources.
4. UI tests for each item; the real app under Xvfb; CHANGELOG, MEMORY, `site.json`; release.

## 5. Size

| Item | Engine | Window | Notes |
| --- | --- | --- | --- |
| Several lines, bands, ranking line | ~150 lines | ~120 lines | Format change with old files loading |
| Two-constraint goals | ~60 | ~20 | Uses the existing goal table |
| Keyboard | — | ~60 | |
| Label the best | ~20 | ~50 | Reuses the label nudging |
| Results to `.xlsx`/CSV | ~80 | ~30 | `.xlsx` adds a Rust crate, a few hundred kB to the installer |
| One roadmap from `site.json` | — | ~40 | |
| Data check | data only | — | About 60 values, as far as published datasheets can be found |

About 800 lines in all, similar to 0.1.1.

## 6. Questions

1. **Release split:** 0.1.2 as in §3 (lines, two-constraint goals, keyboard, best labels, results export, data, one roadmap), with charts side by side moved to 0.1.3? Or keep side by side in 0.1.2 and make it bigger?
2. **Several lines:** is the band and the second index on the same axes what you want from "several lines", with "stiff *and* strong" handled by goals that add a second chart? Or did you have something else in mind?
3. **Results export:** `.xlsx` and CSV (proposed; adds a few hundred kB to the installer), or CSV only (no new dependency)?
4. **Ranking with several lines:** rank by one line you choose (proposed), or by a combination of all the lines?
5. **Data check:** this session's network policy only reaches GitHub, so I cannot open the datasheets themselves, only search results quoting them. To check value by value against the document, the environment's **Network access** needs a broader level or the datasheet hosts added under Allowed domains. Change it in the cloud environment settings (the environment menu in the session's title bar → Edit; steps: https://code.claude.com/docs/en/cloud-environments#network-access). Do you want to open it up, or continue with search-quoted checks, clearly labelled as such?
6. **Indices:** split 0.2 into 0.2.0 (library and builder), 0.2.1 (expressions and formula properties), 0.2.2 (weighted ranking)?
7. **0.1.1 on Windows:** before 0.1.2, could you try Copy Picture, printing, and the update from 0.1.0, and send screenshots of anything wrong? Fixes would go first, as 0.1.2 or a quick 0.1.1-fix release before it.
