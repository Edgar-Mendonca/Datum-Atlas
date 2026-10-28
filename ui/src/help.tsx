// "?" help: a small button beside a tool; a click opens a short card saying what the tool is for
// and how to use it. One place for every explanation, so the whole app teaches itself.
import { For, Show } from "solid-js";
import { openDialog } from "./store";
import { Dialog } from "./ui";

type Topic = { t: string; what: string; how?: string[]; tip?: string };
export const HELP: Record<string, Topic> = {
  goal: { t: "Design goal", what: "Picks what the part must do and what to minimise, and sets up the chart for it: the two axes, a selection line of the right slope through the materials (the best fifth pass), and the limits the goal suggests.",
    how: ["Choose a goal: say “Light, stiff beam” for a beam that must be stiff and as light as possible.", "The line shows the performance index (here E½/ρ). Materials above it are better.", "Drag the line to let more or fewer through; Results ranks them by the index.", "The goal's limits are marked “from the goal” in Stages; change or remove them freely. “Explore” removes the line and those limits."],
    tip: "The goal never removes limits you added yourself." },
  chart: { t: "Property chart", what: "Every material on two properties. Each ellipse spans a material's range of both values; each family has a see-through envelope. Log axes fit materials that differ a million times.",
    how: ["Hover a bubble for its values; click it to see it beside the chart; double-click for its datasheet.", "Wheel to zoom, drag to pan, Fit to see everything.", "Click a family in the legend to hide or show it."] },
  tools: { t: "Pan, Line and Box", what: "What a drag or a click on the chart does.",
    how: ["Pan: drag to move around, click a bubble to pick it.", "Line: click where the selection line should pass; it keeps the slope typed next to it. Drag its round handle to move it.", "Box: drag a rectangle; the materials inside pass."] },
  slope: { t: "Slope of the selection line", what: "On log axes a performance index is a straight line, and its slope comes from the index.",
    how: ["Slope 1: E/ρ (tie), σy/ρ (strong tie), λ/α.", "Slope 2: E½/ρ (beam, column); slope 3: E⅓/ρ (panel).", "Slope 1.5: σy⅔/ρ (strong beam); slope ½: σy²/E (spring) or KIc²/σy.", "“Above passes” keeps the materials with the higher index."] },
  envelopes: { t: "Envelopes", what: "The coloured outline around the bubbles of each family (or class). Where a family is spread out, it gets one envelope per group.", how: ["Families, classes or none, from the list."] },
  guides: { t: "Guide lines", what: "Dashed lines of slope 1, 2 and 3. Moving along one keeps E/ρ, E½/ρ or E⅓/ρ the same, so they show at a glance which materials share an index." },
  bar: { t: "Bar chart", what: "One property for every material: a bar from its low to its high value, grouped by family and sorted. Good for comparing a single property, such as price or max service temperature.",
    how: ["Pick the property in the Y list.", "Failed materials turn grey, as on the bubble chart."] },
  stages: { t: "Selection stages", what: "The steps a material must pass. A material passes when it passes every stage: limits, chart selections (line or box) and families left out.",
    how: ["Each card shows how many materials pass that stage alone; the big number is how many pass them all.", "Click a card to change it."] },
  limits: { t: "Limits", what: "Values the part needs: a minimum, a maximum, or both, in the units shown. Yes/no properties, such as recyclable, can be required.",
    how: ["Add a limit, pick the property, type the value.", "“Typical value” compares the middle of a material's range; “whole range” asks the whole range to pass.", "Materials without the value fail with “no data”, or can be kept and marked."] },
  families: { t: "Families and classes", what: "Leave out whole families or classes, for example because the part must be cast, or cannot be a polymer near heat.", how: ["Untick what cannot be used."] },
  results: { t: "Results", what: "The materials that pass every stage, ranked by the performance index of the selection line, with the reason every other material fails.",
    how: ["“× best” is the material's index divided by the best one's: 1 for the best, 0.5 for half as good.", "Tick “show the materials that fail” to see why each fails.", "Double-click a row for its datasheet."] },
  browse: { t: "Material library", what: "Every material in a table with a family tree. Sort by a heading, select cells and copy them to a spreadsheet, search by name, designation (UNS, EN, ASTM), synonym or use.",
    how: ["Click a family or class to show only it.", "Columns ▾ chooses the properties shown.", "Ctrl+C copies the selected cells with their headings, ready to paste into a spreadsheet; double-click a row for the datasheet."] },
  datasheet: { t: "Datasheet", what: "Every property of one material, grouped, as a range with its unit and a numbered reference. Missing values show as “no data”, never zero.",
    how: ["The small number after a value is its reference, listed at the bottom.", "✓ marks a value checked against its source; the others are drafts.", "Derived properties (per volume, specific values) are calculated from the material's own values."] },
  sources: { t: "Sources", what: "Where every value comes from. Datum Atlas uses open and public sources only (government handbooks, open datasets, published datasheets, typical textbook ranges written up as our own compilation) and never a proprietary database.",
    how: ["Click a source to see which materials and properties cite it.", "Your own materials list the sources you gave."] },
  addmaterial: { t: "Add a material", what: "A material of your own, beside the built-in ones, in every view. Each value needs a source so the datasheet can show where it comes from.",
    how: ["Name and family first; density is required.", "Type a low and a high value (one value: Low only) in the unit shown.", "Say where the values come from: a datasheet, a test report, a book or your own measurement."] },
  units: { t: "Units", what: "Show every value, axis and limit in SI (kg/m³, GPa, MPa, °C) or imperial (lb/in³, 10⁶ psi, ksi, °F). Values are stored in SI, so switching loses nothing." },
  derived: { t: "Derived properties", what: "Properties calculated from the material's own values: price, CO₂ and energy per volume (for minimum-cost and minimum-carbon designs), specific stiffness E/ρ, specific strength σy/ρ, thermal diffusivity and thermal shock resistance.",
    how: ["Ranges hold every combination: a product uses low × low and high × high.", "They appear in the chart axes and in limits like any property."] },
  setup: { t: "Setup", what: "Write down what the part must do: its function, the constraints it must meet, the objective (what to minimise) and the free variables. Then pick the design goal that matches." },
};

/** The small "?" beside a tool. */
export const Help = (p: { k: string }) => <button class="help" title={`What is ${HELP[p.k]?.t ?? "this"}?`} onClick={e => { e.stopPropagation(); helpCard(p.k); }}>?</button>;

export const helpCard = (k: string) => openDialog(() => {
  const h = HELP[k];
  return (
    <Dialog title={h?.t ?? "Help"}>
      <div class="helpcard"><p>{h?.what}</p>
        <Show when={h?.how}><h4>How to use it</h4><ul><For each={h!.how}>{x => <li>{x}</li>}</For></ul></Show>
        <Show when={h?.tip}><p class="muted">{h!.tip}</p></Show></div>
    </Dialog>
  );
});
