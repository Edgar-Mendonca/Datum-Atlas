// Calls into the Rust shell and the shapes it returns (camelCase JSON from crates/core).
declare global {
  interface Window { __TAURI__?: { core: { invoke: (cmd: string, args?: object) => Promise<any> }; event: { listen: (e: string, f: (x: { payload: any }) => void) => Promise<() => void> } } }
}

export const invoke = <T = any>(cmd: string, args?: object): Promise<T> =>
  window.__TAURI__ ? window.__TAURI__.core.invoke(cmd, args) : Promise.reject("Datum Atlas must run inside its desktop window.");
export const listen = (e: string, f: (p: any) => void) => window.__TAURI__?.event.listen(e, x => f(x.payload));

/** A value: [low, high, source, typical?] | [yes/no, source] | [rating, source]. */
export type Val = [number, number, string] | [number, number, string, number] | [boolean, string] | [string, string];
export interface Material {
  id: string; name: string; family: string[]; syn: string[]; des: Record<string, string>; props: Record<string, Val>;
  notes: string; uses: string; status: string; user: boolean; checked: string[];
}
export interface Source { id: string; title: string; author: string; publisher: string; year: string; url: string; licence: string; kind: string; note: string }
export interface Prop { key: string; name: string; sym: string; group: string; unit: string; kind: "range" | "flag" | "rating"; log: boolean; about: string; imperial: [string, number, number] | null }
export interface Node { path: string; name: string; depth: number; count: number }
export interface Db { materials: Material[]; sources: Source[]; userCount: number; props: Prop[]; tree: Node[]; rev: number; /** What happened to your materials file at start (damaged and kept aside, values left out). */ userNote?: string | null }

export type Line = { slope: number; x: number; y: number; above: boolean };
export type ChartSel = { Line: Line } | { Box: { x0: number; x1: number; y0: number; y1: number } } | { Poly: { pts: [number, number][] } };
/** A chart: two properties and its selections (lines, boxes, shapes); a material passes when it passes all. */
export interface Chart { id: number; name: string; x: string; y: string; logX: boolean; logY: boolean; hull: string; labels: boolean; hidden: string[]; guides: number[]; sels: ChartSel[]; goal: string; kind: string }
export interface Weight { prop: string; w: number; high: boolean }
export interface Limit { prop: string; min: number | null; max: number | null; flag: boolean | null; rating: string | null; goal: boolean }
export interface Stages { limits: Limit[]; exclude: string[]; rule: string; missing: string; rankChart: number; rankLine: number; ranking: string; weights: Weight[] }
export interface Project { format: number; name: string; notes: string; charts: Chart[]; stages: Stages; nextId: number; ui: any }

export interface Bubble { id: string; name: string; fam: number; class: string; c: [number, number]; r: [number, number]; x: [number, number]; y: [number, number] }
export interface Hull { name: string; fam: number; pts: [number, number][]; at: [number, number]; label: boolean; ids: string[] }
export interface Axis { key: string; name: string; sym: string; unit: string; log: boolean; lim: [number, number] }
export interface ChartOut { id: number; x: Axis; y: Axis; families: string[]; bubbles: Bubble[]; hulls: Hull[]; missing: [string, string][]; bands: { name: string; fam: number; from: number; to: number }[] }
export interface Row { id: string; name: string; family: string; pass: boolean; reasons: string[]; index: number | null; ratio: number | null; rank: number | null }
export interface Results { total: number; pass: number; stages: { kind: string; name: string; chart: number; pass: number }[]; rows: Row[]; indexName: string; rankChart: number; rankLine: number; lines: { chart: number; k: number; name: string; value: number }[] }
export interface Out { project: Project; path: string | null; dirty: boolean; undo: boolean; redo: boolean; rev: number; charts: ChartOut[]; results: Results; dbRev: number }
export interface Goal {
  id: string; group: string; name: string; index: string; x: string; y: string; slope: number; above: boolean;
  function: string; constraint: string; objective: string; free: string; why: string; limits: [string, number | null, number | null][];
  also: { x: string; y: string; slope: number; above: boolean; index: string; name: string } | null;
}
export interface Options { theme: string; autosave: number; beta: boolean; openLast: boolean; labels: boolean; guides: boolean; dpi: number; units: string; paper: string; landscape: boolean; company: string; logo: string }

/** Three significant figures without trailing zeros (crates/core units::fmt). */
export const fmt = (v: number | null | undefined): string => {
  if (v === null || v === undefined || !isFinite(v)) return "–";
  if (v === 0) return "0";
  const a = Math.abs(v);
  if (a < 1e-3 || a >= 1e6) { const e = Math.floor(Math.log10(a)); return `${+(v / 10 ** e).toFixed(2)}e${e}`; }
  return String(+v.toFixed(Math.max(0, 2 - Math.floor(Math.log10(a)))));
};
export const range = (lo: number, hi: number) => (fmt(lo) === fmt(hi) ? fmt(lo) : `${fmt(lo)} – ${fmt(hi)}`);
export const isRange = (v: Val | undefined): v is [number, number, string] => !!v && typeof v[0] === "number";
/** "2690 – 2710", "Yes", "Good" or "" for no data. */
export const valText = (v: Val | undefined) => !v ? "" : typeof v[0] === "number" ? range(v[0], v[1] as number) : typeof v[0] === "boolean" ? (v[0] ? "Yes" : "No") : String(v[0]);
/** The centre of a range for sorting (geometric mean when positive). */
export const mid = (v: Val | undefined) => isRange(v) ? (v[0] > 0 && v[1] > 0 ? Math.sqrt(v[0] * v[1]) : (v[0] + v[1]) / 2) : NaN;
export const SEP = " › ";

/** Family colours (Okabe-Ito, colour-blind safe), in the order of FAMILIES. */
export const FAMILY_COLORS = ["#0072B2", "#E69F00", "#CC79A7", "#009E73", "#D55E00", "#56B4E9", "#8C6D1F"];
export const famColor = (k: number) => FAMILY_COLORS[k % FAMILY_COLORS.length];

/** Days since 1970-01-01 (local) and ISO dates, for the fill handle's date series (cells.ts). */
export const dayDate = (d: number) => { const x = new Date(1970, 0, 1); x.setDate(x.getDate() + Math.floor(d)); return x; };
export const dateDay = (x: Date) => Math.round((Date.UTC(x.getFullYear(), x.getMonth(), x.getDate()) - Date.UTC(1970, 0, 1)) / 864e5);
export const iso = (d: number) => { const x = dayDate(d); return `${x.getFullYear()}-${String(x.getMonth() + 1).padStart(2, "0")}-${String(x.getDate()).padStart(2, "0")}`; };
export const lineOf = (s: ChartSel | null | undefined) => (s && "Line" in s ? s.Line : null);
export const boxOf = (s: ChartSel | null | undefined) => (s && "Box" in s ? s.Box : null);
export const polyOf = (s: ChartSel | null | undefined) => (s && "Poly" in s ? s.Poly : null);
/** A chart's goal without the "/2" that marks a two-constraint goal's second chart. */
export const goalId = (c: Chart) => c.goal.replace(/\/2$/, "");
