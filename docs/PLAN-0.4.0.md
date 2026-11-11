# Datum Atlas 0.4.0: assistant access, trusted data, family polish (plan)

Status: **questions open** (§6). Written 2026-11-11. The user: "can we add MCP for it, like in other applications? I think this should be a default in all our apps. So what more can we improve?" Built like 0.2.0 and 0.3.0: one big release in stages, each pushed when its checks pass. The 0.3.1 wording changes ship inside it.

---

## 1. Where Atlas stands

Done: property charts, selection, ranking, indices and formulas, compare and report, your own databases and import, the eco audit, greener substitutes, process selection and cost (0.1.0 to 0.3.0).

Not done or thin:
- **No way for an assistant or a script to use Atlas.** Datum Plan has `datum-cli` and an MCP server; Atlas has neither.
- **Data trust.** 252 generic materials; 73 values checked against published datasheets, the rest drafts. Eco and process values are drafts by class. No specific grades beyond a few, no search by equivalent designations.
- **Gaps in the method:** hybrid materials (sandwich panels, lattices, foams with skins) and restricted or critical substances are missing.
- **Family parity with Datum Plan:** no user guide, no colour-blind safe option, eco and process pages are SI only, no `.ods` export, no performance bench.
- **Smaller limits:** processes match materials by class, so a cast alloy is offered sheet forming; water use is not in the data.
- **Not tried on Windows:** printing, Copy Picture, the updater, the 0.2.x and 0.3.0 pages.

## 2. The big update

### A. Command line and MCP server (copied from Datum Plan's `crates/cli`)
1. **`datum-atlas-cli`**, a new crate (`crates/cli`) beside the engine, shipped in the release as `datum-atlas-cli-X.Y.Z.exe`:
   - `search TEXT`, `material ID` (datasheet with sources), `props`, `goals`, `processes`;
   - `results FILE [--csv]` (ranked with reasons), `info FILE`, `ops`, `apply FILE --op JSON --save OUT.datlas`;
   - `eco FILE`, `process FILE`, `export FILE OUT` (.datlas, .xlsx, .csv), `new NAME [TEMPLATE]`, `templates`;
   - `mcp`: the same as tools for an AI assistant (Model Context Protocol over stdio).
2. **An operations catalogue** (`ops.rs` in the engine): every edit the window sends (`Op`), each with a name, a sentence and an example, used by `ops`, by `list_operations` and by Tell Me.
3. **MCP tools** (about 16):
   - database, no project needed: `search_materials`, `get_material`, `list_properties`, `list_goals`, `list_processes`, `greener_substitutes`;
   - project: `new_project`, `open_project`, `project_info`, `list_operations`, `apply_operation`, `apply_goal`, `undo`, `get_results`, `get_chart`, `eco_audit`, `process_selection`, `save_project`.
4. **Safety, as in Plan:** changes only through the named operations; nothing written unless `save_project` is called; only `.datlas`, `.xlsx` and `.csv`; a panic in a tool is caught and changes nothing; tests drive a whole assistant session.
5. **The window:** Help › Use with an AI assistant: a card with the `mcp` command and a settings snippet to copy, with a ? card.

### B. Trust the data
6. **More values checked:** a pass over metals and polymers against producers' and government datasheets; target about 300 checked values (73 today), each with its link.
7. **About 500 materials:** specific grades where public datasheets exist (aluminium tempers, common steels and stainless grades, engineering polymers by type), referenced like today.
8. **Search by equivalent designations:** UNS, EN, JIS, ASTM and common names in one search, and the equivalents on the datasheet (from public standards' cross-reference lists).
9. **Processes match by alloy type:** cast and wrought alloys, thermoplastics and thermosets are told apart, so a cast alloy is not offered sheet forming.

### C. Hybrids and compliance
10. **Hybrid calculator:** a sandwich panel (two skins and a core) and a lattice of a chosen density, worked out from the database values (bending stiffness and mass per area, or modulus and density); the result is a record you can put on any chart beside the materials.
11. **Restricted and critical substances:** flags from public lists (substances limited by law in the EU and the US, critical raw materials lists), shown on datasheets and usable as limits.

### D. Family polish
12. **User guide** (English; Chinese if wanted, as Plan has), in the app and as a PDF in the release.
13. **Colour-blind safe colours** in Options (as in Plan).
14. **Imperial units on the eco and process pages.**
15. **`.ods` export** for results, the eco audit and your materials; Copy Picture on the eco and process charts.
16. **Performance bench** (`npm run bench`): 10,000 materials and a large audit, with a time limit in the tests.
17. Hardening for everything new; UI tests; the real app under Xvfb.

### Not in 0.4.0
- 1,000 materials with pictures and microstructures, and the signed installer (1.0.0).
- Properties against temperature and curves (stress–strain, fatigue): after 0.4.0.

## 3. Roadmap after this

| Version | Theme | Contents |
| --- | --- | --- |
| 0.4.0 | Assistants, trusted data, polish | §2 above (with the 0.3.1 wording) |
| 0.5.0 | Data at temperature | Properties against temperature, simple curves, more processes with process charts, export of material cards for simulation |
| 1.0.0 | Stable | About 1,000 materials with pictures, signed installer |

## 4. Order of work

1. **A** first (small, copied from Plan, and the user wants it in every app).
2. **D** polish (guide, colours, units, `.ods`).
3. **B** data, in batches by family; **C** hybrids and compliance.
4. Checks, real app, CHANGELOG, MEMORY, `site.json`, README; release 0.4.0.

## 5. Size

About 3,500 to 4,500 new lines (CLI and MCP ~600 with tests, ops catalogue ~200, hybrids and compliance ~800, polish ~1,000, UI tests ~400), plus about 250 materials and 230 checked values of data. Several sessions, each stage pushed.

## 6. Questions

1. **The list:** is §2 the 0.4.0 big update? Anything to drop, add or move to later?
2. **MCP:** the same rules as Datum Plan (nothing saved unless asked; only project and export files)? Should the assistant also be able to add your own materials (to “My materials”), or only read the database?
3. **Data:** 500 materials and about 300 checked values the right target? The network is unchanged, so checks use the figures search results quote, labelled as such; opening the environment's network access would let me open each datasheet itself.
4. **Guide:** English only, or English and Chinese like Datum Plan?
5. **Hybrids and compliance:** in 0.4.0, or later so the data work comes first?
6. **Windows:** anything wrong in 0.3.0 on a real PC (printing, Copy Picture, the updater, the eco and process pages)?
