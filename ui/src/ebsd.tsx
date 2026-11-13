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

interface GrainOut { grains: number; inside: number; meanEcd: number | null; sdEcd: number | null; meanArea: number | null; high: number; low: number; twins: number; boundaryMmPerMm2: number; kam: number; schmid: number | null; bins: [number, number, number][] }

export const GrainsPage = () => {
  const [kind, setKind] = createSignal("boundaries"), [tol, setTol] = createSignal(10), [min, setMin] = createSignal(5), [load, setLoad] = createSignal("x"), [slip, setSlip] = createSignal("fcc");
  const [out, setOut] = createSignal<GrainOut | null>(null), [busy, setBusy] = createSignal(false);
  let cv!: HTMLCanvasElement, seq = 0;
  createEffect(on([ebsdMap, kind, tol, min, load, slip], async ([m, k]) => {
    if (!m || !cv) return;
    const my = ++seq; await new Promise(r => setTimeout(r, 40)); if (my !== seq) return;
    setBusy(true);
    try { const r = await invoke<{ w: number; h: number; rgb: string; out: GrainOut }>("ebsd_grains", { kind: k, ask: { tol: tol(), min: min(), low: 2, load: load(), slip: slip() } }); if (my === seq) { paint(cv, r.w, r.h, r.rgb); setOut(r.out); } }
    catch (e) { toast(String(e)); }
    finally { if (my === seq) setBusy(false); }
  }));
  const fv = (v: number | null | undefined, d = 1) => v == null ? "–" : fx(v, d);
  return (
    <div class="page ebsdpage">
      <h1>Grains and boundaries <Help k="egrains" /></h1>
      <Show when={ebsdMap()} fallback={<p class="muted tempty">Open a map on the Maps page first.</p>}>{m => <div class="imgrid">
        <section class="panel">
          <h2>{m().name} <Show when={busy()}><span class="muted small gbusy">working…</span></Show></h2>
          <div class="ivwrap"><canvas ref={cv} /></div>
          <div class="ecolor"><label>Show <select class="gkind" value={kind()} onChange={e => setKind(e.currentTarget.value)}>
            <option value="boundaries">Boundaries over pattern quality</option><option value="grains">Grains</option><option value="kam">Local misorientation (KAM)</option><option value="schmid">Schmid factor</option></select></label></div>
          <Show when={kind() === "boundaries"}><p class="small ekey"><span><i style={{ background: "#000" }} />high-angle (≥ 15°)</span><span><i style={{ background: "#969696" }} />low-angle (2–15°)</span><span><i style={{ background: "#DC1E1E" }} />Σ3 twin</span></p></Show>
          <Show when={kind() === "kam" || kind() === "schmid"}><p class="small ekey"><span class="ramp" />{kind() === "kam" ? "0° to 2° and above" : "0.25 to 0.5"}</p></Show>
        </section>
        <section class="panel icontrols">
          <h2>Settings</h2>
          <label>Grain tolerance <input type="number" class="gtol" min="1" max="30" value={tol()} onChange={e => setTol(Math.min(30, Math.max(1, +e.currentTarget.value)))} /> °</label>
          <label>Smallest grain <input type="number" min="1" value={min()} onChange={e => setMin(Math.max(1, Math.round(+e.currentTarget.value)))} /> points</label>
          <label>Load along <select class="gload" value={load()} onChange={e => setLoad(e.currentTarget.value)}><option value="x">X</option><option value="y">Y</option><option value="z">Z</option></select></label>
          <label>Cubic slip systems <select class="gslip" value={slip()} onChange={e => setSlip(e.currentTarget.value)}><option value="fcc">{"{111}<110> (FCC)"}</option><option value="bcc">{"{110}<111> (BCC)"}</option></select></label>
          <Show when={out()}>{o => <table class="ptable ires gres"><tbody>
            <tr><td>Grains</td><td class="ggrains">{o().grains}</td></tr>
            <tr><td>Mean equivalent diameter (not on the edge)</td><td class="gecd">{fv(o().meanEcd)} µm</td></tr>
            <tr><td>Spread (sd)</td><td>{fv(o().sdEcd)} µm</td></tr>
            <tr><td>Mean grain area</td><td>{fv(o().meanArea)} µm²</td></tr>
            <tr><td>High-angle boundaries</td><td class="ghigh">{fv(o().high)} %</td></tr>
            <tr><td>Low-angle boundaries</td><td>{fv(o().low)} %</td></tr>
            <tr><td>Σ3 twin boundaries</td><td class="gtwin">{fv(o().twins)} %</td></tr>
            <tr><td>Boundary length per area</td><td>{fv(o().boundaryMmPerMm2, 0)} mm/mm²</td></tr>
            <tr><td>Mean local misorientation (KAM)</td><td>{fv(o().kam, 2)}°</td></tr>
            <tr><td>Mean Schmid factor</td><td class="gsf">{fv(o().schmid, 3)}</td></tr></tbody></table>}</Show>
          <p class="muted small">Shares of boundaries are by length. Phase boundaries are not counted.</p>
        </section>
      </div>}</Show>
    </div>
  );
};

interface Figure { family: string; crystal: string; orientations: number; points: [number, number][]; colours: [number, number, number][]; grid: number; density: number[]; maxMrd: number }
const FAMS: Record<string, [string, string][]> = { cubic: [["100", "{100}"], ["110", "{110}"], ["111", "{111}"]], hex: [["0001", "{0001}"], ["10-10", "{101̄0}"], ["11-20", "{112̄0}"]] };
/** Density colour: white at 0 mrd through violet to amber at the top. */
const mrdColour = (v: number, mx: number) => { const t = Math.min(1, Math.log1p(v) / Math.log1p(Math.max(mx, 2))); return t < 0.5 ? `rgb(${255 - 2 * t * 147},${255 - 2 * t * 181},${255 - 2 * t * 73})` : `rgb(${108 + (t - 0.5) * 2 * 134},${74 + (t - 0.5) * 2 * 104},${182 - (t - 0.5) * 2 * 131})`; };

const PoleFigure = (p: { f: Figure; dots: boolean }) => {
  const n = () => p.f.grid, c = 1 / (p.f.grid - 1);
  return <figure class="pf"><svg viewBox="-1.18 -1.18 2.36 2.36">
    <For each={p.f.density.map((v, k) => [v, k] as const).filter(x => Number.isFinite(x[0]) && x[0] > 0.05)}>{([v, k]) => <rect x={2 * (k % n()) * c - 1 - c} y={2 * Math.floor(k / n()) * c - 1 - c} width={2 * c + 0.005} height={2 * c + 0.005} fill={mrdColour(v, p.f.maxMrd)} />}</For>
    <Show when={p.dots}><For each={p.f.points.slice(0, 2500)}>{q => <circle cx={q[0]} cy={-q[1]} r="0.008" fill="var(--text)" opacity=".5" />}</For></Show>
    <circle r="1" fill="none" stroke="var(--text2)" stroke-width="0.012" />
    <text x="1.05" y="0.04" font-size="0.1" fill="var(--text)">X</text><text x="-0.03" y="-1.05" font-size="0.1" fill="var(--text)">Y</text>
  </svg><figcaption>{FAMS[p.f.crystal]?.find(x => x[0] === p.f.family)?.[1] ?? p.f.family}: max <b class="pfmax">{fx(p.f.maxMrd, 1)}</b> × random</figcaption></figure>;
};
const InversePF = (p: { f: Figure }) => {
  const hex = () => p.f.crystal === "hex";
  const edge = () => { const pts: string[] = []; if (hex()) { for (let k = 0; k <= 20; k++) { const a = k / 20 * Math.PI / 6; pts.push(`${Math.cos(a)},${-Math.sin(a)}`); } } else { for (let k = 0; k <= 20; k++) { const t = k / 20, v = [1, t, 1], nn = Math.hypot(...v), z = 1 / nn, x = 1 / nn, y = t / nn; pts.push(`${x / (1 + z) / 0.4142},${-(y / (1 + z)) / 0.4142}`); } } return `M0,0 L${hex() ? "1,0" : "1,0"} ${pts.map(q => "L" + q).join(" ")} Z`; };
  return <figure class="pf"><svg viewBox="-0.12 -1.0 1.25 1.15">
    <path d={edge()} fill="none" stroke="var(--text2)" stroke-width="0.008" />
    <For each={p.f.points.slice(0, 3000)}>{(q, i) => <circle cx={q[0]} cy={-q[1]} r="0.012" fill={`rgb(${p.f.colours[i()].join(",")})`} />}</For>
    <text x="-0.08" y="0.1" font-size="0.07" fill="var(--text)">{hex() ? "[0001]" : "[001]"}</text><text x="0.9" y="0.1" font-size="0.07" fill="var(--text)">{hex() ? "[21̄1̄0]" : "[101]"}</text><text x={hex() ? 0.8 : 0.82} y={hex() ? -0.55 : -0.92} font-size="0.07" fill="var(--text)">{hex() ? "[101̄0]" : "[111]"}</text>
  </svg><figcaption>Inverse pole figure of {p.f.family.toUpperCase()}</figcaption></figure>;
};

export const PoleFiguresPage = () => {
  const [phase, setPhase] = createSignal(1), [dots, setDots] = createSignal(false), [busy, setBusy] = createSignal(false);
  const [pfs, setPfs] = createSignal<Figure[]>([]), [ipfs, setIpfs] = createSignal<Figure[]>([]);
  createEffect(on([ebsdMap, phase], async ([m, ph]) => {
    if (!m) return;
    const crystal = m.phases[ph - 1]?.[0].crystal ?? "cubic";
    setBusy(true);
    try {
      setPfs(await Promise.all(FAMS[crystal].map(f => invoke<Figure>("ebsd_figure", { kind: "pf", phase: ph, of: f[0] }))));
      setIpfs(await Promise.all(["x", "y", "z"].map(d => invoke<Figure>("ebsd_figure", { kind: "ipf", phase: ph, of: d }))));
    } catch (e) { toast(String(e)); setPfs([]); setIpfs([]); }
    finally { setBusy(false); }
  }));
  return (
    <div class="page ebsdpage pfpage">
      <h1>Pole figures <Help k="polefig" /></h1>
      <Show when={ebsdMap()} fallback={<p class="muted tempty">Open a map on the Maps page first.</p>}>{m => <>
        <div class="ttools"><label>Phase <select class="pfphase" value={phase()} onChange={e => setPhase(+e.currentTarget.value)}><For each={m().phases}>{(p, i) => <option value={i() + 1}>{p[0].name}</option>}</For></select></label>
          <label><input type="checkbox" checked={dots()} onChange={e => setDots(e.currentTarget.checked)} /> Show the poles as points</label>
          <Show when={busy()}><span class="muted small pfbusy">working…</span></Show></div>
        <section class="panel"><h2>Pole figures <span class="muted small" style={{ "text-transform": "none" }}>{pfs()[0]?.orientations ?? 0} orientations, density in multiples of random</span></h2>
          <div class="pfrow"><For each={pfs()}>{f => <PoleFigure f={f} dots={dots()} />}</For></div></section>
        <section class="panel"><h2>Inverse pole figures</h2><div class="pfrow"><For each={ipfs()}>{f => <InversePF f={f} />}</For></div></section>
      </>}</Show>
    </div>
  );
};
