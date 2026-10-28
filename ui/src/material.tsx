// Your own materials: the Add Material window (what is needed and why, then the values with a
// source for each), editing and deleting them. They are saved in "My materials.datdb" in the
// app's data folder and appear in Browse, the datasheets, every chart and the selection.
import { For, Show, createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import { Material, Source, Val, Out } from "./api";
import { db, call, setOut, toast, openDialog, closeDialog, openSheet, toShown, toStored, unitOf, loadDb } from "./store";
import { Help } from "./help";
import { Dialog } from "./ui";

const FAMILIES = ["Metals", "Polymers", "Elastomers", "Ceramics and glasses", "Composites", "Foams", "Natural materials"];
const KINDS: [string, string][] = [["datasheet", "Manufacturer's datasheet"], ["user", "My own test or measurement"], ["handbook", "Handbook or standard"], ["textbook-typical", "Textbook"], ["estimate", "Estimate"]];
const OWN = "__own";

type Row = { lo: string; hi: string; src: string };
export const deleteMaterial = (m: Material) => {
  if (!m.user) return toast("Built-in materials cannot be deleted; hide their family on a chart instead.");
  if (!confirm(`Delete your material “${m.name}”? This cannot be undone.`)) return;
  call<Out>("delete_material", { id: m.id }).then(o => { if (o) { setOut(o); toast(`Deleted ${m.name}.`, true); } });
};

/** Opens the Add Material window, empty, to edit one of yours, or (copy) starting from any material. */
export const editMaterial = (base?: Material, copy = false) => openDialog(() => {
  const props = db().props, mine = base?.user && !copy;
  const [f, setF] = createStore({
    name: base ? (copy ? `${base.name} (my copy)` : base.name) : "", fam: base?.family[0] ?? "Metals", cls: base?.family.slice(1).join(" › ") ?? "",
    des: base ? Object.entries(base.des).map(([k, v]) => `${k} ${v}`).join(", ") : "", syn: base?.syn.join(", ") ?? "", uses: base?.uses ?? "", notes: base?.notes ?? "",
    srcTitle: "", srcYear: "", srcKind: "datasheet", recyclable: base?.props.recyclable ? String(base.props.recyclable[0]) : "",
  });
  const [rows, setRows] = createStore<Record<string, Row>>(Object.fromEntries(props.filter(p => p.kind === "range" && p.group !== "Derived").map(p => {
    const v = base?.props[p.key];
    return [p.key, v && typeof v[0] === "number" ? { lo: String(+toShown(p.key, v[0]).toPrecision(6)), hi: String(+toShown(p.key, v[1] as number).toPrecision(6)), src: String(v[2]) } : { lo: "", hi: "", src: OWN }];
  })));
  const [group, setGroup] = createSignal("General");
  const groups = [...new Set(props.filter(p => p.kind === "range" && p.group !== "Derived").map(p => p.group))];
  const sources = () => db().sources;
  const filled = () => Object.values(rows).filter(r => r.lo.trim() || r.hi.trim()).length;
  const save = async () => {
    const ownId = `u-s-${Date.now().toString(36)}`;
    const usesOwn = Object.values(rows).some(r => (r.lo.trim() || r.hi.trim()) && r.src === OWN) || f.recyclable !== "";
    if (usesOwn && !f.srcTitle.trim()) { toast("Say where the values come from: a datasheet, a test report, a book or your own measurement."); return false; }
    const vals: Record<string, Val> = {};
    for (const p of props.filter(p => p.kind === "range" && p.group !== "Derived")) {
      const r = rows[p.key], lo = r.lo.trim(), hi = r.hi.trim() || lo;
      if (!lo && !hi) continue;
      const a = toStored(p.key, +lo.replace(",", ".")), b = toStored(p.key, +hi.replace(",", "."));
      if (!isFinite(a) || !isFinite(b)) { toast(`${p.name}: type numbers.`); setGroup(p.group); return false; }
      vals[p.key] = [Math.min(a, b), Math.max(a, b), r.src === OWN ? ownId : r.src];
    }
    if (f.recyclable) vals.recyclable = [f.recyclable === "true", ownId];
    const des: Record<string, string> = {};
    f.des.split(",").map(s => s.trim()).filter(Boolean).forEach(s => { const [k, ...v] = s.split(/\s+/); des[v.length ? k.toUpperCase() : "Code"] = v.length ? v.join(" ") : k; });
    const material: Partial<Material> = { id: mine ? base!.id : "", name: f.name.trim(), family: [f.fam, ...f.cls.split("›").map(s => s.trim()).filter(Boolean)], syn: f.syn.split(",").map(s => s.trim()).filter(Boolean), des, props: vals, notes: f.notes, uses: f.uses };
    const src: Source[] = usesOwn ? [{ id: ownId, title: f.srcTitle.trim(), year: f.srcYear.trim(), kind: f.srcKind, author: "", publisher: "", url: "", licence: "", note: "" }] : [];
    const o = await call<Out>("save_material", { material, sources: src });
    if (!o) return false;
    setOut(o);
    toast(`${material.name} is saved in your database.`, true);
    // Open its datasheet once the database has reloaded (it can take a moment with many materials).
    const d = await loadDb();
    const m = d.materials.filter(x => x.user && x.name === material.name).pop();
    if (m) openSheet(m.id);
  };
  const classes = () => [...new Set(db().materials.filter(m => m.family[0] === f.fam && m.family.length > 1).map(m => m.family.slice(1).join(" › ")))];
  return (
    <Dialog title={mine ? `Edit ${base!.name}` : "Add a material"} wide okText="Save material" ok={save}>
      <div class="addmat">
        <aside class="howto">
          <h4>What you need <Help k="addmaterial" /></h4>
          <ol>
            <li><b>A name</b> and the <b>family</b> it belongs to (the class is optional, e.g. “Aluminium alloys › Wrought”).</li>
            <li><b>Density</b>: every chart and most performance indices use it.</li>
            <li><b>At least one more property</b>, such as Young's modulus or yield strength.</li>
            <li>For each value a <b>low and a high</b>. One measured value: type it in Low and leave High empty.</li>
            <li>Values in the <b>unit shown</b> next to each property (MPa, GPa, kg/m³…).</li>
            <li><b>Where the values come from</b>: a datasheet, a standard, a book, a test report or your own measurement. It is printed on the datasheet as the reference.</li>
          </ol>
          <p class="muted">Leave properties you do not know empty: they show as “no data”, never as zero, and a limit on them can fail or keep the material (Select › Stages).</p>
          <p class="muted">Your materials are kept on this computer in “My materials.datdb” and appear everywhere: Browse, datasheets, charts and the selection.</p>
        </aside>
        <div class="amform">
          <div class="form">
            <label>Name</label><input value={f.name} placeholder="e.g. Aluminium 6082-T6 from supplier X" onInput={e => setF("name", e.currentTarget.value)} />
            <label>Family</label><select value={f.fam} onChange={e => setF("fam", e.currentTarget.value)}><For each={FAMILIES}>{x => <option>{x}</option>}</For></select>
            <label>Class</label><input list="amcls" value={f.cls} placeholder="optional, e.g. Non-ferrous › Aluminium alloys" onInput={e => setF("cls", e.currentTarget.value)} /><datalist id="amcls"><For each={classes()}>{c => <option value={c} />}</For></datalist>
            <label>Designations</label><input value={f.des} placeholder="e.g. UNS A96082, EN AW-6082" onInput={e => setF("des", e.currentTarget.value)} />
            <label>Also called</label><input value={f.syn} placeholder="trade names or synonyms, separated by commas" onInput={e => setF("syn", e.currentTarget.value)} />
            <label>Typical uses</label><input value={f.uses} onInput={e => setF("uses", e.currentTarget.value)} />
            <label>Source of the values</label><input value={f.srcTitle} placeholder="e.g. Supplier datasheet AL-6082, rev. 3 / Test report 2026-14" onInput={e => setF("srcTitle", e.currentTarget.value)} />
            <label>Kind of source, year</label><span class="amsrc"><select value={f.srcKind} onChange={e => setF("srcKind", e.currentTarget.value)}><For each={KINDS}>{([k, l]) => <option value={k}>{l}</option>}</For></select>
              <input value={f.srcYear} placeholder="year" onInput={e => setF("srcYear", e.currentTarget.value)} /></span>
          </div>
          <div class="dtabs"><For each={groups}>{g => <button classList={{ on: group() === g }} onClick={() => setGroup(g)}>{g}{props.some(p => p.group === g && (rows[p.key]?.lo || rows[p.key]?.hi)) ? " •" : ""}</button>}</For></div>
          <table class="tbl amvals"><thead><tr><th>Property</th><th>Low</th><th>High</th><th>Unit</th><th>Source</th></tr></thead><tbody>
            <For each={props.filter(p => p.kind === "range" && p.group === group())}>{p => <tr title={p.about}>
              <td>{p.name}{p.key === "rho" && <b class="req"> *</b>}</td>
              <td><input inputmode="decimal" value={rows[p.key].lo} onInput={e => setRows(p.key, "lo", e.currentTarget.value)} /></td>
              <td><input inputmode="decimal" value={rows[p.key].hi} onInput={e => setRows(p.key, "hi", e.currentTarget.value)} /></td>
              <td class="muted">{unitOf(p.key)}</td>
              <td><select value={rows[p.key].src} onChange={e => setRows(p.key, "src", e.currentTarget.value)}><option value={OWN}>The source above</option><For each={sources()}>{s => <option value={s.id}>{s.title.slice(0, 60)}</option>}</For></select></td></tr>}</For>
          </tbody></table>
          <Show when={group() === "Eco"}><div class="form"><label>Recyclable</label><select value={f.recyclable} onChange={e => setF("recyclable", e.currentTarget.value)}><option value="">Not known</option><option value="true">Yes</option><option value="false">No</option></select></div></Show>
          <p class="muted">{filled()} {filled() === 1 ? "property" : "properties"} filled · * needed</p>
        </div>
      </div>
    </Dialog>
  );
});
export { closeDialog };
