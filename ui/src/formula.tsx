// Your own properties as formulas of other properties (`[E]^0.5/[rho]`), saved with the project.
// The engine parses and works them out on every material's range; this window only edits the list.
// "Expression Axis" is the same window, then puts the formula on the chart's axis.
import { For, Show, createSignal } from "solid-js";
import { Formula } from "./api";
import { act, P, db, chart, setChart, openDialog, toast } from "./store";
import { Dialog } from "./ui";
import { Help } from "./help";

/** The formula editor; `axis` puts the formula saved on the open chart's X or Y axis. */
export const editFormulas = (axis?: "x" | "y", start?: string) => openDialog(() => {
  const list = () => P().formulas;
  const pick = start ? list().find(f => f.name === start) : undefined;
  const [name, setName] = createSignal(pick?.name ?? ""), [expr, setExpr] = createSignal(pick?.expr ?? ""), [unit, setUnit] = createSignal(pick?.unit ?? ""), [log, setLog] = createSignal(pick?.log ?? true);
  const [old, setOld] = createSignal(pick?.name ?? "");
  let box!: HTMLInputElement;
  const props = () => db().props.filter(q => q.kind === "range" && db().materials.some(m => m.props[q.key]));
  const insert = (t: string) => { const at = box.selectionStart ?? expr().length, e = expr(); setExpr(e.slice(0, at) + t + e.slice(box.selectionEnd ?? at)); box.focus(); box.selectionStart = box.selectionEnd = at + t.length; };
  const load = (f: Formula) => { setName(f.name); setExpr(f.expr); setUnit(f.unit); setLog(f.log); setOld(f.name); };
  const save = async () => {
    const f: Formula = { name: name().trim(), expr: expr().trim(), unit: unit().trim(), log: log() };
    if (!f.name || !f.expr) { toast("Give the formula a name and a formula."); return false; }
    const rest = list().filter(x => x.name !== old());
    const at = list().findIndex(x => x.name === old());
    const next = at < 0 ? [...rest, f] : [...rest.slice(0, at), f, ...rest.slice(at)];
    if (!(await act({ SetFormulas: { formulas: next } }))) return false;
    // A renamed formula keeps its place on charts that showed it under its old name.
    if (axis && chart()) await setChart(c => { c[axis] = `f:${f.name}`; if (axis === "x") c.logX = f.log; else c.logY = f.log; c.sels = []; c.goal = ""; });
    toast(`${f.name} saved: it is in every list of properties (axes, limits, ranking).`, true);
  };
  const remove = (n: string) => { if (confirm(`Delete the formula “${n}”? Charts and limits on it go back to the defaults.`)) act({ SetFormulas: { formulas: list().filter(x => x.name !== n) } }); };
  return (
    <Dialog title={axis ? `Expression on the ${axis.toUpperCase()} axis` : "Formula properties"} wide ok={save} okText={axis ? "Save and plot" : "Save"}>
      <div class="fgrid">
        <div>
          <p class="muted">A property of your own, worked out for every material from its other values, such as <code>[E]^0.5/[rho]</code>. Name a property in brackets by its symbol, key or name. It is saved with this selection and can be used on chart axes, in limits and in the ranking. <Help k="formula" /></p>
          <label class="field">Name<input value={name()} onInput={e => setName(e.currentTarget.value)} placeholder="Specific stiffness of a beam" /></label>
          <label class="field">Formula<input ref={box} class="mono" value={expr()} onInput={e => setExpr(e.currentTarget.value)} placeholder="[E]^0.5/[rho]" /></label>
          <p class="inline"><span class="muted">Insert</span><For each={["+", "−", "×", "÷", "^", "(", ")", "sqrt( )"]}>{t => <button class="btn sm" onClick={() => insert(({ "−": "-", "×": "*", "÷": "/", "sqrt( )": "sqrt(" } as Record<string, string>)[t] ?? t)}>{t}</button>}</For></p>
          <div class="pinsert"><For each={props()}>{q => <button class="btn sm" title={`${q.name} (${q.unit || "no unit"})`} onClick={() => insert(`[${q.key}]`)}>{q.sym || q.name}</button>}</For></div>
          <label class="field">Unit (shown on axes, optional)<input value={unit()} onInput={e => setUnit(e.currentTarget.value)} placeholder="GPa½/(kg/m³)" /></label>
          <label class="chk"><input type="checkbox" checked={log()} onChange={e => setLog(e.currentTarget.checked)} />Log axis (the values are all above zero)</label>
          <p class="muted small">Values are worked out on ranges: the result holds every combination of the low and high values. A material without a value it needs, or a division by a range that holds zero, gets no value (a gap), never zero. Values are in the stored units (kg/m³, GPa, MPa, °C…).</p>
        </div>
        <div>
          <h4>Formulas in this selection</h4>
          <For each={list()} fallback={<p class="muted">None yet.</p>}>{f => <div class="frow" classList={{ on: f.name === old() }}><button class="lnk" onClick={() => load(f)}><b>{f.name}</b></button><code>{f.expr}</code>
            <Show when={chart()}><button class="btn sm" title="Plot on the open chart's Y axis" onClick={() => setChart(c => { c.y = `f:${f.name}`; c.logY = f.log; c.sels = []; c.goal = ""; })}>Y</button><button class="btn sm" title="Plot on the X axis" onClick={() => setChart(c => { c.x = `f:${f.name}`; c.logX = f.log; c.sels = []; c.goal = ""; })}>X</button></Show>
            <button class="btn sm" onClick={() => remove(f.name)}>Delete</button></div>}</For>
          <p><button class="btn sm" onClick={() => load({ name: "", expr: "", unit: "", log: true })}>New formula</button></p>
        </div>
      </div>
    </Dialog>
  );
});
