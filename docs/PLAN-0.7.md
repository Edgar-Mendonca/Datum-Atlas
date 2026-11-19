# PLAN-0.7: X-ray diffraction analysis (brainstorm, stages, questions)

Status: **approved 2026-11-18: Order 1 (diffraction first).** 0.3.23 Diffraction 1 is being built.

## 1. Why

A materials researcher wrote to ask whether an open X-ray diffraction (XRD) analysis tool is feasible within Datum: import measured patterns, identify the crystalline phases against the **Crystallography Open Database (COD)**, and later go further (quantitative analysis, Rietveld refinement). Commercial XRD suites cost more than many students, researchers and small laboratories can pay.

It fits Datum well:
- **Open data.** The COD and all its data are in the public domain (CC0); it only asks that the original authors be credited. That meets our data rule. The licensed powder-diffraction reference database sold by a standards body is never used, not even to check.
- **Atlas has the start.** Crystal › Lattice already builds unit cells, works out d-spacings and Bragg angles, and draws an approximate powder pattern (structure factor with the scattering factor taken as the atomic number, Lorentz–polarisation, multiplicity). The roadmap had "0.4.4 X-ray: real peak heights and measured patterns" and "later: CIF and space groups".
- **The tools are in place.** A Rust engine for the numerics (tested, fast), the bundled Python for publication graphs, research work kept in the project (0.3.20), the ? help cards.

## 2. Brainstorm: what an XRD module could do

Grouped from the everyday to the advanced. Size: S (one release), M (one or two), L (several).

### A. Measured patterns (the everyday work)

| # | Idea | What it gives | Method | Size |
|---|---|---|---|---|
| A1 | **Open a pattern** | 2θ (or d, or Q) against counts, several patterns at once, kept in the project | Readers for text (.xy, .csv, .dat, .txt), the instrument XML format .xrdml (openly documented), the GSAS text format .fxye; other formats when a public description exists | S |
| A2 | **Process it** | Background removal, smoothing, Kα2 stripping, zero shift, normalising, cutting a range | Rolling-ball or polynomial background; Savitzky–Golay; Rachinger Kα2 stripping | S |
| A3 | **Find and fit peaks** | Peak list: 2θ, d, height, area, FWHM, integral breadth; export | Second-derivative peak search; least-squares fit of pseudo-Voigt or Pearson VII profiles (our own Levenberg–Marquardt in the engine) | M |
| A4 | **Crystallite size and strain** | Size from the Scherrer equation; size and microstrain from a Williamson–Hall plot; instrument broadening taken off using a standard | Textbook formulas; instrument profile from a standard sample's pattern (NIST silicon or LaB₆ certificates are public) | S |
| A5 | **Compare patterns** | Overlay and offset many patterns (a heat-treatment series, in-situ steps), a waterfall plot and a heat map | Matplotlib graphs in the Python engine; the window keeps zoom and hover | S |
| A6 | **Index an unknown cell** | Suggest a cubic, tetragonal or hexagonal cell from peak positions | Exhaustive search for high symmetry; lower symmetry later | M |

### B. Calculated patterns from structures

| # | Idea | What it gives | Method | Size |
|---|---|---|---|---|
| B1 | **Read CIF files** | Any structure from the COD or a paper: cell, space group, atoms, occupancy, displacement | A CIF reader (the IUCr's open standard) and the space-group operators | M |
| B2 | **Accurate intensities** | Peak heights right enough to compare with measurements | Angle-dependent atomic scattering factors from openly published coefficients (not the licensed International Tables books); anomalous dispersion from NIST's public tables; Debye–Waller factor; Lorentz–polarisation with a monochromator | M |
| B3 | **Simulated pattern** | A full pattern with chosen peak shapes, wavelength (Cu, Co, Mo, Cr, Fe, Ag; synchrotron) and Kα2, over the measured one | Profile functions from A3 | S |

### C. Phase identification against the COD

| # | Idea | What it gives | Method | Size |
|---|---|---|---|---|
| C1 | **A searchable reference set** | The inorganic COD entries (a few hundred thousand) turned into compact calculated peak lists | Our own build script: CIF → calculated pattern (B1, B2) → the strongest lines, elements, formula, cell, COD id; validated and de-duplicated | M |
| C2 | **Search–match** | Candidate phases ranked by how well their lines fit the measured peaks, with an element filter (must have, may have, must not have) | Figure-of-merit scoring on peak positions and heights; several phases in one pattern (after the first match, the rest is searched again) | M |
| C3 | **Look at a candidate** | Its sticks over the pattern, its formula, cell and the paper it comes from (credited, as the COD asks) | | S |
| C4 | **Your own references** | Add your own CIFs or peak lists to search with | | S |

### D. Quantitative analysis

| # | Idea | What it gives | Method | Size |
|---|---|---|---|---|
| D1 | **Weight fractions by reference intensity ratio** | Rough phase fractions after phase identification | RIR (I/Ic) worked out from the calculated patterns (B2) | S |
| D2 | **Amorphous content** | Crystalline against amorphous with an internal standard | Standard method | S |

### E. Rietveld refinement (the research tool)

| # | Idea | What it gives | Method | Size |
|---|---|---|---|---|
| E1 | **Le Bail / Pawley fit** | Cell parameters and peak shapes without a structure model | Whole-pattern fitting with free intensities | M |
| E2 | **Rietveld refinement** | Phase fractions, cell, peak shape, crystallite size and strain, atom positions, occupancies, displacement, preferred orientation | Our own least squares on the whole pattern (Levenberg–Marquardt with constraints), Thompson–Cox–Hastings pseudo-Voigt profile, March–Dollase orientation; the classic published methods, our own code | L |
| E3 | **Refinement aids** | Fit statistics (Rwp, Rexp, χ²), the difference curve, parameter correlations, a refinement order to follow, undo | | M |
| E4 | **Validation** | Results matching published round-robin samples and NIST certificates within their uncertainty | Test suite | M |

### F. Beyond (ideas for later)

- **Residual stress** by the sin²ψ method (needs tilted scans).
- **Texture** from X-ray pole-figure scans (already on the list), shown beside the EBSD pole figures.
- **Thin films**: grazing-incidence scans, layer thickness from reflectivity fringes.
- **In-situ series**: many patterns over temperature or time, with phase fractions against temperature, linked to the heat-treatment module.
- **Link to the rest of Atlas**: a phase identified → its material in the Library; EBSD phases → their simulated patterns; a lattice parameter → composition (Vegard's law).

## 3. Data and licences

| Need | Open source | Note |
|---|---|---|
| Reference structures | COD (CC0) | Credit the original authors for each entry shown. |
| Atomic scattering factors | Coefficients published openly in the literature, or calculated from public NIST data | The International Tables books are licensed: not used. |
| Anomalous dispersion f′, f″ | NIST's public X-ray form factor tables | US government data. |
| X-ray wavelengths | NIST's public X-ray transition energies | US government data. |
| Test data | NIST standard reference material certificates (silicon, LaB₆, alumina); published round robins; patterns from users who share them | |

The licensed powder-diffraction reference database is never used, and the app never says it copies or replaces any commercial program (family rule).

## 4. Where it lives

1. **A Diffraction module inside Atlas** (recommended to start): it shares the crystal engine, the elements, the graphs and the project file. Pages: *Patterns*, *Peaks*, *Phase ID*, later *Refine*.
2. **Its own Datum app** later, if it grows to full Rietveld and many users want only that. Under the family rules this needs a brief in Datum-Plan first; the engine would move to a shared crate.

## 5. How it fits the roadmap

The X-ray stages replace "0.4.4 X-ray" and "later: CIF". Two possible orders:

**Order 1: diffraction first** (a researcher is waiting for it):

| Release | Theme |
|---|---|
| 0.3.23 | XRD 1: open, process and compare measured patterns; peaks, Scherrer, Williamson–Hall (A1–A5) |
| 0.3.24 | XRD 2: CIF files and accurate calculated patterns (B1–B3) |
| 0.3.25 | XRD 3: phase identification against the COD (C1–C4), RIR fractions (D1) |
| 0.3.26 | Heat treatment and phase diagrams |
| 0.4.0–0.4.3 | Research depth (EBSD and DIC) as planned |
| 0.4.4 onwards | Elastic constants, fit and statistics, simulation export |
| 0.5.x | Rietveld: Le Bail, then Rietveld, then validation (E1–E4) |

**Order 2: keep the plan, add diffraction after heat treatment:** 0.3.23 heat treatment; 0.3.24–0.3.26 XRD 1–3; then 0.4.x; Rietveld 0.5.x.

**Data work alongside** (small, between features): open energy and CO₂ for the non-metals from the US life-cycle database (public domain), and re-referencing the draft values family by family.

## 6. Risks

- **Rietveld is the big one.** Robust refinement is months of work; it needs good starting values, constraints and careful testing. It comes last, after the everyday tools are solid.
- **COD size and quality.** Hundreds of thousands of entries, with duplicates and some errors. The reference set must be built, filtered and checked by a script, and shipped as a data pack (likely 50–150 MB) rather than inside the installer.
- **File formats.** Only text formats and openly documented ones; closed binary formats from instrument makers only when a public description exists.
- **Validation needs real data.** Measured patterns with known phases, from users or public certificates.

## 7. Decisions of 2026-11-18

- **Order 1**: diffraction first (0.3.23–0.3.25), heat treatment at 0.3.26, refinement as a 0.5 series.
- The other questions take the recommendations until the user says otherwise: a Diffraction module inside Atlas; text, .xrdml and .fxye files first; Rietveld last. Test patterns from the researcher, and the eco and re-referencing work between releases, wait for the user.

**2026-11-19:** the COD reference pack is **downloaded on first use** of Phase identification (a separate file on Atlas's own release page, checked against its checksum and kept on the computer; "Import pack file…" for computers without internet), and it holds the **inorganic entries only** to start; an organic pack can follow if users ask. The researcher has not shared test patterns: validation uses public data (NIST silicon and LaB₆ certificates, published examples) until real patterns come.

## 8. Questions for the user (asked 2026-11-18)

1. **Order:** diffraction first (Order 1) or heat treatment first (Order 2)?
2. **Where:** start as a Diffraction module inside Atlas (recommended), or plan a separate Datum app from the start?
3. **First release scope:** is XRD 1 (measured patterns, processing, peaks, size and strain) the right first step?
4. **The COD reference pack:** downloaded on first use of Phase ID (installer stays the same size), or included in the installer (bigger, works offline at once)? Inorganic entries only, or organics too?
5. **File formats:** which instruments do you and the researcher use? (Text exports always work; .xrdml is easy; others depend on a public description.)
6. **Rietveld:** keep it as the last stage (0.5.x), or not plan it yet?
7. **The researcher:** may we ask him for test patterns with known phases, and credit him (by name, if he agrees) in the release notes?
8. **Data work:** fit the non-metal eco data and the re-referencing between these releases, or after the diffraction series?
