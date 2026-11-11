// Help and small dialogs: Getting Started (the method), Glossary, Shortcuts, save before closing.
import { For, Show, createSignal } from "solid-js";
import { Out } from "./api";
import { openDialog, closeDialog, call, P, toast } from "./store";
import { VERSION_NOW } from "./features";
import { Help } from "./help";
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
        <li>A <b>box</b> or a drawn <b>shape</b> picks a window of values; two facing lines make a <b>band</b>; the <b>tree</b> leaves families out.</li></ul>
      <h4>4 · Rank and check</h4>
      <ul><li><b>Results</b> lists what passes, ranked by the index, with “× best” against the leader, and says why each other material fails.</li>
        <li>Several indices at once? Rank by a <b>weighted</b> score instead of one line.</li>
        <li>Put the best few side by side in <b>Compare</b>, open their datasheets, check their references, and make the <b>Report</b>: requirements, charts, shortlist, comparison and datasheets in one document.</li></ul>
      <h4>5 · Check its life and how to make it</h4>
      <ul><li>The <b>Eco audit</b> adds up energy and CO₂ from making the material to the end of the product's life, and names the phase that matters most. <b>Greener substitutes</b> finds materials that do the same job with less CO₂.</li>
        <li><b>Process selection</b> keeps the processes that can make your part from your material, says why the others cannot, and draws the cost per part against batch size.</li></ul>
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
    ["Life phases", "Material, manufacture, transport, use and end of life: the eco audit adds up energy and CO₂ in each."],
    ["Break-even", "The years of use after which a design that costs more to make, but less to use, has caught up with another."],
    ["Economic batch size", "The range of part numbers a process suits: below it the tooling is too dear per part; above it, other processes are faster."],
    ["Tooling cost", "The cost of dies, moulds and fixtures for a process, shared over the batch."],
    ["Tolerance and roughness", "How close to size a process can make a part (± mm), and how smooth its surface is (Ra, µm)."],
    ["Band", "Two facing selection lines on one chart: the materials between them pass."],
    ["Weighted ranking", "Ranking by several properties at once, each scaled from worst to best and given a weight."],
    ["Index library", "Performance indices for common designs, each with its function, constraint, objective and free variable. Plot one, or build your own."],
    ["Formula property", "A property you define from others, such as σy/ρ or E·α. It is calculated for every material, with its range, and works like any other property."],
    ["Radar chart", "Compare's picture: one spoke per property, each material placed between the lowest (centre) and highest (rim) value in the database."],
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
    ["Recycled fraction", "The share of today's supply of a material that is recycled; recycled material usually takes far less energy to make."],
    ["Heat of combustion", "The heat released by burning one kilogram. Burning a material at the end of its life wins back part of it."],
    ["Downcycling", "Reusing a material as a lower-grade one: crushed concrete as fill, chipped wood as board, ground rubber as crumb."],
    ["Database", "A file of materials (.datdb). Your own materials are one; others from colleagues can be opened beside the built-in data."],
    ["Durability rating", "How well a material stands water, acids, alkalis, sunlight (UV) and fire, from Unacceptable to Excellent."],
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

/** Help › Guide: downloads the guide (PDF) for this version from GitHub, checks it, saves it in Downloads and opens it. */
export const guide = () => openDialog(() => {
  const [busy, setBusy] = createSignal(false);
  const get = async () => {
    setBusy(true);
    try { const path = await call<string>("guide"); if (path) { toast(`Saved in Downloads and opened: ${path}`, true); closeDialog(); } }
    finally { setBusy(false); }
  };
  return (
    <Dialog title="Datum Atlas Guide">
      <p>A step-by-step guide to Datum Atlas, with pictures: property charts, design goals and indices, selection and results, compare, your own materials, the eco audit, process selection, the report, and the command line. <Help k="guide" /></p>
      <div class="guidecards"><button class="guidecard" disabled={busy()} onClick={get}><svg class="ic"><use href="#i-manual" /></svg><b>English</b><span>{busy() ? "Downloading…" : "Datum Atlas Guide (PDF)"}</span></button></div>
      <p class="muted">It downloads from GitHub, is checked against the release's checksums, saved in your Downloads folder and opened. It matches this version of Datum Atlas; you need an internet connection.</p>
    </Dialog>
  );
});

/** Help › Use with an AI Assistant: how to connect an assistant to Datum Atlas through the command line. */
export const assistant = () => openDialog(() => {
  const exe = `C:\\Tools\\datum-atlas-cli-${VERSION_NOW}.exe`;
  const cfg = JSON.stringify({ mcpServers: { "datum-atlas": { command: exe, args: ["mcp"] } } }, null, 2);
  const copy = (t: string) => call("copy_text", { text: t }).then(() => toast("Copied: paste it into the assistant's settings.", true));
  return (
    <Dialog title="Use with an AI assistant" wide>
      <div class="guide">
        <p>An AI assistant that speaks the <b>Model Context Protocol</b> can use Datum Atlas: search the materials and read their datasheets, set up a selection, read the ranked results, run the eco audit and process selection, and save the project. <Help k="assistant" /></p>
        <h4>1 · Get the command-line program</h4>
        <p>Download <code>datum-atlas-cli-{VERSION_NOW}.exe</code> from the same release as the installer (Help › Releases) and keep it in a folder of your choice, such as <code>C:\Tools</code>.</p>
        <h4>2 · Add it to the assistant's settings</h4>
        <p>Most assistants take a list of servers like this one. Change the path to where you saved the program.</p>
        <pre class="code">{cfg}</pre>
        <p class="inline"><button class="btn sm" onClick={() => copy(cfg)}>Copy the settings</button><button class="btn sm" onClick={() => copy(`"${exe}" mcp`)}>Copy the command</button></p>
        <h4>3 · Ask</h4>
        <ul><li>“Find light, stiff materials for a bracket that works at 150 °C and costs under 5 USD/kg.”</li>
          <li>“Compare the CO₂ of a steel and an aluminium version of this part over ten years of driving.”</li>
          <li>“Which processes can make a 2 kg hollow aluminium casting, and which is cheapest for 20,000 parts?”</li></ul>
        <p class="muted">The assistant changes a project only through the same edits as the window, and saves nothing unless asked. Your own databases (.datdb) can be added with <code>--db</code> after <code>mcp</code>. Nothing leaves your computer through Datum Atlas.</p>
      </div>
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
