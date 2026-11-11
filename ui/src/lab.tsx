// Testing module › Hardness, Fatigue and Creep: data saved in the project through the Lab edit, and
// what the engine works out (lab.rs): hardness statistics, a Basquin S–N fit, a Larson–Miller creep
// fit. "Make a material" turns all the project's test results into a material of your own.
import { For, Show, createSignal } from "solid-js";
import { act, call, out, P, toast } from "./store";
import type { Material } from "./api";
import { editMaterial } from "./material";
import { Help } from "./help";

export interface HardSet { name: string; scale: string; values: number[] }
export interface FatPoint { s: number; n: number; runout: boolean }
export interface CreepPoint { s: number; t: number; h: number }
export interface Lab { hardness: HardSet[]; fatigue: FatPoint[]; fat_at: number; creep: CreepPoint[]; lmp_c: number; creep_at: [number, number] }
interface HardOut { name: string; scale: string; n: number; mean: number | null; sd: number | null; min: number | null; max: number | null; cov: number | null; ci: [number, number] | null; tabor: number | null }
export interface LabOut { hardness: HardOut[]; fatigue: { points: number; broken: number; sf: number | null; b: number | null; r2: number | null; at: number; strength: number | null; note: string };
  creep: { points: number; c: number | null; found: boolean; a: [number, number] | null; r2: number | null; p: [number, number][]; stress: number | null; pAt: number | null; note: string } }

const SCALES = ["HV", "HB", "HK", "HRC", "HRB", "HR15N", "Shore A", "Shore D"];
const L = () => P().lab;
const R = () => out()?.lab;
const save = (f: (l: Lab) => void) => { const l = structuredClone(L()); f(l); return act({ Lab: { lab: l } }); };
const fx = (v: number | null | undefined, d = 1) => v == null ? "–" : v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
/** 10 to a power, with superscript digits. */
const pow10 = (e: number) => `10${String(e).replace(/./g, c => "⁰¹²³⁴⁵⁶⁷⁸⁹"["0123456789".indexOf(c)] ?? "⁻")}`;
const sci = (v: number) => { if (v < 1e5) return v.toLocaleString("en-GB", { maximumFractionDigits: 0 }); const [m, e] = v.toExponential(2).split("e+"); return `${m} × ${pow10(+e)}`; };
/** The numbers of a pasted block of text: one row per line, numbers split by spaces, tabs, commas or semicolons. */
const rows = (t: string) => t.split(/\r?\n/).map(l => l.trim()).filter(Boolean).map(l => l.split(/[\s;,]+/));
const numsOf = (t: string) => t.split(/[\s;,]+/).map(Number).filter(v => Number.isFinite(v) && v >= 0);

/** Turns the project's tests into a material of your own, in the Add Material dialog. */
export const makeMaterial = async () => {
  const m = await call<Material>("test_material", { name: `${P().name} (tested)`, src: "__own" });
  if (!m) return;
  if (!Object.keys(m.props).length) return toast("No results yet: add tensile specimens, hardness readings or fatigue tests first.");
  editMaterial(m, false, { title: `Tests in the project “${P().name}”`, kind: "user", year: String(new Date().getFullYear()) });
};
const MakeBtn = () => <button class="btn mkmat" title="A material of your own from every test result in this project" onClick={makeMaterial}>Make a material of the results</button>;
const Example = () => <button class="btn labex" title="Made-up hardness, fatigue and creep data, to try the pages" onClick={async () => { const e = await call<Lab>("lab_example"); if (e) await act({ Lab: { lab: e } }); }}>Example (made-up data)</button>;

/** A log–log or log–linear scatter with a fitted line. */
const Plot = (p: { pts: { x: number; y: number; open?: boolean; title: string }[]; line?: [number, number][]; q?: [number, number]; lx: boolean; xl: string; yl: string }) => {
  const W = 560, H = 320, Lm = 56, B = 38;
  const tx = (v: number) => p.lx ? Math.log10(v) : v, ty = (v: number) => Math.log10(v);
  const xs = () => [...p.pts.map(q => tx(q.x)), ...(p.line ?? []).map(q => tx(q[0]))], ys = () => [...p.pts.map(q => ty(q.y)), ...(p.line ?? []).map(q => ty(q[1]))];
  const xr = () => { const a = Math.min(...xs()), b = Math.max(...xs()), m = (b - a || 1) * 0.06; return [p.lx ? Math.floor(a) : a - m, p.lx ? Math.ceil(b) : b + m]; };
  const yr = () => { const a = Math.min(...ys()), b = Math.max(...ys()); return [Math.floor(a * 10) / 10 - 0.05, Math.ceil(b * 10) / 10 + 0.05]; };
  const X = (v: number) => Lm + (tx(v) - xr()[0]) / (xr()[1] - xr()[0]) * (W - Lm - 12), Y = (v: number) => H - B - (ty(v) - yr()[0]) / (yr()[1] - yr()[0]) * (H - B - 12);
  const xt = () => { const [a, b] = xr(); if (p.lx) return Array.from({ length: b - a + 1 }, (_, k) => 10 ** (a + k)); const st = (b - a) / 6, m = 10 ** Math.floor(Math.log10(st)), s = [1, 2, 5, 10].map(k => k * m).find(k => k >= st)!; const o = []; for (let v = Math.ceil(a / s) * s; v <= b; v += s) o.push(+v.toFixed(6)); return o; };
  const yt = () => { const [a, b] = yr(); const o: number[] = []; for (let e = Math.floor(a); e <= Math.ceil(b); e++) for (const k of [1, 2, 5]) { const v = k * 10 ** e; if (Math.log10(v) >= a && Math.log10(v) <= b) o.push(v); } return o; };
  return <Show when={p.pts.length}><svg class="lplot" viewBox={`0 0 ${W} ${H}`}>
    <For each={yt()}>{t => <><line x1={Lm} x2={W - 12} y1={Y(t)} y2={Y(t)} stroke="var(--line)" /><text x={Lm - 6} y={Y(t) + 3} text-anchor="end" font-size="10" fill="var(--text2)">{t}</text></>}</For>
    <For each={xt()}>{t => <><line x1={X(t)} x2={X(t)} y1={H - B} y2={H - B + 4} stroke="var(--text2)" /><text x={X(t)} y={H - B + 15} text-anchor="middle" font-size="10" fill="var(--text2)">{p.lx ? pow10(Math.round(Math.log10(t))) : t}</text></>}</For>
    <line x1={Lm} x2={W - 12} y1={H - B} y2={H - B} stroke="var(--text2)" /><line x1={Lm} x2={Lm} y1={8} y2={H - B} stroke="var(--text2)" />
    <text x={(Lm + W) / 2} y={H - 4} text-anchor="middle" font-size="11" fill="var(--text)">{p.xl}</text>
    <text transform={`translate(13 ${(H - B) / 2}) rotate(-90)`} text-anchor="middle" font-size="11" fill="var(--text)">{p.yl}</text>
    <Show when={p.line}><polyline fill="none" stroke="var(--blue)" stroke-width="1.6" points={p.line!.map(q => `${X(q[0])},${Y(q[1])}`).join(" ")} /></Show>
    <For each={p.pts}>{q => q.open ? <path d={`M${X(q.x) - 5},${Y(q.y)} l4,-4 l0,8 z M${X(q.x)},${Y(q.y)} h6`} fill="none" stroke="var(--text2)" stroke-width="1.4"><title>{q.title}</title></path> : <circle cx={X(q.x)} cy={Y(q.y)} r="3.6" fill="var(--blue)" fill-opacity=".85"><title>{q.title}</title></circle>}</For>
    <Show when={p.q}><circle cx={X(p.q![0])} cy={Y(p.q![1])} r="6" fill="none" stroke="#F2B233" stroke-width="2.2" /></Show>
  </svg></Show>;
};

export const HardnessPage = () => {
  const [text, setText] = createSignal(""), [name, setName] = createSignal(""), [scale, setScale] = createSignal("HV");
  const add = () => { const v = numsOf(text()); if (!v.length) return toast("Type or paste the readings: numbers separated by spaces, commas or new lines."); save(l => l.hardness.push({ name: name().trim() || `Set ${l.hardness.length + 1}`, scale: scale(), values: v })).then(ok => ok && setText("")); };
  return <div class="page labpage">
    <h1>Hardness <Help k="hardness" /></h1>
    <div class="ttools"><Example /><MakeBtn /></div>
    <section class="panel"><h2>Add readings</h2>
      <div class="lrow"><label>Name <input class="hname" value={name()} placeholder="e.g. Plate A, surface" onInput={e => setName(e.currentTarget.value)} /></label>
        <label>Scale <select class="hscale" value={scale()} onChange={e => setScale(e.currentTarget.value)}><For each={SCALES}>{s => <option>{s}</option>}</For></select></label></div>
      <textarea class="hvals" rows="3" placeholder="Readings, e.g. 212 218 209 221 215" value={text()} onInput={e => setText(e.currentTarget.value)} />
      <p><button class="btn pri hadd" onClick={add}>Add the set</button></p></section>
    <Show when={R()?.hardness.length}><section class="panel"><h2>Results</h2>
      <div class="hscroll"><table class="ptable hres"><thead><tr><th>Set</th><th>Scale</th><th>n</th><th>Mean</th><th>Standard deviation</th><th>CoV (%)</th><th>95 % interval of the mean</th><th>Lowest</th><th>Highest</th><th>σy by Tabor (MPa)</th><th /></tr></thead><tbody>
        <For each={R()!.hardness}>{(h, i) => <tr><td>{h.name}</td><td>{h.scale}</td><td>{h.n}</td><td>{fx(h.mean)}</td><td>{fx(h.sd)}</td><td>{fx(h.cov)}</td><td>{h.ci ? `${fx(h.ci[0])} to ${fx(h.ci[1])}` : "–"}</td><td>{fx(h.min)}</td><td>{fx(h.max)}</td><td>{fx(h.tabor, 0)}</td>
          <td><button class="lnk" title="Remove this set" onClick={() => save(l => { l.hardness.splice(i(), 1); })}>✕</button></td></tr>}</For></tbody></table></div>
      <p class="muted small">The interval is where the true mean lies with 95 % confidence (Student's t). Tabor's relation, σy ≈ H/3 with H in MPa, is a rough estimate for metals from Vickers hardness only.</p></section></Show>
  </div>;
};

export const FatiguePage = () => {
  const [text, setText] = createSignal("");
  const f = () => R()?.fatigue;
  const add = () => {
    const pts = rows(text()).map(r => ({ s: +r[0], n: +r[1], runout: /^(r|run|runout|>)/i.test(r[2] ?? "") })).filter(p => p.s > 0 && p.n >= 1);
    if (!pts.length) return toast("One test per line: stress amplitude in MPa, cycles, and r for a run-out.");
    save(l => { l.fatigue.push(...pts); }).then(ok => ok && setText(""));
  };
  const line = (): [number, number][] | undefined => { const x = f(); if (x?.sf == null || x.b == null) return undefined; const ns = L().fatigue.map(p => p.n).concat([x.at]); const a = Math.min(...ns), b = Math.max(...ns); return [a, b].map(n => [n, x.sf! * (2 * n) ** x.b!]); };
  return <div class="page labpage">
    <h1>Fatigue <Help k="fatigue" /></h1>
    <div class="ttools"><Example /><MakeBtn /><label>Strength at <input type="number" class="fatat" value={L().fat_at} onChange={e => save(l => { l.fat_at = +e.currentTarget.value; })} /> cycles</label></div>
    <div class="labgrid">
      <section class="panel"><h2>Tests</h2>
        <textarea class="fvals" rows="4" placeholder={"One test per line: stress amplitude (MPa), cycles, r for a run-out\n400 31000\n260 10000000 r"} value={text()} onInput={e => setText(e.currentTarget.value)} />
        <p><button class="btn pri fadd" onClick={add}>Add the tests</button> <Show when={L().fatigue.length}><button class="btn" onClick={() => save(l => { l.fatigue = []; })}>Clear all</button></Show></p>
        <Show when={L().fatigue.length}><div class="hscroll lscroll"><table class="ptable"><thead><tr><th>σa (MPa)</th><th>Cycles</th><th /><th /></tr></thead><tbody>
          <For each={L().fatigue}>{(p, i) => <tr><td>{p.s}</td><td>{sci(p.n)}</td><td>{p.runout ? "run-out" : ""}</td><td><button class="lnk" onClick={() => save(l => { l.fatigue.splice(i(), 1); })}>✕</button></td></tr>}</For></tbody></table></div></Show>
      </section>
      <section class="panel"><h2>S–N curve</h2>
        <Plot pts={L().fatigue.map(p => ({ x: p.n, y: p.s, open: p.runout, title: `${p.s} MPa, ${sci(p.n)} cycles${p.runout ? ", run-out" : ""}` }))} line={line()} lx xl="Cycles to failure N" yl="Stress amplitude σa (MPa)" q={f()?.strength != null ? [L().fat_at, f()!.strength!] : undefined} />
        <Show when={f()}>{x => <div class="fres"><p>Basquin: σa = <b>{fx(x().sf, 0)}</b> MPa × (2N)<sup>{fx(x().b, 4)}</sup> &nbsp; R² = {fx(x().r2, 3)}, from {x().broken} broken specimens</p>
          <p class="fstr">Fatigue strength at {sci(x().at)} cycles: <b>{fx(x().strength, 0)} MPa</b></p>
          <Show when={x().note}><p class="small tnote">{x().note}</p></Show></div>}</Show></section>
    </div>
  </div>;
};

export const CreepPage = () => {
  const [text, setText] = createSignal("");
  const c = () => R()?.creep;
  const add = () => {
    const pts = rows(text()).map(r => ({ s: +r[0], t: +r[1], h: +r[2] })).filter(p => p.s > 0 && Number.isFinite(p.t) && p.h > 0);
    if (!pts.length) return toast("One test per line: stress in MPa, temperature in °C, hours to rupture.");
    save(l => { l.creep.push(...pts); }).then(ok => ok && setText(""));
  };
  const line = (): [number, number][] | undefined => { const x = c(); if (!x?.a || !x.p.length) return undefined; const ps = x.p.map(q => q[0]).concat(x.pAt ?? []); return [Math.min(...ps), Math.max(...ps)].map(pp => [pp, 10 ** (x.a![0] + x.a![1] * pp)]); };
  return <div class="page labpage">
    <h1>Creep rupture <Help k="creep" /></h1>
    <div class="ttools"><Example /><MakeBtn />
      <label>Larson–Miller C <input type="number" class="lmpc" value={L().lmp_c} onChange={e => save(l => { l.lmp_c = +e.currentTarget.value; })} /> (0: find it)</label>
      <label>Stress to rupture at <input type="number" class="cat0" value={L().creep_at[0]} onChange={e => save(l => { l.creep_at[0] = +e.currentTarget.value; })} /> °C in <input type="number" class="cat1" value={L().creep_at[1]} onChange={e => save(l => { l.creep_at[1] = +e.currentTarget.value; })} /> h</label></div>
    <div class="labgrid">
      <section class="panel"><h2>Tests</h2>
        <textarea class="cvals" rows="4" placeholder={"One test per line: stress (MPa), temperature (°C), hours to rupture\n220 550 3100"} value={text()} onInput={e => setText(e.currentTarget.value)} />
        <p><button class="btn pri cadd" onClick={add}>Add the tests</button> <Show when={L().creep.length}><button class="btn" onClick={() => save(l => { l.creep = []; })}>Clear all</button></Show></p>
        <Show when={L().creep.length}><div class="hscroll lscroll"><table class="ptable"><thead><tr><th>σ (MPa)</th><th>T (°C)</th><th>Hours</th><th>P (×1000)</th><th /></tr></thead><tbody>
          <For each={L().creep}>{(p, i) => <tr><td>{p.s}</td><td>{p.t}</td><td>{sci(p.h)}</td><td>{fx(c()?.p[i()]?.[0], 2)}</td><td><button class="lnk" onClick={() => save(l => { l.creep.splice(i(), 1); })}>✕</button></td></tr>}</For></tbody></table></div></Show>
      </section>
      <section class="panel"><h2>Larson–Miller plot</h2>
        <Plot pts={(c()?.p ?? []).map((q, i) => ({ x: q[0], y: q[1], title: `${q[1]} MPa at ${L().creep[i]?.t} °C, ${sci(L().creep[i]?.h ?? 0)} h` }))} line={line()} lx={false} xl="Larson–Miller parameter P = T (C + log t) / 1000 (T in K, t in h)" yl="Stress (MPa)" q={c()?.stress != null && c()?.pAt != null ? [c()!.pAt!, c()!.stress!] : undefined} />
        <Show when={c()}>{x => <div class="cres2"><p>C = <b>{fx(x().c, 1)}</b>{x().found ? " (found: the best fit)" : ""}; log σ = {fx(x().a?.[0], 3)} {x().a ? (x().a![1] < 0 ? "−" : "+") : ""} {fx(x().a ? Math.abs(x().a![1]) : null, 4)} P &nbsp; R² = {fx(x().r2, 3)}</p>
          <p class="cstr">Stress to rupture at {L().creep_at[0]} °C in {sci(L().creep_at[1])} h: <b>{fx(x().stress, 0)} MPa</b></p>
          <Show when={x().note}><p class="small tnote">{x().note}</p></Show></div>}</Show></section>
    </div>
  </div>;
};
