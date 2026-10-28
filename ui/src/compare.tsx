// Compare: two to six materials side by side. The engine (compare.rs) gives each property's typical
// values, the highest and lowest, and the radar positions; this page lays them out.
import { For, Show, createSignal } from "solid-js";
import { CMP_COLORS, fmt } from "./api";
import { act, out, P, db, mat, propOf, vtext, unitOf, toShown, openSheet, show, chartProps } from "./store";
import { Icon } from "./ui";
import { Help } from "./help";

const GROUPS = ["General", "Mechanical", "Thermal", "Electrical", "Eco", "Derived", "Your formulas"];

/** A radar chart: one spoke per property, each material's outline at its place between the lowest
 *  (centre) and highest (rim) value in the database. */
export const Radar = (p: { size?: number; print?: boolean }) => {
  const S = () => p.size ?? 420, c = () => out()!.compare, R = () => S() / 2 - 70, cx = () => S() / 2, cy = () => S() / 2;
  const n = () => c().radar.keys.length;
  const at = (k: number, v: number): [number, number] => { const a = -Math.PI / 2 + (2 * Math.PI * k) / n(); return [cx() + Math.cos(a) * R() * v, cy() + Math.sin(a) * R() * v]; };
  const ring = (v: number) => Array.from({ length: n() }, (_, k) => at(k, v).join(",")).join(" ");
  return (
    <svg class="radar" width={S()} height={S()} viewBox={`0 0 ${S()} ${S()}`} xmlns="http://www.w3.org/2000/svg" font-family="IBM Plex Sans, Segoe UI, Arial, sans-serif">
      <Show when={n() >= 3} fallback={<text x={S() / 2} y={S() / 2} text-anchor="middle" font-size="12" fill="#5B6878">Pick three or more properties.</text>}>
        <For each={[0.25, 0.5, 0.75, 1]}>{v => <polygon points={ring(v)} fill="none" stroke="#D5DAE0" stroke-width={v === 1 ? 1.2 : 0.7} />}</For>
        <For each={c().radar.keys}>{(k, i) => { const [x, y] = at(i(), 1), [lx, ly] = at(i(), 1.16); return <>
          <line x1={cx()} y1={cy()} x2={x} y2={y} stroke="#D5DAE0" stroke-width=".7" />
          <text x={lx} y={ly + 4} text-anchor={Math.abs(lx - cx()) < 8 ? "middle" : lx > cx() ? "start" : "end"} font-size="11" fill="#333">{propOf(k)?.sym || propOf(k)?.name}</text></>; }}</For>
        <For each={c().ids}>{(_, m) => { const v = () => c().radar.vals[m()] ?? [];
          const pts = () => v().map((x, k) => (x === null ? null : at(k, x).join(","))).filter(Boolean).join(" ");
          return <g><polygon points={pts()} fill={CMP_COLORS[m()]} fill-opacity=".12" stroke={CMP_COLORS[m()]} stroke-width="2" />
            <For each={v()}>{(x, k) => <Show when={x !== null}><circle cx={at(k(), x!)[0]} cy={at(k(), x!)[1]} r="3" fill={CMP_COLORS[m()]} /></Show>}</For></g>; }}</For>
      </Show>
    </svg>
  );
};

export const ComparePage = () => {
  const [q, setQ] = createSignal("");
  const c = () => out()!.compare, ids = () => P().compare;
  const set = (list: string[], radar = P().radar) => act({ Compare: { ids: list, radar } });
  const found = () => { const t = q().trim().toLowerCase(); return t.length < 2 ? [] : db().materials.filter(m => !ids().includes(m.id) && `${m.name} ${m.syn.join(" ")} ${Object.values(m.des).join(" ")}`.toLowerCase().includes(t)).slice(0, 8); };
  const radarKeys = () => c().radar.keys;
  const toggleKey = (k: string) => set(ids(), radarKeys().includes(k) ? radarKeys().filter(x => x !== k) : [...radarKeys(), k]);
  const rowsIn = (g: string) => c().rows.filter(r => (propOf(r.key)?.group ?? "") === g);
  return (
    <div class="page comparepage">
      <h1>Compare <Help k="compare" /></h1>
      <p class="muted">Up to six materials side by side. In every row the highest value is marked ▲ and the lowest ▼; “spread” is the highest divided by the lowest. Values are typical (the middle of each range).</p>
      <div class="ptools">
        <input placeholder="Add a material: name, UNS, EN…" value={q()} onInput={e => setQ(e.currentTarget.value)} style={{ width: "280px" }} />
        <button class="btn sm" disabled={!out()!.results.pass} onClick={() => set(out()!.results.rows.filter(x => x.pass).slice(0, 3).map(x => x.id))}>The best three of the results</button>
        <Show when={ids().length}><button class="btn sm" onClick={() => set([])}>Clear</button></Show>
      </div>
      <Show when={found().length}><div class="cfound"><For each={found()}>{m => <button class="btn sm" disabled={ids().length >= 6} onClick={() => { set([...ids(), m.id]); setQ(""); }}>+ {m.name}</button>}</For></div></Show>
      <Show when={c().ids.length} fallback={<div class="emptybox small"><Icon n="compare" size={36} /><p>Add materials here, from Results (“+ compare”), or from the card beside the chart.</p><button class="btn pri" onClick={() => show("results")}>Results</button></div>}>
        <div class="cmpgrid">
          <div class="cmptable">
            <table class="tbl cmp"><thead><tr><th>Property</th><For each={c().ids}>{(id, m) => <th><i class="dot" style={{ background: CMP_COLORS[m()] }} /><button class="lnk" onClick={() => openSheet(id)}>{mat(id)?.name}</button> <button class="x" title="Remove" onClick={() => set(ids().filter(x => x !== id))}>✕</button></th>}</For><th class="r">spread</th></tr></thead>
              <tbody><For each={GROUPS.filter(g => rowsIn(g).length)}>{g => <>
                <tr class="grp"><td colspan={c().ids.length + 2}>{g}</td></tr>
                <For each={rowsIn(g)}>{r => <tr>
                  <td title={propOf(r.key)?.about}>{propOf(r.key)?.name} <span class="muted small">{unitOf(r.key)}</span></td>
                  <For each={c().ids}>{(id, m) => { const v = mat(id)?.props[r.key]; return <td classList={{ hi: r.hi === m(), lo: r.lo === m() }}>
                    {v ? vtext(r.key, v) : r.vals[m()] !== null ? fmt(toShown(r.key, r.vals[m()]!)) : <span class="muted">no data</span>}{r.hi === m() ? " ▲" : r.lo === m() ? " ▼" : ""}</td>; }}</For>
                  <td class="r muted">{r.spread !== null ? `× ${fmt(r.spread)}` : ""}</td></tr>}</For></>}</For></tbody></table>
          </div>
          <div class="cmpradar">
            <h3>Radar <span class="muted small">centre: lowest in the database · rim: highest</span></h3>
            <Radar />
            <div class="rkeys"><For each={chartProps()}>{x => <label class="chk"><input type="checkbox" checked={radarKeys().includes(x.key)} onChange={() => toggleKey(x.key)} />{x.sym || x.name}</label>}</For></div>
            <p class="muted small">Log properties (density, modulus, price…) are placed on a log scale. Further out is higher, not always better: low density or low price is better for most designs.</p>
          </div>
        </div>
      </Show>
    </div>
  );
};
