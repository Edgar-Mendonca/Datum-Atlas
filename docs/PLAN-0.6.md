# Datum Atlas: research depth (brainstorm, after 0.3.18)

Status: **roadmap agreed in outline** (§7); questions 2–6 of §6 still open. Python engine: decided (§7). Written 2026-11-14 after 0.3.18. The user asked for a strong brainstorm of what Atlas can still gain, from the open projects and course material they used at university. They were reviewed for **what they do and which published methods they use**; Atlas writes its own code from those methods and papers (decision of 2026-11-13). Nothing in this plan copies code or data files.

What Atlas has now: Library, Select, Eco, Make, Microstructure (structures, image analysis, generator with texture), Crystal (elements, lattice calculators, approximate X-ray), Testing (tensile, hardness, fatigue, creep), EBSD (maps, grains, boundaries, twins, KAM, Schmid, pole figures) and DIC (strain maps, strain by grain, slip traces). Corrosion (0.3.19) and heat treatment with phase diagrams (0.3.20) are next on the roadmap.

---

## 1. EBSD and DIC to research grade

The biggest gains for anyone doing EBSD with high-resolution DIC.

| # | Idea | What it gives | Method (to implement from) | Size |
|---|---|---|---|---|
| 1 | **Point-pair registration** of DIC onto EBSD | Click matching points on both maps (or on the speckle image); fit an affine transform by least squares, optionally polynomial or piecewise affine; warp grain ids onto the DIC grid | Least-squares affine / polynomial fitting; nearest-neighbour warping | M |
| 2 | **Grain inspector** | Step grain by grain; draw lines along visible slip bands; each group snapped to the nearest predicted trace with its angular deviation; a table of grain, orientation, Schmid factor, plane, deviation | Trace geometry (have) + angle grouping | M |
| 3 | **Active slip system by RDR** | Which slip *direction* is active, which the trace angle alone cannot tell: the measured ratio of displacement jumps across a band compared with each system's predicted ratio | Relative displacement ratio: Chen & Daly, Exp. Mech. 57 (2017) | M |
| 4 | **Automatic slip-band detection** | Band angles in each grain found from its strain map, without drawing | Radon transform of the grain's strain; peaks above a threshold, minimum separation, rays crossing ≥ 2/3 of the grain | M |
| 5 | **DIC cleaning** | A correlation-value map; masks from thresholds (with dilation); crop; finite (Green–Lagrange) strain beside small strain; strain over a chosen gauge length | E = ½(FᵀF − I); coarsened gradients | S |
| 6 | **Several strain steps** | One experiment with many DIC increments sharing one frame and one registration; each grain's strain tracked across increments | Shared transform; per-increment fields | M |
| 7 | **GND density and GROD** | Geometrically necessary dislocation density from lattice curvature (Nye tensor); each point's misorientation to its grain mean (angle and axis maps) | Pantleon, Scripta Mater. 58 (2008); Ruggles & Fullwood, Ultramicroscopy 133 (2013) | M |
| 8 | **Distance to boundary and neighbours** | A map of each point's distance to the nearest boundary; strain against that distance; the grain neighbour network | Distance transform; adjacency graph | S |
| 9 | **More slip systems and boundary labels** | Pyramidal ⟨c+a⟩ for hexagonal metals; editable slip-system lists; boundaries coloured by misorientation; CSL labels Σ3, Σ5, Σ7, Σ9… with Brandon's limit | Coincidence site lattice (CSL) tables from the literature | S |
| 10 | **Maps and filters** | Euler-angle colour map; an orientation-preserving smoothing filter (Kuwahara); IPF colour of each grain by its active system | Kuwahara filter on misorientation | S |
| 11 | **More EBSD and DIC files** (last, as agreed: hard formats last) | EBSD makers' binary and HDF5 exports; DIC software text exports with their own column layouts; writing `.ctf` back | File layouts from the formats' own published descriptions | L |

## 2. Crystal

| # | Idea | What it gives | Method / data | Size |
|---|---|---|---|---|
| 12 | **Real X-ray intensities** | Peak heights that match measured patterns: tabulated atomic scattering factors with their fall-off, the Lorentz–polarisation factor, multiplicity and a Debye–Waller temperature factor | Cromer–Mann (International Tables Vol. C) or Waasmaier & Kirfel, Acta Cryst. A51 (1995) 416; De Graef & McHenry, *Structure of Materials* ch. 11–12 | S |
| 13 | **Open a measured pattern** | Overlay a measured powder pattern (`.xy`, `.csv`, `.uxd`) on the calculated one; label peaks; fit the lattice parameter from measured peaks | Least squares on 1/d² | S |
| 14 | **Crystal files (CIF) and space groups** | Load any published structure: space group, Wyckoff sites, occupancies; its pattern, density and projections | CIF syntax; symmetry expansion from the space-group operators (International Tables Vol. A); a permissive Rust symmetry library could be used as a dependency | L |
| 15 | **Elastic constants of single crystals** | From Cij (cubic, hexagonal…): Voigt, Reuss and Hill moduli, Poisson's ratio, the anisotropy index, a polar or 3D plot of Young's modulus by direction, sound speeds, Debye temperature; per grain on an EBSD map (the stiffness along the load) | Nye, *Physical Properties of Crystals*; Hill (1952); Ranganathan & Ostoja-Starzewski (2008) for the anisotropy index; published single-crystal Cij | M |
| 16 | **Neutron and electron diffraction** | Neutron powder patterns (scattering lengths can be negative); electron spot patterns for a zone axis with the relativistic wavelength and camera length | Sears, Neutron News 3 (1992) 26; Peng et al., Acta Cryst. A52 (1996) 257 | M |
| 17 | **CSL / Σ calculator** | Σ, axis and angle, and coincidence planes for cubic and hexagonal crystals; feeds the boundary labels of #9 | CSL theory (Ranganathan 1966; Brandon 1966) | S |
| 18 | **Wulff shape** | The equilibrium crystal shape from surface energies of (hkl) families, with facet fractions | Wulff construction | M |
| 19 | **Interatomic potentials (teaching)** | Lennard-Jones and Morse curves: equilibrium spacing, binding energy, modulus from the curvature, checked against a force–stretch table | V″(r₀)/r₀ | S |

## 3. Corrosion (0.3.19) and heat treatment (0.3.20)

| # | Idea | What it gives | Method / data | Size |
|---|---|---|---|---|
| 20 | **Pourbaix (E–pH) diagrams** | Where a metal is immune, passive or corrodes, against potential and pH, for a chosen ion concentration and temperature; alloys later | Nernst (0.0591 V per pH at 25 °C) with published standard Gibbs energies of formation of ions and oxides (Pourbaix's *Atlas*, NBS tables); convex-hull method of Persson et al., PRB 85 (2012) 235438 | M |
| 21 | **Corrosion kinetics** | Weight gain or loss against time fitted to linear, parabolic or logarithmic laws, with rate constants and an Arrhenius plot over temperature | Curve fitting (#24) | S |
| 22 | **Phase diagrams for heat treatment** | Binary T–x diagrams with the lever rule. These need a Gibbs-energy (CALPHAD) engine, which none of the reviewed projects has. Options: (a) our own solver for simple binaries from published assessments; (b) the bundled-Python option the user allowed, with a permissive thermodynamics library and openly licensed databases | Published thermodynamic assessments; a Python candidate is allowed (decision §10 of PLAN-0.5) | L |
| 23 | **Supply risk** | A supply-risk index for each material from the concentration of world production (Herfindahl–Hirschman), as a column and a limit in selection | Gaultois et al., Chem. Mater. 25 (2013); USGS production data (public) | S |

## 4. Data analysis tools (from the course material)

| # | Idea | What it gives | Size |
|---|---|---|---|
| 24 | **Fit any table** | Load a table; fit linear, polynomial, power, exponential, Arrhenius or a formula of your own; parameters with their standard errors, R², residuals, confidence bands | M |
| 25 | **Statistics and tests** | Mean, standard deviation, standard error, confidence intervals; t-tests (paired and unpaired) to compare two samples, e.g. pore or particle sizes before and after strain | S |
| 26 | **Pole figures from X-ray texture scans** | Measured pole figures from X-ray texture text files, beside the EBSD ones | M |

## 5. A bridge to crystal-plasticity simulation

Atlas will not solve crystal plasticity itself. It can prepare and read the files of a common open simulation kit, without using its code:

| # | Idea | What it gives | Size |
|---|---|---|---|
| 27 | **Export a simulation model** | From a generated or EBSD grain map: the grid as a VTK image file (`.vti`), a material file (YAML: phases, elastic constants, one orientation per grain as a quaternion) and a load-case template (uniaxial tension) | M |
| 28 | **Tensor toolkit** | From deformation gradient F and stress P: Cauchy stress, Green–Lagrange and logarithmic strain, Mises equivalents, eigenvalues, spherical and deviatoric parts. Shared by DIC and imported results | S |
| 29 | **Import results and compare with DIC** | Read the simulation's HDF5 results; take the surface; compare simulated and measured strain grain by grain, with slip traces (needs an HDF5 library on Windows: a hard format, so last) | L |

Papers for the method: Roters et al., Comput. Mater. Sci. 158 (2019) 420; Roters et al., Acta Mater. 58 (2010) 1152; Eisenlohr et al., Int. J. Plast. 46 (2013) 37.

---

## 6. Suggested order and questions

A proposal. Every step is one small release as before, each checked and published.

1. **0.3.19 Corrosion** with Pourbaix diagrams (#20) and kinetics (#21), plus the galvanic series and allowance already planned.
2. **0.3.20 Heat treatment** with the decision on #22.
3. **0.4.x "Research depth"**, in order of value:
   - 0.4.0: DIC cleaning and finite strain (#5), tensor toolkit (#28), point-pair registration (#1)
   - 0.4.1: grain inspector (#2) and RDR (#3)
   - 0.4.2: automatic slip bands (#4), several strain steps (#6)
   - 0.4.3: GND and GROD (#7), distance to boundary (#8), more slip systems and CSL labels (#9, #17)
   - 0.4.4: real X-ray intensities and measured patterns (#12, #13)
   - 0.4.5: elastic constants (#15)
   - 0.4.6: fit any table and statistics (#24, #25), supply risk (#23)
   - 0.4.7: simulation export (#27)
   - later: CIF and space groups (#14), neutron and electron diffraction (#16), Wulff (#18), X-ray texture (#26), simulation import (#29), more file formats (#11)
4. The items left from PLAN-0.4.0 (colour-blind colours, `.ods`, grades and checks, equivalents, hybrids, substances) fit between these where they are easiest.

**Questions**
1. **Order:** is research depth (EBSD and DIC first) the right priority after 0.3.20, before the data items of PLAN-0.4.0?
2. **Phase diagrams (#22):** our own simple binary solver, or the bundled-Python option with a thermodynamics library?
3. **Simulation bridge (#27–29):** wanted? Exporting is cheap; importing results needs an HDF5 library in the installer.
4. **Your data:** can your own EBSD and HR-DIC data (from your university work) be used as test cases, privately or as examples?
5. **CIF (#14):** worth the size now, or later?
6. Anything here you do not want, or anything missing that you used often?

---

## 7. Decisions of 2026-11-15 and the roadmap

**Python engine: bundled, as in Datum Plot and Datum Sim.** The user chose this after the family apps were checked:
- The release workflow takes the official embeddable CPython 3.13 for Windows, installs pinned libraries into it (`py/requirements.txt`), tests it, and zips it as the runtime pack inside the installer.
- On first start the app unpacks it into the user's local app data (`runtime/N`, "Preparing the Python engine (first start only)…"); it unpacks again only when N (`py/runtime.txt`) changes. No admin rights; any other Python on the computer is untouched.
- The Rust engine stays in charge: it keeps Python workers running and talks to them in JSON lines (as Datum Plot's `py.rs`, `runtime.rs` and `py/worker.py`, copied, not reinvented). A request that hangs restarts its worker; if Python is missing, the rest of Atlas works and says what is missing.
- Size: the installer grows from about 3 MB to roughly 60–120 MB (Datum Sim with numpy, SciPy and one more library is about 55 MB; Datum Plot with more about 115 MB). Every update is that size. Accepted (the user had allowed 50–100 MB; a download on first use was considered and set aside as more work).
- Uses: graphs that look as good as Datum Plot's (matplotlib), publication export, phase diagrams (a thermodynamics library), heavy numerics. Live interaction (selection charts, maps, sliders) stays in the window.

**Roadmap** (small releases, each checked and published; the user reviews 0.3.18 first):

| Release | Theme | Contents |
| --- | --- | --- |
| 0.3.19 | Python engine and better graphs | A built-in Python engine as in the other Datum apps; graphs drawn by matplotlib with export for publication. |
| 0.3.20 | Keep research work in the project | Images, EBSD maps and DIC fields saved with the project, with their settings; fixes from the review. |
| 0.3.21 | Corrosion | Galvanic compatibility, resistance by environment, corrosion allowance, E–pH (Pourbaix) diagrams and corrosion kinetics. |
| 0.3.22 | Data clean-up | The licensed carbon database and distributor datasheets replaced by open sources (added after the user's data check). |
| 0.3.23 | Heat treatment and phase diagrams | Hardenability, tempering, welding preheat; binary phase diagrams with the lever rule from published thermodynamic assessments. |
| 0.4.0 | Research depth 1: DIC and EBSD together | Point-pair alignment of DIC onto EBSD, DIC masks and finite strain, a tensor toolkit. |
| 0.4.1 | Research depth 2: active slip systems | A grain inspector, slip bands matched to traces, the relative displacement ratio. |
| 0.4.2 | Research depth 3: slip bands and strain steps | Slip bands found automatically; several DIC strain steps in one experiment. |
| 0.4.3 | Research depth 4: dislocations and boundaries | GND density, misorientation to the grain mean, distance to boundaries, Σ boundary labels, more slip systems. |
| 0.4.4 | X-ray | Real X-ray peak heights and measured patterns over calculated ones. |
| 0.4.5 | Elastic constants | Single-crystal elastic constants: moduli by direction, also per EBSD grain. |
| 0.4.6 | Fit and statistics | Fit any table with uncertainties, statistics and t-tests, supply risk. |
| 0.4.7 | Simulation export | Grain maps written as crystal-plasticity simulation input. |
| later | More | CIF and space groups, neutron and electron diffraction, Wulff shapes, X-ray texture, simulation results import, more EBSD/DIC file formats; the items left from PLAN-0.4.0 between releases where easiest |

**Where we begin: 0.3.19, the Python engine.** It is the foundation the rest stands on (graphs, phase diagrams, fitting), it copies proven family code, and it makes the graphs the user found plain look right at once. 0.3.20 then makes research work safe to keep (images, EBSD and DIC are not saved with the project today), with the fixes from the user's review folded in.
