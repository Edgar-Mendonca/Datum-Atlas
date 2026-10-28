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
    const fixed = { options: opts, startup_file: null, recovery: null, check_update: null, recent: [], version: "test", copy_text: null, save_file: null, set_options: undefined, export_results: "C:/Selection results.xlsx", export_database: "C:/My materials.datdb" };
    const engine = ["state", "apply", "history", "new_project", "templates", "database", "save_material", "delete_material", "goal_list", "databases", "open_database", "close_database", "import_preview", "import_materials"];
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
  check("the line shows its index and value", /E½\/ρ = 0\.00\d+/.test(await p.textContent(".cbox svg")), await p.textContent(".cbox svg"));
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
  check("the goal added its suggested limit, marked", (await p.textContent(".stagespage")).includes("from the goal"));
  await p.locator(".limit input[placeholder=min]").last().fill("150"); await p.keyboard.press("Tab");
  await p.waitForTimeout(300);
  await shot("04-stages");
  await p.click('.side button[title="Results"]');
  await p.waitForTimeout(300);
  const st = await p.evaluate(async () => { const s = await window.__TAURI__.core.invoke("state"); return { pass: s.results.pass, rows: s.results.rows.filter(r => r.pass).map(r => r.name) }; });
  const listed = await p.$$eval(".tbl.results tbody tr", rs => rs.map(r => r.children[1].querySelector("b").textContent));
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
  await p.waitForSelector(".sheetpage h1", { timeout: 8000 }).catch(() => {});
  const h1 = await p.textContent(".sheetpage h1").catch(() => "");
  check("the material is saved and its datasheet opens", h1.startsWith("Test alloy X") && (await p.textContent(".refs")).includes("Test report 7"), h1 + " | " + await p.textContent("#toast").catch(() => ""));
  await p.click('.side button[title="Charts"]');
  await p.waitForTimeout(300);
  check("your material is on the chart", !!(await p.$('.cbox ellipse.bub[data-id="u-1"]')));

  // 6b. 0.1.1: design goals, units, bar charts, search, Sources, "?" help, Home tiles.
  await p.click('.side button[title="Charts"]');
  await p.waitForTimeout(300);
  check("the goal list has 25 goals and Explore", (await p.$$eval(".goalpick select option", o => o.length)) === 26);
  const state = () => p.evaluate(async () => await window.__TAURI__.core.invoke("state"));

  // Selections: two-chart goals, several lines and bands, keys, shapes, pass-through, weighted ranking.
  await p.selectOption(".goalpick select", "beam-stiff-strong");
  await p.waitForTimeout(500);
  let s0 = await state();
  check("a two-constraint goal adds its second chart with its own line", s0.project.charts.length === 2 && s0.project.charts[1].goal === "beam-stiff-strong/2" && s0.project.charts[1].sels.length === 1 && (await p.textContent(".ctabs")).includes("Strength of the light beam"));
  await p.click('.cbar button:has-text("+ Line")');
  await p.waitForTimeout(400);
  check("+ Line adds a second line, drawn with its own handle and index", (await p.$$eval(".cbox circle.selh", e => e.length)) === 2 && (await p.$$eval(".selchips .chip", e => e.length)) === 2);
  const before = (await state()).results.stages.find(x => x.kind === "chart").pass;
  await p.click('.cbar .seg button:has-text("Below")');
  await p.waitForTimeout(400);
  s0 = await state();
  check("two facing lines keep a band (fewer pass)", s0.project.charts[0].sels[1].Line.above === false && s0.results.stages.find(x => x.kind === "chart").pass <= before);
  check("the chips mark the line that ranks", (await p.textContent(".selchips")).includes("ranks") && (await p.textContent(".selchips")).includes("rank by it"));
  const y0 = s0.project.charts[0].sels[1].Line.y;
  await p.focus(".cbox");
  await p.keyboard.press("ArrowUp");
  await p.waitForTimeout(300);
  check("the arrow keys move the active line", (await state()).project.charts[0].sels[1].Line.y > y0);
  p.once("dialog", d => d.accept("0.004"));
  await p.click('.cbar button:has-text("Pass through")');
  await p.waitForTimeout(400);
  const lv = (await state()).results.lines.find(l => l.chart === s0.project.charts[0].id && l.k === 1);
  check("Pass through… places the line at the index value typed", lv && Math.abs(lv.value - 0.004) < 1e-9, JSON.stringify(lv));
  await p.focus(".cbox");
  await p.keyboard.press("Delete");
  await p.waitForTimeout(300);
  check("Delete removes the active selection", (await state()).project.charts[0].sels.length === 1);
  await p.click('.cbar .tools button:has-text("Shape")');
  const cb = await p.$eval(".cbox", e => { const r = e.getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; });
  for (const [fx, fy] of [[0.55, 0.15], [0.95, 0.15], [0.95, 0.6]]) { await p.mouse.click(cb.x + cb.w * fx, cb.y + cb.h * fy); await p.waitForTimeout(80); }
  await p.keyboard.press("Enter");
  await p.waitForTimeout(400);
  s0 = await state();
  check("the Shape tool adds a polygon; materials outside fail with a reason", s0.project.charts[0].sels.some(x => x.Poly && x.Poly.pts.length === 3) && (await p.$$eval(".cbox path.selpoly", e => e.length)) === 1
    && s0.results.rows.some(r => !r.pass && r.reasons.some(x => x.includes("outside the shape"))));
  await shot("12-selections");
  await p.click('.cbar .tools button:has-text("Pan")');
  await p.click('.side button[title="Stages"]');
  await p.waitForTimeout(200);
  await p.getByText("By weighted properties").click();
  await p.waitForTimeout(400);
  check("weighted ranking ranks by the weighted properties", (await state()).results.indexName.startsWith("Weighted: 50 % Density (low), 50 % Price (low)") && (await p.textContent(".stagespage")).includes("lower is better"));
  await p.getByText("By the index of a selection line").click();
  await p.waitForTimeout(300);
  await p.click('.side button[title="Charts"]');
  await p.waitForTimeout(300);

  // Indices and formulas: the builder finds an index, plotting it makes a chart; a library index adds its formula; Expression Axis plots a formula.
  await p.click('.side button[title="Index"]');
  await p.waitForTimeout(300);
  await p.selectOption(".bsteps label:nth-child(1) select", "Thin-walled pressure vessel");
  await p.waitForTimeout(200);
  const cards = await p.textContent(".icards");
  check("the index builder finds the indices for a function", cards.includes("Light pressure vessel") && cards.includes("Pressure vessel that leaks before it breaks"), cards.slice(0, 200));
  const nCharts = (await state()).project.charts.length;
  await p.locator(".icard", { hasText: "Light pressure vessel" }).getByText("Plot on a new chart").click();
  await p.waitForTimeout(600);
  s0 = await state();
  const last = s0.project.charts[s0.project.charts.length - 1];
  check("Plot on a new chart adds a chart with the index line", s0.project.charts.length === nCharts + 1 && last.x === "rho" && last.y === "sy" && last.sels.length === 1 && (await p.textContent(".cbox svg")).includes("σy/ρ ="));
  await p.click('.side button[title="Index"]');
  await p.waitForTimeout(300);
  await p.fill(".indexpage input", "light spring");
  await p.waitForTimeout(200);
  await p.locator(".indexpage tr.link", { hasText: "Light spring" }).getByText("Plot").click();
  await p.waitForTimeout(600);
  s0 = await state();
  check("a library index adds the formula property it needs", s0.project.formulas.some(f => f.name === "Elastic energy per volume" && f.expr === "[sy]^2/[E]") && (await p.textContent(".cbox svg")).includes("Elastic energy per volume") && /\(Elastic energy per volume\)\/ρ|Elastic energy per volume\/ρ/.test(await p.textContent(".cbox svg")));
  await p.click('.ribbon button:has-text("Chart")').catch(() => {});
  await p.click('.tabs button:has-text("Chart")').catch(() => {});
  await p.waitForTimeout(200);
  await p.getByText("Expression Axis").click();
  await p.waitForTimeout(300);
  await p.fill('.dlg .field:has-text("Name") input', "Stiff beam index");
  await p.fill('.dlg .field:has-text("Formula") input', "[E]^0.5/[rho]");
  await p.click('.dlg footer .btn.pri');
  await p.waitForTimeout(600);
  s0 = await state();
  const open1 = s0.project.charts.find(c => c.y === "f:Stiff beam index");
  check("Expression Axis saves the formula and plots it on the Y axis", !!open1 && (await p.textContent(".cbox svg")).includes("Stiff beam index"));
  check("the axis list shows the formula plotted", (await p.$eval(".axpick select", e => e.selectedOptions[0].textContent)) === "Stiff beam index");
  await p.getByText("Expression Axis").click();
  await p.waitForTimeout(300);
  await p.fill('.dlg .field:has-text("Name") input', "Broken");
  await p.fill('.dlg .field:has-text("Formula") input', "[E]^0.5/[nothing]");
  await p.click('.dlg footer .btn.pri');
  await p.waitForTimeout(400);
  check("a formula that names an unknown property is refused with a reason", (await p.textContent("#toast").catch(() => "")).includes("[nothing]") && !!(await p.$(".dlg")));
  await p.keyboard.press("Escape");
  await p.waitForTimeout(200);
  await shot("13-index");

  // Compare, export, best labels, styles, charts side by side, label dragging, the report.
  await p.click('.side button[title="Results"]');
  await p.waitForTimeout(300);
  const k3 = Math.min(3, (await state()).results.pass);
  await p.getByText("Compare the best three").click();
  await p.waitForTimeout(500);
  s0 = await state();
  check("Compare the best three fills Compare from the results", k3 >= 2 && s0.project.compare.length === k3 && s0.compare.ids.length === k3 && (await p.$$eval(".tbl.cmp thead th", e => e.length)) === k3 + 2, String(k3));
  check("Compare marks the highest and lowest in each row", (await p.$$eval(".tbl.cmp td.hi", e => e.length)) > 5 && (await p.$$eval(".tbl.cmp td.lo", e => e.length)) > 5);
  await p.fill('.comparepage input[placeholder^="Add a material"]', "7075");
  await p.waitForTimeout(200);
  await p.click('.cfound button');
  await p.waitForTimeout(400);
  check("a material is added to Compare by name", (await state()).project.compare.includes("al-7075-t6"));
  check("the radar draws an outline per material", (await p.$$eval(".comparepage svg.radar polygon[fill-opacity]", e => e.length)) === k3 + 1);
  await shot("14-compare");
  await p.click('.side button[title="Results"]');
  await p.waitForTimeout(200);
  await p.click('.ptools button:has-text("To a spreadsheet")');
  await p.waitForTimeout(300);
  check("To a spreadsheet saves the results as .xlsx", (await p.evaluate(() => window.__calls.some(c => c[0] === "export_results" && c[1].fmt === "xlsx"))) && (await p.textContent("#toast").catch(() => "")).includes("results.xlsx"));
  await p.click('.side button[title="Charts"]');
  await p.waitForTimeout(300);
  await p.click('.cbar button:has-text("Best 5")');
  await p.waitForTimeout(400);
  const labs = await p.$$eval(".cbox text.mlab", e => e.map(x => x.textContent));
  const want = Math.min(5, (await state()).results.pass);
  check("Best 5 labels the five best by rank", labs.length === want && labs[0].startsWith("1 · "), JSON.stringify(labs));
  const lab = await p.$eval(".cbox text.mlab", e => { const r = e.getBoundingClientRect(); return { x: r.left + 6, y: r.top + r.height / 2, id: e.getAttribute("data-id") }; });
  await p.mouse.move(lab.x, lab.y); await p.mouse.down(); await p.mouse.move(lab.x + 60, lab.y + 40, { steps: 5 }); await p.mouse.up();
  await p.waitForTimeout(400);
  const mv = (await state()).project.charts.map(c => c.moved?.[lab.id]).find(Boolean);
  check("a dragged label keeps its place", !!mv && Math.abs(mv[0]) > 30 && Math.abs(mv[1]) > 20, JSON.stringify(mv));
  await p.selectOption('.cbar select[title="Chart style"]', "mono");
  await p.waitForTimeout(400);
  check("the mono style draws envelopes in greys with dashes", (await p.$$eval(".cbox path.hull", e => e.filter(x => x.getAttribute("stroke-dasharray")).length)) > 0);
  await p.selectOption('.cbar select[title="Chart style"]', "classic");
  await p.click('.cbar .seg button[title="4 charts at once"]');
  await p.waitForTimeout(600);
  check("four charts at once, side by side", (await p.$$eval(".board.cgrid .cbox", e => e.length)) === 4);
  await shot("15-grid");
  await p.click('.cbar .seg button[title="1 chart at once"]');
  await p.waitForTimeout(300);
  await p.click('.side button[title="Report"]');
  await p.waitForTimeout(500);
  const rep = await p.textContent(".report");
  const nch = (await state()).project.charts.length;
  check("the report holds the requirements, stages, charts, shortlist, comparison and datasheets", ["1 · Requirements", "2 · Selection stages", "3 · Property charts", "4 · Shortlist", "5 · Comparison", "6 · Datasheets of the candidates", "Sources"].every(h => rep.includes(h)) && (await p.$$eval(".report figure.rfig", e => e.length)) === nch);
  await shot("16-report");

  // Your own data: import from a CSV with the columns guessed, open another database and close it.
  await p.click('.side button[title="Data"]');
  await p.waitForTimeout(300);
  await p.click('.datapage button:has-text("Import from a spreadsheet or CSV")');
  await p.waitForSelector(".imptable", { timeout: 5000 });
  const sel = await p.$$eval(".imptable thead select", e => e.map(x => x.value));
  check("the import guesses each column, with imperial units converted", JSON.stringify(sel) === JSON.stringify(["name", "family", "rho", "E", "sy|imperial", "recyclable", "source"]), JSON.stringify(sel));
  await shot("17-import");
  await p.click(".dlg footer .btn.pri");
  await p.waitForTimeout(800);
  let dbx = await p.evaluate(async () => await window.__TAURI__.core.invoke("database"));
  const ia = dbx.materials.find(m => m.name === "Imported alloy A");
  check("imported rows join your own materials; a bad cell is left out and reported", !!ia && ia.user && Math.abs(ia.props.sy[0] - 275.79) < 0.1 && ia.props.rho[1] === 2710
    && dbx.materials.some(m => m.name === "Imported resin B" && !m.props.sy) && (await p.textContent("#toast").catch(() => "")).includes("abc"));
  await p.click('.datapage button:has-text("Open a database")');
  await p.waitForTimeout(800);
  dbx = await p.evaluate(async () => await window.__TAURI__.core.invoke("database"));
  check("an opened database adds its materials, marked with its name", dbx.materials.some(m => m.name === "Team alloy T" && m.origin === "Team data") && (await p.textContent(".datapage")).includes("Team data"));
  await p.click('.datapage tr:has-text("Team data") button:has-text("Close")');
  await p.waitForTimeout(800);
  dbx = await p.evaluate(async () => await window.__TAURI__.core.invoke("database"));
  check("closing it takes its materials away again", !dbx.materials.some(m => m.name === "Team alloy T"));
  await p.click('.side button[title="Charts"]');
  await p.waitForTimeout(300);
  await p.selectOption(".goalpick select", "beam-cheap");
  await p.waitForTimeout(500);
  let svg = await p.textContent(".cbox svg");
  check("a cost goal plots price per volume and draws its index", svg.includes("Price per volume") && /E½\/\(Cₘρ\) = /.test(svg), svg.slice(-160));
  check("the goal adds its limit, marked", (await p.textContent(".spanel")).includes("Cheap, stiff beam") && (await p.evaluate(async () => (await window.__TAURI__.core.invoke("state")).project.stages.limits.some(l => l.goal && l.prop === "kic"))));
  await p.click('.cbar button:has-text("Why")');
  await p.waitForTimeout(200);
  check("the goal explains its function, constraint and index", /Function.*Constraint.*Objective/s.test(await p.textContent(".dlg")));
  await shot("09-goal");
  await p.keyboard.press("Escape");
  await p.click('.status .mode:has-text("SI units")');
  await p.waitForTimeout(300);
  svg = await p.textContent(".cbox svg");
  check("imperial units relabel the axes", svg.includes("10⁶ psi") && svg.includes("USD/ft³"), svg.slice(-160));
  await p.click('.status .mode:has-text("Imperial units")');
  await p.waitForTimeout(200);
  await p.click('.cbar .seg button:has-text("Bars")');
  await p.waitForTimeout(400);
  const bars = await p.$$eval(".cbox rect.bub", r => r.map(x => +x.getAttribute("width")));
  check("a bar chart draws a narrow bar per material, from its own view (not the bubble chart's zoom)", bars.length >= 100 && Math.max(...bars) < 30, `${bars.length} bars, widest ${Math.max(...bars)}`);
  check("the bar chart is named after its property", (await p.textContent(".ctabs button.on")).includes("Young's modulus") && !(await p.textContent(".ctabs button.on")).includes(" vs "));
  await shot("10-bars");
  await p.click('.cbar .seg button:has-text("Bubbles")');
  await p.waitForTimeout(300);
  await p.keyboard.press("Control+k");
  await p.keyboard.type("A97075");
  await p.waitForTimeout(200);
  await p.keyboard.press("Enter");
  await p.waitForTimeout(300);
  check("Tell Me finds a material by its UNS number", (await p.textContent(".sheetpage h1").catch(() => "")).includes("7075"));
  check("checked values carry a ✓ and cite the datasheet", (await p.locator(".sheetpage .chk.ok").count()) === 3 && (await p.textContent(".sheetpage")).includes("7075 Aluminum Alloy Properties"));
  await p.click('.side button[title="Sources"]');
  await p.waitForTimeout(200);
  await p.click(".sources tbody tr.link >> nth=2");
  await p.waitForTimeout(200);
  check("a source lists the materials that cite it", (await p.textContent(".cites").catch(() => "")).length > 20);
  await shot("11-sources");
  await p.click(".sourcespage h1 .help");
  await p.waitForTimeout(200);
  check("a ? opens what the tool is for", (await p.textContent(".helpcard").catch(() => "")).includes("open and public"));
  await p.keyboard.press("Escape");
  await p.click(".tabs .file");
  await p.waitForTimeout(300);
  const tiles = await p.$$eval(".home .tile b", b => b.map(x => x.textContent));
  const at = t => tiles.indexOf(t);
  check("Home: Material Library, Add a material, Sources, Roadmap in order", at("Material Library") > 0 && at("Material Library") < at("Add a material") && at("Add a material") < at("Sources") && at("Sources") < at("Roadmap"), JSON.stringify(tiles));
  await p.click(".bs aside .back");
  await p.waitForTimeout(200);

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
