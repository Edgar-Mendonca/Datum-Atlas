// Corrosion module: E–pH (Pourbaix) diagrams worked out by the engine from referenced Gibbs
// energies, galvanic couples from potentials you type, and corrosion kinetics (fits, Arrhenius,
// rate from mass loss, allowance). The engine is corrosion.rs; the inputs are kept in the project
// (window settings, under "corrosion").
import { For, Show, createEffect, createResource, createSignal, on } from "solid-js";
import { act, call, out, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";
import { Graph } from "./graph";

interface Region { name: string; kind: string; share: number; cx: number; cy: number }
interface Diagram { nx: number; ny: number; ph: [number, number]; e: [number, number]; cells: number[]; regions: Region[]; notes: string[]; water: [number, number, number][] }
interface Info { metals: { id: string; name: string }[]; notes: string[]; sce: number }
interface Fit { law: string; formula: string; k: number; b: number; r2: number; unit: string; curve: [number, number][] }
interface Kin { fits: Fit[] | null; fitError: string | null; arrhenius: { q: number; a: number; r2: number } | null; arrError: string | null; rate: number | null; allowance: number | null }

const fx = (v: number | null | undefined, d = 2) => v == null || !Number.isFinite(v) ? "–" : v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
const g3 = (v: number) => Math.abs(v) >= 1e4 || (Math.abs(v) < 1e-3 && v !== 0) ? v.toExponential(3) : (+v.toPrecision(4)).toString();
/** The inputs of the corrosion pages, saved with the project (a moment after the last change). */
const kept = (): any => (out()?.project.ui as any)?.corrosion ?? {};
const timers: Record<string, number> = {};
/** Saves sent but not yet back from the engine, so two pages saving at once do not undo each other. */
const pending: Record<string, any> = {};
const keep = (k: string, v: any) => {
  clearTimeout(timers[k]);
  timers[k] = window.setTimeout(() => {
    if (JSON.stringify(kept()[k]) === JSON.stringify(v)) return;
    pending[k] = v;
    act({ Ui: { key: "corrosion", value: { ...kept(), ...pending } } }, true).finally(() => { if (pending[k] === v) delete pending[k]; });
  }, 500);
};
const [info] = createResource(() => invoke<Info>("corrosion_info").catch(() => null));
const pairs = (t: string): [number, number][] => t.split(/\r?\n/).map(l => l.trim().split(/[\s;,]+/).map(Number)).filter(r => r.length >= 2 && Number.isFinite(r[0]) && Number.isFinite(r[1])).map(r => [r[0], r[1]]);

const KIND: Record<string, [number, number, number]> = { immunity: [200, 205, 214], passivity: [196, 181, 232], corrosion: [246, 211, 150] };
const KINDNAME: Record<string, string> = { immunity: "Immunity (the metal is stable)", passivity: "Passivity (a solid oxide or hydroxide)", corrosion: "Corrosion (dissolved ions)" };

export const PourbaixPage = () => {
  const k0 = kept().pb ?? {};
  const [metal, setMetal] = createSignal<string>(k0.metal ?? "fe"), [loga, setLoga] = createSignal<number>(k0.loga ?? -6), [sce, setSce] = createSignal<boolean>(k0.sce ?? false);
  const [d, setD] = createSignal<Diagram | null>(null), [err, setErr] = createSignal("");
  let cv!: HTMLCanvasElement;
  createEffect(on([metal, loga, sce], ([m, a, s]) => keep("pb", { metal: m, loga: a, sce: s }), { defer: true }));
  createEffect(on([metal, loga], async ([m, a]) => {
    try { setD(await invoke<Diagram>("pourbaix", { ask: { metal: m, log_a: a, ph: [-2, 16], e: [-2, 2], nx: 360, ny: 320 } })); setErr(""); }
    catch (e) { setErr(String(e)); setD(null); }
  }));
  // The diagram on a canvas: regions by kind (each a shade), boundaries, the water lines, labels, axes.
  createEffect(() => {
    const g = d(), off = sce() ? info()?.sce ?? 0.241 : 0; if (!g || !cv) return;
    const S = 2, L = 56, T = 14, W = 720, H = 560, pw = W - L - 16, ph = H - T - 46;
    cv.width = W * S; cv.height = H * S; cv.style.aspectRatio = `${W} / ${H}`;
    const x = cv.getContext("2d")!; x.setTransform(S, 0, 0, S, 0, 0);
    const css = getComputedStyle(document.documentElement), ink = css.getPropertyValue("--text").trim() || "#222", bg = css.getPropertyValue("--bg").trim() || "#fff";
    x.fillStyle = bg; x.fillRect(0, 0, W, H);
    const img = x.createImageData(g.nx, g.ny);
    const shade = g.regions.map((r, i) => { const c = KIND[r.kind] ?? [220, 220, 220], t = (i % 3) * 10; return [c[0] - t, c[1] - t, c[2] - t]; });
    for (let j = 0; j < g.ny; j++) for (let i = 0; i < g.nx; i++) {
      const k = j * g.nx + i, c = g.cells[k], edge = (i + 1 < g.nx && g.cells[k + 1] !== c) || (j + 1 < g.ny && g.cells[k + g.nx] !== c);
      const s = edge ? [40, 40, 50] : shade[c]; img.data.set([s[0], s[1], s[2], 255], 4 * k);
    }
    const tmp = document.createElement("canvas"); tmp.width = g.nx; tmp.height = g.ny; tmp.getContext("2d")!.putImageData(img, 0, 0);
    x.imageSmoothingEnabled = false; x.drawImage(tmp, L, T, pw, ph);
    const X = (p: number) => L + (p - g.ph[0]) / (g.ph[1] - g.ph[0]) * pw, Y = (e: number) => T + (g.e[1] - e) / (g.e[1] - g.e[0]) * ph;
    x.save(); x.beginPath(); x.rect(L, T, pw, ph); x.clip(); x.setLineDash([6, 4]); x.strokeStyle = "#0072B2"; x.lineWidth = 1.5;
    for (const k of [1, 2]) { x.beginPath(); x.moveTo(X(g.water[0][0]), Y(g.water[0][k])); x.lineTo(X(g.water[1][0]), Y(g.water[1][k])); x.stroke(); }
    x.restore(); x.setLineDash([]);
    x.fillStyle = "#111"; x.font = "600 12px 'IBM Plex Sans', sans-serif"; x.textAlign = "center";
    for (const r of g.regions) if (r.share > 0.6) x.fillText(r.name, X(r.cx), Y(r.cy) + 4);
    x.strokeStyle = ink; x.lineWidth = 1; x.strokeRect(L, T, pw, ph); x.fillStyle = ink; x.font = "11px 'IBM Plex Sans', sans-serif";
    for (let p = Math.ceil(g.ph[0]); p <= g.ph[1]; p += 2) { x.textAlign = "center"; x.fillText(String(p), X(p), T + ph + 15); x.beginPath(); x.moveTo(X(p), T + ph); x.lineTo(X(p), T + ph + 4); x.stroke(); }
    for (let e = Math.ceil(g.e[0] * 2) / 2; e <= g.e[1] + 1e-9; e += 0.5) { x.textAlign = "right"; x.fillText((e - off).toFixed(1), L - 6, Y(e) + 4); x.beginPath(); x.moveTo(L - 4, Y(e)); x.lineTo(L, Y(e)); x.stroke(); }
    x.textAlign = "center"; x.fillText("pH", L + pw / 2, H - 12);
    x.save(); x.translate(14, T + ph / 2); x.rotate(-Math.PI / 2); x.fillText(`E (V vs ${sce() ? "SCE" : "SHE"})`, 0, 0); x.restore();
  });
  const save = async () => { const p = await call<string | null>("save_file", { name: `E-pH ${metal()} 1e${loga()}.png`, data: cv.toDataURL("image/png") }); if (p) toast(`Saved ${p}`, true); };
  return (
    <div class="page corrpage">
      <h1>E–pH (Pourbaix) diagrams <Help k="pourbaix" /></h1>
      <div class="ttools">
        <label>Metal <select class="pbmetal" value={metal()} onChange={e => setMetal(e.currentTarget.value)}><For each={info()?.metals ?? []}>{m => <option value={m.id}>{m.name}</option>}</For></select></label>
        <label>Dissolved metal <select class="pbact" value={loga()} onChange={e => setLoga(+e.currentTarget.value)}><For each={[0, -2, -4, -6, -8]}>{a => <option value={a}>{a === 0 ? "1" : `10${String(a).replace("-", "⁻").replace(/\d/g, c => "⁰¹²³⁴⁵⁶⁷⁸⁹"[+c])}`} mol/L</option>}</For></select></label>
        <label><input type="checkbox" checked={sce()} onChange={e => setSce(e.currentTarget.checked)} /> Potentials against the calomel electrode (SCE)</label>
        <Show when={d()}><button class="btn sm" onClick={save}>Save as picture</button></Show>
      </div>
      <Show when={!err()} fallback={<p class="err">{err()}</p>}>
        <div class="imgrid">
          <section class="panel"><canvas ref={cv} class="pbcv" /></section>
          <section class="panel">
            <h2>Regions</h2>
            <p class="small ekey"><For each={Object.keys(KIND)}>{k => <span><i style={{ background: `rgb(${KIND[k].join(",")})` }} />{KINDNAME[k]}</span>}</For></p>
            <table class="ptable pbreg"><thead><tr><th>Stable species</th><th>Kind</th><th>Share of the diagram</th></tr></thead>
              <tbody><For each={d()?.regions ?? []}>{r => <tr><td>{r.name}</td><td>{r.kind}</td><td>{fx(r.share, 1)} %</td></tr>}</For></tbody></table>
            <For each={d()?.notes ?? []}>{n => <p class="small tnote">{n}</p>}</For>
            <For each={info()?.notes ?? []}>{n => <p class="muted small">{n}</p>}</For>
          </section>
        </div>
      </Show>
    </div>
  );
};

interface Side { name: string; min: number; max: number }
export const GalvanicPage = () => {
  const k0 = kept().galv ?? {};
  const [a, setA] = createSignal<Side>(k0.a ?? { name: "", min: NaN, max: NaN }), [b, setB] = createSignal<Side>(k0.b ?? { name: "", min: NaN, max: NaN });
  const [area, setArea] = createSignal<number>(k0.area ?? 1), [res, setRes] = createSignal<any>(null), [err, setErr] = createSignal("");
  createEffect(on([a, b, area], ([x, y, r]) => keep("galv", { a: x, b: y, area: r }), { defer: true }));
  createEffect(on([a, b, area], async ([x, y, r]) => {
    if (![x.min, x.max, y.min, y.max].every(Number.isFinite)) { setRes(null); setErr(""); return; }
    try { setRes(await invoke("galvanic", { a: { id: "a", name: x.name || "Metal A", min: x.min, max: x.max }, b: { id: "b", name: y.name || "Metal B", min: y.min, max: y.max }, area: r })); setErr(""); }
    catch (e) { setErr(String(e)); setRes(null); }
  }));
  const Row = (p: { s: () => Side; set: (s: Side) => void; l: string; c: string }) => <tr class={p.c}>
    <td>{p.l}</td><td><input class="gname" value={p.s().name} placeholder="e.g. Steel S275" onChange={e => p.set({ ...p.s(), name: e.currentTarget.value })} /></td>
    <td><input type="number" step="0.01" class="gmin" value={Number.isFinite(p.s().min) ? p.s().min : ""} onChange={e => { const v = e.currentTarget.value === "" ? NaN : +e.currentTarget.value; p.set({ ...p.s(), min: v, max: Number.isFinite(p.s().max) ? p.s().max : v }); }} /></td>
    <td><input type="number" step="0.01" class="gmax" value={Number.isFinite(p.s().max) ? p.s().max : ""} onChange={e => p.set({ ...p.s(), max: e.currentTarget.value === "" ? NaN : +e.currentTarget.value })} /></td></tr>;
  const nm = (id: string) => (id === "a" ? a().name || "Metal A" : b().name || "Metal B");
  return (
    <div class="page corrpage">
      <h1>Galvanic couple <Help k="galvanic" /></h1>
      <p class="muted small">Type the corrosion potential of each metal in the same electrolyte and against the same reference electrode: measured, or from a source you trust. Give a range (lowest, highest) or the same number twice.</p>
      <table class="ptable gtab"><thead><tr><th></th><th>Metal</th><th>Lowest (V)</th><th>Highest (V)</th></tr></thead>
        <tbody><Row s={a} set={setA} l="A" c="ga" /><Row s={b} set={setB} l="B" c="gb" /></tbody></table>
      <div class="ttools"><label>Area of the more noble metal ÷ area of the other <input type="number" class="garea" step="0.1" min="0" value={area()} onChange={e => setArea(Math.max(0, +e.currentTarget.value))} /></label></div>
      <Show when={err()}><p class="err">{err()}</p></Show>
      <Show when={res()}>{r => <section class="panel">
        <table class="ptable ires gres"><tbody>
          <tr><td>Corrodes (the anode)</td><td class="ganode">{nm(r().anode)}</td></tr>
          <tr><td>Protected (the cathode)</td><td>{nm(r().cathode)}</td></tr>
          <tr><td>Potential difference (middles of the ranges)</td><td class="gde">{fx(r().de, 3)} V</td></tr>
          <tr><td>Possible spread</td><td>{fx(r().deMin, 3)} to {fx(r().deMax, 3)} V</td></tr>
          <tr><td>Risk</td><td class="grisk">{r().risk}</td></tr></tbody></table>
        <For each={r().notes}>{n => <p class="small tnote">{n}</p>}</For>
        <p class="muted small">The risk words follow the usual rule of thumb for metals in contact: below about 0.05 V little happens; up to 0.15 V only in a wet, salty place; above 0.25 V even indoors. A potential difference says which metal corrodes, not how fast: that depends on the electrolyte, the areas and the films on the metals.</p>
      </section>}</Show>
    </div>
  );
};

const EX_KIN = "1 0.21\n4 0.40\n9 0.61\n16 0.79\n25 1.01\n36 1.19\n49 1.41";
export const KineticsPage = () => {
  const k0 = kept().kin ?? {};
  const [pts, setPts] = createSignal<string>(k0.pts ?? ""), [arr, setArr] = createSignal<string>(k0.arr ?? "");
  const [m, setM] = createSignal<{ w: number; area: number; rho: number; hours: number; years: number; factor: number }>(k0.m ?? { w: 0, area: 0, rho: 7.87, hours: 0, years: 20, factor: 1 });
  const [res, setRes] = createSignal<Kin | null>(null);
  createEffect(on([pts, arr, m], ([p, a, q]) => keep("kin", { pts: p, arr: a, m: q }), { defer: true }));
  createEffect(on([pts, arr, m], async ([p, a, q]) => setRes(await invoke<Kin>("kinetics", { ask: { pts: pairs(p), arr: pairs(a), ...q } }).catch(() => null))));
  const num = (k: keyof ReturnType<typeof m>, l: string, unit: string, cls = "") => <label>{l} <input type="number" class={cls} step="any" min="0" value={m()[k] || ""} onChange={e => setM({ ...m(), [k]: Math.max(0, +e.currentTarget.value || 0) })} /> {unit}</label>;
  const data = () => pairs(pts());
  return (
    <div class="page corrpage">
      <h1>Corrosion kinetics <Help k="kinetics" /></h1>
      <div class="imgrid">
        <section class="panel">
          <h2>Mass change against time</h2>
          <p class="muted small">One point per line: time (h), mass change (mg/cm²): a gain for oxidation, a loss as a positive number. <button class="lnk kex" onClick={() => setPts(EX_KIN)}>Example (made-up data)</button></p>
          <textarea class="kpts" rows="8" value={pts()} onChange={e => setPts(e.currentTarget.value)} />
          <Show when={res()?.fitError && data().length}><p class="err">{res()!.fitError}</p></Show>
          <Show when={res()?.fits}>{f => <>
            <table class="ptable kfits"><thead><tr><th>Law</th><th>Fit</th><th>Constant</th><th>R²</th></tr></thead>
              <tbody><For each={f()}>{(x, i) => <tr classList={{ best: i() === 0 }}><td>{x.law}</td><td>{x.formula}</td><td>{x.law === "Power" ? `k = ${g3(x.k)}, n = ${fx(x.b, 3)}` : x.law === "Logarithmic" ? `k = ${g3(x.k)}, a = ${g3(x.b)}` : g3(x.k)} <span class="muted small">{x.unit}</span></td><td>{fx(x.r2, 4)}</td></tr>}</For></tbody></table>
            <p class="small tnote kbest">Best fit: <b>{f()[0].law}</b> ({f()[0].formula}).</p>
            <Graph name="Corrosion kinetics" ratio={0.55} spec={{ x: { label: "Time (h)", min: 0 }, y: { label: "Mass change (mg/cm²)" }, legend: true,
              series: [{ name: "Measured", kind: "scatter", x: data().map(q => q[0]), y: data().map(q => q[1]) }, ...f().slice(0, 2).map(x => ({ name: `${x.law} fit`, kind: "line" as const, x: x.curve.map(q => q[0]), y: x.curve.map(q => q[1]) }))] }}
              fallback={<p class="muted small">The graph needs the Python engine.</p>} />
          </>}</Show>
        </section>
        <section class="panel">
          <h2>Temperature (Arrhenius)</h2>
          <p class="muted small">One rate constant per line: temperature (°C), k (any unit, the same for all).</p>
          <textarea class="karr" rows="4" value={arr()} onChange={e => setArr(e.currentTarget.value)} />
          <Show when={res()?.arrhenius}>{a => <table class="ptable ires"><tbody><tr><td>Activation energy Q</td><td class="kq">{fx(a().q, 1)} kJ/mol</td></tr><tr><td>A (k = A·e^(−Q/RT))</td><td>{g3(a().a)}</td></tr><tr><td>R² of ln k against 1/T</td><td>{fx(a().r2, 4)}</td></tr></tbody></table>}</Show>
          <Show when={res()?.arrError}><p class="err">{res()!.arrError}</p></Show>
          <h2>Rate from a mass loss</h2>
          <div class="cform">{num("w", "Mass lost", "g", "kw")}{num("area", "Area exposed", "cm²", "ka")}{num("rho", "Density", "g/cm³", "kr")}{num("hours", "Time", "h", "kh")}</div>
          <table class="ptable ires"><tbody><tr><td>Corrosion rate</td><td class="krate">{res()?.rate == null ? "–" : `${fx(res()!.rate, 4)} mm/year`}</td></tr></tbody></table>
          <h2>Corrosion allowance</h2>
          <div class="cform">{num("years", "Design life", "years")}{num("factor", "Safety factor", "")}</div>
          <table class="ptable ires"><tbody><tr><td>Thickness to add</td><td class="kallow">{res()?.allowance == null ? "–" : `${fx(res()!.allowance, 2)} mm`}</td></tr></tbody></table>
          <p class="muted small">Uniform corrosion only: pitting, crevices and galvanic attack go much faster locally.</p>
        </section>
      </div>
    </div>
  );
};
