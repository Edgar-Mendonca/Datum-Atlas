// UI tests: the built window (ui/dist) in Chromium, with the real engine behind a mocked Tauri
// bridge (cargo run -p datum-atlas-core --example bridge answers one JSON line per request).
// Run: npm run test:ui   (or: npm run build:ui && cargo build -p datum-atlas-core --example bridge && node tests_ui/run.cjs)
// SHOTS=dir saves a screenshot of each step there.
const { chromium } = require("playwright");
const fs = require("fs"), http = require("http"), path = require("path"), { spawn } = require("child_process"), readline = require("readline");
const root = path.join(__dirname, "../ui/dist");
const types = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".woff": "font/woff", ".html": "text/html" };
const srv = http.createServer((q, r) => {
  const f = path.join(root, q.url === "/" ? "index.html" : decodeURIComponent(q.url.split("?")[0]));
  fs.readFile(f, (e, b) => { if (e) { r.writeHead(404); return r.end(); } r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" }); r.end(b); });
});
let failed = 0;
const check = (name, ok, info = "") => { console.log(`${ok ? "ok  " : "FAIL"} ${name}${ok ? "" : "  " + info}`); if (!ok) failed++; };

// The engine: requests go in order, answers come back in order.
const exe = path.join(__dirname, "../target/debug/examples/bridge" + (process.platform === "win32" ? ".exe" : ""));
const eng = spawn(exe, [], { stdio: ["pipe", "pipe", "inherit"] }), waiting = [];
readline.createInterface({ input: eng.stdout }).on("line", l => waiting.shift()(JSON.parse(l)));
const ask = (cmd, args) => new Promise(ok => { waiting.push(ok); eng.stdin.write(JSON.stringify({ cmd, args: args ?? {} }) + "\n"); });

(async () => {
  await new Promise(ok => srv.listen(0, ok));
  const url = `http://localhost:${srv.address().port}/`;
  const b = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on("pageerror", e => errors.push(e.message));
  await p.exposeFunction("__engine", async (cmd, args) => ask(cmd, args));
  await p.addInitScript(() => {
    const opts = { theme: "light", autosave: 0, beta: false, openLast: false, labels: false, guides: true, dpi: 300, paper: "A4", landscape: true, company: "", logo: "" };
    const fixed = { options: opts, startup_file: null, recovery: null, check_update: null, recent: [], version: "test", copy_text: null, save_file: null, set_options: undefined };
    const engine = ["state", "apply", "history", "new_project", "templates", "database", "save_material", "delete_material"];
    window.__calls = [];
    window.__TAURI__ = { core: { invoke: async (c, a) => {
      window.__calls.push([c, a]);
      if (engine.includes(c)) { const r = await window.__engine(c, a); if ("err" in r) throw r.err; return r.ok; }
      if (c === "set_options") { const r = await window.__engine("state", {}); return r.ok; }
      return c in fixed ? fixed[c] : null;
    } }, event: { listen: async () => () => {} } };
  });
  const shot = async n => { if (process.env.SHOTS) await p.screenshot({ path: path.join(process.env.SHOTS, `${n}.png`) }); };
  await p.goto(url);
  await p.waitForTimeout(800);
  await shot("01-home");

  // 1. File › Home: design problems, and opening one shows its chart.
  check("Home lists the design problems", (await p.$$(".home .tile")).length >= 8);
  await p.getByText("Light, stiff beam", { exact: true }).click();
  await p.waitForTimeout(600);
  check("the backstage closes on a template", !(await p.$(".bs")));
  const nb = await p.$$eval(".cbox ellipse.bub", e => e.length), nh = await p.$$eval(".cbox path.hull", e => new Set(e.map(x => x.getAttribute("fill"))).size);
  check("the chart draws a bubble per material", nb >= 100, String(nb));
  check("every family has its envelope", nh === 7, String(nh));
  check("log ticks are labelled in decades", (await p.textContent(".cbox svg")).includes("1000") && (await p.textContent(".cbox svg")).includes("0.01"));
  check("the line shows its index and value", /E\^½\/ρ = 0\.00316/.test(await p.textContent(".cbox svg")), await p.textContent(".cbox svg"));
  const pass0 = +(await p.textContent(".pcount b"));
  check("the stage panel counts what passes", pass0 > 5 && pass0 < 100, String(pass0));
  await shot("02-beam");

  // 2. Hover shows the ranges; a click shows the material beside the chart.
  const box = await p.$eval(".cbox", e => { const r = e.getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; });
  const bub = await p.$eval('.cbox ellipse.bub[data-id="al-6061-t6"]', e => { const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; });
  await p.mouse.move(bub.x, bub.y);
  await p.waitForTimeout(200);
  const tip = await p.textContent(".gtip").catch(() => "");
  check("hover shows the material and its ranges", tip.includes("Aluminium") && tip.includes("kg/m³") && tip.includes("GPa"), tip);
  const under = await p.textContent(".gtip b");
  await p.mouse.click(bub.x, bub.y);
  await p.waitForTimeout(200);
  check("a click (even on the selection line) shows its datasheet beside the chart", (await p.textContent(".mcard h3").catch(() => "")) === under, under);
  await p.mouse.dblclick(bub.x, bub.y);
  await p.waitForTimeout(300);
  const sheet = await p.textContent(".sheetpage").catch(() => "");
  check("a double-click opens the full datasheet with references", sheet.includes(under) && sheet.includes("Young's modulus") && sheet.includes("References") && /MIL-HDBK|Tables of Physical/.test(sheet), sheet.slice(0, 200));
  check("missing values show as no data, not zero", sheet.includes("no data"));
  await shot("03-datasheet");
  await p.click('.side button[title="Charts"]');
  await p.waitForTimeout(300);

  // 3. Dragging the selection line changes the pass count (moving it down lets more through).
  const h = await p.$eval(".cbox circle.selh", e => { const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; });
  await p.mouse.move(h.x, h.y); await p.mouse.down(); await p.mouse.move(h.x, h.y + 60, { steps: 6 }); await p.mouse.up();
  await p.waitForTimeout(400);
  const pass1 = +(await p.textContent(".pcount b"));
  check("dragging the index line down lets more materials pass", pass1 > pass0, `${pass0} → ${pass1}`);
  check("the drag is one Select edit", (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply" && c[1].op.Select).length)) === 1);

  // 4. A limit stage; results match the stages and give reasons.
  await p.click('.side button[title="Stages"]');
  await p.waitForTimeout(200);
  await p.getByText("Add a limit", { exact: true }).first().click();
  await p.waitForTimeout(300);
  await p.fill(".limit input[placeholder=min]", "150"); await p.keyboard.press("Tab");
  await p.waitForTimeout(300);
  await shot("04-stages");
  await p.click('.side button[title="Results"]');
  await p.waitForTimeout(300);
  const st = await p.evaluate(async () => { const s = await window.__TAURI__.core.invoke("state"); return { pass: s.results.pass, rows: s.results.rows.filter(r => r.pass).map(r => r.name) }; });
  const listed = await p.$$eval(".tbl.results tbody tr", rs => rs.map(r => r.children[1].textContent));
  check("the results table lists exactly what passes", JSON.stringify(listed) === JSON.stringify(st.rows), `${listed.length} vs ${st.rows.length}`);
  check("results are ranked with × best", (await p.textContent(".tbl.results thead")).includes("× best") && (await p.textContent(".tbl.results tbody tr td:nth-child(5)")) === "1");
  await p.click(".resultspage .chk input");
  await p.waitForTimeout(200);
  check("failures say why", /Max service temperature .* < 150 °C/.test(await p.textContent(".tbl.results")));
  await shot("05-results");

  // 5. Browse: tree filters the table; selection and Ctrl+C give TSV.
  await p.click('.side button[title="Browse"]');
  await p.waitForTimeout(300);
  const all = await p.$$eval(".bgrid tbody tr", r => r.length);
  await p.click(".tree button:has-text('Polymers')");
  await p.waitForTimeout(200);
  const poly = await p.$$eval(".bgrid tbody tr", r => r.length);
  check("the family tree filters the table", all >= 100 && poly > 10 && poly < all, `${all} → ${poly}`);
  const c1 = await p.$(".bgrid tbody tr:nth-child(2) td:nth-child(1)"), c2 = await p.$(".bgrid tbody tr:nth-child(4) td:nth-child(5)");
  await c1.click(); await c2.click({ modifiers: ["Shift"] });
  await p.keyboard.press("Control+c");
  await p.waitForTimeout(200);
  const copied = await p.evaluate(() => window.__calls.filter(c => c[0] === "copy_text").pop()?.[1].text ?? "");
  check("Ctrl+C copies the selected cells as TSV with headings", copied.split("\n").length === 4 && copied.split("\n")[0].split("\t").length === 5 && copied.includes("Young's modulus (GPa)"), JSON.stringify(copied.slice(0, 120)));
  check("Sum/Average/Count of the selection", (await p.textContent(".fbar .agg")).includes("Count"));
  await shot("06-browse");

  // 6. Add a material of your own: it needs a source, then appears everywhere.
  await p.click(".btools .btn.pri");
  await p.waitForTimeout(300);
  check("the Add Material window explains what is needed", (await p.textContent(".howto")).includes("Density") && (await p.textContent(".howto")).includes("Where the values come from"));
  await p.fill('.amform input[placeholder^="e.g. Aluminium"]', "Test alloy X");
  await p.fill('.amvals tbody tr:nth-child(1) td:nth-child(2) input', "2750");
  await p.click(".dtabs button:has-text('Mechanical')");
  await p.fill('.amvals tbody tr:nth-child(1) td:nth-child(2) input', "71");
  await p.fill('.amvals tbody tr:nth-child(1) td:nth-child(3) input', "73");
  await p.click(".dlg footer .btn.pri");
  await p.waitForTimeout(300);
  check("saving without a source asks for one", (await p.textContent("#toast").catch(() => "")).includes("come from") && !!(await p.$(".addmat")));
  await p.fill('.amform input[placeholder^="e.g. Supplier"]', "Test report 7");
  await shot("07-addmat");
  await p.click(".dlg footer .btn.pri");
  await p.waitForTimeout(600);
  check("the material is saved and its datasheet opens", (await p.textContent(".sheetpage h1").catch(() => "")) === "Test alloy X" && (await p.textContent(".refs")).includes("Test report 7"));
  await p.click('.side button[title="Charts"]');
  await p.waitForTimeout(300);
  check("your material is on the chart", !!(await p.$('.cbox ellipse.bub[data-id="u-1"]')));

  // 7. Ribbon fits without scrolling; planned commands explain when they arrive.
  for (const w of [1280, 1600]) {
    await p.setViewportSize({ width: w, height: 900 });
    for (const t of ["Home", "Select", "Chart", "Data", "Report", "View", "Help"]) {
      await p.click(`.tabs button:has-text("${t}")`);
      await p.waitForTimeout(80);
      const over = await p.$eval(".ribbon", e => e.scrollWidth - e.clientWidth);
      if (over > 1) check(`ribbon ${t} fits at ${w}px`, false, `${over}px over`);
    }
  }
  check("every ribbon tab fits at 1280 and 1600 px", true);
  await p.setViewportSize({ width: 1440, height: 900 });
  await p.click('.tabs button:has-text("Report")');
  await p.click('.ribbon button[title^="Eco Audit"]');
  await p.waitForTimeout(200);
  check("a planned command opens its card", (await p.textContent(".dlg").catch(() => "")).includes("0.4"));
  await p.keyboard.press("Escape");

  // 8. Every view prints; the chart prints as a vector page.
  for (const [v, want] of [["Charts", ".ppage svg.chartsvg"], ["Datasheet", ".pflow .sheetpage"], ["Results", ".pflow .ptable"]]) {
    await p.click(`.side button[title="${v}"]`);
    await p.waitForTimeout(200);
    await p.keyboard.press("Control+p");
    await p.waitForTimeout(400);
    check(`${v} has a print preview`, !!(await p.$(`.pprev ${want}`)), v);
    if (v === "Charts") await shot("08-print");
    await p.click(".bs aside .back");
    await p.waitForTimeout(200);
  }

  // 9. Help, glossary, Options; no script errors.
  await p.click('.tabs button:has-text("Help")');
  await p.click('.ribbon button[title^="Bubble, envelope"]');
  await p.fill(".dlg input.find", "index");
  check("the glossary finds terms", (await p.textContent(".gloss")).includes("Performance index"));
  await p.keyboard.press("Escape");
  await p.click(".tabs .file"); await p.click(".bs aside button:has-text('Options')");
  await p.waitForTimeout(200);
  const rows = await p.$$eval(".opt", os => os.map(o => Math.abs(o.children[1].getBoundingClientRect().left - os[0].children[1].getBoundingClientRect().left)));
  check("Options controls line up", rows.length > 6 && Math.max(...rows) < 2, JSON.stringify(rows));
  check("no script errors", errors.length === 0, errors.join(" | "));

  await b.close(); srv.close(); eng.kill();
  console.log(failed ? `${failed} failed` : "all passed");
  process.exit(failed ? 1 : 0);
})().catch(e => { console.error(e); eng.kill(); process.exit(1); });
