// File › Print: settings on the left, a live preview on the right (Datum Plan's pattern). Charts
// print as vector pages (one per chart, fitted between the header and footer, with a family legend
// and a caption); the datasheet and the results print as flowing pages with the header and footer
// repeated. "Save as PDF" in the print dialog makes a PDF.
import { For, Show, Switch, Match, JSX, createSignal, onCleanup } from "solid-js";
import { Portal } from "solid-js/web";
import { createStore } from "solid-js/store";
import { famColor, fmt } from "./api";
import { out, opts, P, act, view, chart, cur, toast, call, mat , propOf } from "./store";
import { ChartSvg, Win } from "./chart";
import { Datasheet } from "./browse";
import { Pick, Check, Seg } from "./ui";

/** Paper sizes in millimetres (portrait). */
export const PAPERS: [string, number, number][] = [["A4", 210, 297], ["A3", 297, 420], ["A5", 148, 210], ["Letter", 215.9, 279.4], ["Legal", 215.9, 355.6], ["Tabloid", 279.4, 431.8]];
const MM = 96 / 25.4;
type Slots = [string, string, string];
export type PageSetup = { paper: string; landscape: boolean; margins: [number, number, number, number]; which: "this" | "all"; caption: boolean; header: Slots; footer: Slots };
export const CODES: [string, string][] = [["&[Page]", "Page number"], ["&[Pages]", "Total pages"], ["&[Date]", "Today's date"], ["&[Project]", "Selection name"], ["&[Item]", "Chart or material name"],
  ["&[Company]", "Company (Options)"], ["&[Logo]", "Logo (Options)"]];
const defaults = (): PageSetup => ({
  paper: opts().paper || "A4", landscape: opts().landscape !== false, margins: [12, 12, 12, 12], which: "this", caption: true,
  header: ["&[Logo]", "&[Project]", "&[Date]"], footer: ["Datum Atlas", "Page &[Page] of &[Pages]", "&[Company]"],
});
const [ps, setPs] = createStore<PageSetup>(defaults());
let loadedFor = "";
const load = () => { const k = `${out()?.path}`; if (k === loadedFor) return; loadedFor = k; setPs({ ...defaults(), ...((P().ui?.page ?? {}) as Partial<PageSetup>) }); };
const save = () => { if (JSON.stringify(P().ui?.page ?? {}) !== JSON.stringify(ps)) act({ Ui: { key: "page", value: JSON.parse(JSON.stringify(ps)) } }, true); };

const codes = (t: string, page: number, pages: number, name: string) => t.replace(/&\[Page\]/g, String(page)).replace(/&\[Pages\]/g, String(pages))
  .replace(/&\[Date\]/g, new Date().toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" })).replace(/&\[Project\]/g, P().name).replace(/&\[Item\]/g, name).replace(/&\[Company\]/g, opts().company);
const Slot = (p: { t: string; page: number; pages: number; name: string; h: number }) => (
  <For each={p.t.split(/(&\[Logo\])/)}>{part => part === "&[Logo]" ? (opts().logo && <img src={opts().logo} style={{ "max-height": `${p.h}px`, "max-width": "45%" }} />) : <span style={{ "white-space": "pre-line" }}>{codes(part, p.page, p.pages, p.name)}</span>}</For>
);
const HeadFoot = (p: { k: "header" | "footer"; page: number; pages: number; name: string }) => (
  <div class={p.k === "header" ? "phead" : "pfoot"}><For each={[0, 1, 2]}>{i => <div class={`ps${i}`}><Slot t={ps[p.k][i]} page={p.page} pages={p.pages} name={p.name} h={30} /></div>}</For></div>
);

/** What File › Print prints: charts (from the chart and stage views), a datasheet, or the results. */
export const pkind = () => (view() === "datasheet" || view() === "browse" ? "sheet" : view() === "results" ? "results" : "chart");
const paper = () => { const p = PAPERS.find(x => x[0] === ps.paper) ?? PAPERS[0]; return ps.landscape ? [p[2], p[1]] : [p[1], p[2]]; };
const charts = () => (ps.which === "all" ? P().charts : chart() ? [chart()!] : []);

/** One chart page, fitted to the room between the header and the footer. */
const ChartPage = (p: { id: number; n: number; pages: number }) => {
  const [pw, ph] = paper(), c = () => P().charts.find(x => x.id === p.id)!, o = () => out()!.charts.find(x => x.id === p.id)!;
  const W = () => (pw - ps.margins[1] - ps.margins[3]) * MM, H = () => (ph - ps.margins[0] - ps.margins[2]) * MM - (ps.caption ? 150 : 110);
  const win = (): Win => [o().x.lim[0], o().x.lim[1], o().y.lim[0], o().y.lim[1]];
  const lines = () => out()!.results.lines.filter(l => l.chart === p.id);
  return (
    <div class="ppage" style={{ width: `${pw * MM}px`, height: `${ph * MM}px` }}>
      <div class="pin" style={{ left: `${ps.margins[3] * MM}px`, top: `${ps.margins[0] * MM}px`, width: `${W()}px`, height: `${(ph - ps.margins[0] - ps.margins[2]) * MM}px` }}>
        <HeadFoot k="header" page={p.n} pages={p.pages} name={c().name} />
        <div class="pgraph">
          <div class="plegendrow"><For each={o().families.filter(f => !c().hidden.includes(f))}>{f => <span><i style={{ background: famColor(o().families.indexOf(f)) }} />{f}</span>}</For></div>
          <ChartSvg c={c()} o={o()} w={W()} h={H()} win={win()} print />
          <Show when={ps.caption}><p class="pcap"><b>{c().name}.</b> Each ellipse spans a material's range of both properties; envelopes enclose each {c().hull === "class" ? "class" : "family"}.
            {lines().map(l => { const s = c().sels[l.k], above = s && "Line" in s ? s.Line.above : true; return ` A selection line keeps ${l.name} ${above ? "≥" : "≤"} ${fmt(l.value)}.`; }).join("")} {out()!.results.stages.length ? `${out()!.results.pass} of ${out()!.results.total} materials pass the selection (greyed: fail).` : ""} Data: Datum Atlas database, draft values with references.</p></Show>
        </div>
        <HeadFoot k="footer" page={p.n} pages={p.pages} name={c().name} />
      </div>
    </div>
  );
};

const ResultsPrint = () => {
  const r = () => out()!.results;
  return (
    <div class="pinfo"><h1>{P().name}: results</h1>
      <Show when={P().notes}><p style={{ "white-space": "pre-wrap" }}>{P().notes}</p></Show>
      <p>{r().pass} of {r().total} materials pass{r().indexName ? `, ranked by ${r().indexName}` : ""}.</p>
      <ul><For each={r().stages}>{s => <li>{s.name}: {s.pass} pass</li>}</For></ul>
      <Show when={P().stages.limits.length}><p><b>Limits: </b>{P().stages.limits.map(l => { const q = propOf(l.prop); return `${q?.name}${l.min !== null ? ` ≥ ${fmt(l.min)}` : ""}${l.max !== null ? ` ≤ ${fmt(l.max)}` : ""}${l.flag !== null ? `: ${l.flag ? "yes" : "no"}` : ""} ${q?.unit ?? ""}`; }).join("; ")}</p></Show>
      <table class="ptable"><thead><tr><th>Rank</th><th>Material</th><th>Family</th><Show when={r().indexName}><th>{r().indexName}</th><th>× best</th></Show></tr></thead>
        <tbody><For each={r().rows.filter(x => x.pass)}>{x => <tr><td>{x.rank}</td><td>{x.name}</td><td>{x.family}</td><Show when={r().indexName}><td class="r">{fmt(x.index)}</td><td class="r">{x.ratio !== null ? fmt(x.ratio) : ""}</td></Show></tr>}</For></tbody></table>
    </div>
  );
};
/** Flowing document: the header and footer repeat through thead/tfoot; page numbers come from @page. */
const Flow = (p: { children: JSX.Element; name: string }) => (
  <table class="pflow"><thead><tr><td><HeadFoot k="header" page={1} pages={1} name={p.name} /></td></tr></thead>
    <tbody><tr><td>{p.children}</td></tr></tbody><tfoot><tr><td><HeadFoot k="footer" page={1} pages={1} name={p.name} /></td></tr></tfoot></table>
);
const FlowDoc = () => <Flow name={pkind() === "sheet" ? mat(cur())?.name ?? "" : "Results"}>{pkind() === "sheet" ? <Datasheet print /> : <ResultsPrint />}</Flow>;

const [printing, setPrinting] = createSignal(false);
export const PrintPage = () => {
  load();
  onCleanup(save);
  const [zoomP, setZoomP] = createSignal(0.6);
  const printNow = () => {
    save();
    if (pkind() === "chart" && !charts().length) return toast("Nothing to print yet.");
    const [pw, ph] = paper(), css = document.createElement("style"), flow = pkind() !== "chart";
    css.textContent = flow ? `@page { size: ${pw}mm ${ph}mm; margin: ${ps.margins.map(m => `${m}mm`).join(" ")}; @bottom-center { content: "Page " counter(page) " of " counter(pages); font: 9pt 'IBM Plex Sans', sans-serif; color: #55606E; } }` : `@page { size: ${pw}mm ${ph}mm; margin: 0 }`;
    document.head.append(css);
    setPrinting(true);
    document.documentElement.classList.add("printing");
    setTimeout(() => { window.print(); document.documentElement.classList.remove("printing"); css.remove(); setPrinting(false); }, 300);
  };
  const Slots = (p: { k: "header" | "footer"; label: string }) => (
    <div class="pslots"><b>{p.label}</b><For each={["Left", "Center", "Right"]}>{(l, i) => <label><span>{l}</span><textarea rows="2" value={ps[p.k][i()]} onInput={e => setPs(p.k, i(), e.currentTarget.value)} /></label>}</For></div>
  );
  const Pages = () => (
    <Switch>
      <Match when={pkind() === "chart"}><For each={charts()}>{(c, i) => <ChartPage id={c.id} n={i() + 1} pages={charts().length} />}</For></Match>
      <Match when={true}><FlowDoc /></Match>
    </Switch>
  );
  const [tab, setTab] = createSignal("page");
  return (
    <div class="printpage">
      <div class="pset">
        <h1>Print</h1>
        <p class="pwhat">Printing: <b>{pkind() === "chart" ? (ps.which === "all" ? `All charts (${P().charts.length})` : chart()?.name) : pkind() === "sheet" ? `Datasheet: ${mat(cur())?.name ?? "choose a material"}` : "Results"}</b><br />
          <span class="muted">Open a chart, a datasheet or the results first to print it.</span></p>
        <div class="pbtns"><button class="btn pri big" onClick={printNow}><svg class="ic"><use href="#i-print" /></svg> Print / Save as PDF</button></div>
        <p class="muted">In the print window choose your printer, or <b>Save as PDF</b>. {ps.paper} {ps.landscape ? "landscape" : "portrait"}.</p>
        <h4>Settings</h4>
        <Show when={pkind() === "chart"}><label class="pl">Charts</label><Seg value={ps.which} set={v => setPs("which", v as "this")} options={[["this", "This chart"], ["all", "All charts, one per page"]]} />
          <Check label="Caption under each chart (what it shows, the selection)" value={ps.caption} set={v => setPs("caption", v)} /></Show>
        <div class="prow"><label>Paper<Pick value={ps.paper} set={v => setPs("paper", v)} options={PAPERS.map(p => p[0])} /></label><label>Orientation<Pick value={ps.landscape ? "1" : "0"} set={v => setPs("landscape", v === "1")} options={[["0", "Portrait"], ["1", "Landscape"]]} /></label></div>
        <h4>Page Setup</h4>
        <Seg value={tab()} set={setTab} options={[["page", "Margins"], ["header", "Header"], ["footer", "Footer"]]} />
        <div class="ptab">
          <Show when={tab() === "page"}><div class="prow">{(["Top", "Right", "Bottom", "Left"] as const).map((l, i) => <label>{l} (mm)<input type="number" min="0" max="60" value={ps.margins[i]} onChange={e => setPs("margins", i, +e.currentTarget.value)} /></label>)}</div></Show>
          <Show when={tab() === "header"}><Slots k="header" label="Header" /></Show>
          <Show when={tab() === "footer"}><Slots k="footer" label="Footer" /></Show>
          <Show when={tab() !== "page"}><p class="muted pcodes">Codes: <For each={CODES}>{([c, l]) => <code title={l} onClick={() => call("copy_text", { text: c }).then(() => toast(`${c} copied: paste it into a box.`, true))}>{c}</code>}</For></p></Show>
        </div>
        <p><button class="btn sm" onClick={() => setPs(defaults())}>Reset to defaults</button></p>
      </div>
      <div class="pprev">
        <div class="pzoom"><span class="muted">Preview</span><input type="range" min="0.25" max="1.2" step="0.05" value={zoomP()} onInput={e => setZoomP(+e.currentTarget.value)} /><span class="muted">{Math.round(zoomP() * 100)}%</span></div>
        <Show when={pkind() === "chart"} fallback={<div class="pwrap pflowprev" style={{ width: `${paper()[0] * MM * zoomP()}px` }}><div style={{ zoom: zoomP(), padding: ps.margins.map(m => `${m * MM}px`).join(" "), width: `${paper()[0] * MM}px` }}><FlowDoc /></div></div>}>
          <For each={charts()} fallback={<p class="pempty">Open a chart to print it.</p>}>{(c, i) => <div class="pwrap" style={{ width: `${paper()[0] * MM * zoomP()}px`, height: `${paper()[1] * MM * zoomP()}px` }}><div style={{ transform: `scale(${zoomP()})`, "transform-origin": "0 0" }}><ChartPage id={c.id} n={i() + 1} pages={charts().length} /></div><small>Page {i() + 1} of {charts().length}</small></div>}</For>
        </Show>
      </div>
      <Show when={printing()}><Portal mount={document.body}><div id="printroot" classList={{ flow: pkind() !== "chart" }}><Pages /></div></Portal></Show>
    </div>
  );
};
