// Crystal module › Elements: a clickable periodic table of the 118 elements (crates/core elements.rs),
// coloured by category or by a property, an element card with its sources, the library's materials
// built on the element, and the composition of a formula or an alloy (molar mass, wt % and at %).
import { For, Show, createEffect, createResource, createSignal } from "solid-js";
import { call, db, openSheet, show, srcOf } from "./store";
import { invoke } from "./api";
import { Help } from "./help";

export interface Element { z: number; symbol: string; name: string; mass: number; radioactive: boolean; category: string; group: number | null; period: number; row: number; col: number;
  state?: string; density?: number; melt?: number; boil?: number; structure?: string; electroneg?: number; a?: number; c?: number; note?: string; src: string; massSrc: string }
interface Comp { text: string; by: string; mass: number; parts: { symbol: string; name: string; atoms: number; at: number; wt: number }[] }

export const CATS: [string, string][] = [["Alkali metal", "#E0604F"], ["Alkaline earth metal", "#E8964A"], ["Transition metal", "#D9B23A"], ["Post-transition metal", "#7FB069"], ["Metalloid", "#3FA39B"],
  ["Nonmetal", "#4A90D9"], ["Halogen", "#6C7BE0"], ["Noble gas", "#9A6CD9"], ["Lanthanide", "#D46FB0"], ["Actinide", "#B0607A"], ["Superheavy (unknown)", "#8A8F98"]];
/** What the table can be coloured by: [key, label, unit, log]. */
const BY: [string, string, string, boolean][] = [["category", "Category", "", false], ["density", "Density", "kg/m³", true], ["melt", "Melting point", "°C", false], ["electroneg", "Electronegativity", "Pauling", false], ["mass", "Atomic mass", "g/mol", false]];
const num = (v: number, d = 4) => Number(v.toPrecision(d)).toLocaleString("en-GB", { maximumFractionDigits: 6 });

export const [elFocus, setElFocus] = createSignal(26);

export const ElementsPage = () => {
  const [els] = createResource(() => call<Element[]>("elements"));
  const [by, setBy] = createSignal("category");
  const [q, setQ] = createSignal("");
  const el = () => els()?.find(e => e.z === elFocus());
  const key = () => BY.find(b => b[0] === by())!;
  const span = () => { const v = (els() ?? []).map(e => (e as any)[by()] as number | undefined).filter((x): x is number => x != null); const lg = key()[3]; const f = (x: number) => lg ? Math.log10(Math.max(x, 1e-3)) : x; return [Math.min(...v.map(f)), Math.max(...v.map(f)), lg] as const; };
  const fill = (e: Element) => {
    if (by() === "category") return `color-mix(in srgb, ${CATS.find(c => c[0] === e.category)?.[1] ?? "#888"} 30%, var(--bg2))`;
    const v = (e as any)[by()] as number | undefined; if (v == null) return "";
    const [lo, hi, lg] = span(), t = ((lg ? Math.log10(Math.max(v, 1e-3)) : v) - lo) / (hi - lo || 1);
    return `color-mix(in srgb, var(--blue) ${Math.round(8 + t * 72)}%, var(--bg2))`;
  };
  const find = () => { const s = q().trim().toLowerCase(); const e = (els() ?? []).find(x => x.symbol.toLowerCase() === s || x.name.toLowerCase() === s || String(x.z) === s) ?? (els() ?? []).find(x => x.name.toLowerCase().startsWith(s)); if (s && e) setElFocus(e.z); };
  return (
    <div class="page elpage">
      <h1>The elements <Help k="elements" /></h1>
      <div class="eltools">
        <label>Colour by <select value={by()} onChange={e => setBy(e.currentTarget.value)}><For each={BY}>{b => <option value={b[0]}>{b[1]}</option>}</For></select></label>
        <label>Find <input class="elfind" placeholder="Fe, iron or 26" value={q()} onInput={e => setQ(e.currentTarget.value)} onKeyDown={e => e.key === "Enter" && find()} /></label>
        <Show when={by() !== "category"} fallback={<div class="ellegend"><For each={CATS}>{c => <span><i style={{ background: c[1] }} />{c[0]}</span>}</For></div>}>
          <span class="muted small">{key()[1]}{key()[2] ? `, ${key()[2]}` : ""}{key()[3] ? " (log scale)" : ""}: pale is low, deep is high; blank where no value is known.</span>
        </Show>
      </div>
      <div class="pt">
        <For each={els()}>{e => <button class="el" classList={{ on: e.z === elFocus(), gap: by() !== "category" && (e as any)[by()] == null }} style={{ "grid-row": e.row, "grid-column": e.col, background: fill(e) }}
          title={`${e.name}${by() !== "category" ? `: ${(e as any)[by()] != null ? num((e as any)[by()]) + " " + key()[2] : "no value"}` : ""}`} onClick={() => setElFocus(e.z)}>
          <small>{e.z}</small><b>{e.symbol}</b><span>{by() === "category" ? num(e.mass, 5) : (e as any)[by()] != null ? num((e as any)[by()], 3) : "–"}</span></button>}</For>
        <div class="ptgap" style={{ "grid-row": 6, "grid-column": 3 }}>57–71</div><div class="ptgap" style={{ "grid-row": 7, "grid-column": 3 }}>89–103</div>
      </div>
      <div class="elbottom">
        <Show when={el()}>{e => <ElementCard e={e()} />}</Show>
        <CompositionBox />
      </div>
    </div>
  );
};

const ElementCard = (p: { e: Element }) => {
  const e = () => p.e;
  const rows = (): [string, string | undefined][] => [
    ["Atomic number", String(e().z)], ["Atomic mass", `${num(e().mass, 6)} g/mol${e().radioactive && Number.isInteger(e().mass) ? " (longest-lived isotope)" : ""}`],
    ["Category", e().category], ["Group, period", `${e().group ?? "–"}, ${e().period}`], ["State at 20 °C", e().state],
    ["Density", e().density != null ? `${num(e().density!)} kg/m³${e().state === "gas" ? " (0 °C, 1 atm)" : ""}` : undefined],
    ["Melting point", e().melt != null ? `${num(e().melt!, 5)} °C` : undefined], ["Boiling point", e().boil != null ? `${num(e().boil!, 5)} °C` : undefined],
    ["Crystal structure", e().structure], ["Lattice parameters", e().a != null ? `a = ${e().a} Å${e().c != null ? `, c = ${e().c} Å` : ""}` : undefined],
    ["Electronegativity", e().electroneg != null ? `${e().electroneg} (Pauling)` : undefined], ["Radioactive", e().radioactive ? "Yes" : "No"],
  ];
  // Materials in the library named after the element, or of a class named after it.
  const mats = () => { const n = e().name.toLowerCase(), alt = n === "aluminium" ? "aluminum" : n; return db().materials.filter(m => [m.name, ...m.family].some(s => { const t = s.toLowerCase(); return t.startsWith(n) || t.startsWith(alt) || t.includes(` ${n}`); })).slice(0, 12); };
  return (
    <section class="panel elcard">
      <h2><span class="elsym" style={{ background: `color-mix(in srgb, ${CATS.find(c => c[0] === e().category)?.[1] ?? "#888"} 30%, var(--bg2))` }}>{e().symbol}</span>{e().name}</h2>
      <table class="ptable"><tbody><For each={rows()}>{([k, v]) => <tr><td>{k}</td><td classList={{ muted: v == null }}>{v ?? "No reliable value known"}</td></tr>}</For></tbody></table>
      <Show when={e().note}><p class="small">{e().note}</p></Show>
      <Show when={e().a != null && ["BCC", "FCC", "HCP", "diamond cubic", "simple cubic"].includes(e().structure ?? "") && (e().structure !== "HCP" || e().c != null)}>
        <p><button class="btn sm tocalc" onClick={() => { setAsk({ ...ask(), element: e().symbol }); show("calc"); }}>Its crystal, X-ray peaks and projection</button></p></Show>
      <p class="muted small">Sources: {[srcOf(e().src)?.title, srcOf(e().massSrc)?.title].filter(Boolean).join("; ")} (atomic mass). Draft values, to be checked one by one.</p>
      <Show when={mats().length}><h3>In the library</h3><p class="elmats"><For each={mats()}>{m => <button class="btn sm" onClick={() => openSheet(m.id)}>{m.name}</button>}</For></p></Show>
    </section>
  );
};

const CompositionBox = () => {
  const [text, setText] = createSignal("Fe2O3");
  const [by, setBy] = createSignal("formula");
  const [res, setRes] = createSignal<Comp | string | null>(null);
  const run = async () => { try { setRes(await invoke<Comp>("composition", { text: text(), by: by() })); } catch (e) { setRes(String(e)); } };
  run();
  return (
    <section class="panel compbox">
      <h2>Composition <Help k="composition" /></h2>
      <div class="comprow">
        <select value={by()} onChange={e => { setBy(e.currentTarget.value); setText(e.currentTarget.value === "formula" ? "Fe2O3" : e.currentTarget.value === "wt" ? "Al Cu 4.4 Mg 1.5 Mn 0.6" : "Ni 50 Ti 50"); run(); }}>
          <option value="formula">Formula</option><option value="wt">Weight %</option><option value="at">Atom %</option></select>
        <input class="compin" value={text()} onInput={e => setText(e.currentTarget.value)} onKeyDown={e => e.key === "Enter" && run()} />
        <button class="btn sm" onClick={run}>Work out</button>
      </div>
      <Show when={res()} keyed>{r => typeof r === "string" ? <p class="err">{r}</p> : (() => { const c = r; return <>
        <p><b>{c.by === "formula" ? "Molar mass" : "Mean atomic mass"}: {num(c.mass, 6)} g/mol</b></p>
        <table class="ptable comp"><thead><tr><th>Element</th><Show when={c.by === "formula"}><th>Atoms</th></Show><th>at %</th><th>wt %</th></tr></thead>
          <tbody><For each={c.parts}>{x => <tr><td><b>{x.symbol}</b> {x.name}</td><Show when={c.by === "formula"}><td>{num(x.atoms, 4)}</td></Show><td>{x.at.toFixed(3)}</td><td>{x.wt.toFixed(3)}</td></tr>}</For></tbody></table></>; })()}</Show>
    </section>
  );
};

// ── Crystal › Calculators ─────────────────────────────────────────────────────────────────────
type V3 = [number, number, number];
interface Cell { a: number; b: number; c: number; alpha: number; beta: number; gamma: number }
export interface Ask { element: string | null; kind: string; cell: Cell; species: string[]; lambda: number; max: number; hkl: V3; hkl2: V3; uvw: V3; uvw2: V3; centre: V3; poles: number }
interface Peak { hkl: V3; hkil: number[] | null; mult: number; d: number; twoTheta: number; intensity: number }
interface Answer { structure: { kind: string; name: string; system: string; cell: Cell; atoms: [string, V3][] }; basis: [V3, V3, V3]; drawn: [string, V3][]; volume: number; density: number; atomsPerCell: number;
  d: number | null; bragg: number | null; anglePlanes: number | null; angleDirs: number | null; plane4: number[] | null; dir4: number[] | null; peaks: Peak[]; poles: { hkl: V3; x: number; y: number }[] }

/** Miller indices with bars over negatives: (1̄10), [21̄1̄0]. */
export const mi = (h: number[], o = "(", c = ")") => o + h.map(x => x < 0 ? `${-x}̄` : String(x)).join(h.some(x => Math.abs(x) > 9) ? " " : "") + c;
const SP = ["#6C4AB6", "#F2B233"];
export const [ask, setAsk] = createSignal<Ask>({ element: "Fe", kind: "bcc", cell: { a: 2.8665, b: 2.8665, c: 2.8665, alpha: 90, beta: 90, gamma: 90 }, species: ["Fe", "O"], lambda: 1.5418, max: 120, hkl: [1, 1, 0], hkl2: [1, 0, 0], uvw: [1, 1, 1], uvw2: [1, 0, 0], centre: [0, 0, 1], poles: 2 });
const upd = (f: (a: Ask) => void) => { const n = structuredClone(ask()); f(n); setAsk(n); };

const Idx = (p: { v: V3; set: (v: V3) => void; cls?: string }) => <span class={`idx ${p.cls ?? ""}`}><For each={[0, 1, 2]}>{k => <input type="number" step="1" value={p.v[k]} onChange={e => { const v = [...p.v] as V3; v[k] = Math.trunc(+e.currentTarget.value || 0); p.set(v); }} />}</For></span>;
const Num = (p: { v: number; set: (v: number) => void; step?: number; cls?: string }) => <input class={p.cls} type="number" step={p.step ?? 0.001} value={p.v} onChange={e => p.set(+e.currentTarget.value)} />;

/** The unit cell, turned a little so its depth shows: edges, then atoms back to front. */
const CellView = (p: { a: Answer }) => {
  const [yaw, setYaw] = createSignal(-30);
  const proj = () => { const y = yaw() * Math.PI / 180, t = 18 * Math.PI / 180; return (v: V3): V3 => { const x1 = v[0] * Math.cos(y) - v[1] * Math.sin(y), y1 = v[0] * Math.sin(y) + v[1] * Math.cos(y); return [x1, -(v[2] * Math.cos(t) - y1 * Math.sin(t)), v[2] * Math.sin(t) + y1 * Math.cos(t)]; }; };
  const cart = (f: V3): V3 => { const b = p.a.basis; return [0, 1, 2].map(i => f[0] * b[0][i] + f[1] * b[1][i] + f[2] * b[2][i]) as V3; };
  const view = () => {
    const P = proj(), corners = [0, 1, 2, 3, 4, 5, 6, 7].map(k => [k & 1, k >> 1 & 1, k >> 2 & 1] as V3);
    const pts = corners.map(c => P(cart(c))), xs = pts.map(q => q[0]), ys = pts.map(q => q[1]);
    const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)], s = 200 / Math.max(x1 - x0, y1 - y0, 1e-9);
    const map = (q: V3) => [20 + (q[0] - x0) * s, 20 + (q[1] - y0) * s, q[2]];
    const edges = corners.flatMap((c, i) => corners.map((d, j) => j > i && Math.abs(c[0] - d[0]) + Math.abs(c[1] - d[1]) + Math.abs(c[2] - d[2]) === 1 ? [map(pts[i]), map(pts[j])] : null).filter(Boolean)) as number[][][];
    const sp = [...new Set(p.a.structure.atoms.map(a => a[0]))];
    const r = Math.max(5, Math.min(16, 0.13 * Math.min(p.a.structure.cell.a, p.a.structure.cell.b, p.a.structure.cell.c) * s / (p.a.atomsPerCell > 4 ? 1.6 : 1)));
    const atoms = p.a.drawn.map(([e, f]) => { const q = map(P(cart(f))); return { e, x: q[0], y: q[1], z: q[2], col: SP[sp.indexOf(e)] ?? "#888" }; }).sort((u, v) => u.z - v.z);
    return { edges, atoms, r, w: 40 + (x1 - x0) * s, h: 40 + (y1 - y0) * s, sp };
  };
  return <div class="cellview">
    <svg viewBox={`0 0 ${view().w} ${view().h}`} width="260"><For each={view().edges}>{e => <line x1={e[0][0]} y1={e[0][1]} x2={e[1][0]} y2={e[1][1]} stroke="var(--muted)" stroke-width="1.2" />}</For>
      <For each={view().atoms}>{a => <circle cx={a.x} cy={a.y} r={view().r} fill={a.col} stroke="var(--bg)" stroke-width="1.2" opacity=".95" />}</For></svg>
    <input type="range" min="-90" max="90" value={yaw()} onInput={e => setYaw(+e.currentTarget.value)} title="Turn the cell" />
    <div class="small muted"><For each={view().sp}>{(e, i) => <span class="spk"><i style={{ background: SP[i()] }} />{e} </span>}</For></div>
  </div>;
};

const PeaksChart = (p: { ps: Peak[]; max: number; four: boolean }) => {
  const W = 560, H = 190, x = (t: number) => 30 + (t - 0) / p.max * (W - 40), y = (i: number) => H - 24 - i / 100 * (H - 50);
  return <svg class="xrd" viewBox={`0 0 ${W} ${H}`}>
    <line x1="30" y1={H - 24} x2={W - 10} y2={H - 24} stroke="var(--text2)" />
    <For each={Array.from({ length: Math.floor(p.max / 20) + 1 }, (_, k) => k * 20)}>{t => <><line x1={x(t)} y1={H - 24} x2={x(t)} y2={H - 20} stroke="var(--text2)" /><text x={x(t)} y={H - 8} text-anchor="middle" font-size="10" fill="var(--text2)">{t}</text></>}</For>
    <text x={W - 10} y={14} text-anchor="end" font-size="10" fill="var(--text2)">2θ (°) →</text>
    <For each={p.ps}>{q => <><line x1={x(q.twoTheta)} y1={H - 24} x2={x(q.twoTheta)} y2={y(q.intensity)} stroke="var(--blue)" stroke-width="2" />
      <Show when={q.intensity > 8}><text x={x(q.twoTheta)} y={y(q.intensity) - 4} text-anchor="middle" font-size="9" fill="var(--text)">{p.four && q.hkil ? mi(q.hkil, "", "") : mi(q.hkl, "", "")}</text></Show></>}</For>
  </svg>;
};

const StereoView = (p: { poles: Answer["poles"]; centre: V3; label: boolean }) => {
  const R = 120, c = 140;
  return <svg class="stereo" viewBox="0 0 280 280">
    <circle cx={c} cy={c} r={R} fill="none" stroke="var(--text2)" /><line x1={c - R} y1={c} x2={c + R} y2={c} stroke="var(--line)" /><line x1={c} y1={c - R} x2={c} y2={c + R} stroke="var(--line)" />
    <For each={p.poles}>{q => { const big = q.hkl.every(v => Math.abs(v) <= 1); return <><circle cx={c + q.x * R} cy={c - q.y * R} r={big ? 3.2 : 2.2} fill={big ? "var(--blue)" : "var(--text2)"} />
      <Show when={p.label && big}><text x={c + q.x * R + 4} y={c - q.y * R - 3} font-size="8.5" fill="var(--text)">{mi(q.hkl, "", "")}</text></Show></>; }}</For>
  </svg>;
};

export const CalcPage = () => {
  const [info] = createResource(() => call<{ kinds: [string, string, number, string][]; tubes: [string, number][] }>("crystal_info"));
  const [res, setRes] = createSignal<Answer | string | null>(null);
  createEffect(() => { const q = ask(); invoke<Answer>("crystal", { ask: q }).then(setRes, e => setRes(String(e))); });
  const a = () => { const r = res(); return r && typeof r !== "string" ? r : null; };
  const kind = () => info()?.kinds.find(k => k[0] === ask().kind);
  const fromEl = () => !!ask().element;
  const f = (v: number | null | undefined, d = 4) => v == null ? "–" : num(v, d);
  return (
    <div class="page calcpage">
      <h1>Crystal calculators <Help k="crystal" /></h1>
      <div class="calcgrid">
        <section class="panel">
          <h2>Structure <Help k="structure" /></h2>
          <div class="cform">
            <label>From <select class="csrc" value={fromEl() ? "element" : ask().kind} onChange={e => { const v = e.currentTarget.value; upd(q => { if (v === "element") q.element = "Fe"; else { const s = a()?.structure; if (fromEl() && s) { q.cell = s.cell; q.species = [s.atoms[0][0], q.species[1] ?? "O"]; } q.element = null; q.kind = v; } }); }}>
              <option value="element">An element's own structure</option><For each={info()?.kinds}>{k => <option value={k[0]}>{k[1]}</option>}</For></select></label>
            <Show when={fromEl()} fallback={<>
              <p class="muted small">{kind()?.[3]}</p>
              <label>a (Å) <Num cls="ca" v={ask().cell.a} set={v => upd(q => { q.cell.a = v; if (q.kind !== "cell") { q.cell.b = v; if (q.kind !== "hcp") q.cell.c = v; } })} /></label>
              <Show when={ask().kind === "hcp" || ask().kind === "cell"}><label>c (Å) <Num v={ask().cell.c} set={v => upd(q => { q.cell.c = v; })} /></label></Show>
              <Show when={ask().kind === "cell"}><label>b (Å) <Num v={ask().cell.b} set={v => upd(q => { q.cell.b = v; })} /></label>
                <label>α, β, γ (°) <span class="idx"><Num v={ask().cell.alpha} step={0.1} set={v => upd(q => { q.cell.alpha = v; })} /><Num v={ask().cell.beta} step={0.1} set={v => upd(q => { q.cell.beta = v; })} /><Num v={ask().cell.gamma} step={0.1} set={v => upd(q => { q.cell.gamma = v; })} /></span></label></Show>
              <label>Element{(kind()?.[2] ?? 1) > 1 ? "s" : ""} <span class="idx"><input class="sp1" value={ask().species[0]} onChange={e => upd(q => { q.species[0] = e.currentTarget.value.trim(); })} />
                <Show when={(kind()?.[2] ?? 1) > 1}><input class="sp2" value={ask().species[1] ?? ""} onChange={e => upd(q => { q.species[1] = e.currentTarget.value.trim(); })} /></Show></span></label>
            </>}>
              <label>Element <input class="cel" value={ask().element ?? ""} onChange={e => upd(q => { q.element = e.currentTarget.value.trim() || "Fe"; })} /></label>
              <p class="muted small">BCC, FCC, HCP, diamond or simple cubic elements with known lattice parameters (from Crystal › Elements).</p>
            </Show>
          </div>
          <Show when={typeof res() === "string"}><p class="err">{res() as string}</p></Show>
          <Show when={a()}>{x => <div class="cellwrap"><CellView a={x()} />
            <table class="ptable cfacts"><tbody>
              <tr><td>Structure</td><td>{x().structure.name}</td></tr><tr><td>Crystal system</td><td>{x().structure.system}</td></tr>
              <tr><td>Cell</td><td>a = {f(x().structure.cell.a)} Å{x().structure.system !== "cubic" ? `, b = ${f(x().structure.cell.b)}, c = ${f(x().structure.cell.c)} Å` : ""}{x().structure.system !== "cubic" && x().structure.system !== "tetragonal" && x().structure.system !== "orthorhombic" ? `; α, β, γ = ${x().structure.cell.alpha}°, ${x().structure.cell.beta}°, ${x().structure.cell.gamma}°` : ""}</td></tr>
              <tr><td>Atoms per cell</td><td>{x().atomsPerCell}</td></tr><tr><td>Cell volume</td><td>{f(x().volume)} Å³</td></tr>
              <tr><td>Theoretical density</td><td class="cdens">{num(x().density, 4)} kg/m³</td></tr></tbody></table></div>}</Show>
        </section>
        <section class="panel">
          <h2>Planes and directions <Help k="miller" /></h2>
          <div class="cform">
            <label>Plane (hkl) <Idx cls="h1" v={ask().hkl} set={v => upd(q => { q.hkl = v; })} /></label>
            <Show when={a()}>{x => <p class="cres">d{mi(ask().hkl)} = <b>{f(x().d, 5)} Å</b>; Bragg angle 2θ = <b>{x().bragg == null ? "beyond reach (d < λ/2)" : f(x().bragg, 5) + "°"}</b>{x().plane4 ? `; four-index ${mi(x().plane4!)}` : ""}</p>}</Show>
            <label>and plane <Idx cls="h2" v={ask().hkl2} set={v => upd(q => { q.hkl2 = v; })} /></label>
            <Show when={a()}>{x => <p class="cres">Angle between {mi(ask().hkl)} and {mi(ask().hkl2)}: <b>{f(x().anglePlanes, 5)}°</b></p>}</Show>
            <label>Direction [uvw] <Idx cls="u1" v={ask().uvw} set={v => upd(q => { q.uvw = v; })} /></label>
            <label>and direction <Idx cls="u2" v={ask().uvw2} set={v => upd(q => { q.uvw2 = v; })} /></label>
            <Show when={a()}>{x => <p class="cres">Angle between {mi(ask().uvw, "[", "]")} and {mi(ask().uvw2, "[", "]")}: <b>{f(x().angleDirs, 5)}°</b>{x().dir4 ? `; ${mi(ask().uvw, "[", "]")} is ${mi(x().dir4!, "[", "]")} in four indices` : ""}</p>}</Show>
          </div>
          <h2>Stereographic projection <Help k="stereo" /></h2>
          <div class="cform"><label>Centre (hkl) <Idx cls="cc" v={ask().centre} set={v => upd(q => { q.centre = v; })} /></label>
            <label>Indices up to <select value={ask().poles} onChange={e => upd(q => { q.poles = +e.currentTarget.value; })}><option value="1">1</option><option value="2">2</option><option value="3">3</option></select></label></div>
          <Show when={a()}>{x => <StereoView poles={x().poles} centre={ask().centre} label={true} />}</Show>
        </section>
        <section class="panel wide">
          <h2>X-ray powder pattern <Help k="xrd" /></h2>
          <div class="cform crow">
            <label>Tube <select class="tube" value={info()?.tubes.find(t => t[1] === ask().lambda)?.[0] ?? ""} onChange={e => { const t = info()?.tubes.find(x => x[0] === e.currentTarget.value); if (t) upd(q => { q.lambda = t[1]; }); }}>
              <For each={info()?.tubes}>{t => <option value={t[0]}>{t[0]}</option>}</For><option value="">Other</option></select></label>
            <label>λ (Å) <Num cls="lam" v={ask().lambda} step={0.0001} set={v => upd(q => { q.lambda = v; })} /></label>
            <label>2θ up to (°) <Num v={ask().max} step={5} set={v => upd(q => { q.max = v; })} /></label>
          </div>
          <Show when={a()}>{x => <>
            <PeaksChart ps={x().peaks} max={Math.min(175, Math.max(5, ask().max))} four={x().structure.system === "hexagonal"} />
            <div class="hscroll"><table class="ptable peaks"><thead><tr><th>hkl</th><th>Multiplicity</th><th>d (Å)</th><th>2θ (°)</th><th>Intensity</th></tr></thead>
              <tbody><For each={x().peaks}>{q => <tr><td>{q.hkil ? mi(q.hkil) : mi(q.hkl)}</td><td>{q.mult}</td><td>{q.d.toFixed(4)}</td><td>{q.twoTheta.toFixed(3)}</td><td>{q.intensity.toFixed(1)}</td></tr>}</For></tbody></table></div>
            <p class="muted small">Positions from Bragg's law are exact for the cell. Intensities are approximate: each atom scatters in proportion to its atomic number, without the fall-off with angle, temperature or texture, so use them to see which lines are there and roughly how strong.</p></>}</Show>
        </section>
      </div>
    </div>
  );
};
