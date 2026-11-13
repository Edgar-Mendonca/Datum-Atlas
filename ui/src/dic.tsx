// DIC module › Strain maps: open a displacement field (x, y, u, v), draw a strain component with its
// colour scale, the statistics of every component, and a profile along a line drawn on the map.
// The engine (dic.rs) keeps the field and works out the strains.
import { For, Show, createEffect, createSignal, on } from "solid-js";
import { call, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";
import { ebsdMap, paint, setEbsdMap } from "./ebsd";

interface Summary { name: string; w: number; h: number; dx: number; dy: number; points: number; valid: number; notes: string[] }
interface Stat { key: string; name: string; mean: number | null; sd: number | null; min: number | null; max: number | null; p5: number | null; p95: number | null }
const KEYS: [string, string][] = [["exx", "εxx"], ["eyy", "εyy"], ["exy", "εxy"], ["e1", "ε1 (major principal)"], ["e2", "ε2 (minor principal)"], ["gmax", "Maximum shear γmax"], ["eeq", "Equivalent (von Mises)"], ["rot", "Rotation ω"], ["u", "Displacement u"], ["v", "Displacement v"]];
const DISP = new Set(["u", "v", "rot"]);
/** A strain as a percentage; displacements and rotation as they are. */
const sv = (key: string, v: number | null | undefined) => v == null ? "–" : DISP.has(key) ? v.toPrecision(4) : `${(v * 100).toFixed(3)} %`;

export const [dicField, setDicField] = createSignal<Summary | null>(null);

export const StrainPage = () => {
  const [key, setKey] = createSignal("eeq"), [busy, setBusy] = createSignal(false);
  const [range, setRange] = createSignal<[number, number]>([0, 1]), [stats, setStats] = createSignal<Stat[]>([]);
  const [line, setLine] = createSignal<number[][]>([]), [prof, setProf] = createSignal<[number, number][]>([]);
  let cv!: HTMLCanvasElement, ov!: HTMLCanvasElement;
  const load = async (cmd: string) => { setBusy(true); try { const s = await call<Summary | null>(cmd, {}); if (s) { setLine([]); setProf([]); setDicField(s); } } finally { setBusy(false); } };
  createEffect(on([dicField, key], async ([f, k]) => {
    if (!f || !cv) return;
    setBusy(true);
    try { const r = await invoke<{ w: number; h: number; rgb: string; range: [number, number]; stats: Stat[] }>("dic_draw", { key: k }); paint(cv, r.w, r.h, r.rgb); setRange(r.range); setStats(r.stats); }
    catch (e) { toast(String(e)); } finally { setBusy(false); }
  }));
  createEffect(on([line, key], async ([l, k]) => {
    const f = dicField(); if (!ov || !f) return;
    ov.width = f.w * 4; ov.height = f.h * 4;
    const x = ov.getContext("2d")!; x.clearRect(0, 0, ov.width, ov.height);
    if (l.length) { x.strokeStyle = "#111"; x.lineWidth = 3; x.beginPath(); x.moveTo(l[0][0] * 4 + 2, l[0][1] * 4 + 2); if (l[1]) x.lineTo(l[1][0] * 4 + 2, l[1][1] * 4 + 2); x.stroke(); }
    if (l.length === 2) setProf(await invoke<[number, number][]>("dic_profile", { key: k, a: l[0], b: l[1] }).catch(() => []));
  }));
  const click = (e: MouseEvent) => {
    const f = dicField(); if (!f) return;
    const b = ov.getBoundingClientRect(), p = [(e.clientX - b.left) / b.width * f.w - 0.5, (e.clientY - b.top) / b.height * f.h - 0.5];
    setLine(l => l.length === 1 ? [l[0], p] : [p]);
  };
  const save = async () => { const p = await call<string | null>("save_file", { name: `${dicField()!.name.replace(/\.[^.]+$/, "")} ${key()}.png`, data: cv.toDataURL("image/png") }); if (p) toast(`Saved ${p}`, true); };
  const signed = () => !["gmax", "eeq"].includes(key());
  const Profile = () => { const p = prof().filter(q => Number.isFinite(q[1])); if (p.length < 2) return null; const W = 520, H = 160, xm = Math.max(...p.map(q => q[0])), lo = Math.min(...p.map(q => q[1])), hi = Math.max(...p.map(q => q[1])), sp = hi - lo || 1e-9;
    return <svg class="dprof" viewBox={`0 0 ${W} ${H}`}><polyline fill="none" stroke="#6C4AB6" stroke-width="1.6" points={p.map(q => `${40 + (W - 50) * q[0] / xm},${H - 25 - (H - 40) * (q[1] - lo) / sp}`).join(" ")} />
      <line x1="40" x2={W - 10} y1={H - 25} y2={H - 25} stroke="var(--text2)" /><text x={W - 10} y={H - 8} text-anchor="end" font-size="10" fill="var(--text)">Distance along the line (0 to {xm.toFixed(1)})</text>
      <text x="36" y="14" text-anchor="end" font-size="9" fill="var(--text2)">{sv(key(), hi)}</text><text x="36" y={H - 27} text-anchor="end" font-size="9" fill="var(--text2)">{sv(key(), lo)}</text></svg>; };
  return (
    <div class="page dicpage">
      <h1>Strain maps <Help k="dic" /></h1>
      <div class="ttools">
        <button class="btn dopen" onClick={() => load("dic_open")}>Open a displacement field…</button>
        <button class="btn dex" onClick={() => load("dic_example")}>Example (made-up field)</button>
        <Show when={dicField()}><button class="btn sm" onClick={save}>Save as picture</button></Show>
        <Show when={busy()}><span class="muted small dbusy">working…</span></Show>
      </div>
      <Show when={dicField()} fallback={<p class="muted tempty">Open a displacement field exported from your DIC software: a CSV, text or spreadsheet file with columns x, y, u and v (one length unit). Or try the made-up example.</p>}>{f => <div class="imgrid">
        <section class="panel">
          <h2>{f().name} <span class="muted small" style={{ "text-transform": "none" }}>{f().w} × {f().h} points, step {f().dx} × {f().dy}</span></h2>
          <div class="dwrap"><canvas ref={cv} /><canvas ref={ov} class="dov" onClick={click} /></div>
          <div class="ecolor"><label>Show <select class="dkey" value={key()} onChange={e => setKey(e.currentTarget.value)}><For each={KEYS}>{k => <option value={k[0]}>{k[1]}</option>}</For></select></label>
            <span class="dscale" classList={{ signed: signed() }} /><span class="small">{sv(key(), range()[0])} to {sv(key(), range()[1])}</span></div>
          <p class="muted small">Click two points on the map for a profile along the line between them. Grey: no displacement there.</p>
          <Show when={prof().length}><Profile /></Show>
        </section>
        <section class="panel">
          <h2>Statistics</h2>
          <div class="hscroll"><table class="ptable dstats"><thead><tr><th>Component</th><th>Mean</th><th>sd</th><th>5 %</th><th>95 %</th><th>Lowest</th><th>Highest</th></tr></thead>
            <tbody><For each={stats()}>{s => <tr classList={{ on: s.key === key() }}><td>{KEYS.find(k => k[0] === s.key)?.[1]}</td><td class={`d${s.key}`}>{sv(s.key, s.mean)}</td><td>{sv(s.key, s.sd)}</td><td>{sv(s.key, s.p5)}</td><td>{sv(s.key, s.p95)}</td><td>{sv(s.key, s.min)}</td><td>{sv(s.key, s.max)}</td></tr>}</For></tbody></table></div>
          <p class="small">Points with a displacement: {f().valid.toFixed(1)} %.</p>
          <For each={f().notes}>{n => <p class="small tnote">{n}</p>}</For>
          <p class="muted small">Small-strain gradients by central differences; the equivalent strain takes εzz = −(εxx + εyy). Strain by grain, linked to an EBSD map, comes next.</p>
        </section>
      </div>}</Show>
    </div>
  );
};

interface GrainStrain { id: number; points: number; ecd: number; mean: number; sd: number; schmid: number; ipf: [number, number, number]; cx: number; cy: number; traces: [number, number][] }
interface Linked { w: number; h: number; rgb: string; range: [number, number]; grains: GrainStrain[]; rSchmid: number | null; covered: number; notes: string[] }

export const ByGrainPage = () => {
  const [key, setKey] = createSignal("eeq"), [fit, setFit] = createSignal(true), [off, setOff] = createSignal<[number, number]>([0, 0]), [scale, setScale] = createSignal(1);
  const [tol, setTol] = createSignal(10), [load, setLoad] = createSignal("x"), [slip, setSlip] = createSignal("fcc"), [traces, setTraces] = createSignal(true);
  const [res, setRes] = createSignal<Linked | null>(null), [busy, setBusy] = createSignal(false);
  let cv!: HTMLCanvasElement, ov!: HTMLCanvasElement, seq = 0;
  const example = async () => { setBusy(true); try { const r = await call<{ ebsd: any; dic: Summary }>("dic_link_example"); if (r) { setEbsdMap(r.ebsd); setDicField(r.dic); } } finally { setBusy(false); } };
  createEffect(on([ebsdMap, dicField, key, fit, off, scale, tol, load, slip], async () => {
    if (!ebsdMap() || !dicField() || !cv) return;
    const my = ++seq; await new Promise(r => setTimeout(r, 40)); if (my !== seq) return;
    setBusy(true);
    try { const r = await invoke<Linked>("dic_link", { ask: { key: key(), fit: fit(), offset: off(), scale: scale(), grains: { tol: tol(), min: 5, low: 2, load: load(), slip: slip() } } }); if (my === seq) { paint(cv, r.w, r.h, r.rgb); setRes(r); } }
    catch (e) { toast(String(e)); } finally { if (my === seq) setBusy(false); }
  }));
  // Slip traces: a line through each grain's centre for each slip plane, the most stressed in amber.
  createEffect(() => {
    const r = res(), show = traces(); if (!r || !ov) return;
    const k = 4; ov.width = r.w * k; ov.height = r.h * k;
    const x = ov.getContext("2d")!; x.clearRect(0, 0, ov.width, ov.height); if (!show) return;
    for (const g of r.grains) { if (g.points < 40) continue; const L = Math.sqrt(g.points) * 0.45 * k;
      g.traces.forEach((t, i) => { const a = t[0] * Math.PI / 180, dx = Math.cos(a) * L, dy = Math.sin(a) * L; x.strokeStyle = i === 0 ? "#F2B233" : "rgba(255,255,255,.85)"; x.lineWidth = i === 0 ? 3 : 1.5; x.beginPath(); x.moveTo(g.cx * k + 2 - dx, g.cy * k + 2 - dy); x.lineTo(g.cx * k + 2 + dx, g.cy * k + 2 + dy); x.stroke(); }); }
  });
  const Scatter = () => { const r = res(); if (!r || r.grains.length < 3) return null; const W = 360, H = 220, g = r.grains, sx = (v: number) => 40 + (W - 50) * (v - 0.25) / 0.25, ys = g.map(q => q.mean), lo = Math.min(...ys), hi = Math.max(...ys), sy = (v: number) => H - 30 - (H - 45) * (v - lo) / (hi - lo || 1);
    return <svg class="dscat" viewBox={`0 0 ${W} ${H}`}><line x1="40" x2={W - 10} y1={H - 30} y2={H - 30} stroke="var(--text2)" /><line x1="40" x2="40" y1="10" y2={H - 30} stroke="var(--text2)" />
      <For each={g}>{q => <circle cx={sx(q.schmid)} cy={sy(q.mean)} r={Math.min(6, 1.5 + Math.sqrt(q.points) / 8)} fill={`rgb(${q.ipf.join(",")})`} stroke="var(--text2)" stroke-width=".5"><title>{`Grain ${q.id}: ${sv(key(), q.mean)}, Schmid ${q.schmid.toFixed(3)}`}</title></circle>}</For>
      <text x={W - 10} y={H - 12} text-anchor="end" font-size="10" fill="var(--text)">Schmid factor (0.25–0.5)</text><text x="44" y="18" font-size="10" fill="var(--text)">{KEYS.find(k => k[0] === key())?.[1]} of the grain</text></svg>; };
  return (
    <div class="page dicpage">
      <h1>Strain by grain <Help k="bygrain" /></h1>
      <div class="ttools"><button class="btn lex" onClick={example} title="The EBSD example map with a made-up displacement field over it">Example (made-up map and field)</button>
        <Show when={busy()}><span class="muted small lbusy">working…</span></Show></div>
      <Show when={ebsdMap() && dicField()} fallback={<p class="muted tempty">Open an EBSD map (EBSD › Maps) and a displacement field of the same area (DIC › Strain maps), or try the made-up example.</p>}>
        <div class="imgrid">
          <section class="panel">
            <h2>{dicField()!.name} over {ebsdMap()!.name}</h2>
            <div class="dwrap"><canvas ref={cv} /><canvas ref={ov} class="dov" /></div>
            <div class="ecolor"><label>Show <select class="lkey" value={key()} onChange={e => setKey(e.currentTarget.value)}><For each={KEYS.filter(k => !["u", "v"].includes(k[0]))}>{k => <option value={k[0]}>{k[1]}</option>}</For></select></label>
              <Show when={res()}><span class="dscale" classList={{ signed: !["gmax", "eeq"].includes(key()) }} /><span class="small">{sv(key(), res()!.range[0])} to {sv(key(), res()!.range[1])}</span></Show>
              <label class="small"><input type="checkbox" checked={traces()} onChange={e => setTraces(e.currentTarget.checked)} /> Slip traces</label></div>
            <p class="muted small">Black: grain boundaries from the EBSD map. Slip traces: the lines the slip planes make on the surface, through each grain's centre; amber is the plane with the highest Schmid factor.</p>
          </section>
          <section class="panel icontrols">
            <h2>Settings</h2>
            <label><input type="checkbox" class="lfit" checked={fit()} onChange={e => setFit(e.currentTarget.checked)} /> The field covers the same area as the map</label>
            <Show when={!fit()}><label>Offset <input type="number" value={off()[0]} onChange={e => setOff([+e.currentTarget.value, off()[1]])} /> <input type="number" value={off()[1]} onChange={e => setOff([off()[0], +e.currentTarget.value])} /> µm, scale <input type="number" step="0.01" value={scale()} onChange={e => setScale(+e.currentTarget.value)} /></label></Show>
            <label>Grain tolerance <input type="number" value={tol()} onChange={e => setTol(Math.min(30, Math.max(1, +e.currentTarget.value)))} /> °</label>
            <label>Load along <select value={load()} onChange={e => setLoad(e.currentTarget.value)}><option value="x">X</option><option value="y">Y</option><option value="z">Z</option></select>
              <select value={slip()} onChange={e => setSlip(e.currentTarget.value)}><option value="fcc">{"{111}<110> (FCC)"}</option><option value="bcc">{"{110}<111> (BCC)"}</option></select></label>
            <Show when={res()}>{r => <>
              <p class="small">Grains: <b class="lgrains">{r().grains.length}</b>; the field covers {r().covered.toFixed(0)} % of the map; strain against Schmid factor, r = <b class="lr">{r().rSchmid == null ? "–" : r().rSchmid!.toFixed(2)}</b></p>
              <For each={r().notes}>{n => <p class="small tnote">{n}</p>}</For>
              <Scatter />
              <div class="hscroll lscroll"><table class="ptable dstats lgtab"><thead><tr><th>Grain</th><th>Size (µm)</th><th>Mean</th><th>sd</th><th>Schmid</th></tr></thead>
                <tbody><For each={r().grains.slice(0, 60)}>{g => <tr><td><i class="sw" style={{ background: `rgb(${g.ipf.join(",")})` }} />{g.id}</td><td>{g.ecd.toFixed(1)}</td><td>{sv(key(), g.mean)}</td><td>{sv(key(), g.sd)}</td><td>{g.schmid.toFixed(3)}</td></tr>}</For></tbody></table></div>
              <p class="muted small">Most strained first; the first 60 of {r().grains.length}. Swatches: IPF colour of the normal.</p></>}</Show>
          </section>
        </div>
      </Show>
    </div>
  );
};
