// Diffraction module: measured X-ray patterns kept in the project (engine: xrd.rs). Patterns opens,
// lists and overlays them; Peaks processes one (background, smoothing, Kα2), finds and fits its
// peaks, and works out crystallite size (Scherrer) and size with strain (Williamson–Hall).
import { For, Show, createEffect, createSignal, on } from "solid-js";
import { call, setOut, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";
import { Graph } from "./graph";
import { forget, kept, keptId, keepSettings } from "./kept";

interface Info { name: string; points: number; from: number; to: number; anode: string; notes: string[] }
interface Peak { twoTheta: number; d: number; height: number; area: number; fwhm: number; breadth: number; eta: number; sizeNm: number | null; fitOk: boolean }
interface Answer { x: number[]; y: number[]; bg: number[]; net: number[]; lambda: number; peaks: Peak[]; wh: { sizeNm: number | null; strain: number; r2: number; points: [number, number][]; line: [number, number] } | null; notes: string[] }

const ANODES = ["Cu", "Co", "Mo", "Cr", "Fe", "Ag"];
const fx = (v: number | null | undefined, d = 2) => v == null || !Number.isFinite(v) ? "–" : v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
const COLORS = ["#6C4AB6", "#F2B233", "#009E73", "#D55E00", "#0072B2", "#CC79A7", "#56B4E9", "#E69F00"];
/** The patterns kept in the project (what the window shows about them). */
export const patterns = (): Info[] => kept("xrd")?.info ?? [];

const Tools = (p: { busy: boolean; setBusy: (b: boolean) => void }) => {
  const load = async (cmd: string) => { p.setBusy(true); try { setOut(await call(cmd, {})); } finally { p.setBusy(false); } };
  return <div class="ttools">
    <button class="btn xopen" onClick={() => load("xrd_open")}>Open a pattern…</button>
    <button class="btn xex" onClick={() => load("xrd_example")} title="A made-up FCC metal pattern with known crystallite size and strain">Example (made-up pattern)</button>
    <Show when={p.busy}><span class="muted small xbusy">working…</span></Show>
  </div>;
};

export const PatternsPage = () => {
  const [busy, setBusy] = createSignal(false), [stack, setStack] = createSignal(true), [norm, setNorm] = createSignal(false);
  const [data, setData] = createSignal<{ name: string; x: number[]; y: number[] }[]>([]);
  keepSettings("xrd", "overlay", () => ({ stack: stack(), norm: norm() }), v => { setStack(v.stack); setNorm(v.norm); });
  createEffect(on(() => keptId("xrd"), async id => setData(id ? (await call<any[]>("xrd_plot")) ?? [] : [])));
  const series = () => {
    const d = data(), tops = d.map(s => Math.max(...s.y, 1)), big = Math.max(...tops, 1);
    return d.map((s, k) => { const f = norm() ? 100 / tops[k] : 1, off = stack() ? k * (norm() ? 60 : big * 0.6) : 0;
      return { name: s.name, kind: "line" as const, x: s.x, y: s.y.map(v => v * f + off), color: COLORS[k % COLORS.length], width: 1 }; });
  };
  const remove = async (k: number) => { const o = await call("xrd_remove", { index: k }); if (o) setOut(o); };
  return (
    <div class="page xrdpage">
      <h1>Diffraction patterns <Help k="xrdpatterns" /></h1>
      <Tools busy={busy()} setBusy={setBusy} />
      <Show when={patterns().length} fallback={<p class="muted tempty">Open a measured X-ray pattern: two columns of 2θ and counts (.xy, .csv, .dat, .txt), an .xrdml file, or a GSAS .fxye file. Or try the made-up example. Patterns are kept in the project.</p>}>
        <table class="ptable xlist"><thead><tr><th>Pattern</th><th>Points</th><th>2θ range</th><th>Anode</th><th></th></tr></thead>
          <tbody><For each={patterns()}>{(p, k) => <tr><td>{p.name}<For each={p.notes}>{n => <div class="muted small">{n}</div>}</For></td><td>{p.points.toLocaleString("en-GB")}</td><td>{fx(p.from, 2)}–{fx(p.to, 2)}°</td><td>{p.anode || "–"}</td>
            <td><button class="lnk xrm" onClick={() => remove(k())} title="Take this pattern out of the project (Undo brings it back)">Remove</button></td></tr>}</For></tbody></table>
        <div class="ttools"><label><input type="checkbox" class="xstack" checked={stack()} onChange={e => setStack(e.currentTarget.checked)} /> Stack (offset each pattern)</label>
          <label><input type="checkbox" class="xnorm" checked={norm()} onChange={e => setNorm(e.currentTarget.checked)} /> Scale each to 100</label>
          <button class="lnk xforget" onClick={() => forget("xrd")}>Remove all from the project</button></div>
        <Show when={data().length}><Graph name="Diffraction patterns" ratio={0.5} spec={{ x: { label: "2θ (°)" }, y: { label: norm() ? "Intensity (strongest = 100)" : "Counts" }, legend: data().length > 1, series: series() }}
          fallback={<p class="muted small">The graph needs the Python engine.</p>} /></Show>
      </Show>
    </div>
  );
};

export const PeaksPage = () => {
  const [busy, setBusy] = createSignal(false), [sel, setSel] = createSignal(0);
  const [anode, setAnode] = createSignal("Cu"), [lambda, setLambda] = createSignal(1.5406), [zero, setZero] = createSignal(0), [from, setFrom] = createSignal(0), [to, setTo] = createSignal(180);
  const [smooth, setSmooth] = createSignal(0), [bg, setBg] = createSignal(3), [strip, setStrip] = createSignal(true), [minPeak, setMinPeak] = createSignal(2), [k, setK] = createSignal(0.9), [inst, setInst] = createSignal(0);
  const [res, setRes] = createSignal<Answer | null>(null), [err, setErr] = createSignal("");
  keepSettings("xrd", "peaks", () => ({ sel: sel(), anode: anode(), lambda: lambda(), zero: zero(), from: from(), to: to(), smooth: smooth(), bg: bg(), strip: strip(), minPeak: minPeak(), k: k(), inst: inst() }),
    v => { setSel(v.sel); setAnode(v.anode); setLambda(v.lambda); setZero(v.zero); setFrom(v.from); setTo(v.to); setSmooth(v.smooth); setBg(v.bg); setStrip(v.strip); setMinPeak(v.minPeak); setK(v.k); setInst(v.inst); });
  // A pattern that names its anode sets it.
  createEffect(on([sel, () => keptId("xrd")], () => { const a = patterns()[sel()]?.anode; if (a) setAnode(a); }, { defer: true }));
  let seq = 0;
  createEffect(on([() => keptId("xrd"), sel, anode, lambda, zero, from, to, smooth, bg, strip, minPeak, k, inst], async () => {
    if (!patterns().length) { setRes(null); return; }
    const i = Math.min(sel(), patterns().length - 1);
    const my = ++seq; await new Promise(r => setTimeout(r, 60)); if (my !== seq) return;
    setBusy(true);
    try { const r = await invoke<Answer>("xrd_analyse", { index: i, ask: { anode: anode() === "other" ? "" : anode(), lambda: lambda(), zero: zero(), range: [from(), to()], smooth: smooth(), bg: bg(), strip: strip(), min_peak: minPeak(), k: k(), inst: inst() } }); if (my === seq) { setRes(r); setErr(""); } }
    catch (e) { if (my === seq) { setErr(String(e)); setRes(null); } }
    finally { if (my === seq) setBusy(false); }
  }));
  const thin = (x: number[], y: number[]) => { const s = Math.max(1, Math.ceil(x.length / 4000)); return { x: x.filter((_, i) => i % s === 0), y: y.filter((_, i) => i % s === 0) }; };
  const csv = async () => {
    const r = res(); if (!r) return;
    const rows = ["2θ (°),d (Å),Height,Area,FWHM (°),Integral breadth (°),η (Lorentzian share),Scherrer size (nm)", ...r.peaks.map(p => [p.twoTheta, p.d, p.height, p.area, p.fwhm, p.breadth, p.eta, p.sizeNm ?? ""].map(v => typeof v === "number" ? +v.toPrecision(6) : v).join(","))];
    const path = await call<string | null>("save_file", { name: `${patterns()[sel()]?.name.replace(/\.[^.]+$/, "") ?? "pattern"} peaks.csv`, data: rows.join("\n") + "\n" });
    if (path) toast(`Saved ${path}`, true);
  };
  const num = (v: () => number, set: (n: number) => void, cls: string, step = "any", min?: number) => <input type="number" class={cls} step={step} min={min} value={v()} onChange={e => { const n = +e.currentTarget.value; if (Number.isFinite(n)) set(min != null ? Math.max(min, n) : n); }} />;
  const mean = () => { const s = res()?.peaks.map(p => p.sizeNm).filter((v): v is number => v != null) ?? []; return s.length ? s.reduce((a, b) => a + b, 0) / s.length : null; };
  return (
    <div class="page xrdpage">
      <h1>Peaks, size and strain <Help k="xrdpeaks" /></h1>
      <Tools busy={busy()} setBusy={setBusy} />
      <Show when={patterns().length} fallback={<p class="muted tempty">Open a pattern on the Patterns page, or try the made-up example.</p>}>
        <div class="xgrid">
          <section class="panel xset">
            <h2>Settings</h2>
            <label>Pattern <select class="xsel" value={Math.min(sel(), patterns().length - 1)} onChange={e => setSel(+e.currentTarget.value)}><For each={patterns()}>{(p, i) => <option value={i()} selected={i() === Math.min(sel(), patterns().length - 1)}>{p.name}</option>}</For></select></label>
            <label>Anode <select class="xanode" value={anode()} onChange={e => setAnode(e.currentTarget.value)}><For each={ANODES}>{a => <option value={a}>{a} Kα</option>}</For><option value="other">Other wavelength</option></select> <Help k="xrdtube" /></label>
            <Show when={anode() === "other"}><label>Wavelength {num(lambda, setLambda, "xlam", "0.0001", 0.05)} Å</label></Show>
            <label><input type="checkbox" class="xstrip" checked={strip()} onChange={e => setStrip(e.currentTarget.checked)} disabled={anode() === "other"} /> Strip Kα2 (then d uses Kα1)</label>
            <label>Zero shift {num(zero, setZero, "xzero", "0.001")} ° 2θ</label>
            <label>2θ from {num(from, setFrom, "xfrom", "0.5", 0)} to {num(to, setTo, "xto", "0.5", 0)} °</label>
            <label>Smooth <select class="xsmooth" value={smooth()} onChange={e => setSmooth(+e.currentTarget.value)}><For each={[[0, "none"], [5, "5 points"], [9, "9 points"], [15, "15 points"], [25, "25 points"]]}>{o => <option value={o[0]}>{o[1]}</option>}</For></select></label>
            <label>Background width {num(bg, setBg, "xbg", "0.5", 0)} ° <span class="muted small">(0: none)</span></label>
            <label>Smallest peak {num(minPeak, setMinPeak, "xmin", "0.5", 0)} % of the strongest</label>
            <h3>Size and strain</h3>
            <label>Scherrer constant K {num(k, setK, "xk", "0.01", 0.5)}</label>
            <label>Instrument FWHM {num(inst, setInst, "xinst", "0.001", 0)} ° <span class="muted small">(from a standard)</span></label>
            <Show when={err()}><p class="err">{err()}</p></Show>
          </section>
          <section class="panel">
            <Show when={res()}>{r => { const t = () => thin(r().x, r().y), n = () => thin(r().x, r().net), b = () => thin(r().x, r().bg);
              return <Graph name="Pattern and peaks" ratio={0.5} spec={{ x: { label: "2θ (°)" }, y: { label: "Counts" }, legend: true,
                series: [{ name: "Measured", kind: "line", x: t().x, y: t().y, color: "#9a9a9a", width: 0.8 }, { name: "Background", kind: "line", x: b().x, y: b().y, color: "#0072B2", dash: "--", width: 1 }, { name: "Less background" + (strip() && anode() !== "other" ? ", Kα2 stripped" : ""), kind: "line", x: n().x, y: n().y, color: "#6C4AB6", width: 1 }],
                marks: r().peaks.map(p => ({ x: p.twoTheta, y: p.height, marker: "v", color: "#F2B233", size: 5 })) }} fallback={<p class="muted small">The graph needs the Python engine.</p>} />; }}</Show>
            <For each={res()?.notes ?? []}>{n => <p class="small tnote">{n}</p>}</For>
          </section>
        </div>
        <Show when={res()}>{r => <div class="xgrid">
          <section class="panel">
            <h2>Peaks <span class="muted small xcount" style={{ "text-transform": "none" }}>{r().peaks.length} found, λ = {r().lambda.toFixed(5)} Å</span> <button class="btn sm xcsv" onClick={csv}>Peaks to CSV</button></h2>
            <div class="hscroll lscroll"><table class="ptable xpeaks"><thead><tr><th>#</th><th>2θ (°)</th><th>d (Å)</th><th>Height</th><th>Area</th><th>FWHM (°)</th><th>η</th><th>Size (nm)</th></tr></thead>
              <tbody><For each={r().peaks}>{(p, i) => <tr classList={{ off: !p.fitOk }}><td>{i() + 1}</td><td>{fx(p.twoTheta, 3)}</td><td>{fx(p.d, 4)}</td><td>{fx(p.height, 0)}</td><td>{fx(p.area, 1)}</td><td>{fx(p.fwhm, 3)}</td><td>{fx(p.eta, 2)}</td><td>{fx(p.sizeNm, 1)}</td></tr>}</For></tbody></table></div>
          </section>
          <section class="panel">
            <h2>Crystallite size and strain</h2>
            <table class="ptable ires"><tbody>
              <tr><td>Scherrer size, mean of the peaks</td><td class="xsch">{fx(mean(), 1)} nm</td></tr>
              <tr><td>Williamson–Hall size</td><td class="xwhd">{fx(r().wh?.sizeNm, 1)} nm</td></tr>
              <tr><td>Williamson–Hall microstrain</td><td class="xwhe">{r().wh ? `${fx(r().wh!.strain * 100, 3)} %` : "–"}</td></tr>
              <tr><td>R² of the Williamson–Hall line</td><td>{fx(r().wh?.r2, 3)}</td></tr></tbody></table>
            <Show when={r().wh}>{w => <Graph name="Williamson-Hall plot" ratio={0.6} spec={{ x: { label: "4 sin θ" }, y: { label: "β cos θ (rad)" }, legend: false,
              series: [{ kind: "scatter", x: w().points.map(q => q[0]), y: w().points.map(q => q[1]) }, { kind: "line", x: [0, Math.max(...w().points.map(q => q[0]))], y: [w().line[0], w().line[0] + w().line[1] * Math.max(...w().points.map(q => q[0]))], dash: "--" }] }}
              fallback={<p class="muted small">The graph needs the Python engine.</p>} />}</Show>
            <Show when={!r().wh}><p class="muted small">Williamson–Hall needs at least three fitted peaks.</p></Show>
          </section>
        </div>}</Show>
      </Show>
    </div>
  );
};
