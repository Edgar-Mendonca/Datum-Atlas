// Crystal module › Elements: a clickable periodic table of the 118 elements (crates/core elements.rs),
// coloured by category or by a property, an element card with its sources, the library's materials
// built on the element, and the composition of a formula or an alloy (molar mass, wt % and at %).
import { For, Show, createResource, createSignal } from "solid-js";
import { call, db, openSheet, srcOf } from "./store";
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
