# Datum Atlas 0.2.0: the big update (plan)

Status: **go given** (§7), being built. Written 2026-10-28. The user: "Don't split too much… list a big build… make it a big update. This is the only application where we are far behind." This file replaces the small-release split in [PLAN-0.1.2.md](PLAN-0.1.2.md). Its hardening answer (built first) still holds.

---

## 1. Where Atlas stands against the family

| | Datum Plan | Datum Plot | Datum Atlas |
| --- | --- | --- | --- |
| Version | 0.4.1 | 0.2.20 | **0.1.1** |
| Hardening (FAMILY §11) | Yes (`hardening.rs`, `deny.toml`, Windows/macOS CI, SECURITY, CONTRIBUTING) | | **No** |
| Formula engine | Yes (`formula.rs`) | | **No** (expression axes and formula properties wait for it) |
| Spreadsheet in and out | Yes (`calamine`, `rust_xlsxwriter`) | | **No** (copy from the browse table only) |

Everything the big update needs already exists in the family and can be copied rather than invented: Plan's formula engine, hardening module, spreadsheet crates, `deny.toml`, `SECURITY.md` and `CONTRIBUTING.md`.

## 2. The big update: everything that is possible now

One release, **0.2.0**, built in stages. Each stage is pushed when its checks pass. The release goes out when all of it is done and checked in the real app.

### A. Sturdy (hardening, FAMILY §11): built first
1. **Your materials are never lost.**
   - A damaged `My materials.datdb` is kept aside and reported, never written over.
   - Saves go through a temporary file and a rename.
2. **`hardening` tests:**
   - failed edits change nothing;
   - random projects and databases never panic and keep their invariants;
   - damaged `.datlas`, `.datdb`, options, recent and recovery files open or give a clear error;
   - a speed gate on a large project.
3. **Shell:** the state lock recovers from a poisoned mutex.
4. **Supply chain and repo:**
   - `deny.toml` and `cargo deny check`;
   - CI on Windows and macOS as well as Linux;
   - `SECURITY.md`, `CONTRIBUTING.md`.

### B. Selection
5. **Several selection lines on a chart**, and a **band** between two lines. Results rank by the line you choose.
6. **Goals with two constraints:**
   - stiff and strong beam;
   - stiff and strong panel;
   - light, stiff and tough tie.

   These goals add their second chart themselves; the shortlist is what passes both.
7. **Keyboard:**
   - arrow keys move the selected line (Shift for big steps);
   - Delete removes it;
   - "Pass through…" types the index value.
8. **Polygon selection (C8):** draw any shape on a chart.
9. **Weighted ranking (C11):** rank by any property, or by a weighted mix (e.g. 60 % mass, 40 % cost), with the trade-off explained.

### C. Indices and formulas
10. **Index library (C7):** about 40 standard performance indices (ties, beams, panels, columns, shafts, springs, hinges, pressure vessels, flywheels, thermal and electrical uses), each with its derivation. The 22 goals become its first entries.
11. **Index builder (C6):** pick function → constraint → objective → free variable, and get:
    - the index;
    - its derivation, step by step;
    - the chart with the line.
12. **Expression axes (B8):** an axis such as `[Young's modulus]^0.5/[Density]`, with units, on the formula engine copied from Datum Plan.
13. **Formula properties (A13):** your own derived properties as spreadsheet-style formulas, usable in charts, limits and ranking.

### D. Charts
14. **Charts side by side (B13):** two or four charts at once, one selection across them. Hovering a material highlights it on every chart.
15. **Label the best materials (B14):** the top five by the index labelled automatically.
16. **Labels that do not overlap (B9):** material labels placed automatically, with leader lines; drag a label to move it.
17. **Chart styles (B10):** publication, classic family colours, monochrome for print.

### E. Compare and report
18. **Compare (D2):** two to six materials side by side, with the differences highlighted.
19. **Radar charts (D3):** normalised properties of the candidates on one chart.
20. **Selection report (D4):** requirements, goal and index, stages, charts, the ranked shortlist and the datasheets, in one document to print or save as PDF.
21. **Results to a spreadsheet (D5):** the ranked results, with the index, "× best", the stages and every failure reason, as `.xlsx` and CSV.

### F. Data
22. **About 250 materials (A7):** specific grades added to every family:
    - steels, aluminium, titanium and nickel alloys;
    - engineering polymers and elastomers;
    - technical ceramics and glasses;
    - composites, woods and foams.

    Each comes with its designations (UNS, EN, ASTM, ISO) and a source for every value.
23. **Durability ratings:** fresh and salt water, acids, alkalis, UV, flammability. These are rated properties that work in limits.
24. **More values checked against published datasheets:**
    - the 14 metals still in draft;
    - then the polymers;
    - then as many of the new grades as can be found.
25. **Your own data (A8, A9):**
    - several databases (`.datdb`), opened, shared and switched;
    - import from a spreadsheet or CSV with a column-mapping step;
    - export a database the same way.

### G. The app
26. **One roadmap:** Help › Roadmap reads `docs/site.json` (embedded), so the app, the family site and MEMORY show the same plan. Versions are always three numbers.
27. **`?` help** for every new tool, and updated Getting Started and Glossary.
28. **Hardening-level tests for every new feature:** an engine test for each item above, and a UI test for each new view and tool.

### Not in 0.2.0 (the next big update and after)
- **Eco audit:** energy and CO₂ across a product's life (material, manufacture, transport, use, end of life), and greener substitutes. Optional in 0.2.0, see Q2.
- **Process selection:** process records, which processes can make the part, and cost against batch size. This needs a process database built from public sources first.
- 1,000 materials, the user guide, and a signed installer (1.0.0).

## 3. Proposed roadmap after this

| Version | Theme | Contents |
| --- | --- | --- |
| **0.2.0** | The big update | §2: sturdy, selection, indices and formulas, charts, compare and report, 250 materials, your own data |
| 0.3.0 | Eco and process | Eco audit, greener substitutes, process selection |
| 1.0.0 | Stable | ~1,000 materials, user guide, signed installer |

Fixes between big updates go out as small releases (0.2.1, 0.2.2…), as always through the updater.

## 4. Order of work

1. **A. Sturdy:** it protects your data and makes everything after it safer to build.
2. **Engine foundations:**
   - selections as lists (old files load unchanged);
   - the formula engine copied from Plan;
   - the index library;
   - spreadsheet input and output.
3. **B and C:** selection, indices and formulas.
4. **D and E:** charts, compare and report.
5. **F:** data, in batches. The 150 new materials are drafted with sources, then checked as far as published datasheets can be found.
6. **G:** the app; then the full checks, the real app under Xvfb with screenshots, CHANGELOG, MEMORY, `site.json`, README. Then release 0.2.0.

## 5. Size

About 6,000 to 7,000 new lines (engine ~2,500 with tests, window ~3,000, UI tests ~600), plus 150 materials of data. That is several times 0.1.1, so it takes several working sessions. Each stage is pushed as it is finished, so nothing is lost between sessions. The installer grows by the two spreadsheet crates, roughly 0.5 MB.

## 6. Questions

1. **The list:** is §2 the 0.2.0 big update? Anything to drop or add?
2. **Eco audit:** add it to 0.2.0 too (it uses the CO₂ and energy data Atlas already has, plus public transport and processing factors), or keep it for 0.3.0 with process selection?
3. **Checking data:** this environment can only reach GitHub, so values are checked from search results quoting the datasheets. Opening the environment's **Network access** (environment menu in the session title bar → Edit; https://code.claude.com/docs/en/cloud-environments#network-access) would let me open the datasheets themselves. Open it, or keep search-quoted checks, labelled as such?
4. **0.1.1 on Windows:** anything wrong with Copy Picture, printing or the update from 0.1.0? Fixes would go into 0.2.0.

## 7. Answers (2026-10-28)

1. **The list:** all of §2, nothing dropped. Build it.
2. **Eco audit:** not answered; it stays with process selection in 0.3.0, as proposed.
3. **Checking data:** network unchanged, so values are checked from search results quoting the datasheets, labelled as such.
4. **Data rule (the user, repeated and made strict for the 250 materials):** only open, public and free sources:
   - government handbooks and data (public domain);
   - open datasets;
   - producers' and suppliers' own datasheets that are free to read, cited value by value;
   - our own compilation of typical textbook ranges, labelled as ours.

   Never a proprietary or licensed database or an aggregator site, not even to check a value, and never named in public text. `db::check` refuses any source whose kind is not on the open list. CONTRIBUTING.md states the rule.
