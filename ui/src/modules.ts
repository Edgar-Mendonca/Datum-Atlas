// The workbench's modules: each has its pages (sidebar) and its ribbon tabs. The sidebar shows the
// open module's pages; a module's first page opens when you switch to it. Planned modules show
// dimmed with their release (features.ts, area H).
import type { View } from "./store";

export type ModId = "library" | "select" | "eco" | "make" | "micro" | "crystal";
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
  { id: "micro", name: "Microstructure", short: "Micro", icon: "grains", about: "Typical structures of every class of material; image analysis, the generator, EBSD and DIC follow.",
    pages: [["micro", "grains", "Structures"]], tabs: ["home", "data"] },
  { id: "crystal", name: "Crystal", short: "Crystal", icon: "crystal", about: "The elements on a periodic table, and the composition of formulas and alloys; crystal structures, X-ray peaks and projections follow.",
    pages: [["elements", "crystal", "Elements"]], tabs: ["home", "data"] },
];

/** Planned modules: [feature id, name, icon]. Their release is in features.ts. */
export const PLANNED: [string, string, string][] = [
  ["H2", "Testing", "curve"], ["H3", "Image analysis", "image"], ["H4", "Synthetic structures", "grid"],
  ["H5", "EBSD", "layout"], ["H6", "DIC", "xy"], ["H7", "Corrosion", "risk"], ["H8", "Heat treatment", "level"],
];

export const moduleOf = (v: View): Module => MODULES.find(m => m.pages.some(p => p[0] === v)) ?? MODULES[1];
