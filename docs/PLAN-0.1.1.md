# Datum Atlas 0.1.1: plan and menu

Status: **waiting for the user's choices** (§6). Written after the user's review of 0.1.0 (3 screenshots, 2026-10-25). Nothing is built until the user picks.

This file follows the family pattern: feedback → diagnosis → proposal → menu of what can come next → size → questions.

---

## 1. Feedback on 0.1.0

1. **Design goal list** (screenshot 1, from the user's earlier attempt): one drop-down that picks the goal and sets up everything for it. The goals are grouped by what you minimise or maximise:
   - **Minimum mass:** light stiff tie E/ρ, light stiff beam E½/ρ, light stiff panel E⅓/ρ, light strong tie σy/ρ, light strong beam σy⅔/ρ, light strong panel σy½/ρ.
   - **Minimum cost:** cheap stiff beam E½/(Cₘρ), cheap strong tie σy/(Cₘρ).
   - **Minimum carbon:** low-carbon stiff beam E½/(CO₂·ρ).
   - **Elastic design:** spring σy²/E, elastic hinge σy/E.
   - **Damage tolerance:** yield before break KIc/σy, leak before break KIc²/σy.
   - **Thermal:** precision part with little thermal distortion λ/α.
   - "Explore (no goal)" at the top.
   The user remembers this from the established selection tool and wants it here.
2. **Home page tiles:** after Roadmap, add **Material Library** and **Sources**. The order should be: Material Library → Add a material → Sources → Roadmap. Sources is there to show plainly where every value comes from (open sources only, nothing proprietary). The user says to drop it if it doesn't fit; it fits.
3. **Print** (screenshot 3) works; no change was asked.
4. **Next:** update the memory, list what can be built in 0.1.1 and after, so the user can combine and choose.

## 2. Diagnosis

- **Goals.** 0.1.0 has six of these as *templates* on the File page. Each makes a **new selection**, and changing goal means starting again. There is no list inside the chart, and only 6 of the 16 goals exist. Four of the missing ones cannot be drawn yet:
  - cheap beam E½/(Cₘρ), cheap tie σy/(Cₘρ), low-carbon beam E½/(CO₂·ρ): these need **cost per volume** (Cₘ·ρ) and **CO₂ per volume** (CO₂·ρ) on an axis, and 0.1.0 has no derived axes;
  - the other goals only need the right two axes and slope: λ/α is slope 1 on λ–α; KIc²/σy is slope ½ on KIc–σy; σy⅔/ρ is slope 1.5 on σy–ρ.
- **Index names.** The engine writes "E^½/ρ" and "σy^(1/1.5)/ρ". The goal list writes E½/ρ and σy⅔/ρ, which reads better.
- **Home tiles.** The library (Browse) and the sources (Data page) exist but are only reachable from the sidebar. The File page shows design problems, Open, Add a material and Roadmap.

## 3. Proposal for 0.1.1

### 3.1 Design goal (the drop-down)
- **Where:**
  - a **Goal** drop-down at the left of the chart toolbar, and a big **Goal** button in Home and Select;
  - Setup gets the same list, with a short "why" text for each goal.
- **What it does** (engine table `goals.rs`, so the list and the maths live in one place):
  - sets the chart's axes and log scales;
  - places the selection line with the right slope through the middle of the materials ("above passes");
  - ranks the results by that index;
  - with "Explore", it removes the line.
  - Goals never touch your limits or families.
- **The 16 goals from screenshot 1, plus** a few more: stiff column E½/ρ (same line as the beam), light stiff shaft, thermal insulation (minimum λ), heat sink (maximum λ), cheap stiff panel E⅓/(Cₘρ) and low-carbon strong tie σy/(CO₂·ρ). Each has:
  - its group;
  - its index in proper notation (E½/ρ);
  - the function, the constraint, the objective and the free variable, in one line each;
  - the axes and the slope.
- **Derived properties** needed by the cost and carbon goals, computed by the engine from the ranges (product of lows, product of highs) and shown like any property, marked "derived":
  - cost per volume Cₘ·ρ (USD/m³);
  - CO₂ per volume (kg/m³);
  - embodied energy per volume (MJ/m³);
  - specific stiffness E/ρ;
  - specific strength σy/ρ;
  - thermal diffusivity λ/(ρcₚ).
  This is the built-in core of A6, without formulas yet (formulas stay in 0.2).
- **Index names in proper notation everywhere:** E½/ρ, E⅓/ρ, σy⅔/ρ, σy²/E, KIc²/σy.
- Templates on the File page become goals too: picking a tile makes a new selection with that goal.

### 3.2 Home page
- Tiles in this order after the design goals: **Material Library** (opens Browse) → **Add a material** → **Sources** → **Roadmap** (→ Open stays at the end).
- **Sources** page (its own sidebar item, taken out of Data). It lists every source with:
  - its kind, licence and link;
  - how many values use it;
  - a click shows the materials and properties that cite it.
  The top states the rule: open and public sources only, our own compilation, no proprietary database. It also gives the "draft" status of the built-in values.

### 3.3 Planned 0.1.1 items (from the 0.1.0 roadmap)
- **A4 Units:** SI and imperial for every value, chart, limit and datasheet (the engine conversions exist and are tested).
- **A5 Search everywhere:** Tell Me also finds materials (name, UNS/EN/ASTM, synonym, use); Enter opens the datasheet; the found material blinks on the chart.
- **B6 Bar charts:** one property for every material, sorted, coloured by family; the selection greys failures.
- **B11 Copy Picture:** chart to the Windows clipboard (through Rust) for Word and PowerPoint.

### 3.4 Fixes noticed in review
- Envelope labels collide where families meet (Composites over Ceramics). Proposal: nudge the labels apart (a small engine step), ahead of full label placement (B9).
- The index label on the chart can hide a family label: move it along the line to a free spot.

## 4. Menu: everything that can come next

Pick, move or combine freely. IDs match Help › Roadmap. "New" means not in the catalogue yet.

### A. Materials data
| ID | Item | Now planned in |
| --- | --- | --- |
| A4 | Units SI / imperial | 0.1.1 |
| A5 | Search everywhere | 0.1.1 |
| A6 | Derived properties: the built-in set (§3.1) in 0.1.1; your own formulas (Excel syntax) later | 0.1.1 / 0.2 |
| A12 new | **Check the built-in values** one by one against their sources, with table/page references; status "checked" | 0.3 (could start now, a family per release) |
| A7 | 250 materials: specific grades (steels, Al, Ti, Ni alloys, engineering polymers) with designations | 0.3 |
| A13 new | Durability ratings (water, acids, alkalis, UV, flammability), processability, food contact, RoHS/REACH flags | 0.3 |
| A8 | Import / export Excel and CSV with a mapping step | 0.3.1 |
| A9 | Several databases (open, share, switch `.datdb`) | 0.3.1 |
| A10 | Pictures and microstructures on datasheets | 0.3.1 |
| A11 | 1,000 materials | 1.0 |

### B. Charts
| ID | Item | Now planned in |
| --- | --- | --- |
| B12 new | **Design goal drop-down** (§3.1) | 0.1.1 |
| B6 | Bar charts | 0.1.1 |
| B11 | Copy Picture | 0.1.1 |
| B13 new | Two or four charts side by side, one selection shared across them | 0.1.2 |
| B14 new | Label the best materials automatically (top 5 by the index) | 0.1.2 |
| B8 | Expression axes ([Young's modulus]^0.5/[Density]) | 0.2 |
| B9 | Labels that never overlap, with leader lines | 0.3.1 |
| B10 | Styles: publication, classic, monochrome; dark charts | 0.3.1 |

### C. Selection
| ID | Item | Now planned in |
| --- | --- | --- |
| C9 new | Several lines on one chart (e.g. stiffness and strength together) | 0.1.2 |
| C10 new | Move the line with the arrow keys; type the index value it should pass through | 0.1.2 |
| C6 | Index builder: function → objective → constraints → free variable, with the derivation | 0.2 |
| C7 | Library of ~40 indices with explanations (the goal list grows into it) | 0.2 |
| C11 new | Ranking by any property or a weighted mix (e.g. 60 % mass, 40 % cost) | 0.2 |
| C8 | Polygon stage | 0.3.1 |

### D. Compare, report, share
| ID | Item | Now planned in |
| --- | --- | --- |
| D5 new | Export the results table to Excel/CSV | 0.1.2 |
| D2 / D3 | Compare 2–6 materials, radar chart | 0.3 |
| D4 | Selection report (requirements, stages, charts, shortlist, datasheets) to print or PDF | 0.3 |
| D6 new | Word report | 0.3 |

### E–F. Eco and process (the user wants both before 1.0)
| ID | Item | Now planned in |
| --- | --- | --- |
| E1 / E2 | Eco audit: material, manufacture, transport, use, end of life; bar chart per phase | 0.4 |
| E3 | Greener or cheaper substitutes for a material | 0.4 |
| F1–F3 | Process records, process selection, cost per part against batch size | 0.5 |

### G. Learning and the app
| ID | Item | Now planned in |
| --- | --- | --- |
| G4 new | **Teaching mode:** worked examples (beam, panel, spring, pressure vessel, heat sink), each with the derivation and the expected answer | 0.2 |
| G5 new | Exercises for students: a goal and limits, then "check my answer" | 0.3 |
| G2 | User guide | 1.0 |
| G3 | Signed installer (no SmartScreen warning) | 1.0 |

## 5. Size of the proposed 0.1.1

- Engine: `goals.rs` (~22 goals, one table) and derived properties in `db.rs`, plus index notation and label nudging. About 250 lines, with tests:
  - every goal draws its line;
  - its ranking matches the index computed by hand for three materials;
  - derived ranges are the products of the ranges;
  - imperial round trips through the window values.
- Window: goal drop-down and Setup list, Home tiles, Sources page, units switch, search in Tell Me, bar chart, Copy Picture. About 500 lines.
- UI tests for each; screenshots in the real app; release as 0.1.1.

## 6. Questions

1. **0.1.1 contents:** build §3 as proposed (goal list with derived axes, Home tiles and Sources page, units, search, bar charts, Copy Picture, label fixes)? Or move anything out or in from the menu in §4?
2. **Goal list:** the 16 from your screenshot plus the 6 extra in §3.1, or only your 16?
3. **Goal and limits:** should a goal also suggest limits (e.g. the heat sink asks for a minimum conductivity) or only set the chart and the line, as proposed?
4. **Picking a goal on an open selection:** change the open chart (proposed) or add a new chart for it?
5. **Data checking (A12):** start now, one family per release (metals first), or keep it for 0.3?
6. **Students:** is the teaching mode (G4, worked examples) worth bringing forward to 0.2?
