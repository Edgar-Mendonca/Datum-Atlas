// Microstructure › Generator: synthetic 2D grain maps from settings (synth.rs), drawn with a colour
// per grain, measured back, saved as a picture or a map, or sent to image analysis to try it.
import { Show, createEffect, createSignal, on } from "solid-js";
import { call, show, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";
import { setIncoming } from "./imaging";

interface Ask { w: number; h: number; step: number; size: number; spread: number; aspect: number; phase2: number; pores: number; seed: number }
interface Map { w: number; h: number; step: number; labels: string; phase: number[]; grains: number; meanEcd: number | null; sdEcd: number | null; meanAspect: number | null; phase2: number; pores: number; grey: string }

const fromB64 = (s: string) => Uint8Array.from(atob(s), c => c.charCodeAt(0));
const fx = (v: number | null | undefined, d = 1) => v == null ? "–" : v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
/** A colour for grain l: phase 1 in light tints, phase 2 in darker violet-greys. */
const tint = (l: number, ph: number): [number, number, number] => { const h = (l * 2654435761 >>> 0) % 997 / 997; return ph === 2 ? [90 + 40 * h, 70 + 30 * h, 120 + 40 * h] : [170 + 70 * h, 175 + 60 * ((h * 7) % 1), 160 + 80 * ((h * 13) % 1)]; };

export const [synthAsk, setSynthAsk] = createSignal<Ask>({ w: 400, h: 300, step: 1, size: 25, spread: 0.3, aspect: 1, phase2: 0, pores: 0, seed: 1 });

export const SynthPage = () => {
  const [m, setM] = createSignal<Map | null>(null), [err, setErr] = createSignal(""), [busy, setBusy] = createSignal(false);
  const [bounds, setBounds] = createSignal(true);
  let cv!: HTMLCanvasElement, seq = 0;
  const upd = (f: (a: Ask) => void) => { const a = { ...synthAsk() }; f(a); setSynthAsk(a); };
  createEffect(on(synthAsk, async q => {
    const my = ++seq; await new Promise(r => setTimeout(r, 60)); if (my !== seq) return;
    setBusy(true);
    try { const r = await invoke<Map>("synth_generate", { ask: q }); if (my === seq) { setM(r); setErr(""); } } catch (e) { if (my === seq) { setErr(String(e)); setM(null); } }
    finally { if (my === seq) setBusy(false); }
  }));
  createEffect(() => {
    const x = m(), b = bounds(); if (!x || !cv) return;
    cv.width = x.w; cv.height = x.h;
    const raw = fromB64(x.labels), d = new ImageData(x.w, x.h), lab = new Uint16Array(raw.buffer, raw.byteOffset, raw.length / 2);
    for (let k = 0; k < x.w * x.h; k++) {
      const l = lab[k];
      let c: [number, number, number] = l === 0xffff ? [20, 20, 24] : tint(l, x.phase[l]);
      if (b && l !== 0xffff && ((k % x.w < x.w - 1 && lab[k + 1] !== l) || (k + x.w < lab.length && lab[k + x.w] !== l))) c = [40, 40, 50];
      d.data.set([c[0], c[1], c[2], 255], 4 * k);
    }
    cv.getContext("2d")!.putImageData(d, 0, 0);
  });
  const toImages = () => { const x = m(); if (!x) return; setIncoming({ name: `Generated map (seed ${synthAsk().seed})`, w: x.w, h: x.h, px: x.grey, scale: x.step }); show("images"); };
  const savePng = async () => { const p = await call<string | null>("save_file", { name: `Grain map seed ${synthAsk().seed}.png`, data: cv.toDataURL("image/png") }); if (p) toast(`Saved ${p}`, true); };
  const saveCsv = async () => { const t = await call<string>("synth_csv", { ask: synthAsk() }); if (!t) return; const p = await call<string | null>("save_file", { name: `Grain map seed ${synthAsk().seed}.csv`, data: t }); if (p) toast(`Saved ${p}`, true); };
  const num = (k: keyof Ask, label: string, unit: string, step: number, cls = "") => <label>{label} <input type="number" class={cls} step={step} value={synthAsk()[k]} onChange={e => upd(a => { (a as any)[k] = +e.currentTarget.value; })} /> {unit}</label>;
  return (
    <div class="page synthpage">
      <h1>Structure generator <Help k="synth" /></h1>
      <div class="imgrid">
        <section class="panel">
          <h2>Map <span class="muted small" style={{ "text-transform": "none" }}>{synthAsk().w} × {synthAsk().h} px, {fx(synthAsk().w * synthAsk().step, 0)} × {fx(synthAsk().h * synthAsk().step, 0)} µm</span> <Show when={busy()}><span class="muted small sbusy">working…</span></Show></h2>
          <div class="ivwrap"><canvas ref={cv} /></div>
          <p class="small"><label><input type="checkbox" checked={bounds()} onChange={e => setBounds(e.currentTarget.checked)} /> Draw boundaries</label></p>
          <div class="ttools"><button class="btn stoimg" onClick={toImages} title="Open the map's grey picture in image analysis, with its scale">Measure in image analysis</button><button class="btn sm" onClick={savePng}>Save as picture</button><button class="btn sm" onClick={saveCsv}>Save the map (CSV)</button></div>
        </section>
        <section class="panel icontrols">
          <h2>Settings</h2>
          {num("size", "Mean grain size", "µm", 1, "ssize")}
          <label>Size spread <input type="range" class="sspread" min="0" max="1.5" step="0.05" value={synthAsk().spread} onInput={e => upd(a => { a.spread = +e.currentTarget.value; })} /> {fx(synthAsk().spread, 2)}</label>
          <label>Elongation (length / width) <input type="range" class="saspect" min="1" max="6" step="0.1" value={synthAsk().aspect} onInput={e => upd(a => { a.aspect = +e.currentTarget.value; })} /> {fx(synthAsk().aspect, 1)}</label>
          <label>Second phase <input type="range" class="sphase" min="0" max="0.9" step="0.05" value={synthAsk().phase2} onInput={e => upd(a => { a.phase2 = +e.currentTarget.value; })} /> {fx(synthAsk().phase2 * 100, 0)} %</label>
          <label>Pores <input type="range" class="spores" min="0" max="0.3" step="0.01" value={synthAsk().pores} onInput={e => upd(a => { a.pores = +e.currentTarget.value; })} /> {fx(synthAsk().pores * 100, 0)} %</label>
          {num("w", "Width", "px", 10)}{num("h", "Height", "px", 10)}{num("step", "Step", "µm per pixel", 0.1)}
          <label>Seed <input type="number" class="sseed" value={synthAsk().seed} onChange={e => upd(a => { a.seed = Math.max(0, Math.round(+e.currentTarget.value)); })} /> <button class="btn sm snew" onClick={() => upd(a => { a.seed = Math.floor(Math.random() * 1e6); })}>New seed</button></label>
          <Show when={err()}><p class="err">{err()}</p></Show>
          <Show when={m()}>{x => <table class="ptable ires sres"><tbody>
            <tr><td>Grains</td><td class="sgrains">{x().grains}</td></tr>
            <tr><td>Mean equivalent diameter (measured)</td><td class="secd">{fx(x().meanEcd)} µm</td></tr>
            <tr><td>Spread of diameters (sd)</td><td>{fx(x().sdEcd)} µm</td></tr>
            <tr><td>Mean aspect ratio (measured)</td><td class="sasp">{fx(x().meanAspect, 2)}</td></tr>
            <tr><td>Second phase (measured)</td><td class="sp2">{fx(x().phase2)} %</td></tr>
            <tr><td>Pores (measured)</td><td>{fx(x().pores)} %</td></tr></tbody></table>}</Show>
          <p class="muted small">Grains are a power (Laguerre) tessellation of random seeds; sizes are measured back from the map, leaving out grains on its edge. A synthetic map, not a measurement.</p>
        </section>
      </div>
    </div>
  );
};
