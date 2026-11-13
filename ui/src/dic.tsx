// DIC module › Strain maps: open a displacement field (x, y, u, v), draw a strain component with its
// colour scale, the statistics of every component, and a profile along a line drawn on the map.
// The engine (dic.rs) keeps the field and works out the strains.
import { For, Show, createEffect, createSignal, on } from "solid-js";
import { call, toast } from "./store";
import { invoke } from "./api";
import { Help } from "./help";
import { paint } from "./ebsd";

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
