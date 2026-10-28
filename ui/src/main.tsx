// Datum Atlas window: layout, sidebar, status bar, keyboard shortcuts, startup and recovery.
import { render } from "solid-js/web";
import { For, Show, Switch, Match, onMount } from "solid-js";
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-600.css";
import "@fontsource/ibm-plex-sans/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "./styles.css";
import { invoke, listen, Out, Options } from "./api";
import { grid } from "./store";
import { out, setOut, backstage, setBackstage, setOpts, opts, dialogs, call, openDialog, toast, loadOut, setThemeRaw, applyTheme, view, show, View, chart, db, loadDb, mat, cur, P, setChartId, setGoals, imperial, setUnits } from "./store";
import { C } from "./cmds";
import { TitleBar, Ribbon, palette } from "./ribbon";
import { Backstage, update, checkUpdate } from "./backstage";
import { ChartView, ChartBar, ChartTools, SelChips, Legend } from "./chart";
import { Browse, Datasheet } from "./browse";
import { StagePanel, StagesPage, ResultsPage } from "./stages";
import { SetupPage, DataPage, SourcesPage, IndexPage } from "./pages";
import { ReportPage } from "./report";
import { ComparePage } from "./compare";
import { Goal } from "./api";
import { Menu, Toast, Dialog, Icon } from "./ui";
import * as D from "./dialogs";

/** The sidebar: Setup first, then the materials, the selection, and the outputs (FAMILY §6). */
const SIDE: ([View, string, string] | "-")[] = [
  ["setup", "setup", "Setup"], "-", ["browse", "tree", "Browse"], ["datasheet", "datasheet", "Datasheet"], ["charts", "atlas", "Charts"], "-",
  ["stages", "filter", "Stages"], ["results", "results", "Results"], ["compare", "compare", "Compare"], ["index", "builder", "Index"], "-", ["report", "report", "Report"], ["data", "db", "Data"], ["sources", "source", "Sources"],
];

/** The charts shown side by side: the page of n charts that holds the open one, in project order. */
const page = (n: number) => { const cs = P().charts, i = Math.max(0, cs.findIndex(x => x.id === chart()?.id)); return cs.slice(Math.floor(i / n) * n, Math.floor(i / n) * n + n); };
const ChartsPage = () => (
  <Show when={chart()} fallback={<div class="page"><p class="muted">No chart. <button class="btn sm" onClick={() => C.newChart()}>New chart</button></p></div>}>{c => (
    <div class="chartpage">
      <div class="cmain">
        <div class="ctabs"><For each={P().charts}>{x => <button classList={{ on: x.id === c().id }} onClick={() => setChartId(x.id)}><Icon n="atlas" />{x.name}</button>}</For>
          <button class="add" title="New chart" onClick={() => C.newChart()}>＋</button></div>
        <ChartBar c={c()} />
        <ChartTools c={c()} />
        <SelChips c={c()} />
        <Show when={out()!.charts.find(o => o.id === c().id)}>{o => <Legend c={c()} o={o()} />}</Show>
        <Show when={grid() > 1} fallback={<div class="board"><For each={[c().id]}>{() => <ChartView c={c()} />}</For></div>}>
          <div class="board cgrid" classList={{ four: grid() === 4 }}><For each={page(grid())}>{x => <div class="gcell" classList={{ on: x.id === c().id }} onMouseDown={() => setChartId(x.id)}>
            <div class="gtitle">{x.name}</div><For each={[x.id]}>{() => <ChartView c={P().charts.find(y => y.id === x.id)!} />}</For></div>}</For></div>
        </Show>
      </div>
      <StagePanel />
    </div>
  )}</Show>
);

const Status = () => (
  <footer class="status">
    <span>Ready</span>
    <span class="muted">{db().materials.length} materials{db().userCount ? ` (${db().userCount} yours)` : ""}</span>
    <Show when={out()?.results.stages.length}><span class="muted">{out()!.results.pass} of {out()!.results.total} pass the selection</span></Show>
    <Show when={cur() && mat(cur())}><span class="muted">Selected: {mat(cur())!.name}</span></Show>
    <span class="grow" />
    <button class="mode" title="Switch units" onClick={() => setUnits(imperial() ? "si" : "imperial")}>{imperial() ? "Imperial units" : "SI units"}</button>
    <span class="vsc"><button title="Chart" onClick={() => show("charts")}><Icon n="atlas" /></button><button title="Browse" onClick={() => show("browse")}><Icon n="tree" /></button><button title="Results" onClick={() => show("results")}><Icon n="results" /></button></span>
  </footer>
);

const App = () => {
  onMount(async () => {
    try {
      const o = await invoke<Options>("options"); setOpts(o); setThemeRaw(o.theme || "auto"); applyTheme();
      const d = await loadDb();
      if (d.userNote) openDialog(() => <Dialog title="Your materials"><p class="pre">{d.userNote}</p></Dialog>);
      setGoals(await invoke<Goal[]>("goal_list"));
      setOut(await invoke<Out>("state"));
      const start = await invoke<string | null>("startup_file");
      if (start) await loadOut("open", { path: start });
      const rec = await invoke<{ name: string; saved: string } | null>("recovery");
      if (rec) openDialog(() => (
        <Dialog title="Recover unsaved work?" okText="Recover" cancel="Discard" ok={async () => { const r = await call<Out>("recover", { keep: true }); if (r) { setOut(r); setBackstage(null); } }}>
          <p>Datum Atlas closed before “{rec.name}” was saved. A copy from {rec.saved} was kept.</p>
        </Dialog>
      ));
      checkUpdate(true);
      setInterval(() => { if (opts().autosave > 0) invoke("autosave").catch(() => {}); }, Math.max(1, opts().autosave) * 60000);
      listen("close-requested", () => D.confirmClose(() => invoke("quit")));
    } catch (e) { toast(String(e)); }
  });
  const keys = (e: KeyboardEvent) => {
    const ctrl = e.ctrlKey || e.metaKey, k = e.key.toLowerCase(), typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
    if (dialogs().length) return;
    if ((ctrl && k === "k") || (e.altKey && k === "q")) { e.preventDefault(); palette(); }
    else if (ctrl && k === "s") { e.preventDefault(); C.save(e.shiftKey); }
    else if (ctrl && k === "o") { e.preventDefault(); C.open(); }
    else if (ctrl && k === "n") { e.preventDefault(); setBackstage("home"); }
    else if (ctrl && k === "p") { e.preventDefault(); C.print(); }
    else if (ctrl && k === "z" && !typing) { e.preventDefault(); C.undo(); }
    else if (ctrl && k === "y" && !typing) { e.preventDefault(); C.redo(); }
    else if (e.key === "F1") { e.preventDefault(); D.shortcuts(); }
    else if (e.key === "Escape" && backstage() && out()) setBackstage(null);
  };
  return (
    <div class="app" ref={() => document.addEventListener("keydown", keys)}>
      <TitleBar update={update()?.available ? update()!.latest : null} />
      <Ribbon />
      <main class="work">
        <nav class="side"><For each={SIDE}>{x => x === "-" ? <hr /> : <button classList={{ on: view() === x[0] }} onClick={() => show(x[0])} title={x[2]}><Icon n={x[1]} /><span>{x[2]}</span></button>}</For></nav>
        <div class="content">
          <Show when={out()}>
            <Switch fallback={<ChartsPage />}>
              <Match when={view() === "setup"}><SetupPage /></Match>
              <Match when={view() === "browse"}><Browse /></Match>
              <Match when={view() === "datasheet"}><Datasheet /></Match>
              <Match when={view() === "stages"}><StagesPage /></Match>
              <Match when={view() === "results"}><ResultsPage /></Match>
              <Match when={view() === "compare"}><ComparePage /></Match>
              <Match when={view() === "index"}><IndexPage /></Match>
              <Match when={view() === "report"}><ReportPage /></Match>
              <Match when={view() === "data"}><DataPage /></Match>
              <Match when={view() === "sources"}><SourcesPage /></Match>
            </Switch>
          </Show>
        </div>
      </main>
      <Status />
      <Show when={backstage()}><Backstage /></Show>
      <For each={dialogs()}>{d => d()}</For>
      <Menu /><Toast />
    </div>
  );
};

render(() => <App />, document.getElementById("app")!);
