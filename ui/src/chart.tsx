// The property chart: the engine lays out bubbles and family hulls in axis space (log10 on log
// axes); this file only maps them to the screen, draws the axes and ticks, and handles zoom, pan,
// hover, clicks and dragging the selection line or box. `ChartSvg` also draws printed charts.
import { For, Show, createSignal, createEffect, on, onCleanup, onMount } from "solid-js";
import { Chart, ChartOut, ChartSel, Bubble, famColor, fmt, range } from "./api";
import { act, out, failed, cur, setCur, openSheet, openMenu, chartOut, setChart, toast, db, chart, call, opts } from "./store";
import { Icon } from "./ui";
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

const title = (a: ChartOut["x"]) => `${a.name}${a.sym ? `, ${a.sym}` : ""}${a.unit ? ` (${a.unit})` : ""}`;
const lineOf = (s: ChartSel | null | undefined) => (s && "Line" in s ? s.Line : null);
const boxOf = (s: ChartSel | null | undefined) => (s && "Box" in s ? s.Box : null);

/** The chart as SVG at w × h pixels for the window `win`. */
export const ChartSvg = (p: { c: Chart; o: ChartOut; w: number; h: number; win: Win; sel?: ChartSel | null; print?: boolean; hover?: string; legend?: boolean }) => {
  const W = () => Math.max(10, p.w - M.l - M.r), H = () => Math.max(10, p.h - M.t - M.b - (p.legend ? LEG : 0));
  const X = (u: number) => M.l + ((u - p.win[0]) / (p.win[1] - p.win[0])) * W();
  const Y = (v: number) => M.t + (1 - (v - p.win[2]) / (p.win[3] - p.win[2])) * H();
  const tx = () => ticks(p.win[0], p.win[1], p.o.x.log), ty = () => ticks(p.win[2], p.win[3], p.o.y.log);
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
    return { d: `M${X(a)} ${Y(at(a))}L${X(b)} ${Y(at(b))}`, zone: `M${X(a)} ${Y(at(a))}L${X(b)} ${Y(at(b))}L${X(b)} ${Y(edge)}L${X(a)} ${Y(edge)}Z`, hx: X(u), hy: Y(v) };
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
        <For each={p.c.guides}>{n => { const g = () => guide(n); return <path d={g().d} stroke="#5B6878" stroke-width="1" stroke-dasharray="5 4" fill="none" />; }}</For>
        <For each={bubbles()}>{b => {
          const off = () => fail().has(b.id), on = () => b.id === cur() || b.id === p.hover;
          return <ellipse class="bub" data-id={b.id} cx={X(b.c[0])} cy={Y(b.c[1])} rx={Math.max(2.5, (b.r[0] / (p.win[1] - p.win[0])) * W())} ry={Math.max(2.5, (b.r[1] / (p.win[3] - p.win[2])) * H())}
            fill={off() ? "#C3C8CE" : famColor(b.fam)} fill-opacity={off() ? 0.35 : 0.6} stroke={on() ? "#0C1A2B" : off() ? "#A9AFB6" : famColor(b.fam)} stroke-width={on() ? 2 : 0.8} />;
        }}</For>
        <For each={p.o.hulls.filter(h => h.label)}>{h => <text x={X(h.at[0])} y={Y(h.at[1]) - 4} text-anchor="middle" font-size="12" font-weight="600" fill={famColor(h.fam)} stroke="#fff" stroke-width="3" paint-order="stroke" pointer-events="none">{h.name}</text>}</For>
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
        <text x={M.l + W() / 2} y={M.t + H() + 40} text-anchor="middle" font-size="12.5" font-weight="600">{title(p.o.x)}</text>
        <text transform={`translate(15 ${M.t + H() / 2}) rotate(-90)`} text-anchor="middle" font-size="12.5" font-weight="600">{title(p.o.y)}</text>
      </g>
      <For each={p.c.guides}>{n => { const g = guide(n); const m = g.d.match(/L([\d.-]+) ([\d.-]+)/); const [x, y] = m ? [+m[1], +m[2]] : [0, 0];
        return <Show when={y > M.t && y < M.t + H()}><text x={Math.min(x, M.l + W()) - 4} y={y - 5} text-anchor="end" font-size="10" fill="#5B6878">slope {fmt(n)}</text></Show>; }}</For>
      <Show when={p.legend}><g font-size="11" fill="#222"><For each={p.o.families.filter(f => !p.c.hidden.includes(f))}>{(f, i) => <g transform={`translate(${M.l + i() * Math.min(150, W() / 7)} ${p.h - 8})`}>
        <rect y="-9" width="11" height="11" fill={famColor(p.o.families.indexOf(f))} fill-opacity=".7" /><text x="15">{f}</text></g>}</For></g></Show>
      <Show when={line() && lineVal()}>{_ => { const l = line()!; return <text x={Math.min(Math.max(l.hx + 10, M.l + 6), M.l + W() - 180)} y={Math.max(M.t + 14, l.hy - 12)} font-size="11.5" font-weight="600" fill="#5A3C9C" stroke="#fff" stroke-width="3" paint-order="stroke">{lineVal()!.name} = {fmt(lineVal()!.value)}</text>; }}</Show>
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
const keyOf = (c: Chart) => `${c.x}|${c.y}|${c.logX}|${c.logY}`;
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
  const [win, setWinRaw] = createSignal<Win>(wins.get(p.c.id)?.key === keyOf(p.c) ? wins.get(p.c.id)!.w : lim());
  const setWin = (w: Win) => { wins.set(p.c.id, { key: keyOf(p.c), w }); setWinRaw(w); };
  createEffect(on(() => keyOf(p.c), (k, prev) => { if (prev !== undefined && k !== prev) setWin(lim()); }));
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
      if ((dx / rx) ** 2 + (dy / ry) ** 2 <= 1 && rx * ry < area) { best = b; area = rx * ry; }
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
    if ((t.classList.contains("selh") || t.classList.contains("sell")) && l) {
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
          <span>{x.x.name}: {range(t().b.x[0], t().b.x[1])} {x.x.unit}</span><span>{x.y.name}: {range(t().b.y[0], t().b.y[1])} {x.y.unit}</span>
          <Show when={fails() && out()!.results.stages.length}><span classList={{ ok: fails()!.pass, late: !fails()!.pass }}>{fails()!.pass ? `Passes${fails()!.rank ? ` · rank ${fails()!.rank}` : ""}` : `Fails: ${fails()!.reasons[0]}`}</span></Show>
          <span class="muted small">Click to select · double-click for the datasheet</span>
        </div>); }}</Show>
    </div>
  );
};

/** The chart's toolbar: axes, scales, tools, the line's slope and side, envelopes, labels, fit. */
export const ChartBar = (p: { c: Chart }) => {
  const props = () => db().props.filter(q => q.kind === "range" && db().materials.some(m => m.props[q.key]));
  const AxisPick = (q: { k: "x" | "y" }) => <span class="axpick"><b>{q.k.toUpperCase()}</b>
    <select value={p.c[q.k]} onChange={e => { const v = e.currentTarget.value, lg = db().props.find(x => x.key === v)?.log ?? true; setChart(c => { c[q.k] = v; if (q.k === "x") c.logX = lg; else c.logY = lg; c.select = null; }); }}>
      <For each={props()}>{x => <option value={x.key}>{x.name}</option>}</For></select>
    <button class="rb fb" classList={{ on: q.k === "x" ? p.c.logX : p.c.logY }} title="Logarithmic axis" onClick={() => setChart(c => { if (q.k === "x") c.logX = !c.logX; else c.logY = !c.logY; c.select = null; })}>log</button></span>;
  const l = () => lineOf(p.c.select);
  const setLine = (patch: Partial<{ slope: number; above: boolean }>) => { const x = l(); if (x) act({ Select: { chart: p.c.id, select: { Line: { ...x, ...patch } } } }); };
  return (
    <div class="cbar">
      <AxisPick k="y" /><button class="rb fb" title="Swap the axes" onClick={() => setChart(c => { [c.x, c.y, c.logX, c.logY] = [c.y, c.x, c.logY, c.logX]; c.select = null; })}>⇅</button><AxisPick k="x" />
      <span class="sep" />
      <div class="seg tools"><For each={[["pan", "Pan", "Drag to move, wheel to zoom, click a bubble"], ["line", "Line", "Click to place a selection line"], ["box", "Box", "Drag a selection box"]] as const}>{([k, n, t]) =>
        <button classList={{ on: tool() === k }} title={t} onClick={() => setTool(k)}>{n}</button>}</For></div>
      <label class="inline" title="Slope of the selection line: 1 for E/ρ, 2 for E^½/ρ, 3 for E^⅓/ρ">slope <input type="number" step="0.5" style={{ width: "58px" }} value={l()?.slope ?? slope()}
        onChange={e => { const v = +e.currentTarget.value; if (!isFinite(v)) return toast("Type a number for the slope."); setSlope(v); setLine({ slope: v }); }} /></label>
      <Show when={l()}><div class="seg"><button classList={{ on: l()!.above }} onClick={() => setLine({ above: true })}>Above passes</button><button classList={{ on: !l()!.above }} onClick={() => setLine({ above: false })}>Below</button></div></Show>
      <Show when={p.c.select}><button class="btn sm" onClick={() => act({ Select: { chart: p.c.id, select: null } })}>Clear selection</button></Show>
      <span class="grow" />
      <select title="Envelopes" value={p.c.hull} onChange={e => { const v = e.currentTarget.value; setChart(c => { c.hull = v; }); }}><option value="family">Envelopes: families</option><option value="class">Envelopes: classes</option><option value="none">No envelopes</option></select>
      <button class="rb fb" classList={{ on: p.c.labels }} title="Material labels" onClick={() => setChart(c => { c.labels = !c.labels; })}><Icon n="tag" /></button>
      <button class="rb fb" classList={{ on: p.c.guides.length > 0 }} title="Guide lines of slope 1, 2 and 3" onClick={() => setChart(c => { c.guides = c.guides.length ? [] : [1, 2, 3]; })}><Icon n="index" /></button>
      <button class="rb fb" title="Zoom in" onClick={() => zoomChart(1.4)}><Icon n="zin" /></button><button class="rb fb" title="Zoom out" onClick={() => zoomChart(1 / 1.4)}><Icon n="zout" /></button><button class="rb fb" title="Fit all materials" onClick={fitChart}><Icon n="fit" /></button>
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
/** Saves the open chart as SVG (vector, editable text) or PNG at the resolution in Options. */
export const exportChart = async (fmt: "svg" | "png") => {
  const c = chart();
  if (!c || !chartOut(c.id)) return toast("Open a chart first.");
  const text = chartSvgText(c.id), name = `${c.name}.${fmt}`;
  let data = text;
  if (fmt === "png") {
    const img = new Image(), k = Math.max(1, (opts().dpi || 300) / 96);
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(text)}`;
    await img.decode();
    const cv = document.createElement("canvas");
    cv.width = Math.round(900 * k); cv.height = Math.round(640 * k);
    const g = cv.getContext("2d")!; g.fillStyle = "#fff"; g.fillRect(0, 0, cv.width, cv.height); g.drawImage(img, 0, 0, cv.width, cv.height);
    data = cv.toDataURL("image/png");
  }
  const path = await call<string | null>("save_file", { name, data });
  if (path) toast(`Saved ${path}`, true);
};
