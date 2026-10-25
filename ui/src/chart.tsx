// The property chart: the engine lays out bubbles and family hulls in axis space (log10 on log
// axes); this file only maps them to the screen, draws the axes and ticks, and handles zoom, pan,
// hover, clicks and dragging the selection line or box. `ChartSvg` also draws printed charts.
import { For, Show, createSignal, onCleanup, onMount } from "solid-js";
import { Chart, ChartOut, ChartSel, Bubble, famColor, fmt, range } from "./api";
import { act, out, failed, cur, setCur, openSheet, openMenu, chartOut, setChart, toast, db, chart, call, opts, conv, unitOf, toShown, goals, setGoal } from "./store";
import { Icon } from "./ui";
import { Help } from "./help";
import { render } from "solid-js/web";

export type Win = [number, number, number, number];
const M = { l: 66, r: 16, t: 14, b: 48 };
const LEG = 24;
const SUP: Record<string, string> = { "-": "⁻", "0": "⁰", "1": "¹", "2": "²", "3": "³", "4": "⁴", "5": "⁵", "6": "⁶", "7": "⁷", "8": "⁸", "9": "⁹" };
const decade = (k: number) => (k >= -2 && k <= 4 ? fmt(10 ** k) : `10${String(k).replace(/./g, ch => SUP[ch] ?? ch)}`);

/** Major ticks [position, label] and minor tick positions between a and b (axis space). */
export const ticks = (a: number, b: number, log: boolean) => {
  const major: [number, string][] = [], minor: number[] = [];
  if (log) {
    const span = b - a, step = span > 14 ? 2 : 1;
    for (let k = Math.ceil(a); k <= Math.floor(b); k++) if (k % step === 0) major.push([k, decade(k)]);
    if (span < 10) for (let k = Math.floor(a); k <= b; k++) for (let m = 2; m < 10; m++) {
      const u = k + Math.log10(m);
      if (u > a && u < b) { if (span < 1.6 && (m === 2 || m === 5)) major.push([u, fmt(10 ** u)]); else minor.push(u); }
    }
  } else {
    const raw = (b - a) / 6, p = 10 ** Math.floor(Math.log10(raw)), step = [1, 2, 5, 10].map(m => m * p).find(s => s >= raw) ?? p * 10;
    for (let v = Math.ceil(a / step) * step; v <= b; v += step) major.push([v, fmt(Math.abs(v) < step / 1e6 ? 0 : v)]);
    for (let v = Math.ceil(a / (step / 5)) * (step / 5); v <= b; v += step / 5) minor.push(v);
  }
  return { major, minor };
};

/** Ticks for an axis in the units shown (positions stay in stored axis space). */
const ticksFor = (a: number, b: number, log: boolean, key: string) => {
  const q = conv(key);
  if (q.k === 1 && q.c === 0) return ticks(a, b, log);
  if (log) { const d = Math.log10(q.k), t = ticks(a + d, b + d, true); return { major: t.major.map(([u, l]) => [u - d, l] as [number, string]), minor: t.minor.map(u => u - d) }; }
  const t = ticks(a * q.k + q.c, b * q.k + q.c, false);
  return { major: t.major.map(([v, l]) => [(v - q.c) / q.k, l] as [number, string]), minor: t.minor.map(v => (v - q.c) / q.k) };
};
const title = (a: ChartOut["x"]) => { const u = unitOf(a.key) || a.unit; return `${a.name}${a.sym ? `, ${a.sym}` : ""}${u ? ` (${u})` : ""}`; };
/** Moves labels down until they do not overlap (boxes: x centre, y baseline, width). */
const nudge = (ls: { x: number; y: number; w: number }[]) => {
  const out = ls.map((l, i) => ({ ...l, i })).sort((a, b) => a.y - b.y), done: typeof out = [];
  for (const l of out) {
    for (let k = 0; k < 12 && done.some(d => Math.abs(d.x - l.x) < (d.w + l.w) / 2 + 4 && Math.abs(d.y - l.y) < 14); k++) l.y += 14;
    done.push(l);
  }
  return done.sort((a, b) => a.i - b.i).map(l => l.y);
};
const lineOf = (s: ChartSel | null | undefined) => (s && "Line" in s ? s.Line : null);
const boxOf = (s: ChartSel | null | undefined) => (s && "Box" in s ? s.Box : null);

/** The chart as SVG at w × h pixels for the window `win`. */
export const ChartSvg = (p: { c: Chart; o: ChartOut; w: number; h: number; win: Win; sel?: ChartSel | null; print?: boolean; hover?: string; legend?: boolean }) => {
  const W = () => Math.max(10, p.w - M.l - M.r), H = () => Math.max(10, p.h - M.t - M.b - (p.legend ? LEG : 0));
  const X = (u: number) => M.l + ((u - p.win[0]) / (p.win[1] - p.win[0])) * W();
  const Y = (v: number) => M.t + (1 - (v - p.win[2]) / (p.win[3] - p.win[2])) * H();
  const bar = () => p.c.kind === "bar";
  const tx = () => (bar() ? { major: [], minor: [] } : ticksFor(p.win[0], p.win[1], p.o.x.log, p.o.x.key)), ty = () => ticksFor(p.win[2], p.win[3], p.o.y.log, p.o.y.key);
  const hullLabels = () => { const hs = p.o.hulls.filter(h => h.label), ys = nudge(hs.map(h => ({ x: X(h.at[0]), y: Y(h.at[1]) - 4, w: h.name.length * 6.8 }))); return hs.map((h, i) => ({ h, y: ys[i] })); };
  const fail = () => failed();
  const sel = () => (p.sel === undefined ? p.c.select : p.sel);
  const ax = (v: number, log: boolean) => (log ? Math.log10(v) : v);
  const clip = `clip${p.c.id}${p.print ? "p" : "s"}`;
  const lineVal = () => out()?.results.lines.find(l => l.chart === p.c.id);
  /** The selection line across the window, and the side that passes. */
  const line = () => {
    const l = lineOf(sel()); if (!l) return null;
    const [u, v] = [ax(l.x, p.o.x.log), ax(l.y, p.o.y.log)], at = (x: number) => v + l.slope * (x - u);
    const [a, b] = [p.win[0], p.win[1]], edge = l.above ? p.win[3] : p.win[2];
    // Where to write the index: the first point from 70 % along the line that is inside the plot.
    const inside = (t: number) => { const x = a + (b - a) * t, y = Y(at(x)); return y > M.t + 16 && y < M.t + H() - 8 ? [X(x), y] : null; };
    // …and not on a family label.
    const w = ((lineVal()?.name.length ?? 6) + 9) * 7, labs = hullLabels().map(({ h, y }) => ({ x: X(h.at[0]), y, w: h.name.length * 6.8 }));
    const free = (q: number[] | null) => q && !labs.some(l => q[0] + 8 < l.x + l.w / 2 + 4 && q[0] + 8 + w > l.x - l.w / 2 - 4 && Math.abs(q[1] - 10 - l.y) < 16);
    const cand = [0.7, 0.6, 0.8, 0.5, 0.9, 0.4, 0.3, 0.2, 0.85, 0.15].map(inside);
    const lab = cand.find(free) ?? cand.find(Boolean) ?? [X(u), Y(v)];
    return { d: `M${X(a)} ${Y(at(a))}L${X(b)} ${Y(at(b))}`, zone: `M${X(a)} ${Y(at(a))}L${X(b)} ${Y(at(b))}L${X(b)} ${Y(edge)}L${X(a)} ${Y(edge)}Z`, hx: X(u), hy: Y(v), lx: lab[0], ly: lab[1] };
  };
  const guide = (n: number) => { const cu = (p.win[0] + p.win[1]) / 2, cv = (p.win[2] + p.win[3]) / 2, at = (x: number) => cv + n * (x - cu); return { d: `M${X(p.win[0])} ${Y(at(p.win[0]))}L${X(p.win[1])} ${Y(at(p.win[1]))}`, n }; };
  const bubbles = () => [...p.o.bubbles].sort((a, b) => b.r[0] * b.r[1] - a.r[0] * a.r[1]);
  return (
    <svg class="chartsvg" width={p.w} height={p.h} viewBox={`0 0 ${p.w} ${p.h}`} xmlns="http://www.w3.org/2000/svg" font-family="IBM Plex Sans, Segoe UI, Arial, sans-serif">
      <Show when={p.print}><rect width={p.w} height={p.h} fill="#fff" /></Show>
      <defs><clipPath id={clip}><rect x={M.l} y={M.t} width={W()} height={H()} /></clipPath></defs>
      <rect x={M.l} y={M.t} width={W()} height={H()} fill="#fff" />
      <g stroke="#E4E7EB" stroke-width=".7">
        <For each={tx().major}>{([u]) => <line x1={X(u)} x2={X(u)} y1={M.t} y2={M.t + H()} />}</For>
        <For each={ty().major}>{([v]) => <line y1={Y(v)} y2={Y(v)} x1={M.l} x2={M.l + W()} />}</For>
      </g>
      <g clip-path={`url(#${clip})`}>
        <For each={p.o.hulls}>{h => <path class="hull" d={`M${h.pts.map(q => `${X(q[0]).toFixed(1)} ${Y(q[1]).toFixed(1)}`).join("L")}Z`} fill={famColor(h.fam)} fill-opacity=".13" stroke={famColor(h.fam)} stroke-opacity=".55" stroke-width="1.2" />}</For>
        <Show when={line()}>{l => <path d={l().zone} fill="#6C4AB6" fill-opacity=".06" />}</Show>
        <For each={bar() ? [] : p.c.guides}>{n => { const g = () => guide(n); return <path d={g().d} stroke="#5B6878" stroke-width="1" stroke-dasharray="5 4" fill="none" />; }}</For>
        <For each={bubbles()}>{b => {
          const off = () => fail().has(b.id), on = () => b.id === cur() || b.id === p.hover;
          if (bar()) { const x0 = X(b.c[0] - b.r[0]), x1 = X(b.c[0] + b.r[0]), y0 = Y(b.c[1] + Math.max(b.r[1], 0.004 * (p.win[3] - p.win[2]))), y1 = Y(b.c[1] - Math.max(b.r[1], 0.004 * (p.win[3] - p.win[2])));
            return <rect class="bub" data-id={b.id} x={x0} y={y0} width={Math.max(1.5, x1 - x0)} height={Math.max(2, y1 - y0)} fill={off() ? "#C3C8CE" : famColor(b.fam)} fill-opacity={off() ? 0.4 : 0.7} stroke={on() ? "#0C1A2B" : "none"} stroke-width={on() ? 1.6 : 0} />; }
          return <ellipse class="bub" data-id={b.id} cx={X(b.c[0])} cy={Y(b.c[1])} rx={Math.max(2.5, (b.r[0] / (p.win[1] - p.win[0])) * W())} ry={Math.max(2.5, (b.r[1] / (p.win[3] - p.win[2])) * H())}
            fill={off() ? "#C3C8CE" : famColor(b.fam)} fill-opacity={off() ? 0.35 : 0.6} stroke={on() ? "#0C1A2B" : off() ? "#A9AFB6" : famColor(b.fam)} stroke-width={on() ? 2 : 0.8} />;
        }}</For>
        <For each={hullLabels()}>{({ h, y }) => <text x={X(h.at[0])} y={y} text-anchor="middle" font-size="12" font-weight="600" fill={famColor(h.fam)} stroke="#fff" stroke-width="3" paint-order="stroke" pointer-events="none">{h.name}</text>}</For>
        <Show when={p.c.labels}><For each={p.o.bubbles}>{b => <text x={X(b.c[0]) + 4} y={Y(b.c[1]) - 4} font-size="9.5" fill="#3B4859" pointer-events="none">{b.name.split(/[,(]/)[0]}</text>}</For></Show>
        <Show when={boxOf(sel())}>{bx => <rect class="selbox" x={Math.min(X(ax(bx().x0, p.o.x.log)), X(ax(bx().x1, p.o.x.log)))} y={Math.min(Y(ax(bx().y0, p.o.y.log)), Y(ax(bx().y1, p.o.y.log)))}
          width={Math.abs(X(ax(bx().x1, p.o.x.log)) - X(ax(bx().x0, p.o.x.log)))} height={Math.abs(Y(ax(bx().y1, p.o.y.log)) - Y(ax(bx().y0, p.o.y.log)))} fill="#6C4AB6" fill-opacity=".07" stroke="#6C4AB6" stroke-width="1.6" stroke-dasharray="6 3" />}</Show>
        <Show when={line()}>{l => <>
          <path class="sell" d={l().d} stroke="transparent" stroke-width="12" fill="none" />
          <path d={l().d} stroke="#6C4AB6" stroke-width="2.2" fill="none" pointer-events="none" />
          <Show when={!p.print}><circle class="selh" cx={l().hx} cy={l().hy} r="6.5" fill="#fff" stroke="#6C4AB6" stroke-width="2.2" /></Show>
        </>}</Show>
      </g>
      <rect x={M.l} y={M.t} width={W()} height={H()} fill="none" stroke="#333" stroke-width="1" />
      <g stroke="#333" stroke-width="1">
        <For each={tx().major}>{([u]) => <line x1={X(u)} x2={X(u)} y1={M.t + H()} y2={M.t + H() + 5} />}</For>
        <For each={tx().minor}>{u => <line x1={X(u)} x2={X(u)} y1={M.t + H()} y2={M.t + H() + 2.5} />}</For>
        <For each={ty().major}>{([v]) => <line y1={Y(v)} y2={Y(v)} x1={M.l - 5} x2={M.l} />}</For>
        <For each={ty().minor}>{v => <line y1={Y(v)} y2={Y(v)} x1={M.l - 2.5} x2={M.l} />}</For>
      </g>
      <g font-size="11" fill="#222">
        <For each={tx().major}>{([u, t]) => <text x={X(u)} y={M.t + H() + 17} text-anchor="middle">{t}</text>}</For>
        <For each={ty().major}>{([v, t]) => <text x={M.l - 8} y={Y(v) + 4} text-anchor="end">{t}</text>}</For>
        <Show when={bar()}><For each={p.o.bands}>{b => <text x={(X(b.from) + X(b.to)) / 2} y={M.t + H() + 17} text-anchor="middle" font-size="10.5" font-weight="600" fill={famColor(b.fam)}>{X(b.to) - X(b.from) > b.name.length * 6 ? b.name : b.name.split(" ")[0]}</text>}</For></Show>
        <text x={M.l + W() / 2} y={M.t + H() + 40} text-anchor="middle" font-size="12.5" font-weight="600">{title(p.o.x)}</text>
        <text transform={`translate(15 ${M.t + H() / 2}) rotate(-90)`} text-anchor="middle" font-size="12.5" font-weight="600">{title(p.o.y)}</text>
      </g>
      <For each={bar() ? [] : p.c.guides}>{n => { const g = guide(n); const m = g.d.match(/L([\d.-]+) ([\d.-]+)/); const [x, y] = m ? [+m[1], +m[2]] : [0, 0];
        return <Show when={y > M.t && y < M.t + H()}><text x={Math.min(x, M.l + W()) - 4} y={y - 5} text-anchor="end" font-size="10" fill="#5B6878">slope {fmt(n)}</text></Show>; }}</For>
      <Show when={p.legend}><g font-size="11" fill="#222"><For each={p.o.families.filter(f => !p.c.hidden.includes(f))}>{(f, i) => <g transform={`translate(${M.l + i() * Math.min(150, W() / 7)} ${p.h - 8})`}>
        <rect y="-9" width="11" height="11" fill={famColor(p.o.families.indexOf(f))} fill-opacity=".7" /><text x="15">{f}</text></g>}</For></g></Show>
      <Show when={line() && lineVal()}>{_ => { const l = line()!; return <text x={Math.min(Math.max(l.lx + 8, M.l + 6), M.l + W() - 170)} y={l.ly - 10} font-size="11.5" font-weight="600" fill="#5A3C9C" stroke="#fff" stroke-width="3" paint-order="stroke">{lineVal()!.name} = {fmt(lineVal()!.value)}</text>; }}</Show>
    </svg>
  );
};

/** The legend: family colours; a click hides or shows a family on this chart. */
export const Legend = (p: { c: Chart; o: ChartOut }) => (
  <div class="clegend"><For each={p.o.families}>{(f, k) => { const hid = () => p.c.hidden.includes(f), n = () => p.o.bubbles.filter(b => b.fam === k()).length;
    return <button classList={{ off: hid() }} title={hid() ? `Show ${f}` : `Hide ${f}`} onClick={() => setChart(c => { c.hidden = hid() ? c.hidden.filter(x => x !== f) : [...c.hidden, f]; })}>
      <i style={{ background: famColor(k()) }} />{f}{!hid() && <small>{n()}</small>}</button>; }}</For>
    <Show when={p.o.missing.length}><span class="muted" title={p.o.missing.slice(0, 30).map(m => `${db().materials.find(x => x.id === m[0])?.name}: ${m[1]}`).join("\n")}>{p.o.missing.length} without data</span></Show>
  </div>
);

// ---------- the interactive chart ----------
/** Zoom windows per chart, kept while you move between views; reset when the axes change. */
const wins = new Map<number, { key: string; w: Win }>();
const keyOf = (c: Chart) => `${c.kind}|${c.x}|${c.y}|${c.logX}|${c.logY}`;
export const [tool, setTool] = createSignal<"pan" | "line" | "box">("pan");
export const [slope, setSlope] = createSignal(1);
let fitNow: (() => void) | null = null, zoomBy: ((f: number) => void) | null = null;
export const fitChart = () => fitNow?.();
export const zoomChart = (f: number) => zoomBy?.(f);

export const ChartView = (p: { c: Chart }) => {
  let box!: HTMLDivElement;
  const [size, setSize] = createSignal<[number, number]>([800, 560]);
  const o = () => chartOut(p.c.id);
  const lim = (): Win => { const x = o()!; return [x.x.lim[0], x.x.lim[1], x.y.lim[0], x.y.lim[1]]; };
  // The zoom window belongs to the axes it was made for: new axes (or a bar chart) start from the full view.
  const [bump, setBump] = createSignal(0);
  const win = (): Win => { bump(); const w = wins.get(p.c.id); return w && w.key === keyOf(p.c) ? w.w : lim(); };
  const setWin = (w: Win) => { wins.set(p.c.id, { key: keyOf(p.c), w }); setBump(b => b + 1); };
  const [drag, setDrag] = createSignal<ChartSel | null | undefined>(undefined);
  const [tip, setTip] = createSignal<{ x: number; y: number; b: Bubble } | null>(null);
  onMount(() => {
    const ro = new ResizeObserver(() => setSize([box.clientWidth, box.clientHeight])); ro.observe(box); onCleanup(() => ro.disconnect());
    fitNow = () => setWin(lim());
    zoomBy = f => { const w = win(), cx = (w[0] + w[1]) / 2, cy = (w[2] + w[3]) / 2; setWin([cx - (cx - w[0]) / f, cx + (w[1] - cx) / f, cy - (cy - w[2]) / f, cy + (w[3] - cy) / f]); };
  });
  onCleanup(() => { fitNow = null; zoomBy = null; });
  const W = () => size()[0] - M.l - M.r, H = () => size()[1] - M.t - M.b;
  /** Mouse → axis space. */
  const at = (e: MouseEvent): [number, number] => { const r = box.getBoundingClientRect(), w = win(); return [w[0] + ((e.clientX - r.left - M.l) / W()) * (w[1] - w[0]), w[2] + (1 - (e.clientY - r.top - M.t) / H()) * (w[3] - w[2])]; };
  const val = (u: number, log: boolean) => (log ? 10 ** u : u);
  const hit = (e: MouseEvent) => {
    const [u, v] = at(e), w = win(), px = W() / (w[1] - w[0]), py = H() / (w[3] - w[2]);
    let best: Bubble | null = null, area = Infinity;
    for (const b of o()?.bubbles ?? []) {
      const rx = Math.max(2.5, b.r[0] * px), ry = Math.max(2.5, b.r[1] * py), dx = (u - b.c[0]) * px, dy = (v - b.c[1]) * py;
      const inside = p.c.kind === "bar" ? Math.abs(dx) <= rx && Math.abs(dy) <= ry : (dx / rx) ** 2 + (dy / ry) ** 2 <= 1;
      if (inside && rx * ry < area) { best = b; area = rx * ry; }
    }
    return best;
  };
  const wheel = (e: WheelEvent) => {
    e.preventDefault();
    const f = e.deltaY < 0 ? 1.18 : 1 / 1.18, [u, v] = at(e), w = win();
    setWin([u - (u - w[0]) / f, u + (w[1] - u) / f, v - (v - w[2]) / f, v + (w[3] - v) / f]);
  };
  const down = (e: MouseEvent) => {
    if (e.button !== 0 || !o()) return;
    const t = e.target as Element, c = p.c, x = o()!;
    const start = at(e), w0 = win(), [sx, sy] = [e.clientX, e.clientY];
    let moved = false, mv: (m: MouseEvent) => void, up: (m: MouseEvent) => void;
    const l = lineOf(c.select);
    if (c.kind === "bar") {
      mv = m => { const dx = m.clientX - sx, dy = m.clientY - sy; if (!moved && Math.hypot(dx, dy) < 3) return; moved = true; const ku = (w0[1] - w0[0]) / W(), kv = (w0[3] - w0[2]) / H(); setWin([w0[0] - dx * ku, w0[1] - dx * ku, w0[2] + dy * kv, w0[3] + dy * kv]); };
      up = m => { if (!moved) { const b = hit(m); setCur(b ? b.id : ""); } };
    } else if ((t.classList.contains("selh") || t.classList.contains("sell")) && l) {
      // Move the selection line: it keeps its slope and passes through the mouse.
      mv = m => { moved = true; const [u, v] = at(m); setDrag({ Line: { ...l, x: val(u, x.x.log), y: val(v, x.y.log) } }); };
      up = m => { const d = drag(); setDrag(undefined); if (moved && d) act({ Select: { chart: c.id, select: d } }); else { const b = hit(m); setCur(b ? b.id : ""); } };
    } else if (tool() === "box") {
      mv = m => { moved = true; const [u, v] = at(m); setDrag({ Box: { x0: val(start[0], x.x.log), x1: val(u, x.x.log), y0: val(start[1], x.y.log), y1: val(v, x.y.log) } }); };
      up = () => { const d = drag(); setDrag(undefined); if (moved && d) act({ Select: { chart: c.id, select: d } }); };
    } else {
      // Pan; a click without moving picks a bubble, or places the line with the line tool.
      mv = m => { const dx = m.clientX - sx, dy = m.clientY - sy; if (!moved && Math.hypot(dx, dy) < 3) return; moved = true; const ku = (w0[1] - w0[0]) / W(), kv = (w0[3] - w0[2]) / H(); setWin([w0[0] - dx * ku, w0[1] - dx * ku, w0[2] + dy * kv, w0[3] + dy * kv]); };
      up = m => {
        if (moved) return;
        if (tool() === "line") { const [u, v] = at(m); act({ Select: { chart: c.id, select: { Line: { slope: l?.slope ?? slope(), x: val(u, x.x.log), y: val(v, x.y.log), above: l?.above ?? true } } } }); return; }
        const b = hit(m); setCur(b ? b.id : "");
      };
    }
    const end = (m: MouseEvent) => { removeEventListener("mousemove", mv); removeEventListener("mouseup", end); up(m); };
    addEventListener("mousemove", mv); addEventListener("mouseup", end);
  };
  const move = (e: MouseEvent) => { if (e.buttons) return setTip(null); const b = hit(e); setTip(b ? { x: e.clientX, y: e.clientY, b } : null); };
  const menu = (e: MouseEvent) => {
    const b = hit(e);
    openMenu(e, [
      ...(b ? [{ label: `Datasheet: ${b.name}`, icon: "datasheet", run: () => openSheet(b.id) }, "-" as const] : []),
      { label: "Fit", icon: "fit", run: fitChart }, { label: "Zoom In", icon: "zin", run: () => zoomChart(1.4) }, { label: "Zoom Out", icon: "zout", run: () => zoomChart(1 / 1.4) }, "-",
      { label: "Selection Line Here", icon: "index", run: () => { const [u, v] = at(e), x = o()!; act({ Select: { chart: p.c.id, select: { Line: { slope: lineOf(p.c.select)?.slope ?? slope(), x: val(u, x.x.log), y: val(v, x.y.log), above: true } } } }); } },
      { label: "Remove Selection", icon: "del", off: !p.c.select, run: () => act({ Select: { chart: p.c.id, select: null } }) },
      { label: "Labels", on: p.c.labels, run: () => setChart(c => { c.labels = !c.labels; }) },
    ]);
  };
  const fails = () => out()?.results.rows.find(r => r.id === tip()?.b.id);
  return (
    <div class="cbox" ref={box} classList={{ tline: tool() === "line", tbox: tool() === "box" }} onWheel={wheel} onMouseDown={down} onMouseMove={move} onMouseLeave={() => setTip(null)}
      onDblClick={e => { const b = hit(e); if (b) openSheet(b.id); }} onContextMenu={menu}>
      <Show when={o()} fallback={<div class="gwait">Laying out the chart…</div>}>{x => <ChartSvg c={p.c} o={x()} w={size()[0]} h={size()[1]} win={win()} sel={drag()} hover={tip()?.b.id} />}</Show>
      <Show when={tip()}>{t => { const x = o()!; return (
        <div class="gtip" style={{ left: `${t().x + 14}px`, top: `${t().y + 12}px` }}>
          <b>{t().b.name}</b><span class="muted">{db().materials.find(m => m.id === t().b.id)?.family.join(" › ")}</span>
          <Show when={p.c.kind !== "bar"}><span>{x.x.name}: {range(toShown(x.x.key, t().b.x[0]), toShown(x.x.key, t().b.x[1]))} {unitOf(x.x.key)}</span></Show>
          <span>{x.y.name}: {range(toShown(x.y.key, t().b.y[0]), toShown(x.y.key, t().b.y[1]))} {unitOf(x.y.key)}</span>
          <Show when={fails() && out()!.results.stages.length}><span classList={{ ok: fails()!.pass, late: !fails()!.pass }}>{fails()!.pass ? `Passes${fails()!.rank ? ` · rank ${fails()!.rank}` : ""}` : `Fails: ${fails()!.reasons[0]}`}</span></Show>
          <span class="muted small">Click to select · double-click for the datasheet</span>
        </div>); }}</Show>
    </div>
  );
};

/** The chart's toolbar: axes, scales, tools, the line's slope and side, envelopes, labels, fit. */
/** An axis: the property and its log switch. Changing it removes the goal and the selection. */
const AxisPick = (p: { c: Chart; k: "x" | "y" }) => {
  const props = () => db().props.filter(q => q.kind === "range" && db().materials.some(m => m.props[q.key]));
  return <span class="axpick"><b>{p.k.toUpperCase()}</b>
    <select value={p.c[p.k]} onChange={e => { const v = e.currentTarget.value, lg = db().props.find(x => x.key === v)?.log ?? true; setChart(c => { c[p.k] = v; if (p.k === "x") c.logX = lg; else c.logY = lg; c.select = null; c.goal = ""; }); }}>
      <For each={props()}>{x => <option value={x.key}>{x.name}</option>}</For></select>
    <button class="rb fb" classList={{ on: p.k === "x" ? p.c.logX : p.c.logY }} title="Logarithmic axis" onClick={() => setChart(c => { if (p.k === "x") c.logX = !c.logX; else c.logY = !c.logY; c.select = null; })}>log</button></span>;
};

/** The first toolbar row: the design goal, bubbles or bars, and what the chart shows. */
export const ChartBar = (p: { c: Chart }) => {
  const groups = () => [...new Set(goals().map(g => g.group))];
  const goal = () => goals().find(g => g.id === p.c.goal);
  return (
    <div class="cbar">
      <span class="goalpick"><b>Goal</b><select value={p.c.goal} onChange={e => setGoal(e.currentTarget.value)}><option value="">Explore (no goal)</option>
        <For each={groups()}>{gr => <optgroup label={gr}><For each={goals().filter(g => g.group === gr)}>{g => <option value={g.id}>{g.name} — {g.above || g.slope ? "maximise" : "minimise"} {g.index.replace(/ \(.*\)/, "")}</option>}</For></optgroup>}</For></select><Help k="goal" /></span>
      <Show when={goal()}><button class="btn sm" title={goal()!.why} onClick={() => import("./pages").then(m => m.goalCard(goal()!.id))}>Why {goal()!.index}?</button></Show>
      <span class="sep" />
      <div class="seg"><button classList={{ on: p.c.kind !== "bar" }} title="Two properties, a bubble per material" onClick={() => setChart(c => { c.kind = "bubble"; })}>Bubbles</button>
        <button classList={{ on: p.c.kind === "bar" }} title="One property, a bar per material" onClick={() => setChart(c => { c.kind = "bar"; c.select = null; c.goal = ""; c.logY = db().props.find(x => x.key === c.y)?.log ?? true; })}>Bars</button></div><Help k={p.c.kind === "bar" ? "bar" : "chart"} />
      <span class="grow" />
      <Show when={p.c.kind !== "bar"}><select title="Envelopes" value={p.c.hull} onChange={e => { const v = e.currentTarget.value; setChart(c => { c.hull = v; }); }}><option value="family">Envelopes: families</option><option value="class">Envelopes: classes</option><option value="none">No envelopes</option></select><Help k="envelopes" /></Show>
      <button class="rb fb" classList={{ on: p.c.labels }} title="Material labels" onClick={() => setChart(c => { c.labels = !c.labels; })}><Icon n="tag" /></button>
      <button class="rb fb" classList={{ on: p.c.guides.length > 0 }} disabled={p.c.kind === "bar"} title="Guide lines of slope 1, 2 and 3" onClick={() => setChart(c => { c.guides = c.guides.length ? [] : [1, 2, 3]; })}><Icon n="index" /></button>
      <button class="rb fb" title="Zoom in" onClick={() => zoomChart(1.4)}><Icon n="zin" /></button><button class="rb fb" title="Zoom out" onClick={() => zoomChart(1 / 1.4)}><Icon n="zout" /></button><button class="rb fb" title="Fit all materials" onClick={fitChart}><Icon n="fit" /></button>
    </div>
  );
};

/** The second toolbar row: the axes, the tools, and the selection line's slope and side. */
export const ChartTools = (p: { c: Chart }) => {
  const l = () => lineOf(p.c.select);
  const setLine = (patch: Partial<{ slope: number; above: boolean }>) => { const x = l(); if (x) act({ Select: { chart: p.c.id, select: { Line: { ...x, ...patch } } } }); };
  return (
    <div class="cbar">
      <AxisPick c={p.c} k="y" /><Show when={p.c.kind !== "bar"}><button class="rb fb" title="Swap the axes" onClick={() => setChart(c => { [c.x, c.y, c.logX, c.logY] = [c.y, c.x, c.logY, c.logX]; c.select = null; c.goal = ""; })}>⇅</button><AxisPick c={p.c} k="x" />
      <span class="sep" />
      <div class="seg tools"><For each={[["pan", "Pan", "Drag to move, wheel to zoom, click a bubble"], ["line", "Line", "Click to place a selection line"], ["box", "Box", "Drag a selection box"]] as const}>{([k, n, t]) =>
        <button classList={{ on: tool() === k }} title={t} onClick={() => setTool(k)}>{n}</button>}</For></div><Help k="tools" />
      <label class="inline" title="Slope of the selection line: 1 for E/ρ, 2 for E½/ρ, 3 for E⅓/ρ">slope <input type="number" step="0.5" style={{ width: "54px" }} value={l()?.slope ?? slope()}
        onChange={e => { const v = +e.currentTarget.value; if (!isFinite(v)) return toast("Type a number for the slope."); setSlope(v); setLine({ slope: v }); }} /></label><Help k="slope" />
      <Show when={l()}><div class="seg"><button classList={{ on: l()!.above }} onClick={() => setLine({ above: true })}>Above passes</button><button classList={{ on: !l()!.above }} onClick={() => setLine({ above: false })}>Below</button></div></Show>
      <Show when={p.c.select}><button class="btn sm" onClick={() => act({ Select: { chart: p.c.id, select: null } })}>Clear</button></Show>
      </Show>
    </div>
  );
};
/** The chart as a standalone SVG file (white, with its family legend), at w × h pixels (96 per inch). */
export const chartSvgText = (id: number, w = 900, h = 640) => {
  const c = out()!.project.charts.find(x => x.id === id)!, o = chartOut(id)!, div = document.createElement("div");
  const dispose = render(() => <ChartSvg c={c} o={o} w={w} h={h} win={[o.x.lim[0], o.x.lim[1], o.y.lim[0], o.y.lim[1]]} print legend />, div);
  const svg = div.innerHTML;
  dispose();
  return `<?xml version="1.0" encoding="UTF-8"?>\n${svg}`;
};
/** The open chart as a PNG data URL at the resolution in Options. */
const pngOf = async (id: number) => {
  const text = chartSvgText(id), img = new Image(), k = Math.max(1, (opts().dpi || 300) / 96);
  img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(text)}`;
  await img.decode();
  const cv = document.createElement("canvas");
  cv.width = Math.round(900 * k); cv.height = Math.round(640 * k);
  const g = cv.getContext("2d")!; g.fillStyle = "#fff"; g.fillRect(0, 0, cv.width, cv.height); g.drawImage(img, 0, 0, cv.width, cv.height);
  return cv.toDataURL("image/png");
};
/** Copies the open chart to the Windows clipboard as a picture (through Rust). */
export const copyChart = async () => {
  const c = chart();
  if (!c || !chartOut(c.id)) return toast("Open a chart first.");
  try { await call("copy_png", { data: await pngOf(c.id) }); toast("Chart copied as a picture. Paste it into Word, PowerPoint or an email.", true); }
  catch (e) { toast(`Could not copy the picture: ${e}`); }
};
/** Saves the open chart as SVG (vector, editable text) or PNG at the resolution in Options. */
export const exportChart = async (fmt: "svg" | "png") => {
  const c = chart();
  if (!c || !chartOut(c.id)) return toast("Open a chart first.");
  const text = chartSvgText(c.id), name = `${c.name}.${fmt}`;
  const data = fmt === "png" ? await pngOf(c.id) : text;
  const path = await call<string | null>("save_file", { name, data });
  if (path) toast(`Saved ${path}`, true);
};
