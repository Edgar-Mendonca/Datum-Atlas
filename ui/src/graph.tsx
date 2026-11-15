// Graphs drawn by the Python engine (matplotlib), shown in the window with hover values and
// export for publication. A page describes its graph as a spec (py/atlas/graph.py); if the engine
// cannot start, the page's own simple chart is shown instead, with a note.
import { JSX, Show, createEffect, createSignal, on, onCleanup, onMount } from "solid-js";
import { call, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";

export interface Series { name?: string; kind?: "line" | "scatter" | "step" | "bar" | "stem"; x: (number | null)[]; y: (number | null)[]; color?: string; width?: number; dash?: string; marker?: string; size?: number; colors?: string[]; open?: boolean[]; labels?: string[]; barWidth?: number; edge?: boolean }
export interface Spec { title?: string; x?: { label?: string; log?: boolean; min?: number; max?: number }; y?: { label?: string; log?: boolean; min?: number; max?: number };
  series: Series[]; lines?: { x?: number; y?: number; color?: string; dash?: string; label?: string }[]; marks?: { x: number; y: number; text?: string; color?: string; marker?: string; size?: number; fill?: string }[];
  legend?: boolean; legendAt?: string; grid?: boolean }
interface Drawn { svg: string; w: number; h: number; box: [number, number, number, number]; xlim: [number, number]; ylim: [number, number]; xlog: boolean; ylog: boolean; points: [number, number, number, string | null][]; names: (string | null)[] }

/** The engine's state: null until known; an error stops further tries until a restart. */
export const [engineDown, setEngineDown] = createSignal<string | null>(null);
const fmtv = (v: number) => Math.abs(v) >= 1e5 || (Math.abs(v) < 1e-3 && v !== 0) ? v.toExponential(3) : +v.toPrecision(5) + "";

export const Graph = (p: { spec: Spec; name: string; ratio?: number; fallback?: JSX.Element; cls?: string }) => {
  const [d, setD] = createSignal<Drawn | null>(null), [busy, setBusy] = createSignal(false), [tip, setTip] = createSignal<{ x: number; y: number; text: string } | null>(null);
  const [width, setWidth] = createSignal(0);
  let box!: HTMLDivElement, seq = 0;
  onMount(() => { const ro = new ResizeObserver(() => setWidth(Math.round(box.clientWidth))); ro.observe(box); setWidth(Math.round(box.clientWidth)); onCleanup(() => ro.disconnect()); });
  createEffect(on([() => JSON.stringify(p.spec), width], async ([s, w]) => {
    if (!w || engineDown()) return;
    const my = ++seq; await new Promise(r => setTimeout(r, 30)); if (my !== seq) return;
    const theme = document.documentElement.dataset.theme === "dark" ? "dark" : "light";
    setBusy(true);
    try { const r = await invoke<Drawn>("graph", { spec: { ...JSON.parse(s), size: [w, Math.round(w * (p.ratio ?? 0.55))], theme } }); if (my === seq) setD(r); }
    catch (e) { if (my === seq) { setEngineDown(String(e)); setD(null); } }
    finally { if (my === seq) setBusy(false); }
  }));
  /** Mouse to data, then the nearest point within 14 px. */
  const move = (e: MouseEvent) => {
    const r = d(); if (!r) return;
    const rc = box.getBoundingClientRect(), k = rc.width / r.w, mx = (e.clientX - rc.left) / k, my = (e.clientY - rc.top) / k;
    const [x0, y0, x1, y1] = r.box;
    const tx = (v: number) => { const [a, b] = r.xlim, f = r.xlog ? (Math.log10(v) - Math.log10(a)) / (Math.log10(b) - Math.log10(a)) : (v - a) / (b - a); return x0 + f * (x1 - x0); };
    const ty = (v: number) => { const [a, b] = r.ylim, f = r.ylog ? (Math.log10(v) - Math.log10(a)) / (Math.log10(b) - Math.log10(a)) : (v - a) / (b - a); return y1 - f * (y1 - y0); };
    let best: { d: number; p: Drawn["points"][0]; px: number; py: number } | null = null;
    for (const q of r.points) { const px = tx(q[1]), py = ty(q[2]), dd = Math.hypot(px - mx, py - my); if (dd < (best?.d ?? 14)) best = { d: dd, p: q, px, py }; }
    if (!best) return setTip(null);
    const nm = r.names[best.p[0]], lb = best.p[3];
    setTip({ x: best.px * k, y: best.py * k, text: `${lb ? lb + ": " : nm ? nm + ": " : ""}${p.spec.x?.label ? p.spec.x.label.replace(/ \(.*\)$/, "") + " " : "x "}${fmtv(best.p[1])}, ${p.spec.y?.label ? p.spec.y.label.replace(/ \(.*\)$/, "") + " " : "y "}${fmtv(best.p[2])}` });
  };
  const exp = async (fmt: string) => { const path = await call<string | null>("graph_export", { spec: p.spec, fmt, name: p.name }); if (path) toast(`Saved ${path}`, true); };
  return (
    <div class={`pgraph ${p.cls ?? ""}`}>
      <Show when={!engineDown()} fallback={<>{p.fallback}<p class="muted small gdown" title={engineDown() ?? ""}>Simple chart: the graph engine (Python) is not available. Help › About › Python engine to restart it.</p></>}>
        <div class="gbox" ref={box} onMouseMove={move} onMouseLeave={() => setTip(null)} classList={{ busy: busy() }}>
          <Show when={d()} fallback={<div class="gwait muted small" style={{ height: `${Math.round((width() || 400) * (p.ratio ?? 0.55))}px` }}>Drawing…</div>}><div class="gsvg" innerHTML={d()!.svg} /></Show>
          <Show when={tip()}>{t => <><span class="gdot" style={{ left: `${t().x}px`, top: `${t().y}px` }} /><span class="gtip" style={{ left: `${t().x + 10}px`, top: `${t().y - 26}px` }}>{t().text}</span></>}</Show>
        </div>
        <div class="gbar"><Help k="graphs" /><span class="muted small">Export for publication:</span>
          <button class="lnk gpng" onClick={() => exp("png")}>PNG 300 dpi</button><button class="lnk" onClick={() => exp("svg")}>SVG</button><button class="lnk gpdf" onClick={() => exp("pdf")}>PDF</button></div>
      </Show>
    </div>
  );
};

/** After Help › Restart Python engine. */
export const restartEngine = async () => { try { const v = await invoke<any>("restart_engine"); setEngineDown(null); toast(`The Python engine restarted (Python ${v.python}, matplotlib ${v.matplotlib}).`, true); } catch (e) { setEngineDown(String(e)); toast(String(e)); } };
