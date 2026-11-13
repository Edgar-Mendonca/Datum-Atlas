// EBSD module › Maps: open an orientation map export (.ang, .ctf or Datum Atlas's own map) and draw
// it: orientation (IPF colours), phases, pattern quality, confidence. The engine (ebsd.rs) keeps the
// map and makes the pictures; this page only shows them.
import { For, Show, createEffect, createSignal, on } from "solid-js";
import { call, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";
import { IpfKey } from "./synth";

interface Phase { name: string; crystal: string }
export interface Summary { name: string; format: string; w: number; h: number; step: number; points: number; indexed: number; phases: [Phase, number][]; qualityName: string; confName: string; notes: string[] }
const FMT: Record<string, string> = { ang: ".ang text export", ctf: ".ctf text export", datum: "Datum Atlas orientation map" };
const fx = (v: number, d = 1) => v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
const fromB64 = (s: string) => Uint8Array.from(atob(s), c => c.charCodeAt(0));

/** The map open in the EBSD pages. */
export const [ebsdMap, setEbsdMap] = createSignal<Summary | null>(null);

/** Draw an RGB picture from the engine on a canvas. */
export const paint = (cv: HTMLCanvasElement, w: number, h: number, rgb: string) => {
  cv.width = w; cv.height = h;
  const b = fromB64(rgb), d = new ImageData(w, h);
  for (let k = 0; k < w * h; k++) d.data.set([b[3 * k], b[3 * k + 1], b[3 * k + 2], 255], 4 * k);
  cv.getContext("2d")!.putImageData(d, 0, 0);
};

export const MapsPage = () => {
  const [kind, setKind] = createSignal("ipf"), [dir, setDir] = createSignal("z"), [busy, setBusy] = createSignal(false);
  let cv!: HTMLCanvasElement;
  const open = async () => { setBusy(true); try { const s = await call<Summary | null>("ebsd_open", {}); if (s) { setKind("ipf"); setEbsdMap(s); } } finally { setBusy(false); } };
  const example = async () => { setBusy(true); try { const s = await call<Summary>("ebsd_example"); if (s) { setKind("ipf"); setEbsdMap(s); } } finally { setBusy(false); } };
  createEffect(on([ebsdMap, kind, dir], async ([m, k, d]) => {
    if (!m || !cv) return;
    setBusy(true);
    try { const r = await invoke<{ w: number; h: number; rgb: string }>("ebsd_draw", { kind: k, dir: d }); paint(cv, r.w, r.h, r.rgb); }
    catch (e) { toast(String(e)); setKind("ipf"); }
    finally { setBusy(false); }
  }));
  const save = async () => { const p = await call<string | null>("save_file", { name: `${ebsdMap()!.name.replace(/\.[^.]+$/, "")} ${kind()}.png`, data: cv.toDataURL("image/png") }); if (p) toast(`Saved ${p}`, true); };
  return (
    <div class="page ebsdpage">
      <h1>EBSD maps <Help k="ebsd" /></h1>
      <div class="ttools">
        <button class="btn eopen" onClick={open}>Open a map…</button>
        <button class="btn eex" onClick={example} title="A made-up map from the structure generator">Example (made-up map)</button>
        <Show when={ebsdMap()}><button class="btn sm" onClick={save}>Save as picture</button></Show>
        <Show when={busy()}><span class="muted small ebusy">working…</span></Show>
      </div>
      <Show when={ebsdMap()} fallback={<p class="muted tempty">Open an EBSD map: a .ang or .ctf text export, or an orientation map saved by the structure generator. Or try the made-up example.</p>}>{m => <div class="imgrid">
        <section class="panel">
          <h2>{m().name} <span class="muted small" style={{ "text-transform": "none" }}>{m().w} × {m().h} points, step {fx(m().step, 3)} µm</span></h2>
          <div class="ivwrap"><canvas ref={cv} /></div>
          <div class="ecolor">
            <label>Show <select class="ekind" value={kind()} onChange={e => setKind(e.currentTarget.value)}>
              <option value="ipf">Orientation (IPF colours)</option><option value="phase">Phases</option>
              <Show when={m().qualityName}><option value="quality">{m().qualityName}</option></Show><Show when={m().confName}><option value="confidence">{m().confName}</option></Show></select></label>
            <Show when={kind() === "ipf"}><label>of <select class="edir" value={dir()} onChange={e => setDir(e.currentTarget.value)}><option value="z">Z (normal)</option><option value="x">X</option><option value="y">Y</option></select></label></Show>
          </div>
          <Show when={kind() === "ipf"}><div class="ekeys"><For each={[...new Set(m().phases.map(p => p[0].crystal))]}>{c => <IpfKey crystal={c} dir={dir()} />}</For></div></Show>
          <Show when={kind() === "phase"}><p class="small ekey"><For each={m().phases}>{(p, i) => <span><i style={{ background: ["#6C4AB6", "#F2B233", "#009E73", "#D55E00", "#0072B2", "#CC79A7"][i() % 6] }} />{p[0].name}</span>}</For><span><i style={{ background: "#000" }} />not indexed</span></p></Show>
        </section>
        <section class="panel">
          <h2>The map</h2>
          <table class="ptable ires esum"><tbody>
            <tr><td>File</td><td>{FMT[m().format] ?? m().format}</td></tr>
            <tr><td>Points</td><td>{m().points.toLocaleString("en-GB")}</td></tr>
            <tr><td>Area</td><td>{fx(m().w * m().step, 1)} × {fx(m().h * m().step, 1)} µm</td></tr>
            <tr><td>Indexed</td><td class="eidx">{fx(m().indexed)} %</td></tr>
          </tbody></table>
          <h3>Phases</h3>
          <table class="ptable ephases"><thead><tr><th>Phase</th><th>Crystal</th><th>Share of the map</th></tr></thead>
            <tbody><For each={m().phases}>{p => <tr><td>{p[0].name}</td><td>{p[0].crystal === "hex" ? "hexagonal" : "cubic"}</td><td>{fx(p[1])} %</td></tr>}</For></tbody></table>
          <For each={m().notes}>{n => <p class="small tnote">{n}</p>}</For>
          <p class="muted small">Grains, boundaries and twins, local misorientation, Schmid factors and pole figures follow.</p>
        </section>
      </div>}</Show>
    </div>
  );
};
