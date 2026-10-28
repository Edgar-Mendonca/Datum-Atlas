// Import from a spreadsheet or CSV: the engine reads the file and guesses what each column holds
// (sheet.rs); this window shows the first rows, lets you change each column's property, the family
// and the source, and adds the rows to your own materials.
import { For, Show, createSignal } from "solid-js";
import { Out } from "./api";
import { call, db, openDialog, setOut, toast, loadDb, show } from "./store";
import { Dialog } from "./ui";
import { Help } from "./help";

type Preview = { path: string; head: string[]; rows: string[][]; count: number; guess: string[] };
const FIELDS: [string, string][] = [["", "— skip —"], ["name", "Name"], ["family", "Family"], ["class", "Class"], ["designation", "Designation"], ["notes", "Notes"], ["uses", "Uses"], ["source", "Source of the row"]];

export const importMaterials = async () => {
  const p = await call<Preview | null>("import_preview", { path: null });
  if (!p) return;
  openDialog(() => {
    const [map, setMap] = createSignal([...p.guess]);
    const [family, setFamily] = createSignal("Metals"), [source, setSource] = createSignal("");
    const props = () => db().props.filter(x => x.group !== "Derived");
    const set = (c: number, v: string) => setMap(m => m.map((x, i) => (i === c ? v : x)));
    const ok = async () => {
      const r = await call<{ out: Out; added: number; notes: string[] }>("import_materials", { path: p.path, map: map(), family: family(), source: source() });
      if (!r) return false;
      setOut(r.out);
      await loadDb();
      toast(`${r.added} materials added to your own${r.notes.length ? `; ${r.notes.length} notes: ${r.notes.slice(0, 3).join("; ")}${r.notes.length > 3 ? "…" : ""}` : "."}`, !r.notes.length);
      show("data");
    };
    return (
      <Dialog title="Import materials from a spreadsheet or CSV" wide ok={ok} okText={`Add ${p.count} rows to my materials`}>
        <p class="muted">{p.path} · {p.count} rows. Each column is matched to a property by its heading; change any that are wrong. A cell may hold one value or a range (2690 – 2710); blank cells are gaps. Values are read in the units shown, or converted when the heading names an imperial unit (ksi, °F…). <Help k="import" /></p>
        <div class="imptable"><table class="tbl"><thead>
          <tr><For each={p.head}>{(h, c) => <th><div>{h}</div><select value={map()[c()]} onChange={e => set(c(), e.currentTarget.value)}>
            <For each={FIELDS}>{([k, l]) => <option value={k} selected={k === map()[c()]}>{l}</option>}</For>
            <optgroup label="Properties"><For each={props()}>{x => <><option value={x.key} selected={x.key === map()[c()]}>{x.name}{x.unit ? ` (${x.unit})` : ""}</option>
              <Show when={x.imperial}><option value={`${x.key}|imperial`} selected={`${x.key}|imperial` === map()[c()]}>{x.name} ({x.imperial![0]})</option></Show></>}</For></optgroup></select></th>}</For></tr>
        </thead><tbody><For each={p.rows.slice(0, 8)}>{r => <tr><For each={p.head}>{(_, c) => <td>{r[c()] ?? ""}</td>}</For></tr>}</For></tbody></table></div>
        <div class="prow">
          <label>Family when no column says<select value={family()} onChange={e => setFamily(e.currentTarget.value)}><For each={["Metals", "Polymers", "Elastomers", "Ceramics and glasses", "Composites", "Foams", "Natural materials"]}>{f => <option>{f}</option>}</For></select></label>
          <label>Where the values come from<input value={source()} onInput={e => setSource(e.currentTarget.value)} placeholder="Supplier datasheet, test report, book…" /></label>
        </div>
        <p class="muted small">Needed: a name column, and a source (here or in a column). Values the app cannot use are left out and listed. Imported materials join your own: edit or delete them like any other.</p>
      </Dialog>
    );
  });
};

/** Opens another .datdb beside the built-in data. */
export const openDatabase = async () => { const o = await call<Out | null>("open_database", { path: null }); if (o) { setOut(o); await loadDb(); toast("Database opened: its materials are in every view, marked with its name.", true); return true; } return false; };
/** Saves your own materials as .datdb, .xlsx or .csv. */
export const saveMine = async (fmt: string) => { const p = await call<string | null>("export_database", { fmt }); if (p) toast(`Saved ${p}`, true); };

/** The databases open: yours, and others opened beside the built-in data. */
export const Databases = () => {
  type Row = { name: string; path: string; count: number; mine: boolean };
  const [list, setList] = createSignal<Row[]>([]);
  const load = () => call<Row[]>("databases").then(v => v && setList(v));
  load();
  const open = async () => { if (await openDatabase()) load(); };
  const close = async (path: string) => { const o = await call<Out>("close_database", { path }); if (o) { setOut(o); await loadDb(); load(); } };
  const save = saveMine;
  return (
    <section class="panel">
      <h2>Databases <Help k="databases" /></h2>
      <p class="muted">Your own materials live in “My materials”. Open other Datum Atlas databases (.datdb) from colleagues or a team folder: their materials appear beside the built-in ones, marked with the database's name, and can be closed again.</p>
      <table class="tbl"><tbody><For each={list()}>{d => <tr><td><b>{d.name}</b>{d.mine ? <span class="muted"> (yours)</span> : ""}</td><td class="muted small">{d.path}</td><td class="r">{d.count} materials</td>
        <td class="r"><Show when={!d.mine} fallback={<span class="muted small">always open</span>}><button class="btn sm" onClick={() => close(d.path)}>Close</button></Show></td></tr>}</For></tbody></table>
      <p class="inline"><button class="btn" onClick={open}>Open a database…</button><button class="btn" onClick={importMaterials}>Import from a spreadsheet or CSV…</button>
        <span class="muted">Save my materials as</span><button class="btn sm" onClick={() => save("datdb")}>.datdb (to share)</button><button class="btn sm" onClick={() => save("xlsx")}>spreadsheet</button><button class="btn sm" onClick={() => save("csv")}>CSV</button></p>
    </section>
  );
};
