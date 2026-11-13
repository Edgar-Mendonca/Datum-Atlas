// The workbench's modules: each has its pages (sidebar) and its ribbon tabs. The sidebar shows the
// open module's pages; a module's first page opens when you switch to it. Planned modules show
// dimmed with their release (features.ts, area H).
import type { View } from "./store";

export type ModId = "library" | "select" | "eco" | "make" | "micro" | "crystal" | "testing" | "ebsd" | "dic";
export interface Module { id: ModId; name: string; short: string; icon: string; about: string; pages: [View, string, string][]; tabs: string[] }

export const MODULES: Module[] = [
  { id: "library", name: "Library", short: "Library", icon: "db", about: "Every material with its referenced values, your own materials and databases, import and export.",
    pages: [["browse", "tree", "Browse"], ["datasheet", "datasheet", "Datasheet"], ["data", "db", "Data"], ["sources", "source", "Sources"]], tabs: ["home", "data"] },
  { id: "select", name: "Select", short: "Select", icon: "atlas", about: "Property charts, design goals and indices, selection stages, ranked results, compare and the report.",
    pages: [["setup", "setup", "Setup"], ["charts", "atlas", "Charts"], ["stages", "filter", "Stages"], ["results", "results", "Results"], ["compare", "compare", "Compare"], ["index", "builder", "Index"], ["report", "report", "Report"]], tabs: ["home", "select", "chart", "report"] },
  { id: "eco", name: "Eco", short: "Eco", icon: "eco", about: "Energy and CO₂ through a product's life, design options and greener substitutes.",
    pages: [["eco", "eco", "Eco audit"]], tabs: ["home", "report"] },
  { id: "make", name: "Make", short: "Make", icon: "process", about: "Which processes can make your part, why the others cannot, and the cost against batch size.",
    pages: [["process", "process", "Process"], ["processes", "list", "Processes"]], tabs: ["home", "report"] },
  { id: "micro", name: "Microstructure", short: "Micro", icon: "grains", about: "Typical structures of every class of material, and image analysis of your micrographs: phase fraction, porosity and particles; grain size, the generator, EBSD and DIC follow.",
    pages: [["micro", "grains", "Structures"], ["images", "image", "Images"], ["synth", "grid", "Generator"]], tabs: ["home", "data"] },
  { id: "crystal", name: "Crystal", short: "Crystal", icon: "crystal", about: "The elements on a periodic table; compositions; crystal structures with spacings, angles, X-ray peaks and stereographic projections.",
    pages: [["elements", "crystal", "Elements"], ["calc", "builder", "Lattice"]], tabs: ["home", "data"] },
  { id: "testing", name: "Testing", short: "Testing", icon: "curve", about: "Your test data: tensile curves and results, hardness statistics, S–N curves and creep rupture on a Larson–Miller plot; the results become a material of your own.",
    pages: [["tensile", "curve", "Tensile"], ["hardness", "level", "Hardness"], ["fatigue", "xy", "Fatigue"], ["creep", "risk", "Creep"]], tabs: ["home", "data"] },
  { id: "ebsd", name: "EBSD", short: "EBSD", icon: "layout", about: "Orientation maps from EBSD: open .ang and .ctf exports and see orientations, phases and pattern quality; grains, boundaries, Schmid factors and pole figures follow.",
    pages: [["ebsd", "layout", "Maps"], ["ebsdgrains", "grains", "Grains"], ["polefig", "atlas", "Pole figures"]], tabs: ["home", "data"] },
  { id: "dic", name: "DIC", short: "DIC", icon: "xy", about: "Strain maps from digital image correlation: open a displacement field and see every strain component, its statistics and profiles; strain by EBSD grain follows.",
    pages: [["dic", "xy", "Strain maps"]], tabs: ["home", "data"] },
];

/** Planned modules: [feature id, name, icon]. Their release is in features.ts. */
export const PLANNED: [string, string, string][] = [
["H7", "Corrosion", "risk"], ["H8", "Heat treatment", "level"],
];

export const moduleOf = (v: View): Module => MODULES.find(m => m.pages.some(p => p[0] === v)) ?? MODULES[1];
