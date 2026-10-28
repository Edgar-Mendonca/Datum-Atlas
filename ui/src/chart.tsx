// The property chart: the engine lays out bubbles and family hulls in axis space (log10 on log
// axes); this file only maps them to the screen, draws the axes and ticks, and handles zoom, pan,
// hover, clicks and dragging the selection line or box. `ChartSvg` also draws printed charts.
import { For, Show, createSignal, onCleanup, onMount } from "solid-js";
import { Chart, ChartOut, ChartSel, Bubble, Line, famColor, fmt, range, lineOf, boxOf, polyOf, goalId } from "./api";
import { grid, setGrid } from "./store";
import { act, out, failed, cur, setCur, openSheet, openMenu, chartOut, setChart, toast, db, chart, call, opts, conv, unitOf, toShown, goals, setGoal, activeOf, setActiveOf, setSels, P, chartProps, propOf, hoverId, setHoverId, setChartId } from "./store";
import { Icon } from "./ui";
import { Help } from "./help";
import { render } from "solid-js/web";

export type Win = [number, number, number, number];
const M = { l: 66, r: 16, t: 14, b: 48 };
const LEG = 24;
/** The mono style: greys, and a dash per family. */
const MONO = ["#1F2933", "#52606D", "#7B8794", "#3E4C59", "#616E7C", "#9AA5B1", "#323F4B"];
const DASH = ["", "6 3", "2 2", "8 3 2 3", "4 4", "1 3", "10 4"];
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
/** The chart as SVG at w × h pixels for the window `win`. `sels` previews selections being dragged,
 *  `draft` a shape being drawn (axis space), `act` the selection being edited. */
export const ChartSvg = (p: { c: Chart; o: ChartOut; w: number; h: number; win: Win; sels?: ChartSel[]; draft?: [number, number][]; act?: number; print?: boolean; hover?: string; legend?: boolean; labOff?: { id: string; off: [number, number] } | null }) => {
  const W = () => Math.max(10, p.w - M.l - M.r), H = () => Math.max(10, p.h - M.t - M.b - (p.legend ? LEG : 0));
  const X = (u: number) => M.l + ((u - p.win[0]) / (p.win[1] - p.win[0])) * W();
  const Y = (v: number) => M.t + (1 - (v - p.win[2]) / (p.win[3] - p.win[2])) * H();
  const bar = () => p.c.kind === "bar";
  const tx = () => (bar() ? { major: [], minor: [] } : ticksFor(p.win[0], p.win[1], p.o.x.log, p.o.x.key)), ty = () => ticksFor(p.win[2], p.win[3], p.o.y.log, p.o.y.key);
  const hullLabels = () => { const hs = p.o.hulls.filter(h => h.label), ys = nudge(hs.map(h => ({ x: X(h.at[0]), y: Y(h.at[1]) - 4, w: h.name.length * 6.8 }))); return hs.map((h, i) => ({ h, y: ys[i] })); };
  const fail = () => failed();
  // Styles: classic family colours, publication (finer), or mono (greys and dashes for print).
  const mono = () => p.c.style === "mono", pub = () => p.c.style === "publication";
  const col = (f: number) => (mono() ? MONO[f % MONO.length] : famColor(f));
  const dash = (f: number) => (mono() ? DASH[f % DASH.length] : undefined);
  const sels = () => p.sels ?? p.c.sels;
  const ax = (v: number, log: boolean) => (log ? Math.log10(v) : v);
  const clip = `clip${p.c.id}${p.print ? "p" : "s"}`;
  const lineVal = (k: number) => out()?.results.lines.find(l => l.chart === p.c.id && l.k === k);
  /** Each selection line across the window, the side that passes, and where its index is written. */
  const lines = () => {
    const taken = hullLabels().map(({ h, y }) => ({ x: X(h.at[0]), y, w: h.name.length * 6.8 }));
    return sels().flatMap((s, k) => {
      const l = lineOf(s); if (!l) return [];
      const [u, v] = [ax(l.x, p.o.x.log), ax(l.y, p.o.y.log)], at = (x: number) => v + l.slope * (x - u);
      const [a, b] = [p.win[0], p.win[1]], edge = l.above ? p.win[3] : p.win[2];
      // Where to write the index: a point along the line inside the plot, not on a family label or another line's label.
      const inside = (t: number) => { const x = a + (b - a) * t, y = Y(at(x)); return y > M.t + 16 && y < M.t + H() - 8 ? [X(x), y] : null; };
      const w = ((lineVal(k)?.name.length ?? 6) + 9) * 7;
      const free = (q: number[] | null) => q && !taken.some(l => q[0] + 8 < l.x + l.w / 2 + 4 && q[0] + 8 + w > l.x - l.w / 2 - 4 && Math.abs(q[1] - 10 - l.y) < 16);
      const cand = [0.7, 0.6, 0.8, 0.5, 0.9, 0.4, 0.3, 0.2, 0.85, 0.15].map(inside);
      const lab = cand.find(free) ?? cand.find(Boolean) ?? [X(u), Y(v)];
      taken.push({ x: lab[0] + 8 + w / 2, y: lab[1] - 10, w });
      return [{ k, l, d: `M${X(a)} ${Y(at(a))}L${X(b)} ${Y(at(b))}`, zone: `M${X(a)} ${Y(at(a))}L${X(b)} ${Y(at(b))}L${X(b)} ${Y(edge)}L${X(a)} ${Y(edge)}Z`, hx: X(u), hy: Y(v), lx: lab[0], ly: lab[1] }];
    });
  };
  const pts = (q: [number, number][]) => q.map(([x, y]) => `${X(ax(x, p.o.x.log)).toFixed(1)} ${Y(ax(y, p.o.y.log)).toFixed(1)}`).join("L");
  const on = (k: number) => !p.print && p.act === k && sels().length > 1;
  const guide = (n: number) => { const cu = (p.win[0] + p.win[1]) / 2, cv = (p.win[2] + p.win[3]) / 2, at = (x: number) => cv + n * (x - cu); return { d: `M${X(p.win[0])} ${Y(at(p.win[0]))}L${X(p.win[1])} ${Y(at(p.win[1]))}`, n }; };
  const bubbles = () => [...p.o.bubbles].sort((a, b) => b.r[0] * b.r[1] - a.r[0] * a.r[1]);
  /** Material labels (all with Labels on, the five best with "Best" on), each placed where it does not
   *  overlap another label: around its bubble, further out if needed, with a leader line when far.
   *  A label moved by hand stays where it was put. */
  const placed = () => {
    if (bar()) return [];
    const rank = new Map((out()?.results.rows ?? []).filter(r => r.pass && r.rank && r.rank <= 5).map(r => [r.id, r.rank!] as const));
    const show = p.o.bubbles.filter(b => p.c.labels || (p.c.best && rank.has(b.id))).sort((a, b) => (rank.get(a.id) ?? 99) - (rank.get(b.id) ?? 99));
    const boxes = hullLabels().map(({ h, y }) => ({ x: X(h.at[0]) - h.name.length * 3.4, y: y - 11, w: h.name.length * 6.8, h: 14 }));
    const hits = (q: { x: number; y: number; w: number; h: number }) => boxes.some(b => q.x < b.x + b.w && q.x + q.w > b.x && q.y < b.y + b.h && q.y + q.h > b.y);
    const inside = (q: { x: number; y: number; w: number; h: number }) => q.x > M.l + 2 && q.x + q.w < M.l + W() - 2 && q.y > M.t + 2 && q.y + q.h < M.t + H() - 2;
    return show.flatMap(b => {
      const bx = X(b.c[0]), by = Y(b.c[1]); if (bx < M.l || bx > M.l + W() || by < M.t || by > M.t + H()) return [];
      const rx = Math.max(2.5, (b.r[0] / (p.win[1] - p.win[0])) * W()), ry = Math.max(2.5, (b.r[1] / (p.win[3] - p.win[2])) * H());
      const best = rank.has(b.id), t = b.name.split(/[,(]/)[0].trim(), w = (t.length + (best ? 4 : 0)) * (best ? 6.2 : 5.5), h = 12;
      const mv = p.labOff?.id === b.id ? p.labOff.off : p.c.moved[b.id];
      let at: [number, number] | null = mv ? [bx + mv[0], by + mv[1]] : null;
      if (!at) for (const d of [0, 12, 26, 44]) { for (const [ux, uy] of [[1, -1], [1, 1], [-1, -1], [-1, 1], [0, -1], [0, 1], [1, 0], [-1, 0]]) {
        const x = bx + ux * (rx * 0.7 + 3 + d) - (ux < 0 ? w : ux === 0 ? w / 2 : 0), y = by + uy * (ry * 0.7 + 3 + d) + (uy > 0 ? h - 2 : uy === 0 ? 4 : -2);
        const q = { x, y: y - h + 2, w, h }; if (inside(q) && !hits(q)) { at = [x, y]; break; } } if (at) break; }
      if (!at) at = [bx + rx + 3, by - 3];
      boxes.push({ x: at[0], y: at[1] - h + 2, w, h });
      const cx = Math.max(at[0], Math.min(bx, at[0] + w)), cy = Math.max(at[1] - h + 2, Math.min(by, at[1] + 2));
      const far = Math.hypot(cx - bx, cy - by) > Math.max(rx, ry) + 10;
      return [{ id: b.id, t, x: at[0], y: at[1], bx, by, best, rank: rank.get(b.id), lead: far ? [bx, by, cx, cy] as [number, number, number, number] : null }];
    });
  };
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
        <For each={p.o.hulls}>{h => <path class="hull" d={`M${h.pts.map(q => `${X(q[0]).toFixed(1)} ${Y(q[1]).toFixed(1)}`).join("L")}Z`} fill={col(h.fam)} fill-opacity={pub() ? 0.06 : mono() ? 0.05 : 0.13} stroke={col(h.fam)} stroke-opacity={mono() ? 0.9 : 0.55} stroke-width={pub() ? 0.9 : 1.2} stroke-dasharray={dash(h.fam)} />}</For>
        <For each={lines()}>{l => <path d={l.zone} fill="#6C4AB6" fill-opacity=".06" />}</For>
        <For each={bar() ? [] : p.c.guides}>{n => { const g = () => guide(n); return <path d={g().d} stroke="#5B6878" stroke-width="1" stroke-dasharray="5 4" fill="none" />; }}</For>
        <For each={bubbles()}>{b => {
          const off = () => fail().has(b.id), on = () => b.id === cur() || b.id === p.hover || (!p.print && b.id === hoverId());
          if (bar()) { const x0 = X(b.c[0] - b.r[0]), x1 = X(b.c[0] + b.r[0]), y0 = Y(b.c[1] + Math.max(b.r[1], 0.004 * (p.win[3] - p.win[2]))), y1 = Y(b.c[1] - Math.max(b.r[1], 0.004 * (p.win[3] - p.win[2])));
            return <rect class="bub" data-id={b.id} x={x0} y={y0} width={Math.max(1.5, x1 - x0)} height={Math.max(2, y1 - y0)} fill={off() ? "#C3C8CE" : col(b.fam)} fill-opacity={off() ? 0.4 : 0.7} stroke={on() ? "#0C1A2B" : "none"} stroke-width={on() ? 1.6 : 0} />; }
          return <ellipse class="bub" data-id={b.id} cx={X(b.c[0])} cy={Y(b.c[1])} rx={Math.max(2.5, (b.r[0] / (p.win[1] - p.win[0])) * W())} ry={Math.max(2.5, (b.r[1] / (p.win[3] - p.win[2])) * H())}
            fill={off() ? "#C3C8CE" : col(b.fam)} fill-opacity={off() ? 0.35 : pub() ? 0.45 : mono() ? 0.3 : 0.6} stroke={on() ? "#0C1A2B" : off() ? "#A9AFB6" : col(b.fam)} stroke-width={on() ? 2 : 0.8} stroke-dasharray={on() ? undefined : dash(b.fam)} />;
        }}</For>
        <For each={hullLabels()}>{({ h, y }) => <text x={X(h.at[0])} y={y} text-anchor="middle" font-size="12" font-weight="600" fill={mono() ? "#222" : col(h.fam)} stroke="#fff" stroke-width="3" paint-order="stroke" pointer-events="none">{h.name}</text>}</For>
        <For each={placed()}>{l => <>
          <Show when={l.lead}><line x1={l.lead![0]} y1={l.lead![1]} x2={l.lead![2]} y2={l.lead![3]} stroke="#5B6878" stroke-width=".8" /></Show>
          <text class="mlab" data-id={l.id} data-ox={String(l.x - l.bx)} data-oy={String(l.y - l.by)} x={l.x} y={l.y} font-size={l.best ? "10.5" : "9.5"} font-weight={l.best ? "600" : "400"} fill={l.best ? "#0C1A2B" : "#3B4859"} stroke="#fff" stroke-width="2.5" paint-order="stroke"
            pointer-events={p.print ? "none" : "all"}>{l.best && l.rank ? `${l.rank} · ` : ""}{l.t}</text></>}</For>
        <For each={sels()}>{(s, k) => <>
          <Show when={boxOf(s)}>{bx => <rect class="selbox" data-k={k()} x={Math.min(X(ax(bx().x0, p.o.x.log)), X(ax(bx().x1, p.o.x.log)))} y={Math.min(Y(ax(bx().y0, p.o.y.log)), Y(ax(bx().y1, p.o.y.log)))}
            width={Math.abs(X(ax(bx().x1, p.o.x.log)) - X(ax(bx().x0, p.o.x.log)))} height={Math.abs(Y(ax(bx().y1, p.o.y.log)) - Y(ax(bx().y0, p.o.y.log)))} fill="#6C4AB6" fill-opacity=".07" stroke="#6C4AB6" stroke-width={on(k()) ? 2.6 : 1.6} stroke-dasharray="6 3" />}</Show>
          <Show when={polyOf(s)}>{q => <path class="selpoly" data-k={k()} d={`M${pts(q().pts)}Z`} fill="#6C4AB6" fill-opacity=".07" stroke="#6C4AB6" stroke-width={on(k()) ? 2.6 : 1.6} stroke-dasharray="6 3" />}</Show>
        </>}</For>
        <For each={lines()}>{l => <>
          <path class="sell" data-k={l.k} d={l.d} stroke="transparent" stroke-width="12" fill="none" />
          <path d={l.d} stroke="#6C4AB6" stroke-width={on(l.k) ? 3.4 : 2.2} fill="none" pointer-events="none" />
          <Show when={!p.print}><circle class="selh" data-k={l.k} cx={l.hx} cy={l.hy} r="6.5" fill={on(l.k) ? "#6C4AB6" : "#fff"} stroke="#6C4AB6" stroke-width="2.2" /></Show>
        </>}</For>
        <Show when={p.draft?.length}><path d={`M${p.draft!.map(([u, v]) => `${X(u)} ${Y(v)}`).join("L")}`} fill="none" stroke="#6C4AB6" stroke-width="1.6" stroke-dasharray="4 3" />
          <For each={p.draft}>{([u, v], i) => <circle cx={X(u)} cy={Y(v)} r={i() === 0 ? 5 : 3} fill={i() === 0 ? "#fff" : "#6C4AB6"} stroke="#6C4AB6" stroke-width="1.6" />}</For></Show>
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
      <For each={lines()}>{l => <Show when={lineVal(l.k)}>{v => <text x={Math.min(Math.max(l.lx + 8, M.l + 6), M.l + W() - 170)} y={l.ly - 10} font-size="11.5" font-weight="600" fill="#5A3C9C" stroke="#fff" stroke-width="3" paint-order="stroke">{v().name} = {fmt(v().value)}</text>}</Show>}</For>
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
export const [tool, setTool] = createSignal<"pan" | "line" | "box" | "poly">("pan");
export const [slope, setSlope] = createSignal(1);
/** Fit and zoom of each chart on screen, so the buttons act on the open one. */
const views = new Map<number, { fit: () => void; zoom: (f: number) => void }>();
export const fitChart = () => views.get(chart()?.id ?? -1)?.fit();
export const zoomChart = (f: number) => views.get(chart()?.id ?? -1)?.zoom(f);
/** A new selection line on a chart, through (x, y) in stored units; it becomes the one being edited. */
export const addLine = (c: Chart, l: Line) => { setActiveOf(c.id, c.sels.length); return setSels(c, [...c.sels, { Line: l }]); };
/** Removes selection k (or all). */
export const removeSel = (c: Chart, k?: number) => setSels(c, k === undefined ? [] : c.sels.filter((_, i) => i !== k));
/** The line being edited on a chart, if the active selection is a line. */
export const activeLine = (c: Chart) => lineOf(c.sels[activeOf(c)]);
/** A line through the middle of the chart's view, for "Add line" and the ribbon. */
const middle = (c: Chart, n: number): Line => { const w = wins.get(c.id)?.w, o = chartOut(c.id), lim = w ?? (o ? [o.x.lim[0], o.x.lim[1], o.y.lim[0], o.y.lim[1]] : [3, 4, 0, 2]);
  const v = (u: number, log: boolean) => (log ? 10 ** u : u); return { slope: n, x: v((lim[0] + lim[1]) / 2, c.logX), y: v((lim[2] + lim[3]) / 2, c.logY), above: true }; };
export const newLine = (c: Chart, n = activeLine(c)?.slope ?? slope()) => addLine(c, middle(c, n));
/** Moves the active line so its index has the value typed (stored units). */
export const passThrough = (c: Chart) => {
  const k = activeOf(c), l = lineOf(c.sels[k]); if (!l) return toast("Pick a selection line first.");
  const name = out()?.results.lines.find(x => x.chart === c.id && x.k === k)?.name ?? "the index";
  const t = prompt(`Value of ${name} the line should pass through (in the stored units: kg/m³, GPa, MPa…):`); if (t === null) return;
  const value = +t.replace(",", "."); if (!isFinite(value)) return toast("Type a number.");
  act({ PassThrough: { chart: c.id, k, value } });
};

export const ChartView = (p: { c: Chart }) => {
  let box!: HTMLDivElement;
  const [size, setSize] = createSignal<[number, number]>([800, 560]);
  const o = () => chartOut(p.c.id);
  const lim = (): Win => { const x = o()!; return [x.x.lim[0], x.x.lim[1], x.y.lim[0], x.y.lim[1]]; };
  // The zoom window belongs to the axes it was made for: new axes (or a bar chart) start from the full view.
  const [bump, setBump] = createSignal(0);
  const win = (): Win => { bump(); const w = wins.get(p.c.id); return w && w.key === keyOf(p.c) ? w.w : lim(); };
  const setWin = (w: Win) => { wins.set(p.c.id, { key: keyOf(p.c), w }); setBump(b => b + 1); };
  const [drag, setDrag] = createSignal<ChartSel[] | undefined>(undefined);
  const [draft, setDraft] = createSignal<[number, number][]>([]);
  const [tip, setTip] = createSignal<{ x: number; y: number; b: Bubble } | null>(null);
  const [labOff, setLabOff] = createSignal<{ id: string; off: [number, number] } | null>(null);
  onMount(() => {
    const ro = new ResizeObserver(() => setSize([box.clientWidth, box.clientHeight])); ro.observe(box); onCleanup(() => ro.disconnect());
    views.set(p.c.id, { fit: () => setWin(lim()), zoom: f => { const w = win(), cx = (w[0] + w[1]) / 2, cy = (w[2] + w[3]) / 2; setWin([cx - (cx - w[0]) / f, cx + (w[1] - cx) / f, cy - (cy - w[2]) / f, cy + (w[3] - cy) / f]); } });
  });
  onCleanup(() => views.delete(p.c.id));
  const W = () => size()[0] - M.l - M.r, H = () => size()[1] - M.t - M.b;
  /** Mouse → axis space. */
  const at = (e: MouseEvent): [number, number] => { const r = box.getBoundingClientRect(), w = win(); return [w[0] + ((e.clientX - r.left - M.l) / W()) * (w[1] - w[0]), w[2] + (1 - (e.clientY - r.top - M.t) / H()) * (w[3] - w[2])]; };
  const val = (u: number, log: boolean) => (log ? 10 ** u : u);
  const store = (q: [number, number]): [number, number] => [val(q[0], o()!.x.log), val(q[1], o()!.y.log)];
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
  /** Closes the shape being drawn and adds it as a selection. */
  const closeDraft = () => { const d = draft(); setDraft([]); if (d.length >= 3) { setActiveOf(p.c.id, p.c.sels.length); setSels(p.c, [...p.c.sels, { Poly: { pts: d.map(store) } }]); } else if (d.length) toast("A shape needs at least three corners."); };
  const down = (e: MouseEvent) => {
    if (e.button !== 0 || !o()) return;
    box.focus();
    const t = e.target as Element, c = p.c, x = o()!, k = +(t.getAttribute("data-k") ?? -1);
    const start = at(e), w0 = win(), [sx, sy] = [e.clientX, e.clientY];
    let moved = false, mv: (m: MouseEvent) => void, up: (m: MouseEvent) => void;
    const pan = (m: MouseEvent) => { const dx = m.clientX - sx, dy = m.clientY - sy; if (!moved && Math.hypot(dx, dy) < 3) return; moved = true; const ku = (w0[1] - w0[0]) / W(), kv = (w0[3] - w0[2]) / H(); setWin([w0[0] - dx * ku, w0[1] - dx * ku, w0[2] + dy * kv, w0[3] + dy * kv]); };
    const pick = (m: MouseEvent) => { const b = hit(m); setCur(b ? b.id : ""); };
    if (t.classList.contains("mlab")) {
      // Move a material label; it keeps its place (a double-click puts it back).
      const id = t.getAttribute("data-id")!, o0: [number, number] = [+(t.getAttribute("data-ox") ?? 0), +(t.getAttribute("data-oy") ?? 0)];
      mv = m => { moved = true; setLabOff({ id, off: [o0[0] + m.clientX - sx, o0[1] + m.clientY - sy] }); };
      up = () => { const l = labOff(); setLabOff(null); if (moved && l) setChart(c => { c.moved = { ...c.moved, [id]: l.off }; }); };
    }
    else if (c.kind === "bar") { mv = pan; up = m => { if (!moved) pick(m); }; }
    else if (k >= 0 && (t.classList.contains("selh") || t.classList.contains("sell")) && lineOf(c.sels[k])) {
      // Move a selection line: it keeps its slope and passes through the mouse.
      const l = lineOf(c.sels[k])!;
      setActiveOf(c.id, k);
      mv = m => { moved = true; const [u, v] = at(m); setDrag(c.sels.map((s, i) => (i === k ? { Line: { ...l, x: val(u, x.x.log), y: val(v, x.y.log) } } : s))); };
      up = m => { const d = drag(); setDrag(undefined); if (moved && d) setSels(c, d); else pick(m); };
    } else if (tool() === "box") {
      // One box per chart: a new one replaces the old.
      const others = c.sels.filter(s => !boxOf(s));
      mv = m => { moved = true; const [u, v] = at(m); setDrag([...others, { Box: { x0: val(start[0], x.x.log), x1: val(u, x.x.log), y0: val(start[1], x.y.log), y1: val(v, x.y.log) } }]); };
      up = () => { const d = drag(); setDrag(undefined); if (moved && d) { setActiveOf(c.id, d.length - 1); setSels(c, d); } };
    } else if (tool() === "poly") {
      mv = () => {};
      up = m => {
        const q = at(m), d = draft(), w = win(), near = d.length >= 3 && Math.hypot((q[0] - d[0][0]) / (w[1] - w[0]) * W(), (q[1] - d[0][1]) / (w[3] - w[2]) * H()) < 9;
        if (near) closeDraft(); else setDraft([...d, q]);
      };
    } else {
      // Pan; a click without moving picks a bubble, or places the active line with the line tool.
      mv = pan;
      up = m => {
        if (moved) return;
        if (tool() === "line") {
          const [u, v] = at(m), ak = activeOf(c), l = lineOf(c.sels[ak]);
          const nl = { slope: l?.slope ?? slope(), x: val(u, x.x.log), y: val(v, x.y.log), above: l?.above ?? true };
          if (l) setSels(c, c.sels.map((s, i) => (i === ak ? { Line: nl } : s))); else addLine(c, nl);
          return;
        }
        pick(m);
      };
    }
    const end = (m: MouseEvent) => { removeEventListener("mousemove", mv); removeEventListener("mouseup", end); up(m); };
    addEventListener("mousemove", mv); addEventListener("mouseup", end);
  };
  /** Arrow keys move the active line (Shift: ten times further); Delete removes the active selection. */
  const keys = (e: KeyboardEvent) => {
    const c = p.c, k = activeOf(c), l = lineOf(c.sels[k]), x = o();
    if (e.key === "Escape" && draft().length) { setDraft([]); return e.preventDefault(); }
    if (e.key === "Enter" && draft().length) { closeDraft(); return e.preventDefault(); }
    if ((e.key === "Delete" || e.key === "Backspace") && c.sels.length) { e.preventDefault(); return removeSel(c, k); }
    if (!l || !x || !["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(e.key)) return;
    e.preventDefault();
    const w = win(), f = e.shiftKey ? 0.1 : 0.01, dy = (e.key === "ArrowUp" ? 1 : e.key === "ArrowDown" ? -1 : 0) * f * (w[3] - w[2]), dx = (e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0) * f * (w[1] - w[0]);
    const ax = (v: number, log: boolean) => (log ? Math.log10(v) : v);
    setSels(c, c.sels.map((s, i) => (i === k ? { Line: { ...l, x: val(ax(l.x, x.x.log) + dx, x.x.log), y: val(ax(l.y, x.y.log) + dy, x.y.log) } } : s)));
  };
  const move = (e: MouseEvent) => { if (e.buttons) { setHoverId(""); return setTip(null); } const b = hit(e); setTip(b ? { x: e.clientX, y: e.clientY, b } : null); setHoverId(b?.id ?? ""); };
  const menu = (e: MouseEvent) => {
    const b = hit(e), c = p.c;
    openMenu(e, [
      ...(b ? [{ label: `Datasheet: ${b.name}`, icon: "datasheet", run: () => openSheet(b.id) }, "-" as const] : []),
      { label: "Fit", icon: "fit", run: fitChart }, { label: "Zoom In", icon: "zin", run: () => zoomChart(1.4) }, { label: "Zoom Out", icon: "zout", run: () => zoomChart(1 / 1.4) }, "-",
      { label: "New Selection Line Here", icon: "index", run: () => { const [u, v] = at(e), x = o()!; addLine(c, { slope: activeLine(c)?.slope ?? slope(), x: val(u, x.x.log), y: val(v, x.y.log), above: true }); } },
      { label: "Remove This Selection", icon: "del", off: !c.sels.length, run: () => removeSel(c, activeOf(c)) },
      { label: "Remove All Selections", off: !c.sels.length, run: () => removeSel(c) },
      { label: "Labels", on: c.labels, run: () => setChart(c => { c.labels = !c.labels; }) },
    ]);
  };
  const fails = () => out()?.results.rows.find(r => r.id === tip()?.b.id);
  return (
    <div class="cbox" tabindex="0" ref={box} classList={{ tline: tool() === "line", tbox: tool() === "box" || tool() === "poly" }} onWheel={wheel} onMouseDown={down} onMouseMove={move} onMouseLeave={() => { setTip(null); setHoverId(""); }} onKeyDown={keys} onFocus={() => setChartId(p.c.id)}
      onDblClick={e => { if (tool() === "poly") return closeDraft(); const id = (e.target as Element).getAttribute("data-id"); if ((e.target as Element).classList.contains("mlab") && id && p.c.moved[id]) return setChart(c => { const m = { ...c.moved }; delete m[id]; c.moved = m; }); const b = hit(e); if (b) openSheet(b.id); }} onContextMenu={menu}>
      <Show when={o()} fallback={<div class="gwait">Laying out the chart…</div>}>{x => <ChartSvg c={p.c} o={x()} w={size()[0]} h={size()[1]} win={win()} sels={drag()} draft={draft()} act={activeOf(p.c)} hover={tip()?.b.id} labOff={labOff()} />}</Show>
      <Show when={tip()}>{t => { const x = o()!; return (
        <div class="gtip" style={{ left: `${t().x + 14}px`, top: `${t().y + 12}px` }}>
          <b>{t().b.name}</b><span class="muted">{db().materials.find(m => m.id === t().b.id)?.family.join(" › ")}</span>
          <Show when={p.c.kind !== "bar"}><span>{x.x.name}: {range(toShown(x.x.key, t().b.x[0]), toShown(x.x.key, t().b.x[1]))} {unitOf(x.x.key)}</span></Show>
          <span>{x.y.name}: {range(toShown(x.y.key, t().b.y[0]), toShown(x.y.key, t().b.y[1]))} {unitOf(x.y.key)}</span>
          <Show when={fails() && out()!.results.stages.length}><span classList={{ ok: fails()!.pass, late: !fails()!.pass }}>{fails()!.pass ? `Passes${fails()!.rank ? ` · rank ${fails()!.rank}` : ""}` : `Fails: ${fails()!.reasons[0]}`}</span></Show>
          <span class="muted small">Click to select · double-click for the datasheet</span>
        </div>); }}</Show>
      <Show when={tool() === "poly"}><div class="gwait hint">{draft().length ? "Click to add corners; click the first corner, double-click or press Enter to close; Esc cancels." : "Click to place the first corner of the shape."}</div></Show>
    </div>
  );
};

/** An axis: the property and its log switch. Changing it removes the goal and the selections. */
const AxisPick = (p: { c: Chart; k: "x" | "y" }) => {
  const props = chartProps;
  return <span class="axpick"><b>{p.k.toUpperCase()}</b>
    <select value={p.c[p.k]} onChange={e => { const v = e.currentTarget.value, lg = propOf(v)?.log ?? true; setChart(c => { c[p.k] = v; if (p.k === "x") c.logX = lg; else c.logY = lg; c.sels = []; c.goal = ""; }); }}>
      <For each={props()}>{x => <option value={x.key} selected={x.key === p.c[p.k]}>{x.name}</option>}</For></select>
    <button class="rb fb" classList={{ on: p.k === "x" ? p.c.logX : p.c.logY }} title="Logarithmic axis" onClick={() => setChart(c => { if (p.k === "x") c.logX = !c.logX; else c.logY = !c.logY; c.sels = []; })}>log</button></span>;
};

/** The first toolbar row: the design goal, bubbles or bars, and what the chart shows. */
export const ChartBar = (p: { c: Chart }) => {
  const groups = () => [...new Set(goals().filter(g => !g.library).map(g => g.group))];
  const goal = () => goals().find(g => g.id === goalId(p.c));
  return (
    <div class="cbar">
      <span class="goalpick"><b>Goal</b><select value={goalId(p.c)} onChange={e => setGoal(e.currentTarget.value)}><option value="">Explore (no goal)</option>
        <For each={groups()}>{gr => <optgroup label={gr}><For each={goals().filter(g => g.group === gr && !g.library)}>{g => <option value={g.id}>{g.name} — {g.above || g.slope ? "maximise" : "minimise"} {g.index.replace(/ \(.*\)/, "")}</option>}</For></optgroup>}</For></select><Help k="goal" /></span>
      <Show when={goal()?.library}><span class="muted small">Index: {goal()!.name}</span></Show>
      <Show when={goal()}><button class="btn sm" title={goal()!.why} onClick={() => import("./pages").then(m => m.goalCard(goal()!.id))}>Why {goal()!.index}?</button></Show>
      <span class="sep" />
      <div class="seg"><button classList={{ on: p.c.kind !== "bar" }} title="Two properties, a bubble per material" onClick={() => setChart(c => { c.kind = "bubble"; })}>Bubbles</button>
        <button classList={{ on: p.c.kind === "bar" }} title="One property, a bar per material" onClick={() => setChart(c => { c.kind = "bar"; c.sels = []; c.goal = ""; c.logY = db().props.find(x => x.key === c.y)?.log ?? true; })}>Bars</button></div><Help k={p.c.kind === "bar" ? "bar" : "chart"} />
      <span class="grow" />
      <Show when={p.c.kind !== "bar"}><select title="Envelopes" value={p.c.hull} onChange={e => { const v = e.currentTarget.value; setChart(c => { c.hull = v; }); }}><option value="family">Envelopes: families</option><option value="class">Envelopes: classes</option><option value="none">No envelopes</option></select><Help k="envelopes" /></Show>
      <button class="rb fb" classList={{ on: p.c.best }} disabled={p.c.kind === "bar"} title="Label the five best materials by the ranking" onClick={() => setChart(c => { c.best = !c.best; })}>Best 5</button>
      <button class="rb fb" classList={{ on: p.c.labels }} title="Label every material (labels move apart; drag one to place it)" onClick={() => setChart(c => { c.labels = !c.labels; })}><Icon n="tag" /></button>
      <select title="Chart style" value={p.c.style} onChange={e => { const v = e.currentTarget.value; setChart(c => { c.style = v; }); }}><option value="classic">Classic</option><option value="publication">Publication</option><option value="mono">Mono</option></select>
      <div class="seg" title="Charts at once: one, two side by side, or four"><For each={[1, 2, 4]}>{n => <button classList={{ on: grid() === n }} title={`${n} chart${n > 1 ? "s" : ""} at once`} onClick={() => setGrid(n)}>{n === 1 ? "▢" : n === 2 ? "▢▢" : "⊞"}</button>}</For></div>
      <button class="rb fb" classList={{ on: p.c.guides.length > 0 }} disabled={p.c.kind === "bar"} title="Guide lines of slope 1, 2 and 3" onClick={() => setChart(c => { c.guides = c.guides.length ? [] : [1, 2, 3]; })}><Icon n="index" /></button>
      <button class="rb fb" title="Zoom in" onClick={() => zoomChart(1.4)}><Icon n="zin" /></button><button class="rb fb" title="Zoom out" onClick={() => zoomChart(1 / 1.4)}><Icon n="zout" /></button><button class="rb fb" title="Fit all materials" onClick={fitChart}><Icon n="fit" /></button>
    </div>
  );
};

/** What a selection is, in a few words: "E½/ρ above", "box", "shape". */
const selName = (c: Chart, s: ChartSel, k: number) => { const l = lineOf(s); if (!l) return boxOf(s) ? "Box" : "Shape";
  return `${out()?.results.lines.find(x => x.chart === c.id && x.k === k)?.name ?? `slope ${fmt(l.slope)}`} ${l.above ? "above" : "below"}`; };

/** The second toolbar row: the axes, the tools, and the selections with the active line's slope and side. */
export const ChartTools = (p: { c: Chart }) => {
  const l = () => activeLine(p.c);
  const setLine = (patch: Partial<Line>) => { const k = activeOf(p.c), x = l(); if (x) setSels(p.c, p.c.sels.map((s, i) => (i === k ? { Line: { ...x, ...patch } } : s))); };
  return (
    <div class="cbar">
      <AxisPick c={p.c} k="y" /><Show when={p.c.kind !== "bar"}><button class="rb fb" title="Swap the axes" onClick={() => setChart(c => { [c.x, c.y, c.logX, c.logY] = [c.y, c.x, c.logY, c.logX]; c.sels = []; c.goal = ""; })}>⇅</button><AxisPick c={p.c} k="x" />
      <span class="sep" />
      <div class="seg tools"><For each={[["pan", "Pan", "Drag to move, wheel to zoom, click a bubble"], ["line", "Line", "Click to place the active selection line"], ["box", "Box", "Drag a selection box"], ["poly", "Shape", "Click the corners of a shape; the materials inside pass"]] as const}>{([k, n, t]) =>
        <button classList={{ on: tool() === k }} title={t} onClick={() => setTool(k)}>{n}</button>}</For></div><Help k="tools" />
      <label class="inline" title="Slope of the active selection line: 1 for E/ρ, 2 for E½/ρ, 3 for E⅓/ρ">slope <input type="number" step="0.5" style={{ width: "54px" }} value={l()?.slope ?? slope()}
        onChange={e => { const v = +e.currentTarget.value; if (!isFinite(v)) return toast("Type a number for the slope."); setSlope(v); setLine({ slope: v }); }} /></label><Help k="slope" />
      <Show when={l()}><div class="seg"><button classList={{ on: l()!.above }} onClick={() => setLine({ above: true })}>Above passes</button><button classList={{ on: !l()!.above }} onClick={() => setLine({ above: false })}>Below</button></div>
        <button class="btn sm" title="Place the line where its index has a value you type" onClick={() => passThrough(p.c)}>Pass through…</button></Show>
      <button class="btn sm" title="Add another selection line (a band: two lines facing each other)" onClick={() => newLine(p.c)}>+ Line</button>
      </Show>
    </div>
  );
};

/** The chart's selections as chips: click one to edit it, ✕ removes it; the ranking line is marked. */
export const SelChips = (p: { c: Chart }) => {
  const ranks = (k: number) => { const r = out()?.results; return !!r && r.rankChart === p.c.id && r.rankLine === k && P().stages.ranking !== "weighted"; };
  const rankBy = (k: number) => act({ SetStages: { stages: { ...P().stages, rankChart: p.c.id, rankLine: k, ranking: "line" } } });
  return <Show when={p.c.sels.length}><div class="selchips"><b>Selections</b>
    <For each={p.c.sels}>{(s, k) => <span class="chip" classList={{ on: activeOf(p.c) === k() }} onClick={() => setActiveOf(p.c.id, k())}>{k() + 1} · {selName(p.c, s, k())}
      <Show when={lineOf(s)}>{ranks(k()) ? <em title="The results are ranked by this line's index">ranks</em> : <button class="lnk" title="Rank the results by this line's index" onClick={e => { e.stopPropagation(); rankBy(k()); }}>rank by it</button>}</Show>
      <button class="x" title="Remove" onClick={e => { e.stopPropagation(); removeSel(p.c, k()); }}>✕</button></span>}</For>
    <button class="btn sm" onClick={() => removeSel(p.c)}>Clear all</button><Help k="sels" /></div></Show>;
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
  try { await call("copy_png", { data: await pngOf(c.id) }); toast("Chart copied as a picture. Paste it into a document, a slide or an email.", true); }
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
