// File (backstage): Home with design problems and recent projects, export, print, options, updates.
import { For, Show, createSignal, createResource, onMount, JSX } from "solid-js";
import { PAPERS, PrintPage } from "./print";
import { Options, Out } from "./api";
import { backstage, setBackstage, call, loadOut, out, setOut, opts, setOpts, toast, setThemeRaw, applyTheme, chart, db } from "./store";
import { C } from "./cmds";
import * as D from "./dialogs";
import { Icon, Text, Pick, Check } from "./ui";
import { listen, invoke } from "./api";
import { RoadmapPage, soon, Chip } from "./roadmap";
import { MiniChart } from "./pages";
import { exportChart } from "./chart";
import { editMaterial } from "./material";

type Tpl = { id: string; name: string; desc: string; slope: number | null };
type Info = { current: string; latest: string | null; beta: boolean; available: boolean; notes: string; published: string; url: string | null; sums: string | null; size: number; page: string };
export const [update, setUpdate] = createSignal<Info | null>(null);
export const [checking, setChecking] = createSignal(false), [checkedAt, setCheckedAt] = createSignal("");
export const checkUpdate = async (quiet: boolean) => {
  setChecking(true);
  try { setUpdate(await invoke<Info>("check_update")); setCheckedAt(new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })); if (!quiet) toast(update()?.available ? `Version ${update()!.latest} is available.` : "You have the latest version.", true); }
  catch (e) { if (!quiet) toast(`Could not check for updates: ${e}`); }
  finally { setChecking(false); }
};
const GH = "https://github.com/Edgar-Mendonca/Datum-Atlas";
/** Links to the project on GitHub: code, releases (change log) and issues (report a problem). */
export const GitHubLinks = () => (
  <div class="ghlinks">
    <For each={[["", "GitHub", "The project and its code"], ["/releases", "Releases", "Every version and its change log"], ["/issues/new", "Report an issue", "Tell us about a bug or an idea"]]}>{([path, l, t]) =>
      <button class="btn" title={t} onClick={() => call("open_url", { url: GH + path })}><svg class="ic"><use href={path === "/issues/new" ? "#i-bug" : path ? "#i-tag" : "#i-github"} /></svg>{l}</button>}</For>
  </div>
);

/** Release notes: headings, bullets and bold, enough for the changelog. */
const md = (s: string) => s.split("\n").map(l => {
  const e = l.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>").replace(/`(.+?)`/g, "<code>$1</code>");
  return l.startsWith("## ") ? `<h3>${e.slice(3)}</h3>` : l.startsWith("- ") ? `<li>${e.slice(2)}</li>` : e.trim() ? `<p>${e}</p>` : "";
}).join("");

const Home = () => {
  const [tpls] = createResource(() => call<Tpl[]>("templates"));
  const [recent] = createResource(() => call<{ path: string; name: string; modified: string }[]>("recent"));
  const neu = (template: string | null) => loadOut("new_project", { template });
  return (
    <div class="home">
      <div>
        <h1>Good {new Date().getHours() < 12 ? "morning" : new Date().getHours() < 18 ? "afternoon" : "evening"}</h1>
        <p class="muted">Start a selection: every material on a property chart, then limits and a selection line. {db().materials.length} materials are ready, each value with its reference.</p>
        <div class="tiles">
          <For each={tpls() ?? []}>{t => <button class="tile" onClick={() => neu(t.id)}><div class="pic"><MiniChart slope={t.slope} /><small>{t.slope ? `slope ${t.slope}` : "chart"}</small></div><b>{t.name}</b><span>{t.desc}</span></button>}</For>
          <button class="tile" onClick={() => C.open()}><div class="pic"><svg class="ic"><use href="#i-open" /></svg></div><b>Open</b><span>A Datum Atlas selection (.datlas).</span></button>
          <button class="tile" onClick={() => { setBackstage(null); editMaterial(); }}><div class="pic"><svg class="ic"><use href="#i-material" /></svg></div><b>Add a material</b><span>Your own material, with a source for each value.</span></button>
          <button class="tile" onClick={() => setBackstage("roadmap")}><div class="pic"><svg class="ic"><use href="#i-list" /></svg></div><b>Roadmap</b><span>Everything Datum Atlas will do, and when.</span></button>
        </div>
        <h2>Recent</h2>
        <table class="recent"><tbody>
          <For each={recent() ?? []} fallback={<tr><td class="muted">Selections you open or save appear here.</td></tr>}>{r => <tr class="f" onClick={() => loadOut("open", { path: r.path })}><td><b>{r.name}</b><div class="p">{r.path}</div></td><td class="p">{r.modified}</td></tr>}</For>
        </tbody></table>
      </div>
      <UpdateCard />
    </div>
  );
};

const UpdateCard = () => {
  const [pct, setPct] = createSignal<number | null>(null);
  const [ver] = createResource(() => call<string>("version"));
  onMount(() => { listen("update-progress", (p: number) => setPct(p)); if (!update()) checkUpdate(true); });
  const install = async () => { const u = update(); if (!u?.url) return; setPct(0); await call("install_update", { url: u.url, sums: u.sums }); setPct(null); };
  return (
    <div class="card">
      <div class="brand"><img src="logo.svg" alt="" /><div><b>Datum Atlas</b><span class="muted">Version {ver()}{opts().beta ? " · beta channel" : ""}</span></div></div>
      <Show when={update()} fallback={<><span class="pill">{checking() ? "Checking for updates…" : "Not checked yet"}</span><button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check for updates"}</button><GitHubLinks /></>}>{u => <>
        <Show when={u().available} fallback={<span class="pill ok">You have the latest version</span>}><span class="pill warn">Version {u().latest}{u().beta ? " (beta)" : ""} is ready</span></Show>
        <Show when={u().available && u().url}><button class="btn pri wide" disabled={pct() !== null} onClick={install}>{pct() === null ? `Update now (${Math.round(u().size / 1048576)} MB)` : `Downloading… ${pct()}%`}</button></Show>
        <Show when={pct() !== null}><div class="prog"><i style={{ width: `${pct()}%` }} /></div></Show>
        <button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check again"}</button>
        <Show when={checkedAt() && !checking()}><span class="muted small">Checked at {checkedAt()}</span></Show>
        <GitHubLinks />
        <div class="notes"><h4>What's new{u().latest ? ` in ${u().latest}` : ""}</h4><div innerHTML={md(u().notes || "Release notes will appear here.")} /></div>
      </>}</Show>
    </div>
  );
};
const Export = () => (
  <div><h1>Export</h1>
    <Show when={chart()} fallback={<p class="muted">Open a chart first, then come back here to export it.</p>}>{c => <>
      <p class="muted">Exporting <b>{c().name}</b> with every material and its family legend.</p>
      <div class="tiles">
        <button class="tile" onClick={() => exportChart("svg")}><div class="pic"><svg class="ic"><use href="#i-image" /></svg><small>.svg</small></div><b>SVG vector</b><span>Sharp at any size; text stays editable in Inkscape, Illustrator or Word.</span></button>
        <button class="tile" onClick={() => exportChart("png")}><div class="pic"><svg class="ic"><use href="#i-image" /></svg><small>.png</small></div><b>PNG picture</b><span>For slides and documents, at {opts().dpi} dpi (Options).</span></button>
        <button class="tile" onClick={() => setBackstage("print")}><div class="pic"><svg class="ic"><use href="#i-print" /></svg></div><b>PDF</b><span>File › Print and choose “Save as PDF”: vector, with headers and footers.</span></button>
        <button class="tile soon" onClick={() => soon("B11")}><div class="pic"><svg class="ic"><use href="#i-copy" /></svg></div><b>Copy picture</b><span>Paste a chart straight into Word or PowerPoint.</span><Chip id="B11" /></button>
      </div>
    </>}</Show>
  </div>
);

/** One option: name and a one-line explanation on the left, the control on the right. */
const Opt = (p: { l: string; hint?: string; children: JSX.Element }) => <div class="opt"><div><b>{p.l}</b>{p.hint && <small>{p.hint}</small>}</div><div class="ctl">{p.children}</div></div>;
const Sec = (p: { t: string; about: string; children: JSX.Element }) => <section class="osec"><h3>{p.t}</h3><p class="muted">{p.about}</p>{p.children}</section>;

const OptionsPage = () => {
  const [o, setO] = createSignal<Options>({ ...opts() });
  const save = async () => { const r = await call<Out>("set_options", { opts: o() }); if (r) { setOpts(o()); setOut(r); toast("Options saved.", true); } };
  const put = <K extends keyof Options>(k: K, v: Options[K]) => setO({ ...o(), [k]: v });
  return (
    <div class="options"><h1>Options</h1>
      <p class="muted">Settings for Datum Atlas on this computer. Defaults for new charts apply when a selection is created.</p>
      <Sec t="General" about="How the window looks.">
        <Opt l="Theme" hint="Auto follows the Windows light or dark setting. Charts stay on white paper."><Pick value={o().theme || "auto"} set={v => { put("theme", v); setThemeRaw(v); applyTheme(); }} options={[["auto", "Auto (follow Windows)"], ["light", "Light"], ["dark", "Dark"]]} /></Opt>
        <Opt l="Units" hint="SI now; the imperial set comes in 0.1.1."><span class="inline"><Pick value="si" set={() => soon("A4")} options={[["si", "SI (kg/m³, GPa, MPa, °C)"], ["imperial", "Imperial (lb/in³, 10⁶ psi, ksi, °F)"]]} /><Chip id="A4" /></span></Opt>
      </Sec>
      <Sec t="Start-up and saving" about="What happens when Datum Atlas starts, and how your work is protected.">
        <Opt l="When Datum Atlas starts" hint="Open the File page, or reopen the selection you used last."><Pick value={o().openLast ? "last" : "home"} set={v => put("openLast", v === "last")} options={[["home", "Show the File page"], ["last", "Reopen the last selection"]]} /></Opt>
        <Opt l="Recovery copy every" hint="A copy is kept while you work, so a crash or power cut loses little. 0 turns it off."><span class="inline"><input type="number" min="0" style={{ width: "70px" }} value={o().autosave} onChange={e => put("autosave", +e.currentTarget.value)} /> minutes</span></Opt>
      </Sec>
      <Sec t="New charts" about="How a new selection's charts look.">
        <Opt l="Guide lines" hint="Dashed lines of slope 1, 2 and 3 for the indices E/ρ, E^½/ρ and E^⅓/ρ."><Check label="Show guide lines" value={o().guides} set={v => put("guides", v)} /></Opt>
        <Opt l="Material labels" hint="A name beside every bubble (busy with many materials)."><Check label="Show labels" value={o().labels} set={v => put("labels", v)} /></Opt>
        <Opt l="Picture resolution" hint="For PNG export."><Pick value={String(o().dpi)} set={v => put("dpi", +v)} options={[["150", "150 dpi"], ["300", "300 dpi"], ["600", "600 dpi"]]} /></Opt>
      </Sec>
      <Sec t="Printing and branding" about="Your organisation and logo can go in page headers (File › Print).">
        <Opt l="Paper" hint="Default paper for printouts."><span class="inline"><Pick value={o().paper || "A4"} set={v => put("paper", v)} options={PAPERS.map(p => p[0])} /><Pick value={o().landscape ? "1" : "0"} set={v => put("landscape", v === "1")} options={[["0", "Portrait"], ["1", "Landscape"]]} /></span></Opt>
        <Opt l="Organisation" hint="Shown in headers and footers with &[Company]."><Text value={o().company} set={v => put("company", v)} /></Opt>
        <Opt l="Logo" hint="PNG or JPG for page headers (&[Logo])."><span class="inline">{o().logo && <img src={o().logo} class="logoprev" />}<button class="btn sm" onClick={async () => { const l = await call<string | null>("pick_logo"); if (l) put("logo", l); }}>Choose picture…</button>{o().logo && <button class="btn sm" onClick={() => put("logo", "")}>Remove</button>}</span></Opt>
      </Sec>
      <Sec t="Updates" about="Datum Atlas checks GitHub for new versions when it starts. Updates install from File › Updates.">
        <Opt l="Beta versions" hint="Get new features early. They may have bugs."><Check label="Get beta versions" value={o().beta} set={v => put("beta", v)} /></Opt>
      </Sec>
      <p class="osave"><button class="btn pri" onClick={save}>Save options</button><span class="muted">Datum Atlas keeps everything on this computer. Its only connection is the update check to GitHub.</span></p>
    </div>
  );
};

export const Backstage = () => (
  <div class="bs">
    <aside>
      <button class="back" onClick={() => out() && setBackstage(null)} title="Back">←</button>
      <For each={[["home", "home", "Home"], ["new", "new", "New"], ["open", "open", "Open"], ["import", "import", "Import"], ["save", "save", "Save"], ["saveas", "save", "Save As"], ["export", "export", "Export"], ["print", "print", "Print"]]}>{([k, i, l]) =>
        <button classList={{ on: backstage() === k }} onClick={() => {
          if (k === "open") C.open(); else if (k === "import") soon("A8"); else if (k === "new") setBackstage("home"); else if (k === "save") C.save(); else if (k === "saveas") C.save(true);
          else setBackstage(k);
        }}><Icon n={i} size={22} />{l}</button>}</For>
      <span class="grow" />
      <button classList={{ on: backstage() === "roadmap" }} onClick={() => setBackstage("roadmap")}><Icon n="list" size={22} />Roadmap</button>
      <button classList={{ on: backstage() === "options" }} onClick={() => setBackstage("options")}><Icon n="opts" size={22} />Options</button>
      <button classList={{ on: backstage() === "about" }} onClick={() => setBackstage("about")}><Icon n="upd" size={22} />Updates</button>
    </aside>
    <section class="bsmain">
      <Show when={backstage() === "home"}><Home /></Show>
      <Show when={backstage() === "export"}><Export /></Show>
      <Show when={backstage() === "options"}><OptionsPage /></Show>
      <Show when={backstage() === "roadmap"}><RoadmapPage /></Show>
      <Show when={backstage() === "print"}><PrintPage /></Show>
      <Show when={backstage() === "about"}><div class="about"><h1>Updates</h1><UpdateCard /><p><button class="btn" onClick={() => D.shortcuts()}>Keyboard shortcuts</button></p></div></Show>
    </section>
  </div>
);
