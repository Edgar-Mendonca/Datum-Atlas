// Commands shared by the ribbon, menus, shortcuts and Tell Me.
import { act, call, loadOut, setOut, setBackstage, toast, chart, P, show, setChartId, openSheet, cur, setUnits } from "./store";
import { Out, Chart } from "./api";
import { FEATURES, ready } from "./features";
import { soon } from "./roadmap";
import { exportChart, fitChart, zoomChart, setTool, copyChart, activeLine, newLine, removeSel, passThrough } from "./chart";
import { addLimit } from "./stages";
import { editMaterial } from "./material";
import * as D from "./dialogs";

/** Adds a chart of y against x (log axes by default) and opens it. */
const newChart = async (x = "rho", y = "E") => {
  const c: Partial<Chart> = { x, y, logX: true, logY: true, hull: "family", labels: false, hidden: [], guides: [], sels: [], name: "", goal: "", kind: "bubble", best: false, style: "classic", moved: {} };
  if (await act({ AddChart: { chart: c } })) { const cs = P().charts; setChartId(cs[cs.length - 1].id); show("charts"); }
};
export const C = {
  save: async (asNew = false) => { const o = await call<Out | null>("save", { asNew }); if (o) { setOut(o); toast(`Saved ${o.path}`, true); } },
  open: () => loadOut("open", { path: null }),
  undo: async () => setOut(await call<Out>("history", { redo: false })),
  redo: async () => setOut(await call<Out>("history", { redo: true })),
  print: () => setBackstage("print"),
  newChart,
  delChart: () => { const c = chart(); if (!c) return; if (P().charts.length === 1) return toast("A selection keeps at least one chart."); if (confirm(`Delete the chart “${c.name}”?`)) act({ DelChart: { id: c.id } }); },
  /** Sets the slope of the line being edited, or adds a line of that slope. */
  line: (slope: number) => { const c = chart(); if (!c) return; show("charts"); const l = activeLine(c); if (l) act({ Select: { chart: c.id, sels: c.sels.map(s => ("Line" in s && s.Line === l ? { Line: { ...l, slope } } : s)) } }); else newLine(c, slope); setTool("pan"); },
  addLine: () => { const c = chart(); if (c) { show("charts"); newLine(c); } },
  passThrough: () => { const c = chart(); if (c) passThrough(c); },
  clearSel: () => { const c = chart(); if (c?.sels.length) removeSel(c); },
  fit: fitChart, zoom: zoomChart, exportChart,
  addLimit: () => addLimit(),
  addMaterial: () => editMaterial(),
  sheet: () => (cur() ? openSheet(cur()) : show("browse")),
};

/** Tell Me: [label, run, keys]. Design goals and materials are added when searching (ribbon.tsx). */
export const ALL_CMDS: [string, () => void, string?][] = [
  ["Save", () => C.save(), "Ctrl+S"], ["Save As", () => C.save(true), "Ctrl+Shift+S"], ["Open", C.open, "Ctrl+O"], ["New selection (File › Home)", () => setBackstage("home"), "Ctrl+N"],
  ["Print or save as PDF", C.print, "Ctrl+P"], ["Undo", C.undo, "Ctrl+Z"], ["Redo", C.redo, "Ctrl+Y"],
  ["Property chart", () => show("charts")], ["New chart: modulus vs density", () => newChart("rho", "E")], ["New chart: strength vs density", () => newChart("rho", "sy")], ["New chart: toughness vs strength", () => newChart("sy", "kic")],
  ["New chart: conductivity vs expansion", () => newChart("alpha", "lam")], ["New chart: CO₂ vs price", () => newChart("price", "co2")],
  ["Selection line of slope 1 (E/ρ)", () => C.line(1)], ["Selection line of slope 2 (E½/ρ)", () => C.line(2)], ["Selection line of slope 3 (E⅓/ρ)", () => C.line(3)], ["Selection box", () => { show("charts"); setTool("box"); }],
  ["Clear the chart's selection", C.clearSel], ["Add a limit", C.addLimit], ["Stages", () => show("stages")], ["Results", () => show("results")],
  ["Browse materials", () => show("browse")], ["Datasheet", C.sheet], ["Add a material", C.addMaterial], ["Data and sources", () => show("data")], ["Setup: requirements", () => show("setup")],
  ["Copy chart as a picture", copyChart], ["Units: SI", () => setUnits("si")], ["Units: imperial", () => setUnits("imperial")], ["Sources of the values", () => show("sources")], ["Material library", () => show("browse")],
  ["Bar chart of the Y property", () => { const c = chart(); if (c) act({ SetChart: { chart: { ...c, kind: "bar", sels: [], goal: "" } } }); }],
  ["Add another selection line (a band)", C.addLine], ["Pass the line through an index value", C.passThrough], ["Draw a selection shape (polygon)", () => { show("charts"); setTool("poly"); }],
  ["Rank by weighted properties", () => show("stages")],
  ["Export chart as SVG", () => exportChart("svg")], ["Export chart as PNG", () => exportChart("png")], ["Fit the chart", C.fit],
  ["Options", () => setBackstage("options")], ["Check for updates", () => setBackstage("about")], ["Keyboard shortcuts", D.shortcuts, "F1"], ["Getting started", D.gettingStarted], ["Glossary", D.glossary],
  ["Roadmap: what comes when", () => setBackstage("roadmap")],
  ...FEATURES.filter(f => !ready(f[0])).map(f => [`${f[1]} (${f[2]})`, () => soon(f[0])] as [string, () => void]),
];
