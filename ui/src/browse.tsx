// Browse: the family tree on the left and the materials table on the right, on the family grid
// engine (selection.ts): sort by a heading, select cells, rows or columns, Ctrl+C to Excel.
// Datasheet: every property of one material, grouped, with its range, unit and reference.
import { For, Show, createMemo, createSignal } from "solid-js";
import { Material, mid, isRange, SEP, famColor, fmt } from "./api";
import { Help } from "./help";
import { vtext, unitOf, toShown, db, node, setNode, query, setQuery, cur, setCur, openSheet, failed, out, propOf, srcOf, mat, call, toast, openMenu, show } from "./store";
import { createSel, aggregate } from "./selection";
import { toTsv } from "./cells";
import { Icon } from "./ui";
import { editMaterial, deleteMaterial } from "./material";

export const [cols, setCols] = createSignal(["rho", "price", "E", "sy", "ts", "kic", "tmax", "lam", "alpha", "co2"]);
const FAMS = ["Metals", "Polymers", "Elastomers", "Ceramics and glasses", "Composites", "Foams", "Natural materials"];
export const famIndex = (m: Material) => Math.max(0, FAMS.indexOf(m.family[0]));

/** The materials Browse shows: the tree node and the search words. */
export const shown = createMemo(() => {
  const n = node(), words = query().toLowerCase().split(/\s+/).filter(Boolean);
  return db().materials.filter(m => {
    const path = m.family.join(SEP);
    if (n && path !== n && !path.startsWith(n + SEP)) return false;
    const hay = `${m.name} ${m.syn.join(" ")} ${Object.values(m.des).join(" ")} ${m.family.join(" ")} ${m.uses}`.toLowerCase();
    return words.every(w => hay.includes(w));
  });
});

export const Tree = () => (
  <aside class="tree">
    <button classList={{ on: node() === "" }} onClick={() => setNode("")}><b>All materials</b><small>{db().materials.length}</small></button>
    <For each={db().tree}>{n => <button classList={{ on: node() === n.path }} style={{ "padding-left": `${10 + n.depth * 14}px` }} onClick={() => setNode(n.path)}>
      {n.depth === 0 && <i class="dot" style={{ background: famColor(FAMS.indexOf(n.name)) }} />}<span>{n.name}</span><small>{n.count}</small></button>}</For>
  </aside>
);

export const Browse = () => {
  const [sort, setSort] = createSignal<[string, boolean]>(["", true]);
  const keys = () => ["name", "family", ...cols()];
  const rows = createMemo(() => {
    const [k, asc] = sort(), list = [...shown()];
    if (!k) return list;
    const v = (m: Material) => k === "name" ? m.name : k === "family" ? m.family.slice(1).join(" ") : mid(m.props[k]);
    return list.sort((a, b) => { const x = v(a), y = v(b); if (typeof x === "number" && typeof y === "number") { if (isNaN(x)) return 1; if (isNaN(y)) return -1; return asc ? x - y : y - x; } return asc ? String(x).localeCompare(String(y)) : String(y).localeCompare(String(x)); });
  });
  const text = (m: Material, k: string) => k === "name" ? m.name : k === "family" ? m.family.slice(1).join(" › ") : vtext(k, m.props[k]);
  let body!: HTMLTableSectionElement;
  const sel = createSel(() => [rows().length, keys().length], (r) => { const el = body?.querySelectorAll(".brow")[r] as HTMLElement | undefined; el?.scrollIntoView({ block: "nearest" }); });
  const agg = () => aggregate(sel, (r, c) => { const m = rows()[r], k = keys()[c]; return m && c > 1 && isRange(m.props[k]) ? String(mid(m.props[k])) : ""; });
  const copy = () => {
    const g = sel.last(), head = keys().slice(g.c0, g.c1 + 1).map(k => k === "name" ? "Material" : k === "family" ? "Class" : `${propOf(k)?.name} (${unitOf(k)})`);
    const body = rows().slice(g.r0, g.r1 + 1).map(m => keys().slice(g.c0, g.c1 + 1).map(k => text(m, k)));
    call("copy_text", { text: toTsv([head, ...body]) }).then(() => toast(`Copied ${body.length} ${body.length === 1 ? "row" : "rows"}: paste into a spreadsheet.`, true));
  };
  const key = (e: KeyboardEvent) => {
    const c = sel.cur(), ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && e.key.toLowerCase() === "c") { e.preventDefault(); copy(); }
    else if (ctrl && e.key.toLowerCase() === "a") { e.preventDefault(); sel.all(); }
    else if (e.key === "Enter") { const m = rows()[c.row]; if (m) openSheet(m.id); }
    else { const d = ({ ArrowDown: [1, 0], ArrowUp: [-1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] } as Record<string, number[]>)[e.key]; if (d) { e.preventDefault(); sel.pick(c.row + d[0], c.col + d[1], e.shiftKey); setCur(rows()[sel.cur().row]?.id ?? ""); } }
  };
  const colMenu = (e: MouseEvent) => openMenu(e, db().props.filter(p => db().materials.some(m => m.props[p.key])).map(p => ({ label: `${p.name}${p.unit ? ` (${p.unit})` : ""}`, on: cols().includes(p.key), run: () => setCols(cols().includes(p.key) ? cols().filter(x => x !== p.key) : [...cols(), p.key]) })));
  const rowMenu = (e: MouseEvent, m: Material) => openMenu(e, [
    { label: "Datasheet", icon: "datasheet", run: () => openSheet(m.id) }, { label: "Copy", icon: "copy", keys: "Ctrl+C", run: copy }, "-",
    { label: "Add a material…", icon: "material", run: () => editMaterial() },
    ...(m.user ? [{ label: "Edit this material…", icon: "rename", run: () => editMaterial(m) }, { label: "Delete this material", icon: "del", run: () => deleteMaterial(m) }] : [{ label: "Copy as a new material of yours…", icon: "copy", run: () => editMaterial(m, true) }]),
  ]);
  const fail = () => failed();
  const W = (k: string) => (k === "name" ? 250 : k === "family" ? 190 : 104);
  return (
    <div class="browse">
      <Tree />
      <section class="btable">
        <div class="btools"><span class="find"><Icon n="search" /><input placeholder="Search name, designation (UNS, EN), synonym or use" value={query()} onInput={e => setQuery(e.currentTarget.value)} /></span>
          <span class="muted">{rows().length} of {db().materials.length} materials</span><span class="grow" />
          <button class="btn sm" onClick={colMenu}><Icon n="cols" /> Columns ▾</button><button class="btn sm pri" onClick={() => editMaterial()}><Icon n="material" /> Add material</button><Help k="browse" /></div>
        <div class="bscroll" tabindex="0" onKeyDown={key}>
          <table class="bgrid"><thead><tr><For each={keys()}>{(k, j) => <th style={{ "min-width": `${W(k)}px` }} classList={{ r: j() > 1, on: sort()[0] === k, frz: j() === 0 }}
            onClick={e => { if (e.shiftKey || e.ctrlKey) sel.cols(j(), j(), e.shiftKey, e.ctrlKey); else setSort([k, sort()[0] === k ? !sort()[1] : true]); }}
            title={k === "name" || k === "family" ? "" : `${propOf(k)?.about ?? ""} Click to sort.`}>
            {k === "name" ? "Material" : k === "family" ? "Class" : <>{propOf(k)?.name}<small>{unitOf(k)}</small></>}{sort()[0] === k ? (sort()[1] ? " ▲" : " ▼") : ""}</th>}</For></tr></thead>
            <tbody ref={body}><For each={rows()}>{(m, i) => <tr class="brow" classList={{ cur: m.id === cur(), off: fail().has(m.id) }} onDblClick={() => openSheet(m.id)} onContextMenu={e => rowMenu(e, m)}>
              <For each={keys()}>{(k, j) => <td classList={{ r: j() > 1, in: sel.inRange(i(), j()), at: sel.cur().row === i() && sel.cur().col === j(), frz: j() === 0, nodata: j() > 1 && !m.props[k] }}
                onMouseDown={e => { if (e.button) return; sel.pick(i(), j(), e.shiftKey, e.ctrlKey); setCur(m.id); const mv = (ev: MouseEvent) => { const td = (ev.target as HTMLElement).closest("td"); const tr = td?.parentElement; if (!td || !tr) return; sel.drag([...tr.parentElement!.children].indexOf(tr), [...tr.children].indexOf(td)); }; const up = () => { removeEventListener("mouseover", mv); removeEventListener("mouseup", up); }; addEventListener("mouseover", mv); addEventListener("mouseup", up); }}>
                {j() === 0 ? <><i class="dot" style={{ background: famColor(famIndex(m)) }} />{m.name}{m.user && <span class="chip ok">yours</span>}</> : text(m, k) || "–"}</td>}</For></tr>}</For></tbody></table>
        </div>
        <div class="fbar"><span class="fname">{rows()[sel.cur().row]?.name ?? ""}</span><span class="grow" /><span class="agg">{agg()}</span></div>
      </section>
    </div>
  );
};

const GROUPS = ["General", "Mechanical", "Thermal", "Electrical", "Optical", "Durability", "Processing", "Eco", "Derived"];
export const Datasheet = (p: { id?: string; print?: boolean }) => {
  const m = () => mat(p.id ?? cur()) ?? db().materials[0];
  /** Sources in order of first use, numbered for the superscripts. */
  const refs = () => { const ids: string[] = []; Object.values(m()?.props ?? {}).forEach(v => { const s = v[v.length === 4 ? 2 : v.length - 1] as string; if (!ids.includes(s)) ids.push(s); }); return ids; };
  const srcIdx = (v: any[]) => refs().indexOf(String(v.length === 4 ? v[2] : v[v.length - 1])) + 1;
  const row = () => out()?.results.rows.find(r => r.id === m()?.id);
  return (
    <Show when={m()} fallback={<div class="page"><p class="muted">Choose a material in Browse or on a chart.</p></div>}>
      <div class="page sheetpage" classList={{ print: p.print }}>
        <div class="shead"><i class="sw" style={{ background: famColor(famIndex(m())) }} /><div><h1>{m().name} <Show when={!p.print}><Help k="datasheet" /></Show></h1><div class="muted">{m().family.join(" › ")}</div></div><span class="grow" />
          <Show when={!p.print}><span class="inline">
            <button class="btn sm" onClick={() => show("browse")}>Browse</button>
            {m().user ? <button class="btn sm" onClick={() => editMaterial(m())}>Edit</button> : <button class="btn sm" onClick={() => editMaterial(m(), true)}>Copy as my material</button>}</span></Show></div>
        <div class="sfacts">
          <Show when={Object.keys(m().des).length}><div><b>Designations</b><span>{Object.entries(m().des).map(([k, v]) => `${k} ${v}`).join(" · ")}</span></div></Show>
          <Show when={m().syn.length}><div><b>Also called</b><span>{m().syn.join(", ")}</span></div></Show>
          <Show when={m().uses}><div><b>Typical uses</b><span>{m().uses}</span></div></Show>
          <Show when={m().notes}><div><b>Notes</b><span>{m().notes}</span></div></Show>
          <Show when={row() && out()!.results.stages.length}><div><b>Selection</b><span classList={{ ok: row()!.pass, late: !row()!.pass }}>{row()!.pass ? `Passes${row()!.rank ? `, rank ${row()!.rank}` : ""}` : `Fails: ${row()!.reasons.join("; ")}`}</span></div></Show>
          <div><b>Status</b><span>{m().user ? "Your own material." : m().origin ? `From the database “${m().origin}” (opened beside the built-in data).` : m().checked?.length ? `${m().checked.length} values checked against their source (✓); the others are drafts.` : "Draft: compiled from the sources below; not yet checked value by value."}</span></div>
        </div>
        <For each={GROUPS}>{g => { const ps = () => db().props.filter(x => x.group === g); const have = () => ps().filter(x => m().props[x.key]);
          return <Show when={have().length || !p.print}><h2>{g}{g === "Derived" && !p.print ? <> <Help k="derived" /></> : ""}</h2><table class="tbl dsheet"><tbody><For each={p.print ? have() : ps()}>{x => { const v = m().props[x.key];
            return <tr classList={{ nodata: !v }} title={x.about}><td>{x.name}{x.sym && <span class="sym"> {x.sym}</span>}</td><td class="r">{v ? vtext(x.key, v) : "no data"}{v && v.length === 4 ? <span class="muted"> (typical {fmt(toShown(x.key, v[3] as number))})</span> : ""}</td><td class="muted">{v ? unitOf(x.key) : ""}</td><td class="ref">{v ? <sup>{srcIdx(v as any[])}</sup> : ""}{m().checked?.includes(x.key) ? <span class="chk ok" title="Checked against the source">✓</span> : ""}</td></tr>; }}</For></tbody></table></Show>; }}</For>
        <h2>References</h2>
        <ol class="refs"><For each={refs()}>{id => { const s = srcOf(id); return <li><b>{s?.title ?? id}</b>{s ? <span class="muted">{[s.author, s.publisher, s.year].filter(Boolean).join(", ")}. {s.licence}{s.url ? ` · ${s.url}` : ""}{s.note ? `. ${s.note}` : ""}</span> : null}</li>; }}</For></ol>
      </div>
    </Show>
  );
};
