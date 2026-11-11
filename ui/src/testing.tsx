// Testing module › Tensile: specimens with their load–extension or stress–strain data (saved in the
// project through the Tensile edit), the curves, and the results the engine works out (tensile.rs):
// E, Rp0.2, Rm, Ag, A, work to fracture, n and K, with mean, standard deviation and range.
import { For, Show, createSignal } from "solid-js";
import { act, call, out, P, toast } from "./store";
import { CMP_COLORS } from "./api";
import { Help } from "./help";

export interface Specimen { name: string; x: string; y: string; shape: string; w: number; t: number; d: number; area: number; l0: number; on: boolean; data: [number, number][] }
export interface Tensile { specimens: Specimen[]; fit: [number, number]; material: string }
interface Res { name: string; on: boolean; points: number; area: number; e: number | null; rp02: number | null; rm: number | null; ag: number | null; a: number | null; work: number | null; n: number | null; k: number | null;
  toe: number; curve: [number, number][]; truec: [number, number][]; marks: [string, [number, number]][]; notes: string[] }
interface Stat { key: string; name: string; unit: string; n: number; mean: number | null; sd: number | null; min: number | null; max: number | null }
export interface TensileOut { rows: Res[]; stats: Stat[] }
interface Cols { head: string[]; rows: (number | null)[][]; x: number; y: number; xk: string; yk: string; skipped: number }

const KEYS: [keyof Res, string, string, number][] = [["e", "E", "GPa", 1], ["rp02", "Rp0.2", "MPa", 0], ["rm", "Rm", "MPa", 0], ["ag", "Ag", "%", 1], ["a", "A", "%", 1], ["work", "Work", "MJ/m³", 0], ["n", "n", "", 3], ["k", "K", "MPa", 0]];
const XK: [string, string][] = [["ext", "Extension (mm)"], ["strain", "Strain (mm/mm)"], ["strain%", "Strain (%)"]];
const YK: [string, string][] = [["N", "Load (N)"], ["kN", "Load (kN)"], ["MPa", "Stress (MPa)"]];
const fx = (v: number | null | undefined, d: number) => v == null ? "–" : v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
const T = () => P().tensile;
const save = (f: (t: Tensile) => void) => { const t = structuredClone(T()); f(t); return act({ Tensile: { tensile: t } }); };
/** Tick marks at a round step. */
const ticks = (lo: number, hi: number, n = 6) => { const raw = (hi - lo) / n, m = 10 ** Math.floor(Math.log10(raw || 1)), step = [1, 2, 2.5, 5, 10].map(k => k * m).find(s => s >= raw) ?? raw; const out: number[] = []; for (let v = Math.ceil(lo / step) * step; v <= hi + 1e-9; v += step) out.push(+v.toPrecision(10)); return out; };

const Curves = (p: { rows: Res[]; tru: boolean }) => {
  const W = 640, H = 340, L = 54, B = 36;
  const lines = () => p.rows.map((r, i) => ({ r, i, pts: p.tru ? r.truec : r.curve })).filter(x => x.r.on && x.pts.length);
  const xmax = () => Math.max(1, ...lines().flatMap(l => l.pts.map(q => q[0]))) * 1.04, ymax = () => Math.max(10, ...lines().flatMap(l => l.pts.map(q => q[1]))) * 1.08;
  const X = (v: number) => L + v / xmax() * (W - L - 12), Y = (v: number) => H - B - v / ymax() * (H - B - 12);
  return <svg class="tcurve" viewBox={`0 0 ${W} ${H}`}>
    <For each={ticks(0, ymax())}>{t => <><line x1={L} x2={W - 12} y1={Y(t)} y2={Y(t)} stroke="var(--line)" /><text x={L - 6} y={Y(t) + 3} text-anchor="end" font-size="10" fill="var(--text2)">{t}</text></>}</For>
    <For each={ticks(0, xmax())}>{t => <><line x1={X(t)} x2={X(t)} y1={H - B} y2={H - B + 4} stroke="var(--text2)" /><text x={X(t)} y={H - B + 15} text-anchor="middle" font-size="10" fill="var(--text2)">{t}</text></>}</For>
    <line x1={L} x2={W - 12} y1={H - B} y2={H - B} stroke="var(--text2)" /><line x1={L} x2={L} y1={8} y2={H - B} stroke="var(--text2)" />
    <text x={(L + W) / 2} y={H - 4} text-anchor="middle" font-size="11" fill="var(--text)">{p.tru ? "True strain (%)" : "Engineering strain (%)"}</text>
    <text transform={`translate(13 ${(H - B) / 2}) rotate(-90)`} text-anchor="middle" font-size="11" fill="var(--text)">{p.tru ? "True stress (MPa)" : "Engineering stress (MPa)"}</text>
    <For each={lines()}>{l => <><polyline fill="none" stroke={CMP_COLORS[l.i % CMP_COLORS.length]} stroke-width="1.6" points={l.pts.map(q => `${X(q[0]).toFixed(1)},${Y(q[1]).toFixed(1)}`).join(" ")} />
      <Show when={!p.tru}><For each={l.r.marks}>{([k, q]) => <circle cx={X(q[0])} cy={Y(q[1])} r={k === "Fracture" ? 3 : 4} fill={k === "Rm" ? CMP_COLORS[l.i % CMP_COLORS.length] : "var(--bg)"} stroke={CMP_COLORS[l.i % CMP_COLORS.length]} stroke-width="1.5"><title>{`${l.r.name}: ${k} ${q[1].toFixed(0)} MPa at ${q[0].toFixed(2)} %`}</title></circle>}</For></Show></>}</For>
  </svg>;
};

/** The columns of a file being added as a specimen. */
const Adder = (p: { file: { path: string; name: string; cols: Cols }; done: () => void }) => {
  const c = p.file.cols;
  const [s, setS] = createSignal<Specimen>({ name: p.file.name || `Specimen ${T().specimens.length + 1}`, x: c.xk, y: c.yk, shape: "round", w: 0, t: 0, d: 6, area: 0, l0: 30, on: true, data: [] });
  const [xi, setXi] = createSignal(c.x), [yi, setYi] = createSignal(c.y);
  const set = (f: (v: Specimen) => void) => { const v = { ...s() }; f(v); setS(v); };
  const add = async () => {
    const data = c.rows.map(r => [r[xi()], r[yi()]] as [number | null, number | null]).filter((q): q is [number, number] => q[0] != null && q[1] != null);
    if (data.length < 10) return toast("Those two columns have fewer than 10 rows of numbers.");
    if (await save(t => t.specimens.push({ ...s(), data }))) p.done();
  };
  return <section class="panel tadd">
    <h2>Add a specimen from {p.file.path.split(/[\\/]/).pop()}</h2>
    <p class="muted small">{c.rows.length} rows of numbers{c.skipped ? `, after ${c.skipped} lines of heading` : ""}. Check the columns and what they hold, and the specimen's size.</p>
    <div class="tgrid">
      <label>Name <input class="tname" value={s().name} onInput={e => set(v => { v.name = e.currentTarget.value; })} /></label>
      <label>Strain column <select class="txcol" value={xi()} onChange={e => setXi(+e.currentTarget.value)}><For each={c.head}>{(h, i) => <option value={i()}>{h}</option>}</For></select>
        <select value={s().x} onChange={e => set(v => { v.x = e.currentTarget.value; })}><For each={XK}>{k => <option value={k[0]}>{k[1]}</option>}</For></select></label>
      <label>Stress column <select class="tycol" value={yi()} onChange={e => setYi(+e.currentTarget.value)}><For each={c.head}>{(h, i) => <option value={i()}>{h}</option>}</For></select>
        <select value={s().y} onChange={e => set(v => { v.y = e.currentTarget.value; })}><For each={YK}>{k => <option value={k[0]}>{k[1]}</option>}</For></select></label>
      <Size s={s()} set={set} />
    </div>
    <p><button class="btn pri tok" onClick={add}>Add the specimen</button> <button class="btn" onClick={p.done}>Cancel</button></p>
  </section>;
};

const Size = (p: { s: Specimen; set: (f: (v: Specimen) => void) => void }) => <>
  <label>Section <select class="tshape" value={p.s.shape} onChange={e => p.set(v => { v.shape = e.currentTarget.value; })}><option value="round">Round</option><option value="rect">Rectangular</option><option value="area">Area given</option></select>
    <Show when={p.s.shape === "round"}><span>d <input type="number" class="td" value={p.s.d} onChange={e => p.set(v => { v.d = +e.currentTarget.value; })} /> mm</span></Show>
    <Show when={p.s.shape === "rect"}><span>w <input type="number" value={p.s.w} onChange={e => p.set(v => { v.w = +e.currentTarget.value; })} /> × t <input type="number" value={p.s.t} onChange={e => p.set(v => { v.t = +e.currentTarget.value; })} /> mm</span></Show>
    <Show when={p.s.shape === "area"}><span><input type="number" value={p.s.area} onChange={e => p.set(v => { v.area = +e.currentTarget.value; })} /> mm²</span></Show></label>
  <label>Gauge length <span><input type="number" class="tl0" value={p.s.l0} onChange={e => p.set(v => { v.l0 = +e.currentTarget.value; })} /> mm</span></label>
</>;

export const TensilePage = () => {
  const [file, setFile] = createSignal<{ path: string; name: string; cols: Cols } | null>(null);
  const [tru, setTru] = createSignal(false);
  const res = () => out()?.tensile;
  const open = async () => { const f = await call<{ path: string; name: string; cols: Cols } | null>("tensile_read", {}); if (f) setFile(f); };
  const example = async () => { const t = await call<Tensile>("tensile_example"); if (t) await save(x => { x.specimens.push(...t.specimens); }); };
  const edit = (i: number, f: (v: Specimen) => void) => save(t => f(t.specimens[i]));
  const exp = async (fmt: string) => { const p = await call<string | null>("export_results", { fmt, what: "tensile" }); if (p) toast(`Saved ${p}`, true); };
  return (
    <div class="page tenspage">
      <h1>Tensile tests <Help k="tensile" /></h1>
      <div class="ttools">
        <button class="btn timport" onClick={open}>Import test data…</button>
        <button class="btn texample" onClick={example} title="Three made-up specimens of a steel-like curve, to try the page">Example (made-up data)</button>
        <label>Modulus fitted from <input type="number" class="tfit0" value={Math.round(T().fit[0] * 100)} onChange={e => save(t => { t.fit[0] = +e.currentTarget.value / 100; })} /> to <input type="number" class="tfit1" value={Math.round(T().fit[1] * 100)} onChange={e => save(t => { t.fit[1] = +e.currentTarget.value / 100; })} /> % of Rm <Help k="tfit" /></label>
        <Show when={T().specimens.length}><button class="btn sm" onClick={() => exp("xlsx")}>To a spreadsheet</button><button class="btn sm" onClick={() => exp("csv")}>CSV</button></Show>
      </div>
      <Show when={file()}>{f => <Adder file={f()} done={() => setFile(null)} />}</Show>
      <Show when={T().specimens.length} fallback={<p class="muted tempty">No specimens yet. Import a test machine's export (CSV, text or a spreadsheet with a column of extension or strain and one of load or stress), or try the made-up example.</p>}>
        <div class="tmain">
          <section class="panel"><div class="tchead"><h2>Curves</h2><label class="small"><input type="checkbox" class="ttrue" checked={tru()} onChange={e => setTru(e.currentTarget.checked)} /> True stress and strain (up to Rm)</label></div>
            <Show when={res()}>{r => <Curves rows={r().rows} tru={tru()} />}</Show>
            <p class="muted small">Open circles: Rp0.2; filled: Rm; small: fracture.</p></section>
          <section class="panel"><h2>Specimens</h2>
            <div class="hscroll"><table class="ptable tspec"><thead><tr><th>On</th><th>Name</th><th>Points</th><th>Section</th><th>Gauge length</th><th>Columns</th><th /></tr></thead><tbody>
              <For each={T().specimens}>{(s, i) => <tr classList={{ off: !s.on }}>
                <td><input type="checkbox" checked={s.on} onChange={e => edit(i(), v => { v.on = e.currentTarget.checked; })} /></td>
                <td><i class="sw" style={{ background: CMP_COLORS[i() % CMP_COLORS.length] }} /><input value={s.name} onChange={e => edit(i(), v => { v.name = e.currentTarget.value; })} /></td>
                <td>{s.data.length}</td>
                <td>{s.shape === "round" ? `⌀ ${s.d} mm` : s.shape === "rect" ? `${s.w} × ${s.t} mm` : `${s.area} mm²`}</td>
                <td>{s.x === "ext" ? `${s.l0} mm` : "–"}</td>
                <td class="small">{XK.find(k => k[0] === s.x)?.[1]}, {YK.find(k => k[0] === s.y)?.[1]}</td>
                <td><button class="lnk" title="Remove this specimen" onClick={() => save(t => { t.specimens.splice(i(), 1); })}>✕</button></td></tr>}</For></tbody></table></div></section>
        </div>
        <Show when={res()}>{r => <section class="panel"><h2>Results <Help k="tresults" /></h2>
          <div class="hscroll"><table class="ptable tres"><thead><tr><th>Specimen</th><For each={KEYS}>{k => <th>{k[1]}{k[2] ? ` (${k[2]})` : ""}</th>}</For></tr></thead><tbody>
            <For each={r().rows}>{x => <tr classList={{ off: !x.on }}><td>{x.name}</td><For each={KEYS}>{k => <td>{fx(x[k[0]] as number | null, k[3])}</td>}</For></tr>}</For>
            <For each={[["Mean", "mean"], ["Standard deviation", "sd"], ["Lowest", "min"], ["Highest", "max"]] as const}>{([l, f]) => <tr class="stat"><td>{l}</td><For each={KEYS}>{k => <td>{fx(r().stats.find(s => s.key === k[0])?.[f], k[3])}</td>}</For></tr>}</For>
          </tbody></table></div>
          <For each={r().rows.filter(x => x.notes.length)}>{x => <p class="small tnote"><b>{x.name}:</b> {[...new Set(x.notes)].join(" ")}</p>}</For>
          <p class="muted small">Statistics cover the specimens that are on. A value that cannot be worked out is left blank, never zero.</p>
        </section>}</Show>
      </Show>
    </div>
  );
};
