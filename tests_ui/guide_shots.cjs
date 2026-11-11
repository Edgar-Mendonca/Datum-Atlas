// Screenshots for the guide (docs/guide/img): the built window (ui/dist) with the real engine
// behind the mocked bridge (the bridge example, as in run.cjs), a light, stiff beam set up with
// an eco audit and a cast part, and numbered markers drawn on the page.
// Run: npm run build:ui && cargo build -p datum-atlas-core --example bridge && node tests_ui/guide_shots.cjs
const { chromium } = require("playwright");
const fs = require("fs"), http = require("http"), path = require("path"), { spawn } = require("child_process"), readline = require("readline");
const root = path.join(__dirname, "../ui/dist"), out = path.join(__dirname, "../docs/guide/img");
const types = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".woff": "font/woff", ".html": "text/html" };
const srv = http.createServer((q, r) => {
  const f = path.join(root, q.url === "/" ? "index.html" : decodeURIComponent(q.url.split("?")[0]));
  fs.readFile(f, (e, b) => { if (e) { r.writeHead(404); return r.end(); } r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" }); r.end(b); });
});
const exe = path.join(__dirname, "../target/debug/examples/bridge" + (process.platform === "win32" ? ".exe" : ""));
const eng = spawn(exe, [], { stdio: ["pipe", "pipe", "inherit"] }), waiting = [];
readline.createInterface({ input: eng.stdout }).on("line", l => waiting.shift()(JSON.parse(l)));
const PAGE_MOD = { Setup: "Select", Charts: "Select", Stages: "Select", Results: "Select", Compare: "Select", Index: "Select", Report: "Select", Browse: "Library", Datasheet: "Library", Data: "Library", Sources: "Library", "Eco audit": "Eco", Eco: "Eco", Process: "Make", Processes: "Make", Structures: "Microstructure" };
/** Opens a page: switches module first when the page is in another module. */
const goTo = async (p, t) => {
  if (t === "Eco") t = "Eco audit";
  if (!(await p.$(`.side button[title="${t}"]`))) { await p.click(".side .modsw"); await p.click(`.menu button:has(span:text-is("${PAGE_MOD[t]}"))`); await p.waitForTimeout(250); }
  await p.click(`.side button[title="${t}"]`);
};
const ask = (cmd, args) => new Promise(ok => { waiting.push(ok); eng.stdin.write(JSON.stringify({ cmd, args: args ?? {} }) + "\n"); });
const op = o => ask("apply", { op: o }).then(r => { if (r.err) throw new Error(r.err); });

(async () => {
  fs.mkdirSync(out, { recursive: true });
  await new Promise(ok => srv.listen(0, ok));
  const b = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
  const p = await b.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1.5 });
  await p.exposeFunction("__engine", async (cmd, args) => ask(cmd, args));
  await p.addInitScript(() => {
    const opts = { theme: "light", autosave: 0, beta: false, openLast: false, labels: false, guides: true, dpi: 300, units: "si", paper: "A4", landscape: true, company: "Northwind Engineering", logo: "" };
    const fixed = { options: opts, startup_file: null, recovery: null, check_update: null, recent: [], version: "0.3.3", copy_text: null, save_file: null };
    const engine = ["state", "apply", "history", "new_project", "templates", "database", "save_material", "delete_material", "goal_list", "databases", "open_database", "close_database", "import_preview", "import_materials", "eco_data", "substitutes", "processes", "microstructure"];
    window.__TAURI__ = { core: { invoke: async (c, a) => {
      if (engine.includes(c)) { const r = await window.__engine(c, a); if ("err" in r) throw r.err; return r.ok; }
      if (c === "set_options") return (await window.__engine("state", {})).ok;
      return c in fixed ? fixed[c] : null;
    } }, event: { listen: async () => () => {} } };
  });
  const wait = ms => p.waitForTimeout(ms);
  const url = `http://localhost:${srv.address().port}/`;
  await p.goto(url); await wait(900);

  /** Numbered markers: [selector, {x,y,width,height} or "text:…", number, nth match, "corner" | "right" | "in" | "below"]. */
  const mark = marks => p.evaluate(ms => {
    document.querySelectorAll(".gmark").forEach(e => e.remove());
    for (const [s, n, k, pos] of ms) {
      const find = q => q.startsWith("grp:") ? [...document.querySelectorAll(".ribbon .grp")].find(g => (g.querySelector(".cap")?.textContent ?? "").replace("?", "").trim().toLowerCase() === q.slice(4).toLowerCase())
        : q.startsWith("text:") ? [...document.querySelectorAll("body *")].filter(e => e.textContent.includes(q.slice(5)) && e.getBoundingClientRect().width > 0).sort((a, b) => a.textContent.length - b.textContent.length)[0]
        : document.querySelectorAll(q)[k ?? 0];
      const el = typeof s === "string" ? find(s) : null;
      const r = el ? el.getBoundingClientRect() : s;
      if (!r || !r.width) { console.warn("no marker target", s); continue; }
      const box = document.createElement("div");
      box.className = "gmark";
      Object.assign(box.style, { position: "fixed", left: `${r.x - 3}px`, top: `${r.y - 3}px`, width: `${r.width + 6}px`, height: `${r.height + 6}px`, border: "3px solid #E4572E", borderRadius: "6px", zIndex: 99999, pointerEvents: "none" });
      const dot = document.createElement("div");
      dot.textContent = n;
      const where = pos ?? (r.x < 14 || r.y < 14 ? "right" : "corner");
      const at = { corner: ["-14px", "-14px"], right: [`${r.width + 10}px`, `${Math.min(r.height / 2 - 10, 40)}px`], in: ["6px", "6px"], below: ["0px", `${r.height + 10}px`] }[where];
      Object.assign(dot.style, { position: "absolute", left: at[0], top: at[1], width: "26px", height: "26px", borderRadius: "13px", background: "#E4572E", color: "#fff", font: "700 14px/26px 'IBM Plex Sans', sans-serif", textAlign: "center" });
      box.append(dot);
      document.body.append(box);
    }
  }, marks);
  const shot = async (name, marks = [], clip) => {
    await mark(marks); await wait(150);
    if (typeof clip === "string") clip = await p.evaluate(s => { const r = document.querySelector(s).getBoundingClientRect(); return { x: Math.max(0, r.x - 24), y: Math.max(0, r.y - 24), width: Math.min(innerWidth, r.width + 48), height: Math.min(innerHeight, r.height + 48) }; }, clip);
    await p.screenshot({ path: path.join(out, `${name}.png`), clip });
    await mark([]); console.log("shot", name);
  };
  const side = async t => { await goTo(p, t); await wait(600); };
  const tab = async t => { if (!(await p.$(`.tabs button:text-is('${t}')`))) await side(t === 'Data' ? 'Browse' : 'Charts'); await p.click(`.tabs button:text-is('${t}')`); await wait(250); };
  const TOP = { x: 0, y: 0, width: 1440, height: 160 };
  const esc = async () => { await p.keyboard.press("Escape"); await wait(200); };

  // 1. Home (File › Home).
  await shot("home", [[".tiles.mods", 1], [".home .tiles", 2, 1], ["text:Add a material", 3], [".bs aside", 4, 0, "in"]]);

  // A project: a light, stiff beam, three materials to compare, an eco audit with two options, a cast part.
  await ask("new_project", { template: "beam" });
  await op({ Name: { name: "Bracket" } });
  await op({ Notes: { text: "A bracket loaded in bending. Its stiffness is fixed; make it as light as possible. It works below 120 °C and must cost less than 20 USD/kg." } });
  const st = (await ask("state")).ok.project.stages;
  st.limits.push({ prop: "price", min: null, max: 20, flag: null, rating: null, goal: false });
  await op({ SetStages: { stages: st } });
  await op({ Compare: { ids: ["al-6061-t6", "mg-az31b", "cfrp-qi"], radar: [] } });
  const part = (material, mass, process, eol) => ({ name: "Bracket", material, mass, qty: 1, process, removed: 0.2, recycled: 30, eol });
  const use = { kind: "mobile", kw: 0, hours: 0, days: 365, vehicle: "car-petrol", km: 15000 };
  await op({ Eco: { eco: { grid: "eu", options: [
    { name: "Steel", parts: [part("steel-1020", 2.4, "deform", "recycle")], legs: [{ mode: "truck", km: 800 }], use, years: 12 },
    { name: "Aluminium", parts: [part("al-6061-t6", 1.1, "extrude-metal", "recycle")], legs: [{ mode: "truck", km: 800 }], use, years: 12 } ] } } });
  await op({ Process: { sel: { material: "al-a356-t6", mass: 1.1, section: 4, shape: "3d-hollow", tol: 0.5, rough: 0, batch: 20000, price: 0, join: "steel-1020", purpose: "corrosion" } } });
  await p.goto(url); await wait(900);
  await p.click(".bs aside .back"); await wait(500);
  await side("Charts");

  // 2. The window.
  const tabsBox = await p.evaluate(() => { const t = [...document.querySelectorAll(".tabs > button")]; const a = t[0].getBoundingClientRect(), z = t[t.length - 1].getBoundingClientRect(); return { x: a.x + 2, y: a.y + 2, width: z.right - a.x - 2, height: a.height - 4 }; });
  await shot("window", [[tabsBox, 1, 0, "right"], [".ribbon", 2], [".side", 3, 0, "in"], [".cbar", 4], [".cbox", 5, 0, "in"], [".spanel", 6], [".status", 7], ["text:Tell me what you want", 8]]);
  // 3. The chart bar and tools, the selection chips and the legend.
  await shot("chart-tools", [[".goalpick", 1], ["text:Why E", 2], [".seg.tools", 3], [".selchips", 4], [".clegend", 5]], { x: 66, y: 165, width: 1085, height: 225 });
  // 4. Why this index?
  await p.click("text=Why E½/ρ?"); await wait(300);
  await shot("why", [], ".dlg"); await esc();
  // 5. Ribbon tabs.
  await tab("Select"); await shot("select-tab", [["grp:Goal", 1], ["grp:Stages", 2], ["grp:Index", 3], ["grp:Results", 4]], TOP);
  await tab("Chart"); await shot("chart-tab", [["grp:Chart", 1], ["grp:Axes", 2], ["grp:Tools", 3], ["grp:Show", 4], ["grp:Share", 5]], TOP);
  await tab("Report"); await shot("report-tab", [["grp:Compare", 1], ["grp:Report", 2], ["grp:Eco", 3], ["grp:Process", 4]], TOP);
  await tab("Home");
  // 6. Views.
  await side("Setup"); await shot("setup");
  await side("Stages"); await shot("stages");
  await side("Results"); await shot("results", [[".tbl tbody tr", 1, 0], ["text:× best", 2]]);
  await side("Browse"); await shot("browse", [[".tree", 1, 0, "in"], [".find", 2], [".btable", 3, 0, "in"]]);
  await side("Compare"); await shot("compare", [[".cmptable", 1, 0, "in"], [".cmpradar", 2]]);
  await p.click(".cmp th button.lnk >> nth=0"); await wait(600);
  await shot("datasheet", [[".shead .inline", 1], [".sfacts", 2], [".micrograph", 3], [".tbl.dsheet", 4, 1]]);
  await p.click('.micrograph button:has-text("Typical structure")'); await wait(900);
  await shot("structures", [[".structpage .focus", 1], [".structpage .sgrid", 2]]);
  await p.click(".side .modsw"); await wait(300);
  await shot("modules", [], { x: 0, y: 150, width: 420, height: 520 });
  await p.keyboard.press("Escape"); await p.mouse.click(700, 600); await wait(200);
  await side("Index"); await shot("index");
  await side("Eco"); await shot("eco", [[".otabs", 1], [".ecoparts", 2], [".ecokpi", 3], [".ecodom", 4], [".ecocharts", 5]]);
  await p.click(".otabs button >> nth=1"); await wait(300);
  await p.evaluate(() => document.querySelector(".ecogrid > div:last-child section:last-child")?.scrollIntoView()); await wait(300);
  await shot("eco-options", [], ".ecogrid > div:last-child section:last-child");
  await p.click('.ecoparts button.lnk:has-text("greener?")'); await wait(500);
  await shot("greener", [], ".dlg"); await esc();
  await side("Process"); await shot("process", [[".procpage .panel", 1, 0], [".procpage .panel", 2, 1], [".procpage svg", 3, 0], [".proctable", 4]]);
  await side("Report"); await shot("report", [[".ptools", 1]]);
  await side("Data"); await shot("data");
  await side("Sources"); await shot("sources");
  // 7. Help: a ? card, the assistant dialog.
  await side("Charts");
  await p.click(".cbar .help >> nth=0"); await wait(300); await shot("helpcard", [], ".dlg"); await esc();
  await tab("Help"); await shot("help-tab", [["grp:Learn", 1], ["grp:Assistant", 2]], TOP);
  await p.click('.ribbon button[title^="Connect an AI assistant"]'); await wait(300); await shot("assistant", [], ".dlg"); await esc();
  await tab("Home");
  // 8. Print.
  await p.click(".tabs .file"); await wait(200);
  await p.click(".bs aside button:has-text('Print')"); await wait(1200);
  await shot("print");
  await b.close(); srv.close(); eng.kill();
})().catch(e => { console.error(e); eng.kill(); process.exit(1); });
