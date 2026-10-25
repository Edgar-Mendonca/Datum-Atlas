// Window state: the open project (with its charts laid out and its results), the database, the view
// shown, the material in the datasheet, the open chart, menus, dialogs and toasts.
import { createSignal, batch, JSX } from "solid-js";
import { invoke, Out, Options, Db, Material, Chart, ChartOut, Goal, Val, fmt } from "./api";

export const [out, setOutRaw] = createSignal<Out | null>(null);
export const [tick, setTick] = createSignal(0);
export const P = () => out()!.project;

/** The database (built-in + the user's), reloaded when the shell says it changed. */
export const [db, setDb] = createSignal<Db>({ materials: [], sources: [], userCount: 0, props: [], tree: [], rev: 0 });
export const loadDb = async () => { const d = await invoke<Db>("database"); setDb(d); return d; };
export const setOut = (o: Out | null | undefined) => {
  if (!o) return;
  const reload = o.dbRev !== out()?.dbRev;
  batch(() => { setOutRaw(o); setTick(t => t + 1); fixChart(); });
  if (reload) loadDb().catch(() => {});
};
export const mat = (id: string): Material | undefined => db().materials.find(m => m.id === id);
export const propOf = (key: string) => db().props.find(p => p.key === key);
export const srcOf = (id: string) => db().sources.find(s => s.id === id);
/** The design goals (engine table), loaded once. */
export const [goals, setGoals] = createSignal<Goal[]>([]);

// ---------- units: values are stored in SI; the window shows SI or imperial ----------
export const imperial = () => opts().units === "imperial";
/** The unit shown for a property, and value shown = stored × k + c. */
export const conv = (key: string) => { const p = propOf(key), im = imperial() ? p?.imperial : null; return im ? { unit: im[0], k: im[1], c: im[2] } : { unit: p?.unit ?? "", k: 1, c: 0 }; };
export const toShown = (key: string, v: number) => { const q = conv(key); return v * q.k + q.c; };
export const toStored = (key: string, v: number) => { const q = conv(key); return (v - q.c) / q.k; };
export const unitOf = (key: string) => conv(key).unit;
/** A value as shown: "2690 – 2710" (in the units chosen), "Yes", "Good", or "" for no data. */
export const vtext = (key: string, v: Val | undefined) => {
  if (!v) return "";
  if (typeof v[0] === "number") { const a = fmt(toShown(key, v[0])), b = fmt(toShown(key, v[1] as number)); return a === b ? a : `${a} – ${b}`; }
  return typeof v[0] === "boolean" ? (v[0] ? "Yes" : "No") : String(v[0]);
};
export const setUnits = (u: string) => { setOpts({ ...opts(), units: u }); invoke("set_options", { opts: opts() }).catch(() => {}); };

/** auto, light or dark; auto follows Windows. */
export const [theme, setThemeRaw] = createSignal("auto");
const darkMq = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)") : null;
export const applyTheme = () => { document.documentElement.dataset.theme = theme() === "auto" ? (darkMq?.matches ? "dark" : "light") : theme(); };
darkMq?.addEventListener("change", () => theme() === "auto" && applyTheme());
export const setTheme = (t: string) => { setThemeRaw(t); applyTheme(); setOpts({ ...opts(), theme: t }); invoke("set_options", { opts: opts() }).catch(() => {}); };
export const [opts, setOpts] = createSignal<Options>({ theme: "auto", autosave: 5, beta: false, openLast: false, labels: false, guides: true, dpi: 300, units: "si", paper: "A4", landscape: true, company: "", logo: "" });

// ---------- what is shown ----------
export type View = "setup" | "browse" | "datasheet" | "charts" | "stages" | "results" | "compare" | "index" | "report" | "data" | "sources";
export const [view, setView] = createSignal<View>("charts");
export const [backstage, setBackstage] = createSignal<string | null>("home");
export const [tab, setTab] = createSignal("home");
/** The material shown in the datasheet, and the one highlighted on charts and in tables. */
export const [cur, setCur] = createSignal("");
export const [chartId, setChartId] = createSignal(0);
/** Browse: the family-tree node chosen ("" for all) and the search text. */
export const [node, setNode] = createSignal("");
export const [query, setQuery] = createSignal("");
export const chart = (): Chart | undefined => out()?.project.charts.find(c => c.id === chartId()) ?? out()?.project.charts[0];
export const chartOut = (id = chart()?.id): ChartOut | undefined => out()?.charts.find(c => c.id === id);
const fixChart = () => { const cs = out()?.project.charts ?? []; if (!cs.some(c => c.id === chartId())) setChartId(cs[0]?.id ?? 0); };
export const show = (v: View) => { setView(v); if (v === "charts") setTab("chart"); else if (v === "browse" || v === "datasheet" || v === "data" || v === "sources") setTab("data"); else if (v === "stages" || v === "results") setTab("select"); };
export const openSheet = (id: string) => { setCur(id); show("datasheet"); };
/** Did a material fail the stages? (greyed on charts) */
export const failed = () => { const s = new Set<string>(); const r = out()?.results; if (r && r.stages.length) r.rows.forEach(x => { if (!x.pass) s.add(x.id); }); return s; };

// ---------- editing ----------
export const act = async (op: object, quiet = false): Promise<boolean> => {
  try { setOut(await invoke<Out>("apply", { op })); return true; }
  catch (e) { if (!quiet) toast(String(e)); return false; }
};
export const call = async <T = any>(cmd: string, args?: object): Promise<T | undefined> => {
  try { return await invoke<T>(cmd, args); } catch (e) { toast(String(e)); return undefined; }
};
/** Loads a project (new, opened, recovered) and shows its first chart. */
export const loadOut = async (cmd: string, args?: object) => {
  const o = await call<Out | null>(cmd, args);
  if (o) { setOut(o); setBackstage(null); setChartId(o.project.charts[0]?.id ?? 0); show("charts"); }
  return o;
};
/** Applies a design goal to the open chart ("" to explore). */
export const setGoal = (goal: string): Promise<boolean> => { const c = chart(); if (!c) { toast("Open a chart first."); return Promise.resolve(false); } return act({ Goal: { chart: c.id, goal } }); };
/** Changes the open chart's settings. */
export const setChart = (f: (c: Chart) => void) => { const c = chart(); if (!c) return toast("Open a chart first."); const n = structuredClone(c); f(n); return act({ SetChart: { chart: n } }); };

// ---------- messages, menus, dialogs ----------
export const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
let timer = 0;
export const toast = (text: string, ok = false) => { setMsg({ text, ok }); clearTimeout(timer); timer = window.setTimeout(() => setMsg(null), ok ? 3000 : 7000); };
export type Item = { label: string; run?: () => void; keys?: string; off?: boolean; on?: boolean; sub?: Item[]; icon?: string } | "-";
export const [menu, setMenu] = createSignal<{ x: number; y: number; items: Item[] } | null>(null);
export const openMenu = (e: MouseEvent | { x: number; y: number }, items: Item[]) => {
  if ("preventDefault" in e) e.preventDefault();
  const r = (e as MouseEvent).currentTarget instanceof HTMLElement && (e as MouseEvent).type === "click" ? ((e as MouseEvent).currentTarget as HTMLElement).getBoundingClientRect() : null;
  setMenu({ x: r ? r.left : (e as MouseEvent).clientX ?? (e as any).x, y: r ? r.bottom + 2 : (e as MouseEvent).clientY ?? (e as any).y, items });
};
export const [dialogs, setDialogs] = createSignal<(() => JSX.Element)[]>([]);
export const openDialog = (d: () => JSX.Element) => setDialogs(v => [...v, d]);
export const closeDialog = () => setDialogs(v => v.slice(0, -1));
