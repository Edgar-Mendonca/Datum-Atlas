// Pages of the sidebar besides Browse, the chart and the stages: Setup (the design problem), Data
// (the databases and their sources), and the laid-out pages of features still to come.
import { For, JSX, Show, createResource, createSignal } from "solid-js";
import { P, act, db, show, loadOut, call, setNode, openSheet, goals, setGoal, chart, openDialog, propOf, unitOf, toShown } from "./store";
import { Icon, Dialog } from "./ui";
import { Help } from "./help";
import { fmt } from "./api";
import { soon, Chip } from "./roadmap";
import { editMaterial, deleteMaterial } from "./material";
import { addLimit } from "./stages";
import { C } from "./cmds";

type Tpl = { id: string; name: string; desc: string; slope: number | null };

/** A tiny property chart for template tiles: three family bubbles and, when there is one, the index line. */
export const MiniChart = (p: { slope: number | null }) => (
  <svg viewBox="0 0 100 60" class="mini">
    <path d="M8 4v50h88" fill="none" stroke="#6C4AB6" stroke-width="2" />
    <ellipse cx="34" cy="38" rx="14" ry="7" transform="rotate(-30 34 38)" fill="#0072B2" fill-opacity=".5" />
    <ellipse cx="58" cy="24" rx="15" ry="7" transform="rotate(-30 58 24)" fill="#E69F00" fill-opacity=".5" />
    <ellipse cx="72" cy="40" rx="10" ry="6" transform="rotate(-30 72 40)" fill="#009E73" fill-opacity=".5" />
    {p.slope !== null && <path d={`M14 ${50 - 0} L92 ${Math.max(4, 50 - 78 * 0.22 * p.slope)}`} stroke="#6C4AB6" stroke-width="1.8" stroke-dasharray="3 3" />}
    <path d="M86 6l4 4-4 4-4-4z" fill="#F2B233" />
  </svg>
);

export const SetupPage = () => {
  const [tpls] = createResource(() => call<Tpl[]>("templates"));
  return (
    <div class="page setuppage"><div class="options">
      <h1>Setup</h1>
      <p class="muted">What the part must do. Selecting a material by the method takes four steps: write down the requirements, plot the materials on property charts, select with limits and index lines, then rank and check the best candidates.</p>
      <section class="osec">
        <h3>The selection</h3>
        <div class="opt"><div><b>Name</b><small>Shown in the title bar and printed headers.</small></div><div class="ctl"><input value={P().name} onChange={e => act({ Name: { name: e.currentTarget.value } })} /></div></div>
        <div class="opt"><div><b>Requirements</b><small>Function (what it does), constraints (what it must meet), objective (what to minimise or maximise), free variables (what you may change).</small></div>
          <div class="ctl"><textarea rows="7" placeholder={"Function: a beam loaded in bending\nConstraints: length and stiffness fixed; works at 150 °C\nObjective: minimum mass\nFree variable: the size of the cross-section"} value={P().notes} onChange={e => act({ Notes: { text: e.currentTarget.value } })} /></div></div>
      </section>
      <section class="osec">
        <h3>Design goal <Help k="goal" /></h3>
        <p class="muted">Pick what the part must do. The chart, the selection line and the suggested limits follow; you can change them afterwards.</p>
        <div class="goallist"><For each={[...new Set(goals().filter(g => !g.library).map(g => g.group))]}>{gr => <div class="ggroup"><h4>{gr}</h4>
          <For each={goals().filter(g => g.group === gr && !g.library)}>{g => <button class="goal" classList={{ on: chart()?.goal === g.id }} onClick={() => setGoal(g.id).then(() => show("charts"))} title={g.why}>
            <b>{g.name}</b><span class="idx">{g.index}</span><small>{g.function} · {g.objective}</small></button>}</For></div>}</For></div>
        <p class="muted">{goals().filter(g => g.library).length} more indices (pressure vessels, flywheels, columns, heat exchangers, conductors…) are in the <button class="lnk" onClick={() => show("index")}>Index library and builder</button>.</p>
      </section>
      <section class="osec">
        <h3>Next steps</h3>
        <div class="steps">
          <button class="tile" onClick={() => show("charts")}><Icon n="atlas" size={26} /><b>1 · Property chart</b><span>See every material on two properties.</span></button>
          <button class="tile" onClick={() => addLimit()}><Icon n="filter" size={26} /><b>2 · Limits</b><span>Values the part needs, such as max service temperature.</span></button>
          <button class="tile" onClick={() => show("charts")}><Icon n="index" size={26} /><b>3 · Index line</b><span>Draw a line of slope 1, 2 or 3 to select by a performance index.</span></button>
          <button class="tile" onClick={() => show("results")}><Icon n="results" size={26} /><b>4 · Results</b><span>The ranked shortlist, and why the rest fail.</span></button>
        </div>
      </section>
      <section class="osec">
        <h3>Start from a design problem</h3>
        <p class="muted">A new selection with its chart, index line and limits set (your current one stays open until you save or discard it).</p>
        <div class="tiles"><For each={tpls() ?? []}>{t => <button class="tile" onClick={() => loadOut("new_project", { template: t.id })}><div class="pic"><MiniChart slope={t.slope} /></div><b>{t.name}</b><span>{t.desc}</span></button>}</For></div>
      </section>
    </div></div>
  );
};

/** Data: the built-in database, your own materials and every source. */
export const DataPage = () => {
  const mine = () => db().materials.filter(m => m.user);
  return (
    <div class="page datapage">
      <h1>Data</h1>
      <p class="muted">Datum Atlas ships with {db().materials.length - db().userCount} generic materials from every family. Every value is compiled from public sources and points to its reference; no proprietary database is used. Add your own materials beside them.</p>
      <section class="panel">
        <h2><Icon n="material" /> Your materials <small class="muted">{mine().length}</small></h2>
        <p class="muted">Saved on this computer in “My materials.datdb”, with a source for each value. They appear in Browse, the datasheets, every chart and the selection.</p>
        <table class="tbl"><tbody><For each={mine()} fallback={<tr><td class="muted">None yet. Add a material of your own, or copy a built-in one and change its values.</td></tr>}>{m =>
          <tr><td><b class="link" onClick={() => openSheet(m.id)}>{m.name}</b></td><td class="muted">{m.family.join(" › ")}</td><td class="muted">{Object.keys(m.props).length} properties</td>
            <td class="r"><button class="btn sm" onClick={() => editMaterial(m)}>Edit</button> <button class="btn sm" onClick={() => deleteMaterial(m)}>Delete</button></td></tr>}</For></tbody></table>
        <p class="inline"><button class="btn pri" onClick={() => editMaterial()}><Icon n="material" /> Add a material</button><button class="btn" onClick={() => soon("A8")}>Import from a spreadsheet or CSV</button><Chip id="A8" /></p>
      </section>
      <section class="panel">
        <h2><Icon n="db" /> Built-in materials by family</h2>
        <div class="famrow"><For each={db().tree.filter(n => n.depth === 0)}>{n => <button class="btn sm" onClick={() => { setNode(n.path); show("browse"); }}>{n.name} · {n.count}</button>}</For></div>
      </section>
      <p><button class="btn" onClick={() => show("sources")}><Icon n="source" /> Sources of the values</button></p>
    </div>
  );
};

/** A page whose feature comes later: what it will do, when, and what to use now. */
export const Later = (p: { icon: string; title: string; ids: string[]; children: JSX.Element }) => (
  <div class="page emptypage"><div class="emptybox"><Icon n={p.icon} size={42} /><h1>{p.title}</h1>{p.children}
    <div class="later"><For each={p.ids}>{id => <button class="btn" onClick={() => soon(id)}><Chip id={id} /> {id}</button>}</For></div></div></div>
);
/** Puts an index (goal or library entry) on a new chart, so the open one is kept. */
export const plotIndex = async (id: string) => { await C.newChart(); await setGoal(id); show("charts"); };

/** Index: the builder (function → constraint → objective gives the index) and the whole library. */
export const IndexPage = () => {
  const [fn, setFn] = createSignal(""), [con, setCon] = createSignal(""), [obj, setObj] = createSignal(""), [q, setQ] = createSignal("");
  const uniq = (v: string[]) => [...new Set(v)].sort();
  const fns = () => uniq(goals().map(g => g.function));
  const cons = () => uniq(goals().filter(g => !fn() || g.function === fn()).map(g => g.constraint));
  const objs = () => uniq(goals().filter(g => (!fn() || g.function === fn()) && (!con() || g.constraint === con())).map(g => g.objective));
  const found = () => goals().filter(g => (!fn() || g.function === fn()) && (!con() || g.constraint === con()) && (!obj() || g.objective === obj()));
  const lib = () => { const t = q().toLowerCase(); return goals().filter(g => !t || `${g.name} ${g.index} ${g.function} ${g.objective} ${g.constraint}`.toLowerCase().includes(t)); };
  return (
    <div class="page indexpage">
      <h1>Performance indices <Help k="index" /></h1>
      <p class="muted">A performance index says which material property, or mix of properties, makes a part best at its job. It follows from the function of the part, its constraint, the objective and the free variable. On a property chart with log axes an index is a straight line.</p>
      <section class="panel builder">
        <h2><Icon n="builder" /> Index builder</h2>
        <div class="bsteps">
          <label>1 · Function<select value={fn()} onChange={e => { setFn(e.currentTarget.value); setCon(""); setObj(""); }}><option value="">Any</option><For each={fns()}>{x => <option>{x}</option>}</For></select></label>
          <label>2 · Constraint<select value={con()} onChange={e => { setCon(e.currentTarget.value); setObj(""); }}><option value="">Any</option><For each={cons()}>{x => <option>{x}</option>}</For></select></label>
          <label>3 · Objective<select value={obj()} onChange={e => setObj(e.currentTarget.value)}><option value="">Any</option><For each={objs()}>{x => <option>{x}</option>}</For></select></label>
        </div>
        <div class="icards"><For each={found().slice(0, 6)}>{g => <div class="icard">
          <small>{g.function} · {g.constraint} · {g.objective}</small><b>{g.name}</b><span class="bigidx">{g.above || g.slope ? "Maximise" : "Minimise"} <span class="idx">{g.index}</span></span>
          <p class="small">{g.why}</p>
          <p class="inline"><button class="btn sm pri" onClick={() => plotIndex(g.id)}>Plot on a new chart</button><button class="btn sm" onClick={() => setGoal(g.id).then(() => show("charts"))}>Use the open chart</button><button class="btn sm" onClick={() => goalCard(g.id)}>Details</button></p></div>}</For></div>
        <Show when={found().length > 6}><p class="muted">{found().length - 6} more match; narrow the steps above or see the library below.</p></Show>
      </section>
      <section class="panel">
        <h2><Icon n="list" /> Index library <span class="muted small">{goals().length} indices</span></h2>
        <p class="inline"><input placeholder="Search: beam, cost, σy, conductor…" value={q()} onInput={e => setQ(e.currentTarget.value)} /></p>
        <table class="tbl"><thead><tr><th>Index</th><th>For</th><th>Function</th><th>Objective</th><th>Chart</th><th /></tr></thead><tbody>
          <For each={[...new Set(lib().map(g => g.group))]}>{gr => <>
            <tr class="grp"><td colspan="6">{gr}</td></tr>
            <For each={lib().filter(g => g.group === gr)}>{g => <tr class="link" onClick={() => goalCard(g.id)}>
              <td><span class="idx">{g.index}</span></td><td>{g.name}</td><td class="muted">{g.function}</td><td class="muted">{g.objective}</td>
              <td class="muted small">{propOf(g.y)?.name ?? g.y.replace(/^f:/, "")} vs {propOf(g.x)?.name ?? g.x}{g.also ? " + a second chart" : ""}</td>
              <td><button class="btn sm" onClick={e => { e.stopPropagation(); plotIndex(g.id); }}>Plot</button></td></tr>}</For></>}</For>
        </tbody></table>
      </section>
    </div>
  );
};

/** A goal explained: function, constraint, objective, free variable, the index and why. */
export const goalCard = (id: string) => openDialog(() => {
  const g = goals().find(x => x.id === id)!;
  return (
    <Dialog title={g.name} extra={<><button class="btn" onClick={() => plotIndex(g.id)}>Plot on a new chart</button><button class="btn" onClick={() => setGoal(g.id)}>Apply to the open chart</button></>}>
      <div class="helpcard">
        <p class="bigidx">{g.above || g.slope ? "Maximise" : "Minimise"} <b>{g.index}</b></p>
        <table class="tbl mini"><tbody>
          <tr><td>Function</td><td>{g.function}</td></tr><tr><td>Constraint</td><td>{g.constraint}</td></tr>
          <tr><td>Objective</td><td>{g.objective}</td></tr><tr><td>Free variable</td><td>{g.free}</td></tr>
          <tr><td>Chart</td><td>{propOf(g.y)?.name} against {propOf(g.x)?.name}, {g.slope ? `line of slope ${fmt(g.slope)}` : "horizontal line"}; {g.above ? "above" : "below"} passes</td></tr>
          <Show when={g.formulas.length}><tr><td>Formula properties</td><td>{g.formulas.map(([n, e, u]) => `${n} = ${e}${u ? ` (${u})` : ""}`).join("; ")}; added to the selection when you plot it</td></tr></Show>
          <Show when={g.also}>{a => <tr><td>Second chart</td><td>{propOf(a().y)?.name} against {propOf(a().x)?.name}, line of slope {fmt(a().slope)} for {a().index}; the goal adds this chart, and the shortlist passes both</td></tr>}</Show>
          <Show when={g.limits.length}><tr><td>Suggested limits</td><td>{g.limits.map(([k, a, b]) => `${propOf(k)?.name}${a !== null ? ` ≥ ${fmt(toShown(k, a))}` : ""}${b !== null ? ` ≤ ${fmt(toShown(k, b))}` : ""} ${unitOf(k)}`).join("; ")}</td></tr></Show>
        </tbody></table>
        <h4>Why</h4><p>{g.why}</p>
      </div>
    </Dialog>
  );
});

/** Sources: where every value comes from, and which materials cite each source. */
export const SourcesPage = () => {
  const [open, setOpen] = createSignal("");
  const cites = (id: string) => db().materials.flatMap(m => Object.entries(m.props).filter(([, v]) => v[v.length === 4 ? 2 : v.length - 1] === id).map(([k]) => [m, k] as const));
  const kinds: Record<string, string> = { handbook: "Handbook", government: "Government data", "open-dataset": "Open dataset", datasheet: "Datasheet", "textbook-typical": "Our compilation", estimate: "Estimate", derived: "Calculated", user: "Your source" };
  const checked = () => db().materials.reduce((n, m) => n + (m.checked?.length ?? 0), 0);
  return (
    <div class="page sourcespage">
      <h1>Sources <Help k="sources" /></h1>
      <div class="rule"><b>Open sources only.</b> Every value names where it comes from: public government handbooks and data, open datasets, published datasheets, and typical textbook ranges written up as our own compilation. No proprietary database is used, copied or named.
        <span class="muted"> Built-in values are drafts until checked against their source; {checked()} values are checked so far (✓ on the datasheets).</span></div>
      <table class="tbl sources"><thead><tr><th>Source</th><th>Kind</th><th>Licence</th><th class="r">Values</th></tr></thead><tbody>
        <For each={db().sources}>{s => { const n = () => cites(s.id).length; return <>
          <tr class="link" classList={{ sel: open() === s.id }} onClick={() => setOpen(open() === s.id ? "" : s.id)}>
            <td><b>{s.title}</b><div class="muted">{[...new Set([s.author, s.publisher, s.year].filter(Boolean))].join(", ")}{s.url ? ` · ${s.url}` : ""}</div><Show when={s.note}><div class="muted small">{s.note}</div></Show></td>
            <td>{kinds[s.kind] ?? s.kind}</td><td class="muted">{s.licence}</td><td class="r">{n()} {open() === s.id ? "▴" : "▾"}</td></tr>
          <Show when={open() === s.id}><tr><td colspan="4" class="cites"><For each={[...new Set(cites(s.id).map(x => x[0]))]}>{m => <span class="cite"><b class="link" onClick={() => openSheet(m.id)}>{m.name}</b>: {cites(s.id).filter(x => x[0] === m).map(x => propOf(x[1])?.name).join(", ")}</span>}</For></td></tr></Show>
        </>; }}</For>
      </tbody></table>
    </div>
  );
};
