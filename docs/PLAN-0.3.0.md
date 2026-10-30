# Datum Atlas 0.3.0: eco and process, one big update (plan)

Status: **questions open** (§6). Written 2026-10-30. The user: "write the 0.3.0 plan as one big update". Built like 0.2.0: stages, each pushed when its checks pass, one release at the end.

---

## 1. Where Atlas stands

| | Today (0.2.1) | Needed for 0.3.0 |
| --- | --- | --- |
| CO₂ and energy of primary production | 188 of 252 materials | All 252, plus recycled content where public data exists |
| Processing ratings (castability, machinability, weldability) | Defined, **no values** | Values for every metal and polymer |
| End of life | Recyclable yes/no only | Recycle, downcycle, combust, landfill, biodegrade; energy and CO₂ of recycling |
| Processes | None | A process database, from public sources |
| Life-cycle factors (transport, electricity, processing) | None | A small table of public factors, each with its source |

The same data rule as 0.2.0 holds for all of it (PLAN-0.2.0 §7.4): only open, public and free sources; never a proprietary or licensed database or an aggregator, not even to check a value; gaps stay gaps, never zero.

## 2. The big update

### A. Eco data (built first; everything else uses it)
1. **CO₂ and embodied energy for every material**, from open life-cycle datasets and government reports; the 64 gaps filled where a public source exists, left as gaps where none does.
2. **End of life per material:** recyclable, downcyclable, combustible (with heat of combustion), landfill, biodegradable; the energy and CO₂ of recycling and the typical recycled fraction in today's supply.
3. **Processing energy and CO₂** for the main primary processes (casting, rolling and forging, extrusion, moulding, machining per kg removed), as typical ranges with sources.
4. **Factors table** (`data/eco.json`): transport by sea, rail, road and air (MJ and kg CO₂ per tonne·km); electricity CO₂ by region; fuels. Each with its public source and year.
5. **Water use** where public data exists (gap otherwise).

### B. Eco audit
6. **Bill of materials:** parts with material, mass, process, and the recycled content used; picked from the database, or from the shortlist in one click.
7. **Life phases:** material, manufacture, transport (legs with distance and mode), use (energy over the life: electrical, or fuel for moving mass, with the mass-to-fuel factor shown), end of life (one choice per part).
8. **Results:** energy and CO₂ per phase and per part, as bar charts and a table; the phase that dominates is said in words, with what usually helps for it.
9. **Design options side by side:** two to four versions of the product (e.g. steel against aluminium against composite), compared phase by phase, with the break-even life where the lines cross.
10. **End-of-life credit** shown separately, never mixed in silently.
11. The audit is part of the `.datlas` project, has undo, and goes into the selection report and to a spreadsheet.

### C. Greener substitutes
12. **"Like this, but greener":** for a material, the materials that meet the same limits and index within a chosen tolerance, ranked by CO₂ per unit of function (the index, not per kg), with what you give up shown.
13. **Eco goals in the library:** minimum CO₂ and energy versions of the tie, beam, panel, column and shaft (some exist; the set completed), and the eco charts (CO₂ against modulus, strength, price).

### D. Process data
14. **A process database** (`data/processes.json`), about 40 processes from public sources (university course notes, government manufacturing guides, open textbooks, producers' own guides):
    - shaping: sand, die and investment casting; injection, blow and compression moulding; extrusion; forging; rolling; sheet forming; powder methods; composite lay-up and filament winding;
    - machining: turning, milling, drilling, grinding, EDM, laser and water-jet cutting;
    - joining: welding (arc, laser, friction), brazing, soldering, adhesives, fasteners;
    - additive: powder-bed fusion, extrusion (FDM), resin (SLA), binder jetting;
    - finishing: painting, anodising, plating, polishing.
15. **Each process record:** the materials it suits (by family and class); part mass and section thickness; shapes it can make (flat, prismatic, 3-D solid, hollow, sheet); tolerance and roughness; economic batch size; relative tooling and equipment cost, rate; energy and CO₂ per kg; notes and sources.
16. **Processing ratings for materials:** castability, machinability and weldability filled for the metals and polymers (already defined, empty today).

### E. Process selection
17. **Your part:** material (or the shortlist), mass, smallest section, shape, tolerance, roughness, batch size.
18. **Stages like material selection:** each requirement removes processes that cannot do it, and every failed process says why. Process charts: tolerance against roughness, mass range, section thickness, economic batch size, as bar charts per process.
19. **Material–process matrix:** which of the shortlisted materials each process can make.
20. **Cost model:** cost per part against batch size for each process that passes (material, tooling spread over the batch, equipment and labour per hour at its rate), drawn as lines on a log chart, with the cheapest process for each batch size. Every input is shown and can be changed; the default costs are labelled as indicative.
21. **Joining and finishing pickers:** which joins work for two materials, which finishes for one.

### F. The app
22. **New pages:** Eco audit, Processes (browse and datasheets, like materials), Process selection. Ribbon tabs and Home tiles; the dimmed "Planned" placeholders for E and F go live.
23. **Report:** new sections for the eco audit and the chosen process, each optional.
24. **? help** for every new tool, Getting Started step 5 (eco and process), Glossary terms (life phase, functional unit, break-even, economic batch size, tooling cost).
25. **Projects:** older `.datlas` files open unchanged; the new parts are optional.

### G. Sturdy and checked
26. The hardening tests cover the new parts: random audits and process requirements never panic; damaged eco and process data are refused by `db::check`; a speed gate on a large audit.
27. Unit tests: audit sums, transport and use phases, break-even, cost curves, process filtering, against hand-worked examples written into the tests.
28. UI tests for every new page; the real app under Xvfb with screenshots.

### Not in 0.3.0 (1.0.0 and after)
- About 1,000 materials, a user guide, a signed installer.
- Full life-cycle assessment (impact categories beyond energy, CO₂ and water): out of scope; Atlas does a design-stage audit, said plainly in the app.

## 3. Roadmap after this

| Version | Theme | Contents |
| --- | --- | --- |
| 0.3.0 | Eco and process | §2 above |
| 1.0.0 | Stable | ~1,000 materials, user guide, signed installer |

## 4. Order of work

1. **A:** eco data and factors, with sources; `db::check` rules for them.
2. **B and C:** eco audit engine (`crates/core/src/eco.rs`) with tests, then its page; greener substitutes and eco goals.
3. **D:** process data (`data/processes.json`, `crates/core/src/process.rs`), processing ratings.
4. **E:** process selection, matrix, cost model; their pages.
5. **F and G:** report, help, glossary, hardening and UI tests; the real app; CHANGELOG, MEMORY, `site.json`, README. Then release 0.3.0.

Each stage is pushed when clippy, the tests and the UI tests pass.

## 5. Size

About 5,000 to 6,000 new lines (engine ~2,200 with tests, window ~2,500, UI tests ~500), plus the eco data for 252 materials and about 40 process records. No new crates are expected, so the installer stays near 3 MB. Several working sessions, each pushed.

## 6. Questions

1. **The list:** is §2 the 0.3.0 big update? Anything to drop or add?
2. **Use phase:** keep it simple (electricity used, or mass moved with a fuel factor per kg·km), or add more use models (heating and cooling, standby)?
3. **Cost model:** indicative default costs (labelled as such) that you can change, or only your own numbers with blank defaults?
4. **Processes:** is about 40 the right number, and are the five groups (shaping, machining, joining, additive, finishing) right?
5. **Data checks:** the network is unchanged, so eco and process values come from what search results quote of public sources, labelled as such. Keep that, or open the environment's network access so the documents themselves can be opened?
6. **Windows:** anything wrong in 0.2.0 or 0.2.1 on a real PC (printing, Copy Picture, the updater, the new publisher name)? Fixes go first in 0.3.0.
