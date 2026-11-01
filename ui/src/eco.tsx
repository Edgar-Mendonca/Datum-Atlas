// Eco audit: design options with their parts, transport, use and end of life; the engine (eco.rs)
// works out energy and CO₂ for each life phase as ranges, says which phase matters most, and where
// options break even. Also Greener Substitutes: materials that do the same job with less CO₂.
import { For, Show, createSignal, createResource } from "solid-js";
import { Eco, EcoData, EcoOpt, EcoPart, Subs, Val2, R2, fmt, CMP_COLORS } from "./api";
import { act, out, P, db, mat, call, openDialog, show, openSheet, toggleCompare, propOf, cur, toast } from "./store";
import { Dialog, Icon } from "./ui";
import { Help } from "./help";

export const PHASES = ["Material", "Manufacture", "Transport", "Use", "End of life"];
const PH_COLORS = ["#6C4AB6", "#E69F00", "#56B4E9", "#D55E00", "#009E73"];
const EOL: [string, string][] = [["landfill", "Landfill"], ["burn", "Burn for energy"], ["downcycle", "Downcycle"], ["recycle", "Recycle"], ["none", "Not counted"]];
let cache: EcoData | null = null;
export const ecoData = async () => cache ?? (cache = (await call<EcoData>("eco_data")) ?? null);

const [optK, setOptK] = createSignal(0);
const eco = (): Eco => P().eco;
/** Changes the audit: a copy is edited and sent as one edit (undo takes it back). */
const edit = (f: (e: Eco) => void) => { const e = structuredClone(eco()); f(e); return act({ Eco: { eco: e } }); };
const editOpt = (f: (o: EcoOpt) => void) => edit(e => { const o = e.options[Math.min(optK(), e.options.length - 1)]; if (o) f(o); });
const newOpt = (n: number): EcoOpt => ({ name: `Option ${n}`, parts: [], legs: [{ mode: "truck", km: 500 }], use: { kind: "none", kw: 0, hours: 0, days: 365, vehicle: "car-petrol", km: 0 }, years: 10 });
const part = (material: string, name: string): EcoPart => ({ name, material, mass: 1, qty: 1, process: "none", removed: 0, recycled: 0, eol: "landfill" });
/** A range as "12 (10 – 14)". */
const rtext = (r: R2) => Math.abs(r[1] - r[0]) < 1e-9 * Math.max(1, Math.abs(r[1])) ? fmt(r[0]) : `${fmt((r[0] + r[1]) / 2)} (${fmt(r[0])} – ${fmt(r[1])})`;
const mid = (r: R2) => (r[0] + r[1]) / 2;

/** Starts an audit (or adds a part to the open option) with a material. */
export const auditMaterial = async (id: string) => {
  const m = mat(id); if (!m) return;
  const ok = eco().options.length
    ? await editOpt(o => { o.parts.push(part(id, m.name)); })
    : await edit(e => { const o = newOpt(1); o.parts.push(part(id, m.name)); e.options.push(o); });
  if (ok) { show("eco"); toast(`${m.name} added to the eco audit: set its mass.`, true); }
};

/** Bars for the five phases of one value (mj or co2): each phase's middle value with its range. */
const PhaseBars = (p: { v: Val2[]; k: "mj" | "co2"; w?: number }) => {
  const W = () => p.w ?? 380, H = 150, L = 92;
  const max = () => Math.max(1e-9, ...p.v.map(x => Math.abs(x[p.k][1])), ...p.v.map(x => Math.abs(x[p.k][0])));
  const neg = () => p.v.some(x => x[p.k][0] < 0);
  const z = () => neg() ? L + (W() - L - 10) / 2 : L, sx = (v: number) => z() + v / max() * (W() - L - 10) / (neg() ? 2 : 1);
  return (
    <svg class="ecobars" width={W()} height={H} viewBox={`0 0 ${W()} ${H}`}>
      <For each={p.v}>{(x, i) => { const y = 8 + i() * 27, r = x[p.k]; return <g>
        <text x={L - 6} y={y + 13} text-anchor="end" font-size="11" fill="currentColor">{PHASES[i()]}</text>
        <rect x={Math.min(sx(0), sx(mid(r)))} y={y} width={Math.abs(sx(mid(r)) - sx(0))} height={18} fill={PH_COLORS[i()]} opacity=".85" />
        <line x1={sx(r[0])} x2={sx(r[1])} y1={y + 9} y2={y + 9} stroke="currentColor" stroke-width="1.2" />
        <line x1={sx(r[0])} x2={sx(r[0])} y1={y + 5} y2={y + 13} stroke="currentColor" /><line x1={sx(r[1])} x2={sx(r[1])} y1={y + 5} y2={y + 13} stroke="currentColor" />
      </g>; }}</For>
      <line x1={sx(0)} x2={sx(0)} y1={4} y2={H - 6} stroke="currentColor" stroke-width=".8" />
    </svg>
  );
};

/** Cumulative CO₂ (or energy) of each option against years: where the lines cross, they break even. */
const BreakEven = (p: { k: 0 | 1 }) => {
  const os = () => out()!.eco.options, W = 420, H = 210, L = 54, B = 26;
  const T = () => Math.max(1, ...eco().options.map(o => o.years)) * 1.5;
  const ymax = () => Math.max(1e-9, ...os().map(o => o.fixed[p.k] + o.rate[p.k] * T()));
  const sx = (t: number) => L + t / T() * (W - L - 10), sy = (v: number) => H - B - v / ymax() * (H - B - 10);
  return (
    <svg class="ecobars" width={W} height={H} viewBox={`0 0 ${W} ${H}`}>
      <line x1={L} x2={W - 10} y1={H - B} y2={H - B} stroke="currentColor" /><line x1={L} x2={L} y1={8} y2={H - B} stroke="currentColor" />
      <For each={[0, 0.5, 1]}>{f => <><text x={sx(T() * f)} y={H - 8} text-anchor="middle" font-size="10" fill="currentColor">{fmt(T() * f)}</text>
        <text x={L - 4} y={sy(ymax() * f) + 4} text-anchor="end" font-size="10" fill="currentColor">{fmt(ymax() * f)}</text></>}</For>
      <text x={W - 10} y={H - B - 4} text-anchor="end" font-size="10" fill="currentColor">years of use</text>
      <For each={os()}>{(o, i) => <><line x1={sx(0)} y1={sy(o.fixed[p.k])} x2={sx(T())} y2={sy(o.fixed[p.k] + o.rate[p.k] * T())} stroke={CMP_COLORS[i()]} stroke-width="2" />
        <text x={sx(T()) - 4} y={sy(o.fixed[p.k] + o.rate[p.k] * T()) - 4} text-anchor="end" font-size="10.5" fill={CMP_COLORS[i()]}>{o.name}</text></>}</For>
    </svg>
  );
};

const num = (v: number, set: (x: number) => void, w = 70, step = "any") => <input type="number" min="0" step={step} style={{ width: `${w}px` }} value={+v.toFixed(6)} onChange={e => { const x = +e.currentTarget.value; if (isFinite(x) && x >= 0) set(x); else e.currentTarget.value = String(v); }} />;

export const EcoPage = () => {
  const [d] = createResource(ecoData);
  const k = () => Math.min(optK(), Math.max(0, eco().options.length - 1));
  const o = () => eco().options[k()], r = () => out()!.eco.options[k()];
  const fams = () => [...new Set(db().materials.map(m => m.family[0]))];
  const best = () => out()!.results.rows.filter(x => x.pass).slice(0, 3);
  return (
    <div class="page ecopage">
      <h1>Eco audit <Help k="eco" /></h1>
      <p class="muted">Energy and CO₂ of a product through its life: making its materials, manufacture, transport, use, and what happens at the end. Every figure is a range from the materials' values and public factors; gaps are listed, never counted as zero. A design-stage audit, not a full life-cycle assessment.</p>
      <Show when={eco().options.length} fallback={<div class="emptybox small"><Icon n="eco" size={36} /><p>Describe the product as parts of a material and mass, then how it travels and how it is used.</p>
        <p class="inline"><button class="btn pri" onClick={() => edit(e => { e.options.push(newOpt(1)); })}>Start an audit</button>
          <Show when={best().length}><button class="btn" onClick={() => edit(e => { best().forEach((x, i) => { const n = newOpt(i + 1); n.name = x.name; n.parts.push(part(x.id, "Part")); e.options.push(n); }); })}>One option for each of the best three results</button></Show></p></div>}>
        <div class="ptools">
          <div class="otabs"><For each={eco().options}>{(x, i) => <button classList={{ on: i() === k() }} onClick={() => setOptK(i())}><i class="dot" style={{ background: CMP_COLORS[i()] }} />{x.name}</button>}</For>
            <Show when={eco().options.length < 4}><button title="A new design option" onClick={() => edit(e => { e.options.push(newOpt(e.options.length + 1)); }).then(() => setOptK(eco().options.length - 1))}>＋</button>
              <button title="A copy of this option, to change one thing" onClick={() => edit(e => { const c = structuredClone(e.options[k()]); c.name += " (copy)"; e.options.push(c); }).then(() => setOptK(eco().options.length - 1))}>Copy</button></Show></div>
          <label class="inline">Name <input value={o().name} onChange={e => editOpt(x => { x.name = e.currentTarget.value || x.name; })} style={{ width: "160px" }} /></label>
          <button class="btn sm" onClick={() => edit(e => { e.options.splice(k(), 1); }).then(() => setOptK(0))}>Delete option</button>
          <span class="grow" />
          <label class="inline">Electricity <select value={eco().grid} onChange={e => edit(x => { x.grid = e.currentTarget.value; })}><For each={d()?.grid ?? []}>{g => <option value={g.id} selected={g.id === eco().grid}>{g.name} ({fmt(mid(g.co2))} kg CO₂/kWh)</option>}</For></select></label>
        </div>
        <div class="ecogrid">
          <div>
            <section class="panel"><h2>Parts <Help k="ecoParts" /></h2>
              <div class="hscroll"><table class="tbl ecoparts"><thead><tr><th>Part</th><th>Material</th><th>Mass, kg</th><th>Number</th><th>Process</th><th title="Mass machined away, as a percentage of the finished part">Machined, %</th><th title="Recycled content of the part">Recycled, %</th><th>End of life</th><th /></tr></thead>
                <tbody><For each={o().parts}>{(p, i) => { const set = (f: (q: EcoPart) => void) => editOpt(x => f(x.parts[i()])); return <tr>
                  <td><input value={p.name} style={{ width: "110px" }} onChange={e => set(q => { q.name = e.currentTarget.value; })} /></td>
                  <td><select value={p.material} style={{ width: "170px" }} onChange={e => set(q => { q.material = e.currentTarget.value; })}><option value="">— choose —</option>
                    <For each={fams()}>{f => <optgroup label={f}><For each={db().materials.filter(m => m.family[0] === f)}>{m => <option value={m.id} selected={m.id === p.material}>{m.name}</option>}</For></optgroup>}</For></select>
                    <Show when={p.material}><button class="lnk" title="Materials that do the same job with less CO₂" onClick={() => greener(p.material, i())}>greener?</button></Show></td>
                  <td>{num(p.mass, v => set(q => { q.mass = v; }))}</td><td>{num(p.qty, v => set(q => { q.qty = v; }), 50)}</td>
                  <td><select value={p.process} style={{ width: "150px" }} onChange={e => set(q => { q.process = e.currentTarget.value; })}><For each={d()?.processes ?? []}>{f => <option value={f.id} selected={f.id === p.process}>{f.name}</option>}</For></select></td>
                  <td>{num(p.removed * 100, v => set(q => { q.removed = v / 100; }), 56)}</td>
                  <td>{num(p.recycled, v => set(q => { q.recycled = Math.min(100, v); }), 56)}<Show when={mat(p.material)?.props.rc_frac}>{v => <span class="muted small" title="The recycled share of today's supply"> typical {fmt(v()[0] as number)}–{fmt(v()[1] as number)}</span>}</Show></td>
                  <td><select value={p.eol} onChange={e => set(q => { q.eol = e.currentTarget.value; })}><For each={EOL}>{([v, l]) => <option value={v} selected={v === p.eol}>{l}</option>}</For></select></td>
                  <td><button class="x" title="Remove" onClick={() => editOpt(x => { x.parts.splice(i(), 1); })}>✕</button></td></tr>; }}</For></tbody></table></div>
              <p class="inline"><button class="btn sm" onClick={() => editOpt(x => { x.parts.push(part(cur() && mat(cur()) ? cur() : "", `Part ${x.parts.length + 1}`)); })}>+ Part</button>
                <span class="muted small">Mass is the finished part; “machined away” adds the chips to the material and charges machining.</span></p>
            </section>
            <section class="panel"><h2>Transport <Help k="ecoTransport" /></h2>
              <For each={o().legs}>{(l, i) => <p class="inline"><select value={l.mode} onChange={e => editOpt(x => { x.legs[i()].mode = e.currentTarget.value; })}><For each={d()?.transport ?? []}>{f => <option value={f.id} selected={f.id === l.mode}>{f.name}</option>}</For></select>
                {num(l.km, v => editOpt(x => { x.legs[i()].km = v; }), 80)} km <button class="x" onClick={() => editOpt(x => { x.legs.splice(i(), 1); })}>✕</button></p>}</For>
              <button class="btn sm" onClick={() => editOpt(x => { x.legs.push({ mode: "truck", km: 100 }); })}>+ Leg</button>
            </section>
            <section class="panel"><h2>Use <Help k="ecoUse" /></h2>
              <p class="inline"><For each={[["none", "Uses no energy"], ["static", "Powered (electricity)"], ["mobile", "Carried in a vehicle"]]}>{([v, l]) => <label class="chk"><input type="radio" name="usek" checked={o().use.kind === v} onChange={() => editOpt(x => { x.use.kind = v; })} />{l}</label>}</For></p>
              <Show when={o().use.kind === "static"}><p class="inline">{num(o().use.kw, v => editOpt(x => { x.use.kw = v; }), 70)} kW, {num(o().use.hours, v => editOpt(x => { x.use.hours = Math.min(24, v); }), 50)} hours a day, {num(o().use.days, v => editOpt(x => { x.use.days = Math.min(366, v); }), 56)} days a year</p></Show>
              <Show when={o().use.kind === "mobile"}><p class="inline"><select value={o().use.vehicle} onChange={e => editOpt(x => { x.use.vehicle = e.currentTarget.value; })}><For each={d()?.vehicles ?? []}>{f => <option value={f.id} selected={f.id === o().use.vehicle}>{f.name}</option>}</For></select>
                travelling {num(o().use.km, v => editOpt(x => { x.use.km = v; }), 90)} km a year <span class="muted small">(the fuel to carry the product's own mass)</span></p></Show>
              <p />
              <div class="inline">Life {num(o().years, v => editOpt(x => { x.years = v; }), 60)} years</div>
            </section>
          </div>
          <div>
            <Show when={r()}>{x => <section class="panel ecores"><h2>{x().name}: {fmt(x().mass)} kg</h2>
              <div class="ecokpi"><div><b>{fmt(mid(x().total.co2))}</b> kg CO₂<span class="muted">{rtext(x().total.co2)}</span></div><div><b>{fmt(mid(x().total.mj))}</b> MJ<span class="muted">{rtext(x().total.mj)}</span></div>
                <div><b>{fmt(mid(x().net.co2))}</b> kg CO₂ net<span class="muted">with the end-of-life credit</span></div></div>
              <Show when={x().dominant}><p class="ecodom"><b>{x().dominant}</b> is the largest phase. {x().advice}</p></Show>
              <div class="ecocharts"><div><h3>CO₂, kg</h3><PhaseBars v={x().phases} k="co2" /></div><div><h3>Energy, MJ</h3><PhaseBars v={x().phases} k="mj" /></div></div>
              <table class="tbl mini"><thead><tr><th>Phase</th><th class="r">CO₂, kg</th><th class="r">Energy, MJ</th></tr></thead><tbody>
                <For each={x().phases}>{(v, i) => <tr><td><i class="dot" style={{ background: PH_COLORS[i()] }} />{PHASES[i()]}</td><td class="r">{rtext(v.co2)}</td><td class="r">{rtext(v.mj)}</td></tr>}</For></tbody></table>
              <Show when={x().parts.length > 1}><h3>By part</h3><table class="tbl mini"><thead><tr><th>Part</th><th>Material</th><th class="r">kg</th><th class="r">Material CO₂</th><th class="r">Manufacture CO₂</th></tr></thead>
                <tbody><For each={x().parts}>{p => <tr><td>{p.name}</td><td>{p.material}</td><td class="r">{fmt(p.mass)}</td><td class="r">{fmt(mid(p.materialV.co2))}</td><td class="r">{fmt(mid(p.make.co2))}</td></tr>}</For></tbody></table></Show>
              <Show when={x().notes.length}><ul class="ecnotes"><For each={x().notes}>{n => <li>{n}</li>}</For></ul></Show>
            </section>}</Show>
            <Show when={eco().options.length > 1}><section class="panel"><h2>Options side by side <Help k="ecoOptions" /></h2>
              <table class="tbl mini"><thead><tr><th>Option</th><th class="r">kg</th><For each={PHASES}>{ph => <th class="r">{ph}</th>}</For><th class="r">Total CO₂</th></tr></thead>
                <tbody><For each={out()!.eco.options}>{(x, i) => <tr><td><i class="dot" style={{ background: CMP_COLORS[i()] }} />{x.name}</td><td class="r">{fmt(x.mass)}</td><For each={x.phases}>{v => <td class="r">{fmt(mid(v.co2))}</td>}</For><td class="r"><b>{fmt(mid(x.total.co2))}</b></td></tr>}</For></tbody></table>
              <h3>Cumulative CO₂ over the years of use</h3><BreakEven k={1} />
              <For each={out()!.eco.breakeven}>{b => <p class="small">{out()!.eco.options[b.b].name} against {out()!.eco.options[b.a].name}: {b.co2Years !== null ? `the CO₂ breaks even after ${fmt(b.co2Years)} years of use` : "no break-even: one is lower at every age"}{b.mjYears !== null ? `; the energy after ${fmt(b.mjYears)} years` : ""}.</p>}</For>
            </section></Show>
          </div>
        </div>
      </Show>
    </div>
  );
};

/** Greener substitutes for a material: same stages, a similar index, less CO₂ for the job. `part`: the audit part to swap. */
export const greener = (id: string, partK?: number) => {
  const m = mat(id); if (!m) return toast("Pick a material first: click one on a chart or in Browse.");
  openDialog(() => {
    const [tol, setTol] = createSignal(0.1);
    const [s] = createResource(tol, t => call<Subs>("substitutes", { id, tol: t }));
    const name = (k: string) => propOf(k)?.name ?? k;
    return (
      <Dialog title={`Greener substitutes for ${m.name}`} wide>
        <p class="muted">Materials that pass the same stages, with an index at least as good as you allow, and less CO₂ for the job. <Help k="greener" /></p>
        <p class="inline">Allow an index up to <select value={tol()} onChange={e => setTol(+e.currentTarget.value)}><For each={[0, 0.05, 0.1, 0.2, 0.5]}>{t => <option value={t} selected={t === tol()}>{t * 100} %</option>}</For></select> lower.</p>
        <Show when={s()}>{x => <>
          <p class="small"><b>Basis:</b> {x().basis}.</p>
          <Show when={x().rows.length} fallback={<p class="muted">{x().note}</p>}>
            <table class="tbl mini"><thead><tr><th>Material</th><th>Family</th><th class="r">CO₂ for the job</th><th class="r">Index</th><th>You give up</th><th /></tr></thead>
              <tbody><For each={x().rows}>{r => <tr><td><button class="lnk" onClick={() => openSheet(r.id)}>{r.name}</button></td><td>{r.family}</td><td class="r"><b>× {fmt(r.co2)}</b></td><td class="r">{r.index !== null ? `× ${fmt(r.index)}` : ""}</td>
                <td class="small">{r.gives.map(([k, v]) => k === "price" ? `price × ${fmt(v)}` : `${name(k)} × ${fmt(v)}`).join("; ")}</td>
                <td><Show when={partK !== undefined} fallback={<button class="lnk" onClick={() => toggleCompare(r.id)}>+ compare</button>}><button class="lnk" onClick={() => editOpt(o => { const p = o.parts[partK!]; if (p) p.material = r.id; })}>use in the audit</button></Show></td></tr>}</For></tbody></table>
          </Show></>}</Show>
      </Dialog>
    );
  });
};
