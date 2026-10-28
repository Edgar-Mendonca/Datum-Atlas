// The selection report: the requirements, the goal and its index, the stages, every chart, the
// ranked shortlist, the comparison and a datasheet of each candidate with its sources, as one
// document. The Report page shows it; File › Print prints it (or saves it as PDF).
import { For, Show } from "solid-js";
import { CMP_COLORS, fmt, goalId, lineOf } from "./api";
import { out, P, act, db, mat, propOf, vtext, unitOf, srcOf, goals, setBackstage, toShown } from "./store";
import { ChartSvg } from "./chart";
import { Radar } from "./compare";
import { Check } from "./ui";
import { Help } from "./help";

type Parts = { charts: boolean; shortlist: boolean; compare: boolean; sheets: boolean; n: number };
const parts = (): Parts => ({ charts: true, shortlist: true, compare: true, sheets: true, n: 3, ...((P().ui?.report ?? {}) as Partial<Parts>) });
const setPart = (k: keyof Parts, v: boolean | number) => act({ Ui: { key: "report", value: { ...parts(), [k]: v } } }, true);
const KEYS = ["rho", "price", "E", "sy", "ts", "kic", "elong", "tmax", "lam", "alpha", "co2", "ee", "recyclable"];

/** The candidates whose datasheets close the report: those compared, else the best of the results. */
const candidates = () => { const c = P().compare; return (c.length ? c : out()!.results.rows.filter(r => r.pass).map(r => r.id)).slice(0, parts().n); };

export const ReportDoc = (p: { print?: boolean }) => {
  const r = () => out()!.results, pr = () => P();
  const goal = () => goals().find(g => g.id === goalId(pr().charts.find(c => c.id === r().rankChart) ?? { goal: "" } as any));
  const cited = () => { const ids = new Set<string>(); for (const id of candidates()) for (const v of Object.values(mat(id)?.props ?? {})) ids.add(String(v[v.length === 4 ? 2 : v.length - 1])); return [...ids].map(srcOf).filter(Boolean); };
  return (
    <div class="report" classList={{ print: p.print }}>
      <h1>Material selection: {pr().name}</h1>
      <p class="muted">Datum Atlas · {new Date().toLocaleDateString(undefined, { day: "numeric", month: "long", year: "numeric" })} · {db().materials.length} materials considered</p>
      <h2>1 · Requirements</h2>
      <Show when={pr().notes} fallback={<p class="muted">No requirements written (Setup).</p>}><p class="pre">{pr().notes}</p></Show>
      <Show when={goal()}>{g => <div class="rgoal"><p><b>Design goal: {g().name}.</b> {g().above || g().slope ? "Maximise" : "Minimise"} <span class="idx">{g().index}</span>.</p>
        <table class="ptable"><tbody><tr><td>Function</td><td>{g().function}</td></tr><tr><td>Constraint</td><td>{g().constraint}</td></tr><tr><td>Objective</td><td>{g().objective}</td></tr><tr><td>Free variable</td><td>{g().free}</td></tr></tbody></table>
        <p>{g().why}</p></div>}</Show>
      <h2>2 · Selection stages</h2>
      <p>{r().stages.length ? `${r().pass} of ${r().total} materials pass every stage.` : "No stages: every material passes."}</p>
      <ul>
        <For each={pr().stages.limits}>{l => { const q = propOf(l.prop); return <li>Limit: {q?.name}{l.min !== null ? ` ≥ ${fmt(toShown(l.prop, l.min))}` : ""}{l.max !== null ? ` ≤ ${fmt(toShown(l.prop, l.max))}` : ""} {l.min !== null || l.max !== null ? unitOf(l.prop) : ""}{l.flag !== null ? `: ${l.flag ? "yes" : "no"}` : ""}{l.rating ? `: ${l.rating} or better` : ""}{l.goal ? " (suggested by the goal)" : ""}</li>; }}</For>
        <For each={pr().charts.filter(c => c.sels.length)}>{c => <li>Chart “{c.name}”: {c.sels.map((s, k) => { const l = lineOf(s), v = r().lines.find(x => x.chart === c.id && x.k === k); return l ? `${v?.name ?? "line"} ${l.above ? "≥" : "≤"} ${v ? fmt(v.value) : ""}` : "Box" in s ? "inside a box" : "inside a drawn shape"; }).join("; ")} — {r().stages.find(s => s.chart === c.id)?.pass} pass</li>}</For>
        <Show when={pr().stages.exclude.length}><li>Left out: {pr().stages.exclude.join(", ")}</li></Show>
        <li>Ranked by {r().indexName || "name (no index line)"}.</li>
      </ul>
      <Show when={parts().charts && pr().charts.length}><h2>3 · Property charts</h2>
        <For each={pr().charts}>{c => { const o = () => out()!.charts.find(x => x.id === c.id); return <Show when={o()}><figure class="rfig">
          <ChartSvg c={c} o={o()!} w={700} h={470} win={[o()!.x.lim[0], o()!.x.lim[1], o()!.y.lim[0], o()!.y.lim[1]]} print legend />
          <figcaption><b>{c.name}.</b> Each ellipse spans a material's range of both properties; envelopes enclose each {c.hull === "class" ? "class" : "family"}; greyed materials fail the selection.</figcaption></figure></Show>; }}</For></Show>
      <Show when={parts().shortlist}><h2>4 · Shortlist</h2>
        <table class="ptable"><thead><tr><th>Rank</th><th>Material</th><th>Family</th><Show when={r().indexName}><th>{r().indexName}</th><th>× best</th></Show></tr></thead>
          <tbody><For each={r().rows.filter(x => x.pass).slice(0, 15)}>{x => <tr><td>{x.rank}</td><td>{x.name}</td><td>{x.family}</td><Show when={r().indexName}><td class="r">{fmt(x.index)}</td><td class="r">{x.ratio !== null ? fmt(x.ratio) : ""}</td></Show></tr>}</For></tbody></table>
        <Show when={r().pass > 15}><p class="muted">… and {r().pass - 15} more pass.</p></Show></Show>
      <Show when={parts().compare && out()!.compare.ids.length}><h2>5 · Comparison</h2>
        <div class="rcmp"><Radar size={360} print /><table class="ptable"><thead><tr><th>Property</th><For each={out()!.compare.ids}>{(id, m) => <th><i class="dot" style={{ background: CMP_COLORS[m()] }} />{mat(id)?.name}</th>}</For></tr></thead>
          <tbody><For each={out()!.compare.rows.filter(x => KEYS.includes(x.key))}>{row => <tr><td>{propOf(row.key)?.name} <span class="muted">{unitOf(row.key)}</span></td>
            <For each={out()!.compare.ids}>{(id, m) => <td classList={{ hi: row.hi === m(), lo: row.lo === m() }}>{vtext(row.key, mat(id)?.props[row.key])}</td>}</For></tr>}</For></tbody></table></div></Show>
      <Show when={parts().sheets && candidates().length}><h2>6 · Datasheets of the candidates</h2>
        <For each={candidates()}>{id => { const m = () => mat(id)!; return <Show when={mat(id)}><div class="rsheet"><h3>{m().name}</h3><p class="muted">{m().family.join(" › ")}{Object.keys(m().des).length ? ` · ${Object.entries(m().des).map(([k, v]) => `${k} ${v}`).join(", ")}` : ""}</p>
          <table class="ptable"><tbody><For each={KEYS.filter(k => m().props[k])}>{k => { const v = m().props[k]; return <tr><td>{propOf(k)?.name}</td><td>{vtext(k, v)} {unitOf(k)}</td><td class="muted small">{srcOf(String(v[v.length === 4 ? 2 : v.length - 1]))?.title}{m().checked?.includes(k) ? " ✓" : ""}</td></tr>; }}</For></tbody></table>
          <Show when={m().uses}><p class="small">Uses: {m().uses}</p></Show></div></Show>; }}</For></Show>
      <h2>Sources</h2>
      <ul class="small"><For each={cited()}>{s => <li>{s!.title}{s!.publisher ? `, ${s!.publisher}` : ""}{s!.year ? `, ${s!.year}` : ""}{s!.url ? ` · ${s!.url}` : ""}</li>}</For></ul>
      <p class="muted small">Values are generic ranges compiled from open and public sources; ✓ marks a value checked against a published datasheet, the others are drafts. Check the supplier's data for the grade and condition you will use before you decide.</p>
    </div>
  );
};

export const ReportPage = () => (
  <div class="page reportpage">
    <div class="ptools"><h1>Selection report <Help k="report" /></h1><span class="grow" />
      <Check label="Charts" value={parts().charts} set={v => setPart("charts", v)} /><Check label="Shortlist" value={parts().shortlist} set={v => setPart("shortlist", v)} />
      <Check label="Comparison" value={parts().compare} set={v => setPart("compare", v)} /><Check label="Datasheets" value={parts().sheets} set={v => setPart("sheets", v)} />
      <label class="inline">of <input type="number" min="1" max="6" style={{ width: "46px" }} value={parts().n} onChange={e => setPart("n", Math.max(1, Math.min(6, +e.currentTarget.value || 3)))} /> candidates</label>
      <button class="btn pri" onClick={() => setBackstage("print")}>Print / Save as PDF</button></div>
    <div class="rpaper"><ReportDoc /></div>
  </div>
);
