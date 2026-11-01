// Processes: the records (browse and a card for each), and process selection: the part to make,
// the processes that can make it and why the others cannot, the cost against batch size, which
// processes suit the shortlisted materials, and joining and finishing. The engine is process.rs.
import { For, Show, createSignal, createResource } from "solid-js";
import { Process, ProcSel, fmt, CMP_COLORS } from "./api";
import { act, out, P, db, mat, call, openDialog, show, cur } from "./store";
import { Dialog } from "./ui";
import { Help } from "./help";

let cache: Process[] | null = null;
export const loadProcesses = async () => cache ?? (cache = (await call<Process[]>("processes")) ?? []);
const [procs] = createResource(loadProcesses);
const proc = (id: string) => procs()?.find(p => p.id === id);
const GROUPS = ["Shaping", "Machining", "Additive", "Joining", "Finishing"];
const SHAPES: [string, string][] = [["", "Any"], ["flat", "Flat"], ["sheet", "Sheet (dished, folded)"], ["prismatic", "Prismatic (constant section)"], ["3d-solid", "3-D solid"], ["3d-hollow", "3-D hollow"]];
const PURPOSES = ["corrosion", "wear", "decoration", "UV", "insulation", "conductivity", "smoothness", "hygiene", "strength"];
const r2 = (r: [number, number], u = "") => `${fmt(r[0])} – ${fmt(r[1])}${u ? ` ${u}` : ""}`;

/** A process record. */
export const procCard = (id: string) => { const p = proc(id); if (!p) return;
  const sel = ["Shaping", "Machining", "Additive"].includes(p.group);
  openDialog(() => <Dialog title={p.name} wide>
    <p class="muted">{p.group}{p.kind !== p.group ? ` · ${p.kind}` : ""}</p><p>{p.note}</p>
    <table class="tbl mini"><tbody>
      <tr><td>Materials</td><td>{p.fam.map(f => f.split(" › ").pop()).join(", ")}</td></tr>
      <Show when={sel}>
        <tr><td>Part mass</td><td>{r2(p.mass, "kg")}</td></tr><tr><td>Section thickness</td><td>{r2(p.section, "mm")}</td></tr>
        <tr><td>Shapes</td><td>{p.shapes.map(s => SHAPES.find(x => x[0] === s)?.[1] ?? s).join(", ")}</td></tr>
        <tr><td>Tolerance</td><td>± {r2(p.tol, "mm")}</td></tr><tr><td>Roughness Ra</td><td>{r2(p.rough, "µm")}</td></tr>
        <tr><td>Economic batch</td><td>{r2(p.batch, "parts")}</td></tr>
        <tr><td>Tooling</td><td>{r2(p.tooling, "USD")}</td></tr><tr><td>Rate</td><td>{r2(p.rate, "parts an hour")}</td></tr>
        <tr><td>Overhead</td><td>{r2(p.overhead, "USD an hour")}</td></tr><tr><td>Material wasted</td><td>{fmt(p.scrap[0] * 100)} – {fmt(p.scrap[1] * 100)} %</td></tr>
      </Show>
      <Show when={p.group === "Joining"}><tr><td>Permanent</td><td>{p.permanent ? "Yes" : "No: can be taken apart"}</td></tr><tr><td>Watertight</td><td>{p.watertight ? "Yes" : "No"}</td></tr></Show>
      <Show when={p.group === "Finishing"}><tr><td>For</td><td>{p.purposes.join(", ")}</td></tr></Show>
    </tbody></table>
    <p class="muted small">Typical ranges compiled from general engineering knowledge; costs are indicative, not quotes. Ask your supplier.</p>
  </Dialog>);
};

export const ProcessesPage = () => {
  const [g, setG] = createSignal("");
  const list = () => (procs() ?? []).filter(p => !g() || p.group === g());
  return (
    <div class="page">
      <div class="ptools"><h1>Processes <Help k="processes" /></h1><span class="grow" />
        <select value={g()} onChange={e => setG(e.currentTarget.value)}><option value="">All groups</option><For each={GROUPS}>{x => <option value={x}>{x}</option>}</For></select>
        <button class="btn pri" onClick={() => show("process")}>Process selection</button></div>
      <table class="tbl"><thead><tr><th>Process</th><th>Group</th><th>Materials</th><th class="r">Part mass, kg</th><th class="r">Tolerance, mm</th><th class="r">Economic batch</th></tr></thead>
        <tbody><For each={list()}>{p => { const sel = ["Shaping", "Machining", "Additive"].includes(p.group); return <tr>
          <td><button class="lnk" onClick={() => procCard(p.id)}>{p.name}</button></td><td>{p.kind}</td><td class="small">{p.fam.map(f => f.split(" › ").pop()).join(", ")}</td>
          <td class="r">{sel ? r2(p.mass) : ""}</td><td class="r">{sel ? `± ${fmt(p.tol[0])}` : ""}</td><td class="r">{sel ? r2(p.batch) : ""}</td></tr>; }}</For></tbody></table>
    </div>
  );
};

/** Cost per part against batch size, log–log: a band per process that can make the part. */
const CostChart = () => {
  const rows = () => out()!.process.rows.filter(r => r.curve.length).slice(0, 8);
  const W = 520, H = 280, L = 58, B = 30;
  const ys = () => rows().flatMap(r => r.curve.flatMap(c => [c[1], c[2]])).filter(v => v > 0);
  const lo = () => Math.log10(Math.max(1e-3, Math.min(...ys()))), hi = () => Math.log10(Math.max(...ys()));
  const sx = (n: number) => L + Math.log10(n) / 7 * (W - L - 10), sy = (v: number) => H - B - (Math.log10(Math.max(v, 1e-9)) - lo()) / Math.max(0.1, hi() - lo()) * (H - B - 10);
  const decades = () => { const a = Math.floor(lo()), b = Math.ceil(hi()); return Array.from({ length: b - a + 1 }, (_, k) => 10 ** (a + k)); };
  return (
    <Show when={rows().length} fallback={<p class="muted">No process can make this part yet, or the material has no price: see the reasons below.</p>}>
      <svg class="ecobars" width={W} height={H} viewBox={`0 0 ${W} ${H}`}>
        <line x1={L} x2={W - 10} y1={H - B} y2={H - B} stroke="currentColor" /><line x1={L} x2={L} y1={8} y2={H - B} stroke="currentColor" />
        <For each={[1, 10, 100, 1e3, 1e4, 1e5, 1e6, 1e7]}>{n => <text x={sx(n)} y={H - 12} text-anchor="middle" font-size="10" fill="currentColor">{fmt(n)}</text>}</For>
        <For each={decades()}>{v => <Show when={sy(v) >= 8 && sy(v) <= H - B}><text x={L - 4} y={sy(v) + 4} text-anchor="end" font-size="10" fill="currentColor">{fmt(v)}</text></Show>}</For>
        <text x={W - 10} y={H - B - 4} text-anchor="end" font-size="10" fill="currentColor">parts in the batch</text>
        <text x={L + 4} y={14} font-size="10" fill="currentColor">USD per part</text>
        <line x1={sx(P().process.batch)} x2={sx(P().process.batch)} y1={8} y2={H - B} stroke="currentColor" stroke-dasharray="3 3" />
        <For each={rows()}>{(r, i) => <g>
          <polygon points={[...r.curve.map(c => `${sx(c[0])},${sy(c[1])}`), ...[...r.curve].reverse().map(c => `${sx(c[0])},${sy(c[2])}`)].join(" ")} fill={CMP_COLORS[i() % 6]} opacity=".12" />
          <polyline points={r.curve.map(c => `${sx(c[0])},${sy(Math.sqrt(c[1] * c[2]))}`).join(" ")} fill="none" stroke={CMP_COLORS[i() % 6]} stroke-width="2" />
        </g>}</For>
      </svg>
      <div class="legend inline"><For each={rows()}>{(r, i) => <span><i class="dot" style={{ background: CMP_COLORS[i() % 6] }} />{r.name}</span>}</For></div>
    </Show>
  );
};

const num = (v: number, set: (x: number) => void, w = 80) => <input type="number" min="0" step="any" style={{ width: `${w}px` }} value={+v.toFixed(6)} onChange={e => { const x = +e.currentTarget.value; if (isFinite(x) && x >= 0) set(x); else e.currentTarget.value = String(v); }} />;

export const ProcessPage = () => {
  const s = () => P().process, o = () => out()!.process;
  const set = (f: (x: ProcSel) => void) => { const n = structuredClone(s()); f(n); return act({ Process: { sel: n } }); };
  const fams = () => [...new Set(db().materials.map(m => m.family[0]))];
  const matSel = (v: string, f: (id: string) => void, none = "— choose —") => <select value={v} style={{ width: "220px" }} onChange={e => f(e.currentTarget.value)}><option value="">{none}</option>
    <For each={fams()}>{fa => <optgroup label={fa}><For each={db().materials.filter(m => m.family[0] === fa)}>{m => <option value={m.id} selected={m.id === v}>{m.name}</option>}</For></optgroup>}</For></select>;
  const cheapAt = () => { const c = o().cheapest; if (!c.length) return []; const out2: [number, number, string][] = []; c.forEach(([n, id]) => { const last = out2[out2.length - 1]; if (last && last[2] === id) last[1] = n; else out2.push([n, n, id]); }); return out2; };
  const name = (id: string) => proc(id)?.name ?? id;
  return (
    <div class="page procpage">
      <h1>Process selection <Help k="process" /></h1>
      <p class="muted">Which processes can make your part from your material, and why the others cannot; then the cost per part against batch size. Process data and costs are indicative typical ranges.</p>
      <div class="ecogrid">
        <div>
          <section class="panel"><h2>The part</h2>
            <div class="prow"><label>Material {matSel(s().material, v => set(x => { x.material = v; }))}</label>
              <Show when={cur() && cur() !== s().material && mat(cur())}><button class="btn sm" onClick={() => set(x => { x.material = cur(); })}>Use {mat(cur())!.name}</button></Show></div>
            <div class="prow"><label>Mass, kg{num(s().mass, v => v > 0 && set(x => { x.mass = v; }))}</label><label>Thinnest section, mm{num(s().section, v => v > 0 && set(x => { x.section = v; }))}</label>
              <label>Shape<select class="pshape" value={s().shape} onChange={e => set(x => { x.shape = e.currentTarget.value; })}><For each={SHAPES}>{([v, l]) => <option value={v} selected={v === s().shape}>{l}</option>}</For></select></label></div>
            <div class="prow"><label>Tolerance needed, ± mm (0: any){num(s().tol, v => set(x => { x.tol = v; }))}</label><label>Roughness Ra, µm (0: any){num(s().rough, v => set(x => { x.rough = v; }))}</label>
              <label>Parts in the batch{num(s().batch, v => v >= 1 && set(x => { x.batch = v; }))}</label></div>
            <div class="prow"><label>Material price, USD/kg (0: the material's own{o().price ? `, ${fmt(o().price![0])}–${fmt(o().price![1])}` : ""}){num(s().price, v => set(x => { x.price = v; }))}</label></div>
            <Show when={o().notes.length}><ul class="ecnotes"><For each={o().notes}>{n => <li>{n}</li>}</For></ul></Show>
          </section>
          <section class="panel"><h2>Stages</h2>
            <table class="tbl mini"><tbody><For each={o().stages}>{([n, c]) => <tr><td>{n}</td><td class="r">{c} of {o().rows.length} pass</td></tr>}</For>
              <tr><td><b>All stages</b></td><td class="r"><b>{o().pass} of {o().rows.length}</b></td></tr></tbody></table>
          </section>
          <section class="panel"><h2>Joining and finishing <Help k="joinfinish" /></h2>
            <div class="prow"><label>Join it to {matSel(s().join, v => set(x => { x.join = v; }), "— the same material —")}</label>
              <label>Finish for<select value={s().purpose} onChange={e => set(x => { x.purpose = e.currentTarget.value; })}><option value="">Anything</option><For each={PURPOSES}>{v => <option value={v} selected={v === s().purpose}>{v}</option>}</For></select></label></div>
            <Show when={s().material} fallback={<p class="muted">Choose the material first.</p>}>
              <p><b>Joining:</b> <For each={o().joins} fallback={<span class="muted">none in the records</span>}>{(id, i) => <>{i() ? ", " : ""}<button class="lnk" onClick={() => procCard(id)}>{name(id)}</button>{proc(id)?.permanent ? "" : " (can be taken apart)"}</>}</For></p>
              <p><b>Finishing:</b> <For each={o().finishes} fallback={<span class="muted">none in the records</span>}>{(id, i) => <>{i() ? ", " : ""}<button class="lnk" onClick={() => procCard(id)}>{name(id)}</button></>}</For></p>
            </Show>
          </section>
        </div>
        <div>
          <section class="panel"><h2>Cost per part <Help k="cost" /></h2>
            <CostChart />
            <Show when={cheapAt().length}><p class="small"><b>Cheapest:</b> {cheapAt().map(([a, b, id]) => `${name(id)} from ${fmt(a)} to ${fmt(b)} parts`).join("; ")}.</p></Show>
          </section>
          <section class="panel"><h2>Processes ({o().pass} can make it)</h2>
            <table class="tbl mini proctable"><thead><tr><th>Process</th><th>Group</th><th class="r">USD per part at {fmt(s().batch)}</th><th>Why not</th></tr></thead>
              <tbody><For each={o().rows}>{r => <tr classList={{ fail: !r.pass }}><td><button class="lnk" onClick={() => procCard(r.id)}>{r.name}</button></td><td>{r.kind}</td>
                <td class="r">{r.cost ? `${fmt(Math.sqrt(r.cost[0] * r.cost[1]))} (${fmt(r.cost[0])} – ${fmt(r.cost[1])})` : ""}</td><td class="small">{r.reasons.join("; ")}</td></tr>}</For></tbody></table>
          </section>
          <Show when={o().matrix.mats.length}><section class="panel"><h2>Processes for the shortlist</h2>
            <p class="muted small">{P().compare.length ? "The materials in Compare" : "The best materials of the results"}: ● the process works it.</p>
            <div class="hscroll"><table class="tbl mini matrix"><thead><tr><th>Process</th><For each={o().matrix.mats}>{m => <th title={m[1]}>{m[1].length > 18 ? m[1].slice(0, 17) + "…" : m[1]}</th>}</For></tr></thead>
              <tbody><For each={o().matrix.procs}>{(p, i) => <Show when={o().matrix.ok[i()].some(Boolean)}><tr><td>{p[1]}</td><For each={o().matrix.ok[i()]}>{v => <td class="c">{v ? "●" : ""}</td>}</For></tr></Show>}</For></tbody></table></div>
          </section></Show>
        </div>
      </div>
    </div>
  );
};

