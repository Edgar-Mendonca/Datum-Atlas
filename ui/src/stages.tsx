// Selection stages (limits, the chart selections, families left out), the stage panel beside the
// chart, and the Results page: what passes, ranked by the index, and why the others fail.
import { For, Show, createSignal } from "solid-js";
import { Limit, Stages, Material, Weight, fmt, SEP, famColor, lineOf, goalId } from "./api";
import { act, out, P, db, cur, setCur, openSheet, show, setChartId, mat, propOf, srcOf, toShown, toStored, unitOf, vtext, goals, chartProps, formulaProps } from "./store";
import { Help } from "./help";
import { Icon, Check } from "./ui";

const setStages = (f: (s: Stages) => void) => { const s = structuredClone(P().stages); f(s); return act({ SetStages: { stages: s } }); };
const num = (v: string) => (v.trim() === "" || isNaN(+v) ? null : +v);
/** A typed limit in the units shown → stored units, and back (rounded for display). */
const inStored = (k: string, v: string) => { const x = num(v); return x === null ? null : toStored(k, x); };
const shownOf = (k: string, v: number | null) => (v === null ? "" : String(+toShown(k, v).toPrecision(6)));

/** One limit: property, then min/max with the unit, or yes/no, or the lowest rating. */
const LimitRow = (p: { l: Limit; k: number }) => {
  const pr = () => propOf(p.l.prop);
  const put = (patch: Partial<Limit>) => setStages(s => { s.limits[p.k] = { ...s.limits[p.k], ...patch }; });
  return (
    <div class="limit">
      <select value={p.l.prop} onChange={e => put({ prop: e.currentTarget.value, min: null, max: null, flag: null, rating: null, goal: false })}>
        <For each={[...db().props.filter(x => db().materials.some(m => m.props[x.key])), ...formulaProps()]}>{x => <option value={x.key} selected={x.key === p.l.prop}>{x.name}</option>}</For></select>
      <Show when={pr()?.kind === "range"}>
        <span class="inline"><input type="number" placeholder="min" value={shownOf(p.l.prop, p.l.min)} onChange={e => put({ min: inStored(p.l.prop, e.currentTarget.value) })} /><span class="muted">to</span>
          <input type="number" placeholder="max" value={shownOf(p.l.prop, p.l.max)} onChange={e => put({ max: inStored(p.l.prop, e.currentTarget.value) })} /><span class="muted unit">{unitOf(p.l.prop)}</span></span>
      </Show>
      <Show when={pr()?.kind === "flag"}><select value={p.l.flag === null ? "" : String(p.l.flag)} onChange={e => put({ flag: e.currentTarget.value === "" ? null : e.currentTarget.value === "true" })}><option value="">Either</option><option value="true">Must be yes</option><option value="false">Must be no</option></select></Show>
      <Show when={pr()?.kind === "rating"}><select value={p.l.rating ?? ""} onChange={e => put({ rating: e.currentTarget.value || null })}><option value="">Any</option><For each={["Poor", "Acceptable", "Good", "Excellent"]}>{r => <option value={r}>{r} or better</option>}</For></select></Show>
      <Show when={p.l.goal}><span class="chip ok" title="Suggested by the design goal; a new goal replaces it">from the goal</span></Show>
      <button class="x" title="Remove this limit" onClick={() => setStages(s => { s.limits.splice(p.k, 1); })}>×</button>
    </div>
  );
};

export const addLimit = (prop = "tmax") => { setStages(s => { s.limits.push({ prop, min: null, max: null, flag: null, rating: null, goal: false }); }); show("stages"); };

/** Select › Stages: every stage of the selection in one page. */
export const StagesPage = () => {
  const tops = () => db().tree.filter(n => n.depth <= 1);
  const out_ = (path: string) => P().stages.exclude.some(e => path === e || path.startsWith(e + SEP));
  const toggle = (path: string) => setStages(s => { const on = s.exclude.includes(path); s.exclude = on ? s.exclude.filter(e => e !== path) : [...s.exclude.filter(e => !e.startsWith(path + SEP)), path]; });
  const r = () => out()!.results;
  return (
    <div class="page stagespage">
      <h1>Selection stages <Help k="stages" /></h1>
      <p class="muted">A material passes when it passes every stage. {r().pass} of {r().total} materials pass now. Change a stage and the charts and the results follow.</p>
      <section class="panel">
        <h2><Icon n="filter" /> Limits <Help k="limits" /> <small class="muted">{P().stages.limits.length ? `${r().stages.find(s => s.kind === "limit")?.pass ?? 0} pass` : "none yet"}</small></h2>
        <p class="muted">The values the part needs: minimum and maximum values, yes/no properties and ratings.</p>
        <For each={P().stages.limits}>{(l, k) => <LimitRow l={l} k={k()} />}</For>
        <p class="inline"><button class="btn sm pri" onClick={() => addLimit()}>Add a limit</button>
          <span class="muted">A range passes on</span><select value={P().stages.rule} onChange={e => { const v = e.currentTarget.value; setStages(s => { s.rule = v; }); }}><option value="mean">its typical (mean) value</option><option value="all">its whole range</option></select>
          <span class="muted">Materials without the value</span><select value={P().stages.missing} onChange={e => { const v = e.currentTarget.value; setStages(s => { s.missing = v; }); }}><option value="fail">fail</option><option value="flag">pass, marked “no data”</option></select></p>
      </section>
      <section class="panel">
        <h2><Icon n="index" /> Chart selections</h2>
        <p class="muted">Lines, boxes and shapes drawn on the charts. A material passes a chart when it passes every selection on it: two lines facing each other keep a band.</p>
        <For each={P().charts} fallback={<p class="muted">No charts.</p>}>{c => <div class="limit">
          <b>{c.name}</b><span class="muted">{c.sels.length ? c.sels.map(s => { const l = lineOf(s); return l ? `line of slope ${fmt(l.slope)}, ${l.above ? "above" : "below"} passes` : "Box" in s ? "box" : "shape"; }).join("; ") : "no selection"}</span>
          <Show when={c.sels.length}><span class="muted small">{r().stages.find(s => s.chart === c.id)?.pass} pass</span></Show>
          <button class="btn sm" onClick={() => { setChartId(c.id); show("charts"); }}>Open the chart</button></div>}</For>
      </section>
      <Ranking />
      <section class="panel">
        <h2><Icon n="tree" /> Families and classes <Help k="families" /></h2>
        <p class="muted">Untick what cannot be used, for example because of how the part is made.</p>
        <div class="treestage"><For each={tops()}>{n => <label class="chk" classList={{ sub: n.depth > 0 }}><input type="checkbox" checked={!out_(n.path)} onChange={() => toggle(n.path)} />{n.name} <small class="muted">{n.count}</small></label>}</For></div>
      </section>
      <p><button class="btn pri" onClick={() => show("results")}>See the results</button></p>
    </div>
  );
};

/** How the materials that pass are ordered: by a selection line's index, or by weighted properties. */
const Ranking = () => {
  const st = () => P().stages, r = () => out()!.results, weighted = () => st().ranking === "weighted";
  const props = chartProps;
  const total = () => st().weights.reduce((a, w) => a + w.w, 0) || 1;
  const setW = (k: number, patch: Partial<Weight>) => setStages(s => { s.weights[k] = { ...s.weights[k], ...patch }; });
  return (
    <section class="panel">
      <h2><Icon n="results" /> Ranking <Help k="ranking" /></h2>
      <p class="muted">How the materials that pass are ordered in Results.</p>
      <p class="inline"><label class="chk"><input type="radio" name="rank" checked={!weighted()} onChange={() => setStages(s => { s.ranking = "line"; })} />By the index of a selection line</label>
        <select disabled={weighted() || !r().lines.length} value={`${r().rankChart}:${r().rankLine}`} onChange={e => { const [c, k] = e.currentTarget.value.split(":").map(Number); setStages(s => { s.rankChart = c; s.rankLine = k; s.ranking = "line"; }); }}>
          <For each={r().lines} fallback={<option value="0:0">No selection line yet</option>}>{l => <option value={`${l.chart}:${l.k}`}>{l.name} on “{P().charts.find(c => c.id === l.chart)?.name}”</option>}</For></select></p>
      <p class="inline"><label class="chk"><input type="radio" name="rank" checked={weighted()} onChange={() => setStages(s => { s.ranking = "weighted"; if (!s.weights.length) s.weights = [{ prop: "rho", w: 1, high: false }, { prop: "price", w: 1, high: false }]; })} />By weighted properties</label>
        <span class="muted">each property scaled so the best material that passes scores 1, then averaged by weight</span></p>
      <Show when={weighted()}>
        <For each={st().weights}>{(w, k) => <div class="limit">
          <select value={w.prop} onChange={e => { const v = e.currentTarget.value; setW(k(), { prop: v }); }}><For each={props()}>{x => <option value={x.key} selected={x.key === w.prop}>{x.name}</option>}</For></select>
          <select value={w.high ? "high" : "low"} onChange={e => { const v = e.currentTarget.value === "high"; setW(k(), { high: v }); }}><option value="low">lower is better</option><option value="high">higher is better</option></select>
          <label class="inline">weight <input type="number" min="0" step="1" style={{ width: "60px" }} value={w.w} onChange={e => { const v = +e.currentTarget.value; if (isFinite(v) && v >= 0) setW(k(), { w: v }); }} /></label>
          <span class="muted">{fmt((100 * w.w) / total())} %</span>
          <button class="btn sm" onClick={() => setStages(s => { s.weights.splice(k(), 1); })}>Remove</button></div>}</For>
        <p><button class="btn sm" onClick={() => setStages(s => { s.weights.push({ prop: "E", w: 1, high: true }); })}>Add a property</button></p>
      </Show>
    </section>
  );
};

/** The material picked on the chart, as a small datasheet. */
const MaterialCard = (p: { m: Material }) => {
  const row = () => out()!.results.rows.find(r => r.id === p.m.id);
  const keys = ["rho", "price", "E", "sy", "ts", "kic", "tmax", "lam", "alpha", "co2"];
  return (
    <div class="mcard">
      <h3>{p.m.name}</h3><div class="muted">{p.m.family.join(" › ")}</div>
      <Show when={row() && out()!.results.stages.length}><div class="pill" classList={{ ok: row()!.pass, err: !row()!.pass }}>{row()!.pass ? `Passes${row()!.rank ? ` · rank ${row()!.rank}` : ""}` : "Fails"}</div>
        <Show when={!row()!.pass}><ul class="why"><For each={row()!.reasons}>{x => <li>{x}</li>}</For></ul></Show></Show>
      <table class="tbl mini"><tbody><For each={keys.filter(k => p.m.props[k])}>{k => <tr><td>{propOf(k)?.name}</td><td class="r">{vtext(k, p.m.props[k])}</td><td class="muted">{unitOf(k)}</td></tr>}</For></tbody></table>
      <p class="muted small">{p.m.uses}</p>
      <button class="btn sm pri" onClick={() => openSheet(p.m.id)}>Full datasheet</button> <button class="btn sm" onClick={() => setCur("")}>Close</button>
    </div>
  );
};

/** Beside the chart: the picked material, the pass count, each stage, and the top of the ranking. */
export const StagePanel = () => {
  const r = () => out()!.results;
  return (
    <aside class="spanel">
      <Show when={cur() && mat(cur())}><MaterialCard m={mat(cur())!} /></Show>
      <Show when={goals().find(g => g.id === goalId(P().charts.find(c => c.id === r().rankChart) ?? { goal: "" } as any))}>{g => <div class="gcard"><small>Goal</small><b>{g().name}</b><span>{g().above || g().slope ? "Maximise" : "Minimise"} <span class="idx">{g().index}</span></span></div>}</Show>
      <div class="pcount"><b>{r().pass}</b><span>of {r().total} materials pass</span><Help k="stages" /></div>
      <For each={r().stages} fallback={<p class="muted">No stages yet. Draw a selection line on the chart (Line tool), add a limit, or leave families out.</p>}>{s =>
        <button class="scard" onClick={() => (s.kind === "chart" ? (setChartId(s.chart), show("charts")) : show("stages"))}><Icon n={s.kind === "limit" ? "filter" : s.kind === "chart" ? "index" : "tree"} /><span>{s.name}</span><b>{s.pass}</b></button>}</For>
      <p class="inline"><button class="btn sm" onClick={() => addLimit()}>Add a limit</button><button class="btn sm" onClick={() => show("stages")}>All stages</button></p>
      <Show when={r().stages.length}>
        <h4>Best {r().indexName ? <>by <span class="idx">{r().indexName}</span></> : "materials"}</h4>
        <table class="tbl mini rank"><tbody><For each={r().rows.filter(x => x.pass).slice(0, 12)}>{x => <tr class="link" classList={{ sel: x.id === cur() }} onClick={() => setCur(x.id)} onDblClick={() => openSheet(x.id)}>
          <td class="muted">{x.rank}</td><td><i class="dot" style={{ background: famColor(Math.max(0, ["Metals", "Polymers", "Elastomers", "Ceramics and glasses", "Composites", "Foams", "Natural materials"].indexOf(x.family))) }} />{x.name}</td><td class="r">{x.ratio !== null ? fmt(x.ratio) : ""}</td></tr>}</For></tbody></table>
        <button class="btn sm" onClick={() => show("results")}>All results</button>
      </Show>
    </aside>
  );
};

/** Select › Results: the ranked shortlist and the reasons for every failure. */
export const ResultsPage = () => {
  const [all, setAll] = createSignal(false);
  const r = () => out()!.results;
  const rows = () => r().rows.filter(x => all() || x.pass);
  return (
    <div class="page resultspage">
      <h1>Results <Help k="results" /></h1>
      <p class="muted">{r().stages.length ? `${r().pass} of ${r().total} materials pass ${r().stages.length} ${r().stages.length === 1 ? "stage" : "stages"}.` : "No stages yet: every material passes."}
        {r().indexName ? ` Ranked by the performance index ${r().indexName} (higher is better); “× best” compares each with the best one.` : " Draw a selection line on a chart to rank by its index."}</p>
      <div class="ptools"><Check label="Show the materials that fail, with the reasons" value={all()} set={setAll} /><span class="grow" /><button class="btn sm" onClick={() => show("stages")}>Edit the stages</button></div>
      <table class="tbl results"><thead><tr><th>Rank</th><th>Material</th><th>Family</th><Show when={r().indexName}><th class="r"><span class="idx">{r().indexName}</span></th><th class="r">× best</th></Show><th>Result</th></tr></thead>
        <tbody><For each={rows()}>{x => <tr class="link" classList={{ fail: !x.pass, sel: x.id === cur() }} onClick={() => setCur(x.id)} onDblClick={() => openSheet(x.id)}>
          <td class="muted">{x.rank ?? ""}</td><td><b>{x.name}</b></td><td class="muted">{x.family}</td>
          <Show when={r().indexName}><td class="r">{fmt(x.index)}</td><td class="r">{x.ratio !== null ? fmt(x.ratio) : ""}</td></Show>
          <td>{x.pass ? <span class="ok">Passes{x.reasons.length ? ` (${x.reasons.join("; ")})` : ""}</span> : <span class="late">Fails: {x.reasons.join("; ")}</span>}</td></tr>}</For></tbody></table>
      <Show when={!rows().length}><p class="muted">Nothing passes. Relax a limit, move the selection line, or bring a family back.</p></Show>
      <p class="muted small">Values are generic ranges; check the datasheet and its references before you decide. {srcOf("atlas-typ") ? "Built-in values are drafts until checked value by value." : ""}</p>
    </div>
  );
};
