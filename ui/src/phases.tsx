// Diffraction › Structures: crystal structures from CIF files (engine: cif.rs), kept in the project,
// with their accurate calculated powder pattern, alone or over a measured pattern.
import { For, Show, createEffect, createSignal, on } from "solid-js";
import { call, setOut, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";
import { Graph } from "./graph";
import { kept, keptId, keepSettings } from "./kept";
import { patterns } from "./xrd";

interface Info { file: string; name: string; formula?: string; spaceGroup?: string; cell?: { a: number; b: number; c: number; alpha: number; beta: number; gamma: number }; atoms?: number; density?: number; z?: number | null;
  cod?: string; authors?: string; journal?: string; year?: string; title?: string; notes?: string[]; ops?: number; error?: string }
interface Line { hkl: [number, number, number]; mult: number; d: number; twoTheta: number; intensity: number }
interface Calc { lines: Line[]; lambda: number; profile?: { x: number[]; y: number[] }; over?: { name: string; x: number[]; y: number[]; series: { name: string; y: number[] }[] } }

const ANODES = ["Cu", "Co", "Mo", "Cr", "Fe", "Ag"];
const COLORS = ["#6C4AB6", "#F2B233", "#009E73", "#D55E00", "#0072B2", "#CC79A7"];
const fx = (v: number | null | undefined, d = 2) => v == null || !Number.isFinite(v) ? "–" : v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
const hkl = (h: number[]) => h.map(v => v < 0 ? `${-v}̅` : String(v)).join(" ");
export const structures = (): Info[] => kept("cif")?.info ?? [];
/** The source of a structure, credited as the open database asks. */
const credit = (s: Info) => [s.authors, s.journal && `${s.journal}${s.year ? ` (${s.year})` : ""}`, s.cod && `COD ${s.cod}`].filter(Boolean).join(" · ");

export const PhasesPage = () => {
  const [busy, setBusy] = createSignal(false), [sel, setSel] = createSignal(0), [anode, setAnode] = createSignal("Cu"), [lambda, setLambda] = createSignal(1.5406);
  const [max, setMax] = createSignal(120), [fwhm, setFwhm] = createSignal(0.1), [ka2, setKa2] = createSignal(true), [mono, setMono] = createSignal(0), [over, setOver] = createSignal(-1), [withs, setWiths] = createSignal<number[]>([0]);
  const [res, setRes] = createSignal<Calc | null>(null), [err, setErr] = createSignal("");
  keepSettings("cif", "calc", () => ({ sel: sel(), anode: anode(), lambda: lambda(), max: max(), fwhm: fwhm(), ka2: ka2(), mono: mono(), over: over(), withs: withs() }),
    v => { setSel(v.sel); setAnode(v.anode); setLambda(v.lambda); setMax(v.max); setFwhm(v.fwhm); setKa2(v.ka2); setMono(v.mono); setOver(v.over); setWiths(v.withs); });
  const load = async (cmd: string) => { setBusy(true); try { const o = await call(cmd, {}); if (o) { setOut(o); setSel(structures().length - 1); setWiths([structures().length - 1]); } } finally { setBusy(false); } };
  const remove = async (k: number) => { const o = await call("cif_remove", { index: k }); if (o) { setOut(o); setSel(0); setWiths([0]); } };
  const cur = () => Math.min(sel(), Math.max(0, structures().length - 1));
  let seq = 0;
  createEffect(on([() => keptId("cif"), () => keptId("xrd"), sel, anode, lambda, max, fwhm, ka2, mono, over, withs], async () => {
    const s = structures()[cur()];
    if (!s || s.error) { setRes(null); return; }
    const my = ++seq; await new Promise(r => setTimeout(r, 60)); if (my !== seq) return;
    setBusy(true);
    const o = over() >= 0 && over() < patterns().length ? over() : null;
    const w = withs().filter(i => i < structures().length && !structures()[i].error);
    try { const r = await invoke<Calc>("cif_pattern", { index: cur(), over: o, with: w.length ? w : [cur()], ask: { anode: anode() === "other" ? "" : anode(), lambda: lambda(), max: max(), mono: mono(), fwhm: fwhm(), kalpha2: ka2() } }); if (my === seq) { setRes(r); setErr(""); } }
    catch (e) { if (my === seq) { setErr(String(e)); setRes(null); } }
    finally { if (my === seq) setBusy(false); }
  }));
  const csv = async () => {
    const r = res(); if (!r) return;
    const rows = ["h,k,l,Multiplicity,d (Å),2θ (°),Intensity (strongest = 100)", ...r.lines.map(q => [...q.hkl, q.mult, +q.d.toPrecision(6), +q.twoTheta.toFixed(4), +q.intensity.toFixed(2)].join(","))];
    const path = await call<string | null>("save_file", { name: `${structures()[cur()]?.name ?? "structure"} lines.csv`, data: rows.join("\n") + "\n" });
    if (path) toast(`Saved ${path}`, true);
  };
  const num = (v: () => number, set: (n: number) => void, cls: string, step = "any", min?: number) => <input type="number" class={cls} step={step} min={min} value={v()} onChange={e => { const n = +e.currentTarget.value; if (Number.isFinite(n)) set(min != null ? Math.max(min, n) : n); }} />;
  const toggle = (i: number) => setWiths(w => w.includes(i) ? w.filter(x => x !== i) : [...w, i].sort());
  return (
    <div class="page xrdpage">
      <h1>Crystal structures <Help k="xrdphases" /></h1>
      <div class="ttools">
        <button class="btn copen" onClick={() => load("cif_open")}>Open a structure (CIF)…</button>
        <button class="btn cex" onClick={() => load("cif_example")} title="Rock salt, written out atom by atom from the crystal module's textbook cell">Example (rock salt)</button>
        <Show when={busy()}><span class="muted small cbusy">working…</span></Show>
      </div>
      <Show when={structures().length} fallback={<p class="muted tempty">Open a crystal structure as a CIF file, for instance from the Crystallography Open Database (crystallography.net, free and in the public domain), or try the example. Its powder pattern is calculated, and can be laid over your measured patterns.</p>}>
        <div class="hscroll"><table class="ptable clist"><thead><tr><th>Structure</th><th>Space group</th><th>Cell (Å, °)</th><th>Atoms in the cell</th><th>Density (g/cm³)</th><th>Source</th><th></th></tr></thead>
          <tbody><For each={structures()}>{(s, i) => <tr classList={{ on: i() === cur() }} onClick={() => setSel(i())}>
            <td><b>{s.name}</b><div class="muted small">{s.formula} · {s.file}</div><Show when={s.error}><div class="err small">{s.error}</div></Show><For each={s.notes ?? []}>{n => <div class="muted small">{n}</div>}</For></td>
            <td>{s.spaceGroup || "–"}<Show when={s.ops}><div class="muted small">{s.ops} operations</div></Show></td>
            <td>{s.cell ? `${fx(s.cell.a, 4)} ${fx(s.cell.b, 4)} ${fx(s.cell.c, 4)}; ${fx(s.cell.alpha, 1)} ${fx(s.cell.beta, 1)} ${fx(s.cell.gamma, 1)}` : "–"}</td>
            <td>{s.atoms ?? "–"}<Show when={s.z}><div class="muted small">Z = {s.z}</div></Show></td><td>{fx((s.density ?? NaN) / 1000, 3)}</td>
            <td class="small">{credit(s) || "–"}<Show when={s.title}><div class="muted small">{s.title}</div></Show></td>
            <td><button class="lnk crm" onClick={e => { e.stopPropagation(); remove(i()); }}>Remove</button></td></tr>}</For></tbody></table></div>
        <div class="xgrid">
          <section class="panel xset">
            <h2>Calculation</h2>
            <label>Structure <select class="csel" onChange={e => setSel(+e.currentTarget.value)}><For each={structures()}>{(s, i) => <option value={i()} selected={i() === cur()}>{s.name}</option>}</For></select></label>
            <label>Anode <select class="canode" onChange={e => setAnode(e.currentTarget.value)}><For each={ANODES}>{a => <option value={a} selected={a === anode()}>{a} Kα</option>}</For><option value="other" selected={anode() === "other"}>Other wavelength</option></select></label>
            <Show when={anode() === "other"}><label>Wavelength {num(lambda, setLambda, "clam", "0.0001", 0.05)} Å</label></Show>
            <label><input type="checkbox" checked={ka2()} onChange={e => setKa2(e.currentTarget.checked)} disabled={anode() === "other"} /> With Kα2</label>
            <label>2θ up to {num(max, setMax, "cmax", "5", 10)} °</label>
            <label>Peak width (FWHM) {num(fwhm, setFwhm, "cfwhm", "0.01", 0.005)} °</label>
            <label>Monochromator 2θ {num(mono, setMono, "cmono", "0.1", 0)} ° <span class="muted small">(0: none)</span></label>
            <h3>Over a measured pattern</h3>
            <label>Pattern <select class="cover" onChange={e => setOver(+e.currentTarget.value)}><option value={-1} selected={over() < 0}>None</option><For each={patterns()}>{(p, i) => <option value={i()} selected={i() === over()}>{p.name}</option>}</For></select></label>
            <Show when={over() >= 0 && patterns().length}><div class="cwith"><For each={structures()}>{(s, i) => <label><input type="checkbox" checked={withs().includes(i())} onChange={() => toggle(i())} disabled={!!s.error} /> {s.name}</label>}</For></div></Show>
            <Show when={err()}><p class="err">{err()}</p></Show>
          </section>
          <section class="panel">
            <Show when={res()?.over} fallback={<Show when={res()?.profile}>{pr => <Graph name="Calculated pattern" ratio={0.5} spec={{ x: { label: "2θ (°)" }, y: { label: "Intensity (strongest = 100)", min: 0 }, legend: false,
              series: [{ kind: "line", x: pr().x, y: pr().y, color: "#6C4AB6", width: 1 }, { kind: "stem", x: res()!.lines.map(q => q.twoTheta), y: res()!.lines.map(q => q.intensity), color: "#F2B233" }] }} fallback={<p class="muted small">The graph needs the Python engine.</p>} />}</Show>}>
              {o => <Graph name="Calculated over measured" ratio={0.5} spec={{ x: { label: "2θ (°)" }, y: { label: "Counts (background taken off)" }, legend: true,
                series: [{ name: o().name, kind: "line", x: o().x, y: o().y, color: "#9a9a9a", width: 0.9 }, ...o().series.map((s, k) => ({ name: `${s.name} (calculated)`, kind: "line" as const, x: o().x, y: s.y, color: COLORS[k % COLORS.length], width: 1 }))] }}
                fallback={<p class="muted small">The graph needs the Python engine.</p>} />}
            </Show>
            <p class="muted small">Each calculated pattern is scaled so its strongest point meets the measured pattern's. Peak heights depend on the sample too (preferred orientation, absorption); positions are what to compare first.</p>
          </section>
        </div>
        <Show when={res()}>{r => <section class="panel">
          <h2>Lines of {structures()[cur()]?.name} <span class="muted small clines" style={{ "text-transform": "none" }}>{r().lines.length} lines, λ = {r().lambda.toFixed(5)} Å</span> <button class="btn sm ccsv" onClick={csv}>Lines to CSV</button></h2>
          <div class="hscroll lscroll"><table class="ptable cpeaks"><thead><tr><th>h k l</th><th>Multiplicity</th><th>d (Å)</th><th>2θ (°)</th><th>Intensity</th></tr></thead>
            <tbody><For each={r().lines.slice(0, 300)}>{q => <tr><td>{hkl(q.hkl)}</td><td>{q.mult}</td><td>{fx(q.d, 4)}</td><td>{fx(q.twoTheta, 3)}</td><td>{fx(q.intensity, 1)}</td></tr>}</For></tbody></table></div>
        </section>}</Show>
      </Show>
    </div>
  );
};
