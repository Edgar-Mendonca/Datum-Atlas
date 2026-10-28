<div align="center">

<img src="docs/brand/datum-atlas-icon.svg" width="88" alt="Datum Atlas">

# Datum Atlas

**Choose the right material: property charts, selection and referenced data, in one app for Windows.**

[![Latest release](https://img.shields.io/github/v/release/Edgar-Mendonca/Datum-Atlas?label=release&color=6C4AB6)](https://github.com/Edgar-Mendonca/Datum-Atlas/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/Edgar-Mendonca/Datum-Atlas/ci.yml?label=CI&logo=githubactions&logoColor=white)](https://github.com/Edgar-Mendonca/Datum-Atlas/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/github/downloads/Edgar-Mendonca/Datum-Atlas/total?color=6C4AB6)](https://github.com/Edgar-Mendonca/Datum-Atlas/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-6C4AB6)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D4?logo=windows&logoColor=white)](https://github.com/Edgar-Mendonca/Datum-Atlas/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-engine-B7410E?logo=rust&logoColor=white)](crates/core)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![SolidJS](https://img.shields.io/badge/SolidJS-UI-2C4F7C?logo=solid&logoColor=white)](ui/src)

[![Download for Windows](https://img.shields.io/badge/Download_for_Windows-6C4AB6?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/Edgar-Mendonca/Datum-Atlas/releases/latest)
[![What's new](https://img.shields.io/badge/What%27s_new-A58BEA?style=for-the-badge&logo=readthedocs&logoColor=white)](CHANGELOG.md)
[![Report an issue](https://img.shields.io/badge/Report_an_issue-C0392B?style=for-the-badge&logo=github&logoColor=white)](https://github.com/Edgar-Mendonca/Datum-Atlas/issues/new)
[![Memory and roadmap](https://img.shields.io/badge/Memory_%26_roadmap-0C1A2B?style=for-the-badge&logo=bookstack&logoColor=white)](docs/MEMORY.md)

</div>

<img src="docs/images/chart.png" alt="Datum Atlas: the design goal Light, stiff beam on Young's modulus against density, with its line E½/ρ and the materials ranked by it" width="100%">

## What it does

Datum Atlas is part of Datum; see its page: https://edgar-mendonca.github.io/Datum/apps/atlas

Datum Atlas helps engineers, designers and students pick a material and show why it is the right one:

1. **Pick a design goal** (light stiff beam, cheap strong tie, spring, heat sink…) or write down the requirements: what the part does, what it must meet, what to minimise.
2. **See every material on a property chart**: two properties on log axes; each material is a bubble spanning its ranges, each family has its envelope.
3. **Select**: limits (max service temperature ≥ 150 °C, price ≤ 5 USD/kg, recyclable), a **selection line** for a performance index (E^½/ρ for a light, stiff beam is a line of slope 2), a box, or whole families left out.
4. **Rank and check**: the materials that pass, ranked by the index, and the reason every other one fails.

## Features (0.1.1)

- **About 100 materials** from every family: metals, polymers, elastomers, ceramics and glasses, composites, foams and natural materials. **Every value has a reference**, compiled from public sources (U.S. government handbooks, NIST, USGS, the Wood Handbook, open datasets and typical textbook ranges). No proprietary database is used.
- **22 design goals** in a list beside the chart, grouped as minimum mass, minimum cost, minimum carbon, elastic design, damage tolerance and thermal. A goal sets the axes, draws the selection line of the right slope through the best materials, adds the limits it needs (marked, so you can change them) and ranks the results by its index. “Why?” explains the function, constraint, objective and free variable behind it.
- **Derived properties**: price, CO₂ and energy per volume, specific stiffness and strength, thermal diffusivity and thermal shock resistance, calculated from each material's own values.
- **SI or imperial units** everywhere: axes, tables, datasheets and limits. Values are stored in SI, so switching loses nothing.
- **Bar charts** of one property for every material, grouped by family.
- **Sources page**: every source, what kind it is, its licence, and which materials and values cite it. Metals are being checked against published datasheets value by value (51 values in 15 metals so far, marked ✓).
- **? help** beside the tools: a click says what the tool is for and how to use it.
- **Tell Me** (Ctrl+K) finds commands, design goals and materials (by name, UNS or EN number).
- **Your own materials**: the Add Material window explains what is needed (a name, the family, density, at least one more property, low and high values in the unit shown, and where the values come from). They appear in every view.
- **Property charts** of any two properties, log or linear; family envelopes; hover for ranges, click for the datasheet; zoom, pan, guide lines of slope 1, 2 and 3.
- **Selection**: limit, chart (line or box) and family stages; results ranked by the index with “× best”; failures explained.
- **Browse** with a family tree and a spreadsheet-like table (sort, select, copy to a spreadsheet), and **datasheets** with grouped properties and numbered references.
- Design problems to start from: light stiff beam, strong tie, stiff panel, springs, hot and light, damage tolerant.
- Print or save as PDF (charts, datasheets, results); export charts as SVG or PNG, or copy the chart as a picture.
- Light, dark and auto themes; the in-app updater; recovery of unsaved work.

<table><tr>
<td><img src="docs/images/browse.png" alt="Browse: family tree and materials table"></td>
<td><img src="docs/images/results.png" alt="Results ranked by E^½/ρ"></td>
</tr><tr>
<td><img src="docs/images/datasheet.png" alt="A datasheet with references"></td>
<td><img src="docs/images/add-material.png" alt="Add a material of your own"></td>
</tr><tr>
<td colspan="2"><img src="docs/images/sources.png" alt="Sources: every source with the materials and values that cite it"></td>
</tr></table>

## Coming next

Several selection lines and charts, results to a spreadsheet and more values checked against datasheets (0.1.2); the index builder, expression axes and formula properties (0.2.0); compare, radar charts, the selection report and 250 materials (0.3.0); your own databases with spreadsheet and CSV import (0.3.1); eco audit (0.4.0); process selection (0.5.0). See Help › Roadmap in the app, or [docs/MEMORY.md](docs/MEMORY.md).

## Install

Download `DatumAtlas-x.y.z-setup.exe` from the [latest release](https://github.com/Edgar-Mendonca/Datum-Atlas/releases/latest) and run it (per-user install, no administrator rights). Windows SmartScreen may warn on the first install because the installer is not signed yet. Updates install from File › Updates.

## Build

Rust (stable), Node 22, and on Linux the WebKitGTK development libraries.

```
npm ci
npm run build:ui
cargo test --workspace
npx tauri build --bundles nsis
```

The engine (`crates/core`) holds the materials data, units, chart geometry and selection, with unit tests; the window (`ui/src`, SolidJS) only shows what it computes. `npm run test:ui` runs the Playwright tests against the real engine.

## Data

Values are in `data/*.json`, one file per family, with their sources in `data/sources.json`. Each value is `[low, high, source]`. Built-in values are “draft” until each one is checked against a published source; checked values list the property in the material's `checked` list and cite that datasheet, and the tests refuse a checked value that cites only our own compilation; corrections with a reference are welcome as issues or pull requests.

## Licence

MIT. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
