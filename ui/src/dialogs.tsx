// Help and small dialogs: Getting Started (the method), Glossary, Shortcuts, save before closing.
import { For, Show, createSignal } from "solid-js";
import { Out } from "./api";
import { openDialog, closeDialog, call, P } from "./store";
import { Dialog } from "./ui";

export const gettingStarted = () => openDialog(() => (
  <Dialog title="Getting Started" wide>
    <div class="guide">
      <p><b>Datum Atlas</b> helps you choose a material by the method engineers learn for it: requirements → property charts → selection → ranking → a documented choice.</p>
      <h4>1 · Write down what the part must do</h4>
      <ul><li><b>Function</b>: what it does (carries a load in bending, holds pressure, conducts heat).</li>
        <li><b>Constraints</b>: what it must meet (stiffness, strength, 150 °C, cost, recyclable).</li>
        <li><b>Objective</b>: what to minimise or maximise (mass, cost, CO₂).</li>
        <li><b>Free variables</b>: what you may change (thickness, cross-section, the material). Write these in Setup.</li></ul>
      <h4>2 · Look at the materials on a property chart</h4>
      <ul><li>Two properties on log axes, such as Young's modulus against density. Each material is a <b>bubble</b> spanning its range of both values; each family has an <b>envelope</b>.</li>
        <li>Hover a bubble for its values, click it to see its datasheet beside the chart, double-click for the full datasheet. Wheel to zoom, drag to pan.</li></ul>
      <h4>3 · Select</h4>
      <ul><li><b>Limits</b> remove materials that cannot work: max service temperature at least 150 °C, price at most 5 USD/kg, recyclable.</li>
        <li>A <b>selection line</b> picks the materials that do the job best. A light, stiff beam needs a high E^½/ρ: on the modulus–density chart that is a line of slope 2, and the materials above it pass. Tie: slope 1 (E/ρ). Panel: slope 3 (E^⅓/ρ).</li>
        <li>A <b>box</b> picks a window of values; the <b>tree</b> leaves families out.</li></ul>
      <h4>4 · Rank and check</h4>
      <ul><li><b>Results</b> lists what passes, ranked by the index, with “× best” against the leader, and says why each other material fails.</li>
        <li>Open the datasheets of the best few, check their references, and print the chart, the results and the datasheets.</li></ul>
      <p class="muted">Start from File › Home: “Light, stiff beam” sets up steps 2–4 for you. Planned features are shown dimmed; Help › Roadmap says when each arrives.</p>
    </div>
  </Dialog>
));

const GLOSSARY: [string, [string, string][]][] = [
  ["The method", [
    ["Property chart", "Two material properties plotted against each other, usually on log axes, with every material shown. Also called a material selection chart."],
    ["Bubble", "One material on a chart: an ellipse spanning its range of both properties."],
    ["Envelope (hull)", "The smooth outline around all the bubbles of one family or class."],
    ["Family", "Metals, polymers, elastomers, ceramics and glasses, composites, foams or natural materials. Each has its colour."],
    ["Class", "A group inside a family, such as aluminium alloys inside metals."],
    ["Performance index", "A combination of properties to maximise for a design, such as E^½/ρ for a light, stiff beam. It comes from the function, the objective and the constraints."],
    ["Selection line", "A line on a chart along which the index has one value. Its slope comes from the index: 1 for E/ρ, 2 for E^½/ρ, 3 for E^⅓/ρ on modulus–density axes. Materials above it are better."],
    ["Guide lines", "Dashed lines of slope 1, 2 and 3 showing the directions of the classic indices."],
    ["Stage", "One step of the selection: limits, a chart selection, or families left out. A material passes when it passes every stage."],
    ["Limit", "A minimum or maximum value (or yes/no, or a lowest rating) that a material must meet."],
    ["× best", "A material's index divided by the best material's: 1 for the best, 0.5 for half as good."],
  ]],
  ["Properties", [
    ["Density, ρ", "Mass per volume, kg/m³. Water is 1000; aluminium about 2700; steel about 7850."],
    ["Young's modulus, E", "Stiffness: stress per elastic strain, in GPa. Rubber 0.002, wood 10, aluminium 70, steel 210."],
    ["Yield strength, σy", "The stress at which permanent deformation starts (0.2 % proof stress for metals), in MPa."],
    ["Tensile strength, σts", "The highest stress before breaking in tension. For ceramics and glass, the flexural strength is given."],
    ["Fracture toughness, KIc", "Resistance to a crack growing, in MPa·m½. Glass about 0.7, steel 50–150."],
    ["Max service temperature", "The highest temperature for long use without unacceptable loss of properties."],
    ["Thermal conductivity, λ", "How well heat flows through, W/(m·K). Copper 390, steel 50, polymers 0.2, foams 0.03."],
    ["Thermal expansion, α", "Strain per degree, in µstrain/°C (10⁻⁶/°C)."],
    ["CO₂ footprint", "CO₂ released to make one kilogram from raw materials, kg/kg."],
    ["Embodied energy", "Energy used to make one kilogram from raw materials, MJ/kg."],
  ]],
  ["Data", [
    ["Range", "Values for a generic material spread with grade, processing and test. Datum Atlas keeps the low and the high."],
    ["Typical value", "The value most often met inside the range (geometric mean when not given)."],
    ["No data", "A property not known for a material. Shown as a gap, never as zero. A limit on it can fail the material or keep it, marked."],
    ["Reference", "Where a value comes from: each value points to a source, listed on the datasheet."],
    ["Draft value", "A built-in value compiled from the named source but not yet checked against its exact table or page."],
    ["Log axis", "An axis where each step multiplies by 10, so materials that differ a thousand times fit on one chart. Zero and negative values cannot be shown."],
  ]],
];
export const glossary = () => openDialog(() => {
  const [q, setQ] = createSignal("");
  return (
    <Dialog title="Glossary" wide>
      <input class="find" placeholder="Search terms" value={q()} onInput={e => setQ(e.currentTarget.value)} />
      <div class="gloss"><For each={GLOSSARY}>{([h, list]) => { const f = () => list.filter(x => (x[0] + x[1]).toLowerCase().includes(q().toLowerCase())); return <Show when={f().length}><h4>{h}</h4><dl><For each={f()}>{([t, d]) => <><dt>{t}</dt><dd>{d}</dd></>}</For></dl></Show>; }}</For></div>
    </Dialog>
  );
});

export const shortcuts = () => openDialog(() => (
  <Dialog title="Keyboard Shortcuts">
    <table class="keys"><tbody><For each={[["Wheel", "Zoom the chart around the mouse"], ["Drag", "Pan the chart (Pan tool), move the selection line by its handle"], ["Click / double-click", "Show a material beside the chart / open its datasheet"],
      ["Ctrl+C", "Copy the selected table cells (paste into a spreadsheet)"], ["Ctrl+A", "Select the whole table"], ["Arrows, Shift", "Move and extend the table selection"], ["Enter", "Open the datasheet of the table row"],
      ["Ctrl+Z / Y", "Undo, redo"], ["Ctrl+S", "Save"], ["Ctrl+Shift+S", "Save As"], ["Ctrl+O", "Open"], ["Ctrl+N", "New (File › Home)"], ["Ctrl+P", "Print"], ["Ctrl+K or Alt+Q", "Tell me what you want to do"], ["F1", "This list"]]}>{([k, d]) => <tr><td>{k}</td><td>{d}</td></tr>}</For></tbody></table>
  </Dialog>
));

export const confirmClose = (then: () => void) => openDialog(() => (
  <Dialog title="Save changes?" okText="Save" cancel="Cancel" ok={async () => { const o = await call<Out | null>("save", { asNew: false }); if (o) then(); return !!o; }}
    extra={<button class="btn" onClick={() => { closeDialog(); then(); }}>Don't save</button>}>
    <p>Do you want to save the changes to “{P().name}”?</p>
  </Dialog>
));
