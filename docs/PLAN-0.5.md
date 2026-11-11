# Datum Atlas as a materials workbench (plan)

Status: **brainstorm, questions open** (§9). Written 2026-11-11. Nothing here is built until the answers are in.

The user: "Atlas can be a big material analysis software as well. The material selection can just be one module. … We take their code as reference and build our own from scratch, as per our requirement. … So let's brainstorm a lot, then we can start working on the update." The user studied materials science at the University of Manchester and used DefDAP and DAMASK there.

---

## 1. The idea in one paragraph

Datum Atlas grows from a materials selector into a **materials workbench**: one window, one project file (`.datlas`), several modules that share one materials library. Selection, the eco audit and process selection stay as they are, as modules. New modules analyse real materials: micrographs (grain size, phases, porosity), EBSD maps (orientations, grains, texture), DIC strain maps, mechanical test data (tensile, hardness, fatigue), and synthetic microstructures you can generate and send to a mesh. Results from the analysis modules can become materials in the library, so what you measure is what you then select with.

## 2. What Atlas has today (0.3.4), as modules

| Module | What it holds today | Engine | Window |
| --- | --- | --- | --- |
| **Library** | 252 referenced materials, datasheets with sources and ✓, families tree, your own materials, other databases, spreadsheet/CSV import and export, formula properties, schematic microstructures | `db.rs`, `model.rs`, `sheet.rs`, `formula.rs`, `micro.rs` | Browse, Datasheet, Data, Sources |
| **Select** | Property charts, 25 design goals, 52 indices and the builder, limits, lines, bands, boxes, shapes, weighted ranking, results with reasons, compare and radar, the selection report | `chart.rs`, `geometry.rs`, `goals.rs`, `select.rs`, `compare.rs`, `export.rs` | Setup, Charts, Stages, Results, Compare, Index, Report |
| **Eco** | Eco audit (five phases, options, break-even), greener substitutes, eco data and factors | `eco.rs` | Eco |
| **Make** | 55 processes, process selection with reasons, cost against batch size, joins and finishes | `process.rs` | Process, Processes |
| **Shared** | Units, undo, `.datlas`, named operations, command line, assistant connection (MCP), guide, ? help, updater | `doc.rs`, `ops.rs`, `units.rs`, `crates/cli` | File, Help, Tell Me |

So the restructure is mostly **naming and navigation**, not new code: the sidebar and ribbon group by module, Home offers a tile per module, and the project file already holds each module's part (`charts`/`stages`, `eco`, `process`).

## 3. How modules look in the window

- **A module switcher** at the top of the sidebar: Library · Select · Eco · Make · Microstructure · EBSD · DIC · Testing · Crystal. The sidebar below it shows only that module's pages; the ribbon shows that module's tabs.
- **Home** shows a tile per module, and the design problems stay under Select.
- **One project file** holds every module's work. Large measured data (EBSD maps, image stacks, DIC fields) stay in their own files: the project stores the path, a checksum and the settings, and the results are recomputed or cached beside it.
- **The library is shared:** any module can add a measured material ("Add to my materials" from a tensile test, or a grain size into a datasheet note), with the test as its source.
- Every new tool keeps the family rules: a ? card beside it, undo, units, print, export, the command line and the assistant connection.

## 4. New modules: what each would do

### 4.1 Microstructure (image analysis of micrographs)
For optical and SEM images: the work most engineers do every week.
- Open PNG, JPEG, TIFF (16-bit too); read the scale from the file where it is stored, or **calibrate** by drawing on the scale bar.
- **Grain size:** line-intercept and planimetric methods, the grain size number and mean intercept length, with the lines drawn on the image; grains from boundaries (threshold, skeleton, watershed).
- **Phase fraction** by threshold or colour (point count and area), with the histogram.
- **Porosity and particles:** count, size distribution, shape (aspect ratio, circularity), nearest-neighbour spacing.
- **Cast iron graphite:** nodularity and nodule count.
- **Layers and coatings:** thickness along a line.
- Results table, plots (size distributions), an annotated image, a report page.

### 4.2 EBSD (orientation maps)
Rebuilt from published methods, with DefDAP and other open tools read as references (§6).
- Read EBSD text exports (`.ang`, `.ctf`); HDF5 later (§9 Q5).
- Orientation maths: quaternions, Euler angles, crystal symmetry (cubic, hexagonal), misorientation and disorientation.
- **Maps:** IPF colouring (with the colour key), band contrast or image quality, phase map, Euler map.
- **Grains:** reconstruction by misorientation threshold, grain size and shape, cleaning (small grains, unindexed points).
- **Boundaries:** low and high angle, twin (Σ3) and other CSL boundaries.
- **Local misorientation:** KAM, GROD, and a GND density estimate.
- **Texture:** pole figures and inverse pole figures; ODF sections later.
- **Schmid factors** for a loading direction, by slip system.

### 4.3 DIC (strain maps)
- Read displacement fields (x, y, u, v) from text or CSV exports.
- **Strain:** εxx, εyy, εxy, effective (von Mises) strain, maximum shear; smoothing and gradient windows.
- Maps, histograms, line profiles, statistics by region.
- **EBSD–DIC linking** (the core of DefDAP's purpose): pick matching points, transform one map onto the other, then strain by grain, by boundary distance, and **slip trace analysis** (which slip systems match the bands seen in the strain map).

### 4.4 Testing (mechanical test data)
A quick, high-value module.
- **Tensile:** import load–extension or stress–strain (CSV, spreadsheet); Young's modulus by fitted range, 0.2 % proof stress, UTS, uniform and total elongation, the hardening exponent n and strength coefficient K (Hollomon), true stress–strain; several specimens with mean and spread.
- **Hardness:** sets of indents with statistics; conversions between scales where published conversion formulas apply, labelled as approximate.
- **Fatigue:** S–N data fitted by Basquin, endurance estimate; **creep:** Larson–Miller parameter.
- **"Add to my materials":** the results become a material in the library, with the test as its source, ready for Select.

### 4.5 Synthetic microstructures (generator)
Built from scratch with MicroStructPy's approach as reference (§6).
- 2D grains with a chosen size distribution (lognormal), aspect ratio and alignment (rolled metals), two or more phases with fractions, inclusions and pores.
- Orientations assigned at random or from a texture; an EBSD-style IPF view.
- **Export:** image, SVG, and a mesh or grain geometry for finite elements (to Datum Mesh in the Datum family, and as plain text for others).
- The same engine draws the datasheets' schematics, so these improve too.

### 4.6 Crystal (elements and crystals)
- The 118 elements and a clickable periodic table (the planned 0.3.5).
- Crystal structures (FCC, BCC, HCP and common compounds), lattice parameters, slip systems.
- Calculators: Miller indices and directions, interplanar spacing, Bragg angles (X-ray peak positions for a lattice), stereographic projection.
- Later: binary phase diagrams for common systems with the lever rule, from published data.

## 5. Open data (Zenodo and others)

- **Example datasets** for every analysis module, so a new user can try each tool at once: open EBSD and DIC datasets (many are on Zenodo under CC BY from university groups, including Manchester's), open micrographs (public domain from government labs, CC BY from papers).
- **Downloaded on demand**, not in the installer (like the guide): from the dataset's DOI, checked, cached, and cited on screen and in reports.
- **Real micrographs for the library:** where an open dataset holds a material Atlas has, its datasheet shows the real image beside the schematic, with the citation.
- Only open licences that allow reuse by anyone (public domain, CC0, CC BY). Not non-commercial-only licences, because Atlas is MIT and anyone may reuse it.

## 6. Building from scratch, with open tools as references

The user's direction: read their code as reference, write our own, to our needs. How that stays clean:
- **Permissive licences (DefDAP: Apache 2.0; MicroStructPy: MIT):** we may read the code to understand methods, and write our own in Rust. Where our code follows theirs closely, we credit them in NOTICE as their licences ask.
- **Copyleft licences (DAMASK: GPL/AGPL; other GPL orientation and texture toolboxes):** we use their papers and manuals for the methods, never their code, so that Atlas stays MIT.
- **Methods from published papers and standards' public descriptions** (intercept counting, Mackenzie distributions, KAM, GND estimates, slip trace analysis), each cited in the help card and the guide.
- **No Python inside the app:** everything in Rust (`crates/…`), tested against known answers (synthetic images with known grain size, rotations with known misorientation, analytic strain fields).

## 7. Engine layout

- `crates/core`: stays the library, selection, eco and make engine.
- New crates, each pure Rust and unit-tested:
  - `crates/image`: image loading, calibration, thresholds, segmentation, measurements;
  - `crates/orient`: quaternions, symmetry, EBSD readers, grains, boundaries, texture;
  - `crates/strain`: DIC readers, strain fields, linking to EBSD;
  - `crates/testing`: tensile, hardness, fatigue, creep;
  - `crates/synth`: the generator, which `micro.rs` would then use.
- The shell (`src-tauri`) and the command line call them; the MCP server gains tools per module.
- Image and HDF5 reading need new crates. Pure-Rust ones first; licences checked with `cargo deny` as now.

## 8. A possible order (to agree, §9)

| Step | Release | Contents |
| --- | --- | --- |
| 1 | 0.4.0 Modules | The module switcher, sidebar and ribbon by module, Home tiles; Library, Select, Eco and Make as modules; no new analysis yet |
| 2 | 0.4.x Crystal | Elements and periodic table (the old 0.3.5), crystal structures, calculators |
| 3 | 0.4.x Testing | Tensile, hardness, fatigue and creep; "Add to my materials" |
| 4 | 0.5.x Microstructure | Image calibration, grain size, phase fraction, porosity, particles, nodularity |
| 5 | 0.6.x Synthetic microstructures | The generator, IPF view, export to a mesh |
| 6 | 0.7.x EBSD | Readers, orientation maths, maps, grains, boundaries, KAM/GND, pole figures, Schmid factors |
| 7 | 0.8.x DIC | Strain maps, statistics, EBSD–DIC linking, slip traces |
| 8 | alongside | The old 0.3.6–0.3.12 steps (display and export, processes by alloy type, metal and polymer grades and checks, equivalents, hybrids, substances) slotted between modules, and example datasets from open sources |

Small numbered releases throughout, as now; each a full release with tests, guide chapter, site and MEMORY.

## 9. Questions

1. **The modules:** are these the right ones (Library, Select, Eco, Make, Microstructure, EBSD, DIC, Testing, Synthetic, Crystal)? Anything to add (corrosion, phase diagrams, heat treatment) or drop?
2. **The order:** start with the module restructure (0.4.0), then which first: Testing (quickest to useful), Microstructure (most engineers), or EBSD and DIC (closest to your Manchester work)?
3. **The remaining 0.3.x steps** (elements, display and export, processes by alloy type, grades and checks, equivalents, hybrids, substances): finish them before 0.4.0, or slot them between modules?
4. **Who is it for first:** industry engineers (micrographs, tensile tests, hardness) or researchers (EBSD, DIC, crystal plasticity)? It decides the defaults and the examples.
5. **Files to read:** which EBSD and DIC exports should come first? `.ang` and `.ctf` text files are straightforward; HDF5-based formats need a reader (pure-Rust readers are limited; a native library adds weight). Images: PNG, JPEG, TIFF; do you need any microscope's own image format?
6. **Code references:** confirm the approach in §6 (permissive code read as reference with credit; copyleft only through papers), so Atlas stays MIT.
7. **Example data:** download on demand from Zenodo and similar (small installer), or bundle a few small samples in the installer (works offline)?
8. **The family:** this widens Atlas beyond the brief in the hub (`docs/briefs/DATUM-ATLAS.md` in Datum Plan). Should the brief and the family site be updated to "materials workbench", and should Atlas export meshes to Datum Mesh?
9. **2D only at first?** 3D (serial sections, 3D EBSD, 3D synthetic microstructures) is much larger; I suggest 2D first.
