// Microstructure › Images: open a micrograph, calibrate it, threshold one phase and measure it with
// the engine (image.rs): area fraction, particles, sizes and shapes. The window only decodes the
// picture into grey bytes and draws the engine's mask over it.
import { For, Show, createEffect, createSignal, on } from "solid-js";
import { call, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";

interface Part { area: number; ecd: number; perim: number; circ: number; aspect: number; cx: number; cy: number; edge: boolean }
interface Answer { threshold: number; otsu: number; hist: number[]; fraction: number; unit: string; particles: Part[];
  stats: { count: number; density: number | null; mean: number | null; sd: number | null; d10: number | null; d50: number | null; d90: number | null; aspect: number | null; circ: number | null; meanArea: number | null };
  bins: [number, number, number][]; mask: string; notes: string[] }
interface Img { name: string; w: number; h: number; px: string; made?: boolean }

const MAXPX = 2_500_000;
const toB64 = (b: Uint8Array) => { let s = ""; for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode(...b.subarray(i, i + 0x8000)); return btoa(s); };
const fromB64 = (s: string) => Uint8Array.from(atob(s), c => c.charCodeAt(0));
const fx = (v: number | null | undefined, d = 2) => v == null ? "–" : v.toLocaleString("en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });

/** Decode a picture file into grey bytes, shrunk to at most MAXPX pixels. */
const decode = async (f: File): Promise<Img> => {
  const bmp = await createImageBitmap(f);
  const k = Math.min(1, Math.sqrt(MAXPX / (bmp.width * bmp.height))), w = Math.max(1, Math.round(bmp.width * k)), h = Math.max(1, Math.round(bmp.height * k));
  const c = document.createElement("canvas"); c.width = w; c.height = h;
  const x = c.getContext("2d")!; x.drawImage(bmp, 0, 0, w, h);
  const d = x.getImageData(0, 0, w, h).data, g = new Uint8Array(w * h);
  for (let i = 0; i < w * h; i++) g[i] = Math.round(0.299 * d[4 * i] + 0.587 * d[4 * i + 1] + 0.114 * d[4 * i + 2]);
  if (k < 1) toast(`The image was shrunk to ${w} × ${h} pixels for measuring; set the scale on this size.`, true);
  return { name: f.name, w, h, px: toB64(g) };
};

export const ImagesPage = () => {
  const [img, setImg] = createSignal<Img | null>(null);
  const [scale, setScale] = createSignal(0), [bottom, setBottom] = createSignal(0), [auto, setAuto] = createSignal(true), [t, setT] = createSignal(128);
  const [dark, setDark] = createSignal(true), [smooth, setSmooth] = createSignal(0), [minA, setMinA] = createSignal(4), [edge, setEdge] = createSignal(true), [overlay, setOverlay] = createSignal(true);
  const [res, setRes] = createSignal<Answer | null>(null), [err, setErr] = createSignal("");
  const [pick, setPick] = createSignal<number[][] | null>(null), [barLen, setBarLen] = createSignal(50);
  let cv!: HTMLCanvasElement;
  const run = async () => {
    const i = img(); if (!i) return;
    const roi = [0, 0, i.w, Math.max(2, Math.round(i.h * (1 - bottom() / 100)))];
    try { setRes(await invoke<Answer>("image_analyse", { ask: { w: i.w, h: i.h, px: i.px, scale: scale(), roi, smooth: smooth(), threshold: auto() ? null : t(), dark: dark(), min_area: minA(), edge: edge() } })); setErr(""); }
    catch (e) { setErr(String(e)); setRes(null); }
  };
  createEffect(on([img, scale, bottom, auto, t, dark, smooth, minA, edge], () => { run(); }));
  createEffect(() => { const r = res(); if (r && auto()) setT(r.threshold); });
  // Draw the grey image and, over it, the engine's mask: violet counted, amber not counted, grey outside.
  createEffect(() => {
    const i = img(), r = res(), ov = overlay(); if (!i || !cv) return;
    cv.width = i.w; cv.height = i.h;
    const g = fromB64(i.px), m = r ? fromB64(r.mask) : null, d = new ImageData(i.w, i.h);
    for (let k = 0; k < i.w * i.h; k++) {
      let [R, G, B] = [g[k], g[k], g[k]];
      const v = ov && m ? m[k] : 0;
      if (v === 1) [R, G, B] = [0.45 * R + 0.55 * 108, 0.45 * G + 0.55 * 74, 0.45 * B + 0.55 * 182];
      else if (v === 2) [R, G, B] = [0.5 * R + 0.5 * 242, 0.5 * G + 0.5 * 178, 0.5 * B + 0.5 * 51];
      else if (v === 3) [R, G, B] = [0.5 * R + 60, 0.5 * G + 60, 0.5 * B + 60];
      d.data.set([R, G, B, 255], 4 * k);
    }
    cv.getContext("2d")!.putImageData(d, 0, 0);
    const p = pick(); if (p?.length) { const x = cv.getContext("2d")!; x.strokeStyle = "#F2B233"; x.lineWidth = Math.max(2, i.w / 300); x.beginPath(); x.moveTo(p[0][0], p[0][1]); if (p[1]) x.lineTo(p[1][0], p[1][1]); x.stroke(); }
  });
  const click = (e: MouseEvent) => {
    const p = pick(); if (!p || !img()) return;
    const b = cv.getBoundingClientRect(), x = (e.clientX - b.left) * img()!.w / b.width, y = (e.clientY - b.top) * img()!.h / b.height;
    const n = [...p, [x, y]];
    if (n.length === 2) { const len = Math.hypot(n[1][0] - n[0][0], n[1][1] - n[0][1]); if (len > 1) { setScale(+(barLen() / len).toPrecision(5)); toast(`Scale: ${(barLen() / len).toPrecision(4)} µm per pixel (${len.toFixed(1)} px = ${barLen()} µm).`, true); } setPick(null); }
    else setPick(n);
  };
  const open = (e: Event) => { const f = (e.currentTarget as HTMLInputElement).files?.[0]; if (f) decode(f).then(i => { setImg(i); setScale(0); setBottom(0); }, () => toast("That picture could not be read. Save it as PNG or JPEG and try again.")); };
  const example = async () => { const e = await call<{ w: number; h: number; px: string }>("image_example"); if (e) { setImg({ name: "Example (made-up image)", ...e, made: true }); setScale(0.5); setBottom(9); setSmooth(1); } };
  const csv = async () => {
    const r = res(); if (!r) return;
    const u = r.unit, rows = [[`Area (${u}²)`, `Equivalent diameter (${u})`, `Perimeter (${u})`, "Circularity", "Aspect ratio", `x (${u})`, `y (${u})`, "Touches the edge"].join(","),
      ...r.particles.map(p => [p.area, p.ecd, p.perim, p.circ, p.aspect, p.cx, p.cy, p.edge ? "yes" : "no"].map(v => typeof v === "number" ? +v.toPrecision(6) : v).join(","))];
    const path = await call<string | null>("save_file", { name: `${img()!.name.replace(/\.[^.]+$/, "")} particles.csv`, data: rows.join("\n") + "\n" });
    if (path) toast(`Saved ${path}`, true);
  };
  const u = () => res()?.unit ?? "px";
  return (
    <div class="page imgpage">
      <h1>Image analysis <Help k="images" /></h1>
      <div class="ttools">
        <label class="btn iopen">Open an image…<input type="file" accept="image/png,image/jpeg,image/bmp,image/webp,image/gif" style={{ display: "none" }} onChange={open} /></label>
        <button class="btn iex" onClick={example} title="A made-up test image with dark round particles and a data bar">Example (made-up image)</button>
        <Show when={res()}><button class="btn sm" onClick={csv}>Particles to CSV</button></Show>
      </div>
      <Show when={img()} fallback={<p class="muted tempty">Open a micrograph (PNG, JPEG, BMP or WebP), or try the made-up example. The picture stays on your computer; only its measurements are shown.</p>}>{i => <div class="imgrid">
        <section class="panel">
          <h2>{i().name} <span class="muted small">{i().w} × {i().h} px</span></h2>
          <div class="ivwrap" classList={{ pick: !!pick() }}><canvas ref={cv} onClick={click} /></div>
          <p class="small ikey"><label><input type="checkbox" checked={overlay()} onChange={e => setOverlay(e.currentTarget.checked)} /> Overlay</label><i style={{ background: "#6C4AB6" }} />measured<i style={{ background: "#F2B233" }} />not counted (small or on the edge)<i style={{ background: "#888" }} />left out</p>
          <Show when={i().made}><p class="muted small">A made-up image: its scale bar is 100 px long, 50 µm.</p></Show>
        </section>
        <section class="panel icontrols">
          <h2>Settings</h2>
          <label>Scale <input type="number" class="iscale" step="0.0001" value={scale()} onChange={e => setScale(Math.max(0, +e.currentTarget.value))} /> µm per pixel <Show when={!scale()}><span class="muted small">(0: sizes in pixels)</span></Show></label>
          <label>Scale bar <input type="number" value={barLen()} onChange={e => setBarLen(+e.currentTarget.value)} /> µm <button class="btn sm ibar" onClick={() => { setPick([]); toast("Click both ends of the scale bar on the image.", true); }}>Set from the scale bar</button></label>
          <label>Ignore the bottom <input type="number" class="ibottom" min="0" max="50" value={bottom()} onChange={e => setBottom(Math.min(50, Math.max(0, +e.currentTarget.value)))} /> % (the data bar)</label>
          <label>The phase is <select class="iphase" value={dark() ? "dark" : "bright"} onChange={e => setDark(e.currentTarget.value === "dark")}><option value="dark">darker (pores, graphite, dark particles)</option><option value="bright">brighter</option></select></label>
          <label><input type="checkbox" class="iauto" checked={auto()} onChange={e => setAuto(e.currentTarget.checked)} /> Threshold by Otsu's method</label>
          <label>Threshold <input type="range" class="ithr" min="0" max="255" value={t()} disabled={auto()} onInput={e => setT(+e.currentTarget.value)} /> {t()}</label>
          <Show when={res()}>{r => <svg class="ihist" viewBox="0 0 256 60"><For each={r().hist}>{(c, k) => <line x1={k() + 0.5} x2={k() + 0.5} y1={60} y2={60 - 58 * Math.sqrt(c / Math.max(...r().hist))} stroke={(dark() ? k() <= r().threshold : k() > r().threshold) ? "#6C4AB6" : "var(--text2)"} />}</For><line x1={r().threshold + 0.5} x2={r().threshold + 0.5} y1="0" y2="60" stroke="#F2B233" stroke-width="1.5" /></svg>}</Show>
          <label>Smooth <select value={smooth()} onChange={e => setSmooth(+e.currentTarget.value)}><option value="0">none</option><option value="1">once</option><option value="2">twice</option><option value="3">three times</option></select></label>
          <label>Smallest particle <input type="number" class="imin" min="1" value={minA()} onChange={e => setMinA(Math.max(1, Math.round(+e.currentTarget.value)))} /> pixels</label>
          <label><input type="checkbox" checked={edge()} onChange={e => setEdge(e.currentTarget.checked)} /> Leave out particles touching the edge</label>
          <Show when={err()}><p class="err">{err()}</p></Show>
        </section>
        <Show when={res()}>{r => <>
          <section class="panel">
            <h2>Results</h2>
            <table class="ptable ires"><tbody>
              <tr><td>Area fraction of the phase</td><td class="ifrac">{fx(r().fraction)} %</td></tr>
              <tr><td>Particles measured</td><td class="icount">{r().stats.count}</td></tr>
              <tr><td>Particles per mm²</td><td>{fx(r().stats.density, 0)}</td></tr>
              <tr><td>Equivalent diameter, mean ± sd</td><td class="iecd">{fx(r().stats.mean)} ± {fx(r().stats.sd)} {u()}</td></tr>
              <tr><td>D10 / D50 / D90</td><td>{fx(r().stats.d10)} / {fx(r().stats.d50)} / {fx(r().stats.d90)} {u()}</td></tr>
              <tr><td>Mean area</td><td>{fx(r().stats.meanArea)} {u()}²</td></tr>
              <tr><td>Mean circularity (1 = a circle)</td><td>{fx(r().stats.circ, 3)}</td></tr>
              <tr><td>Mean aspect ratio</td><td>{fx(r().stats.aspect)}</td></tr>
              <tr><td>Threshold (Otsu's: {r().otsu})</td><td>{r().threshold}</td></tr>
            </tbody></table>
            <For each={r().notes}>{n => <p class="small tnote">{n}</p>}</For>
          </section>
          <section class="panel">
            <h2>Size distribution</h2>
            <Show when={r().bins.length} fallback={<p class="muted small">No particles measured.</p>}>
              <svg class="isize" viewBox="0 0 400 170">{(() => { const mx = Math.max(...r().bins.map(b => b[2]), 1), n = r().bins.length, bw = 360 / n; return <>
                <For each={r().bins}>{(b, k) => <><rect x={30 + k() * bw} y={140 - 120 * b[2] / mx} width={bw - 2} height={120 * b[2] / mx} fill="#6C4AB6" fill-opacity=".8"><title>{`${fx(b[0])}–${fx(b[1])} ${u()}: ${b[2]}`}</title></rect>
                  <Show when={k() % 2 === 0}><text x={30 + k() * bw} y="155" font-size="9" fill="var(--text2)">{fx(b[0], 1)}</text></Show></>}</For>
                <line x1="30" x2="390" y1="140" y2="140" stroke="var(--text2)" /><text x="390" y="168" text-anchor="end" font-size="10" fill="var(--text)">Equivalent diameter ({u()})</text><text x="28" y="14" text-anchor="end" font-size="9" fill="var(--text2)">{mx}</text></>; })()}</svg>
            </Show>
            <div class="hscroll lscroll"><table class="ptable iparts"><thead><tr><th>#</th><th>Area ({u()}²)</th><th>Diameter ({u()})</th><th>Circularity</th><th>Aspect</th><th>Edge</th></tr></thead>
              <tbody><For each={r().particles.slice(0, 200)}>{(p, k) => <tr classList={{ off: p.edge }}><td>{k() + 1}</td><td>{fx(p.area)}</td><td>{fx(p.ecd)}</td><td>{fx(p.circ, 3)}</td><td>{fx(p.aspect)}</td><td>{p.edge ? "yes" : ""}</td></tr>}</For></tbody></table></div>
            <Show when={r().particles.length > 200}><p class="muted small">The first 200 of {r().particles.length}; the CSV has them all (up to 5,000).</p></Show>
          </section></>}</Show>
      </div>}</Show>
    </div>
  );
};
