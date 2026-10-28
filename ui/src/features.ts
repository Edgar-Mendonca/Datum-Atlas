// The feature catalogue (docs/PLAN-0.1.md §7 and §10). Every planned command in the window points
// at one of these IDs, so the whole app is laid out now and fills in release by release. When a
// feature ships, set its release to the shipping version and replace its planned command.
export const VERSION_NOW = "0.1.1";
export const RELEASES: Record<string, string> = {
  "0.1": "Database, datasheets, property charts and selection", "0.1.1": "Design goals, units, search, bar charts, sources and ? help",
  "0.1.2": "Several lines and charts, keyboard, results to a spreadsheet", "0.2": "Index builder, expression axes and formula properties", "0.3": "Compare, radar charts, the report and 250 materials",
  "0.3.1": "Your own databases, import and chart styles", "0.4": "Eco audit", "0.5": "Process selection", "1.0": "1,000 materials and the user guide",
};
export const AREAS: Record<string, string> = {
  A: "Materials data", B: "Property charts", C: "Selection", D: "Compare and report", E: "Eco audit", F: "Process selection", G: "Help and the app",
};
/** [id, name, release, what it means (plain words)] */
export type Feature = [string, string, string, string];
export const FEATURES: Feature[] = [
  ["A1", "Built-in database", "0.1", "About 100 generic materials from every family, each value with its source."],
  ["A2", "Datasheets", "0.1", "Every property of a material, grouped, with its range, unit and reference."],
  ["A3", "Your own materials", "0.1", "Add, edit and delete materials of your own, with a source for each value."],
  ["A4", "Units: SI and imperial", "0.1.1", "Switch every value, chart and limit between SI and imperial units."],
  ["A5", "Search everything", "0.1.1", "Find materials by name, synonym, designation (UNS, EN, ASTM, JIS) or use, from anywhere."],
  ["A6", "Derived properties", "0.1.1", "Price, CO₂ and energy per volume, specific stiffness and strength, thermal diffusivity and shock resistance, calculated from each material's values."],
  ["A13", "Formula properties", "0.2", "Your own derived properties as spreadsheet-style formulas."],
  ["A7", "250 materials", "0.3", "Specific grades, ratings for durability and processing, all referenced."],
  ["A8", "Import from a spreadsheet or CSV", "0.3.1", "Bring your own data in with a column-mapping step; export it the same way."],
  ["A9", "Several databases", "0.3.1", "Open, share and switch between .datdb databases, for a team or a project."],
  ["A10", "Pictures and microstructures", "0.3.1", "A picture on each datasheet."],
  ["A12", "Values checked against their sources", "0.1.1", "Each value checked against the document it cites, marked ✓; the metals first, then a family per release."],
  ["A14", "Sources page", "0.1.1", "Every source, its licence and the values that cite it: open and public sources only."],
  ["A11", "1,000 materials", "1.0", "A database grown with the community, every value referenced."],
  ["B12", "Design goals", "0.1.1", "22 goals (light, cheap or low-carbon ties, beams and panels; springs, hinges, damage tolerance, thermal): the axes, the selection line and suggested limits in one pick."],
  ["B1", "Bubble charts", "0.1", "Any two properties on log or linear axes; each material an ellipse spanning its ranges."],
  ["B2", "Family envelopes", "0.1", "A smooth, see-through envelope around each family or class."],
  ["B3", "Hover, click, zoom and pan", "0.1", "Ranges under the mouse, the datasheet on a click, zoom with the wheel, pan by dragging."],
  ["B4", "Guide lines", "0.1", "Lines of slope 1, 2 and 3 for the classic indices E/ρ, E^½/ρ and E^⅓/ρ."],
  ["B5", "Several charts per project", "0.1.1", "Keep as many charts as you need; selections combine across them."],
  ["B6", "Bar charts", "0.1.1", "One property for every material, sorted, coloured by family."],
  ["B7", "Export pictures", "0.1", "SVG with editable text, PNG at 150–600 dpi, and PDF through File › Print."],
  ["B13", "Charts side by side", "0.1.2", "Two or four charts at once, one selection shared across them."],
  ["B14", "Label the best materials", "0.1.2", "The top five by the index labelled automatically."],
  ["B8", "Expression axes", "0.2", "Plot formulas such as [Young's modulus]^0.5/[Density] on an axis."],
  ["B9", "Labels that do not overlap", "0.3.1", "Material labels placed automatically, with leader lines when moved."],
  ["B10", "Chart styles", "0.3.1", "Publication, classic family colours and monochrome print styles."],
  ["B11", "Copy picture", "0.1.1", "Copy a chart and paste it straight into a document, a slide or an email."],
  ["C1", "Limit stage", "0.1", "Minimum and maximum values, yes/no properties and ratings; materials without data fail with a reason or are kept and marked."],
  ["C2", "Selection line", "0.1", "A line of any slope on a chart (a performance index): drag it, and the materials above it pass."],
  ["C3", "Selection box", "0.1", "Drag a box on a chart: the materials inside pass."],
  ["C4", "Tree stage", "0.1", "Leave whole families or classes out."],
  ["C5", "Results with reasons", "0.1", "The materials that pass, ranked by the index, and why each other one fails."],
  ["C9", "Several selection lines on one chart", "0.1.2", "Stiffness and strength together, for example."],
  ["C10", "Move the line with the keyboard", "0.1.2", "Arrow keys move the line; type the index value it should pass through."],
  ["C6", "Index builder", "0.2", "Pick the function (tie, beam, panel, column, spring…), the objective and the constraints; get the index, its derivation and the chart."],
  ["C7", "Index library", "0.2", "About 40 standard performance indices with explanations."],
  ["C8", "Polygon stage", "0.3.1", "Draw any shape on a chart to select inside it."],
  ["D1", "Print", "0.1", "Charts, datasheets and results print at real size or fitted, with headers and footers; save as PDF."],
  ["D5", "Results to a spreadsheet", "0.1.2", "Export the ranked results with the reasons to a spreadsheet or CSV."],
  ["D2", "Compare", "0.3", "Two to six materials side by side, with the differences highlighted."],
  ["D3", "Radar charts", "0.3", "Normalised properties of the candidates on one spider chart."],
  ["D4", "Selection report", "0.3", "Requirements, stages, charts, the ranked shortlist and the datasheets in one document."],
  ["E1", "Eco audit", "0.4", "Energy and CO₂ through the life of a product: material, manufacture, transport, use and end of life."],
  ["E2", "Eco charts", "0.4", "A bar chart per life phase and a comparison of design options."],
  ["E3", "Greener substitutes", "0.4", "Materials like this one, but with less CO₂ or less cost."],
  ["F1", "Process records", "0.5", "Casting, moulding, machining, forming, joining and additive manufacturing, with size, shape, tolerance, roughness and batch size."],
  ["F2", "Process selection", "0.5", "Which processes can make your part from your material, and why others cannot."],
  ["F3", "Cost model", "0.5", "Cost per part against batch size for each process."],
  ["G1", "Getting started and glossary", "0.1", "The method explained for newcomers, and every term in plain words."],
  ["G4", "“?” help on every tool", "0.1.1", "A small ? beside each tool opens a card saying what it is for and how to use it."],
  ["G2", "User guide", "1.0", "A full guide with worked examples for teaching and design."],
  ["G3", "Signed installer", "1.0", "No SmartScreen warning on install."],
];
export const feature = (id: string) => FEATURES.find(f => f[0] === id);
const parts = (v: string) => [...v.split(".").map(Number), 0, 0].slice(0, 3);
const older = (a: string, b: string) => { const x = parts(a), y = parts(b); for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] < y[i]; return true; };
/** Ready in this version (its release is this one or older)? */
export const ready = (id: string) => { const f = feature(id); return !!f && older(f[2], VERSION_NOW); };
