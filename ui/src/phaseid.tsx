// Diffraction › Phase ID: search–match of a measured pattern's peaks against the reference pack of
// inorganic structures from the Crystallography Open Database (engine: phaseid.rs). The pack is
// downloaded once (checked) or imported from a file; accepted phases get rough weight fractions.
import { For, Show, createEffect, createResource, createSignal, on, onCleanup } from "solid-js";
import { call, setOut, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";
import { Graph } from "./graph";
import { kept, keptId, keepSettings } from "./kept";
import { patterns } from "./xrd";

interface Status { present: boolean; entries?: number; date?: string; want: string; size?: number; error?: string }
interface Hit { cod: number; name: string; formula: string; sg: string; score: number; found: number; strong: number; explained: number; shift: number; density: number; lines: [number, number][] }
interface Res { hits: Hit[]; chosen: { cod: number; name: string; formula: string; sg: string; lines: [number, number][] }[]; fractions: { cod: number; name: string; weight: number | null; line: number | null; note: string }[]; peaks: [number, number][]; lambda: number; x: number[]; y: number[]; notes: string[] }

const COLORS = ["#6C4AB6", "#009E73", "#D55E00", "#0072B2", "#CC79A7", "#56B4E9"];
const fx = (v: number | null | undefined, d = 1) => v == null || !Number.isFinite(v) ? "–" : v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
const els = (s: string) => s.split(/[\s,;]+/).map(x => x.trim()).filter(Boolean);
const [status, { refetch }] = createResource(() => invoke<Status>("cod_status").catch(() => ({ present: false, want: "" } as Status)));

export const PhaseIdPage = () => {
  const [busy, setBusy] = createSignal(false), [sel, setSel] = createSignal(0), [must, setMust] = createSignal(""), [may, setMay] = createSignal(""), [not, setNot] = createSignal(""), [tol, setTol] = createSignal(0.15);
  const [chosen, setChosen] = createSignal<number[]>([]), [peek, setPeek] = createSignal<number | null>(null);
  const [res, setRes] = createSignal<Res | null>(null), [err, setErr] = createSignal(""), [dl, setDl] = createSignal<string>("");
  keepSettings("xrd", "phaseid", () => ({ sel: sel(), must: must(), may: may(), not: not(), tol: tol(), chosen: chosen() }),
    v => { setSel(v.sel); setMust(v.must); setMay(v.may); setNot(v.not); setTol(v.tol); setChosen(v.chosen); });
  // The Peaks page's settings decide how the peaks are found.
  const ask = () => { const p = (kept("xrd")?.settings as any)?.peaks ?? {}; return { anode: p.anode === "other" ? "" : p.anode ?? "Cu", lambda: p.lambda ?? 0, zero: p.zero ?? 0, range: [p.from ?? 0, p.to ?? 180], smooth: p.smooth ?? 0, bg: p.bg ?? 3, strip: p.strip ?? true, min_peak: p.minPeak ?? 2, k: p.k ?? 0.9, inst: p.inst ?? 0 }; };
  const cur = () => Math.min(sel(), Math.max(0, patterns().length - 1));
  let seq = 0;
  createEffect(on([() => status()?.present, () => keptId("xrd"), sel, must, may, not, tol, chosen], async () => {
    if (!status()?.present || !patterns().length) { setRes(null); return; }
    const my = ++seq; await new Promise(r => setTimeout(r, 120)); if (my !== seq) return;
    setBusy(true);
    try { const r = await invoke<Res>("phase_search", { index: cur(), ask: ask(), query: { must: els(must()), may: els(may()), not: els(not()), tol: tol(), chosen: chosen(), limit: 30 } }); if (my === seq) { setRes(r); setErr(""); } }
    catch (e) { if (my === seq) { setErr(String(e)); setRes(null); } }
    finally { if (my === seq) setBusy(false); }
  }));
  let un: (() => void) | undefined;
  const ev = (window as any).__TAURI__?.event;
  ev?.listen?.("codprogress", (e: any) => setDl(`${(e.payload.done / 1e6).toFixed(0)}${e.payload.total ? ` of ${(e.payload.total / 1e6).toFixed(0)}` : ""} MB`)).then((f: () => void) => { un = f; });
  onCleanup(() => un?.());
  const download = async () => { setBusy(true); setDl("starting…"); try { const s = await call<Status>("cod_download"); if (s) { await refetch(); toast(`Reference pack ready: ${s.entries?.toLocaleString("en-GB")} structures.`, true); } } finally { setBusy(false); setDl(""); } };
  const importPack = async () => { setBusy(true); try { const s = await call<Status | null>("cod_import", {}); if (s) { await refetch(); toast(`Reference pack imported: ${s.entries?.toLocaleString("en-GB")} structures.`, true); } } finally { setBusy(false); } };
  const example = async () => { setBusy(true); try { const o = await call("xrd_example_mix"); if (o) { setOut(o); setSel(patterns().length - 1); setChosen([]); } } finally { setBusy(false); } };
  const accept = (c: number) => { setChosen(v => v.includes(c) ? v : [...v, c]); setPeek(null); };
  const drop = (c: number) => setChosen(v => v.filter(x => x !== c));
  const series = () => {
    const r = res(); if (!r) return [];
    const top = Math.max(...r.y, 1);
    const s: any[] = [{ name: patterns()[cur()]?.name ?? "Measured", kind: "line", x: r.x, y: r.y, color: "#9a9a9a", width: 0.9 }];
    r.chosen.forEach((c, k) => s.push({ name: c.name, kind: "stem", x: c.lines.map(l => l[0]), y: c.lines.map(l => l[1] / 100 * top), color: COLORS[k % COLORS.length] }));
    const h = r.hits.find(x => x.cod === peek());
    if (h) s.push({ name: `${h.name} (candidate)`, kind: "stem", x: h.lines.map(l => l[0]), y: h.lines.map(l => -l[1] / 100 * top * 0.5), color: "#F2B233" });
    return s;
  };
  return (
    <div class="page xrdpage">
      <h1>Phase identification <Help k="xrdid" /></h1>
      <Show when={status()?.present} fallback={<section class="panel pack">
        <h2>Reference pack</h2>
        <p>Phase identification compares your pattern with the inorganic structures of the Crystallography Open Database (free, in the public domain), each reduced to its strongest lines. The pack is downloaded once (checked against its checksum) and kept on this computer; after that it works offline.</p>
        <Show when={status()?.error}><p class="err">{status()!.error}</p></Show>
        <div class="ttools"><button class="btn pri pdl" onClick={download} disabled={busy()}>Download the reference pack</button><button class="btn pim" onClick={importPack} disabled={busy()}>Import a pack file…</button>
          <Show when={dl()}><span class="muted small">Downloading {dl()}</span></Show></div>
        <p class="muted small">For a computer without internet, download the file cod-inorganic-{status()?.want}.pack from Datum Atlas's release page on another computer and import it here.</p>
      </section>}>
        <div class="ttools"><span class="muted small ppack">Reference pack: {status()?.entries?.toLocaleString("en-GB")} inorganic structures (COD {status()?.date})</span>
          <button class="btn sm pex" onClick={example} title="A made-up pattern of corundum and quartz, 60 and 40 % by mass">Example: corundum + quartz (made-up pattern)</button>
          <Show when={busy()}><span class="muted small pbusy">working…</span></Show></div>
        <Show when={patterns().length} fallback={<p class="muted tempty">Open a measured pattern on the Patterns page, or add the example above.</p>}>
          <div class="xgrid">
            <section class="panel xset">
              <h2>Search</h2>
              <label>Pattern <select class="psel" onChange={e => { setSel(+e.currentTarget.value); setChosen([]); }}><For each={patterns()}>{(p, i) => <option value={i()} selected={i() === cur()}>{p.name}</option>}</For></select></label>
              <p class="muted small">Peaks are found with the settings of the Peaks page (anode, background, Kα2).</p>
              <label>Must contain <input class="pmust" placeholder="e.g. Ti O" value={must()} onChange={e => setMust(e.currentTarget.value)} /></label>
              <label>May also contain <input class="pmay" placeholder="empty: any element" value={may()} onChange={e => setMay(e.currentTarget.value)} /></label>
              <label>Must not contain <input class="pnot" placeholder="e.g. Pb" value={not()} onChange={e => setNot(e.currentTarget.value)} /></label>
              <label>Position tolerance <input type="number" class="ptol" step="0.01" min="0.02" max="1" value={tol()} onChange={e => setTol(Math.min(1, Math.max(0.02, +e.currentTarget.value || 0.15)))} /> ° 2θ</label>
              <Show when={err()}><p class="err">{err()}</p></Show>
              <h3>Accepted phases</h3>
              <Show when={res()?.fractions.length} fallback={<p class="muted small">Accept the candidate that fits best; the search then looks for what is left.</p>}>
                <table class="ptable pfrac"><thead><tr><th>Phase</th><th>Weight %</th><th></th></tr></thead>
                  <tbody><For each={res()!.fractions}>{(f, k) => <tr><td><i class="pdot" style={{ background: COLORS[k() % COLORS.length] }} />{f.name} <span class="muted small">COD {f.cod}</span><Show when={f.note}><div class="muted small">{f.note}</div></Show></td><td>{fx(f.weight)}</td><td><button class="lnk" onClick={() => drop(f.cod)}>Remove</button></td></tr>}</For></tbody></table>
                <p class="muted small">Rough fractions by the reference intensity ratio, from each phase's strongest line of its own; preferred orientation and overlaps make them approximate.</p>
              </Show>
            </section>
            <section class="panel">
              <Show when={res()}><Graph name="Phase identification" ratio={0.5} spec={{ x: { label: "2θ (°)" }, y: { label: "Counts (background taken off)" }, legend: true, series: series(), lines: [{ y: 0, color: "#bbbbbb" }] }} fallback={<p class="muted small">The graph needs the Python engine.</p>} /></Show>
              <p class="muted small">Accepted phases' lines point up in their colours; the candidate you click points down in amber.</p>
            </section>
          </div>
          <Show when={res()}>{r => <section class="panel">
            <h2>Candidates <span class="muted small pcount" style={{ "text-transform": "none" }}>{r().hits.length} found from {r().peaks.length} peaks</span></h2>
            <div class="hscroll lscroll"><table class="ptable phits"><thead><tr><th>#</th><th>Phase</th><th>Formula</th><th>Space group</th><th>COD</th><th>Score</th><th>Strong lines found</th><th>Explains</th><th>Mean shift</th><th></th></tr></thead>
              <tbody><For each={r().hits}>{(h, i) => <tr classList={{ on: h.cod === peek() }} onClick={() => setPeek(h.cod === peek() ? null : h.cod)}>
                <td>{i() + 1}</td><td><b>{h.name}</b></td><td>{h.formula}</td><td>{h.sg}</td><td>{h.cod}</td><td>{fx(h.score, 0)}</td><td>{h.found} of {h.strong}</td><td>{fx(h.explained, 0)} %</td><td>{fx(h.shift, 3)}°</td>
                <td><button class="btn sm pacc" onClick={e => { e.stopPropagation(); accept(h.cod); }}>Accept</button></td></tr>}</For></tbody></table></div>
            <p class="muted small">Structures from the Crystallography Open Database (crystallography.net, public domain): open COD entries there to see each structure's authors and paper, and credit them in your work. A score ranks how well the strong lines are found, how much of the pattern they explain, and how close they sit; always check the lines on the graph.</p>
          </section>}</Show>
        </Show>
      </Show>
    </div>
  );
};
