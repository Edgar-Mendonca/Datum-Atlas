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
const PAGE_MOD = { Setup: "Select", Charts: "Select", Stages: "Select", Results: "Select", Compare: "Select", Index: "Select", Report: "Select", Browse: "Library", Datasheet: "Library", Data: "Library", Sources: "Library", "Eco audit": "Eco", Eco: "Eco", Process: "Make", Processes: "Make", Structures: "Microstructure", Elements: "Crystal", Lattice: "Crystal", Tensile: "Testing" };
/** Opens a page: switches module first when the page is in another module. */
const goTo = async (p, t) => {
  if (t === "Eco") t = "Eco audit";
  if (!(await p.$(`.side button[title="${t}"]`))) { await p.click(".side .modsw"); await p.click(`.menu button:has(span:text-is("${PAGE_MOD[t]}"))`); await p.waitForTimeout(250); }
  await p.click(`.side button[title="${t}"]`);
};
/** Opens a ribbon tab, opening a page of a module that has it first. */
const tabTo = async (p, t) => { if (!(await p.$(`.tabs button:text-is("${t}")`))) await goTo(p, t === "Data" ? "Browse" : "Charts"); await p.click(`.tabs button:text-is("${t}")`); };
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
    const fixed = { options: opts, startup_file: null, recovery: null, check_update: null, recent: [], version: "test", copy_text: null, save_file: null, set_options: undefined, export_results: "C:/Selection results.xlsx", export_database: "C:/My materials.datdb", guide: "C:/Users/me/Downloads/Datum Atlas Guide.pdf" };
    const engine = ["state", "apply", "history", "new_project", "templates", "database", "save_material", "delete_material", "goal_list", "databases", "open_database", "close_database", "import_preview", "import_materials", "eco_data", "substitutes", "processes", "microstructure", "elements", "composition", "crystal", "crystal_info", "tensile_read", "tensile_example"];
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
  await goTo(p, "Charts");
  await p.waitForTimeout(300);

  // 3. Dragging the selection line changes the pass count (moving it down lets more through).
  const h = await p.$eval(".cbox circle.selh", e => { const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; });
  await p.mouse.move(h.x, h.y); await p.mouse.down(); await p.mouse.move(h.x, h.y + 60, { steps: 6 }); await p.mouse.up();
  await p.waitForTimeout(400);
  const pass1 = +(await p.textContent(".pcount b"));
  check("dragging the index line down lets more materials pass", pass1 > pass0, `${pass0} → ${pass1}`);
  check("the drag is one Select edit", (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply" && c[1].op.Select).length)) === 1);

  // 4. A limit stage; results match the stages and give reasons.
  await goTo(p, "Stages");
  await p.waitForTimeout(200);
  await p.getByText("Add a limit", { exact: true }).first().click();
  await p.waitForTimeout(300);
  check("the goal added its suggested limit, marked", (await p.textContent(".stagespage")).includes("from the goal"));
  await p.locator(".limit input[placeholder=min]").last().fill("150"); await p.keyboard.press("Tab");
  await p.waitForTimeout(300);
  await shot("04-stages");
  await goTo(p, "Results");
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
  await goTo(p, "Browse");
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
  check("a datasheet has a micrograph slot that says when there is none", (await p.textContent(".micrograph")).includes("No open-licence micrograph"));
  await p.click('.micrograph button:has-text("Typical structure")');
  await p.waitForTimeout(500);
  check("“Typical structure” opens the Microstructure module on that material", (await p.textContent(".side .modsw")).includes("Micro") && !!(await p.$(".structpage .focus .micro svg")) && (await p.$$(".structpage .scard")).length >= 30);
  await p.click(".side .modsw");
  check("the module switcher lists the modules and the planned ones with their release", (await p.textContent(".menu")).includes("Library") && (await p.textContent(".menu")).includes("EBSD (0.3.14)"));
  await p.keyboard.press("Escape"); await p.mouse.click(700, 500); await p.waitForTimeout(150);
  // Crystal › Elements: the periodic table, an element card, colour by a property, composition.
  await goTo(p, "Elements");
  await p.waitForSelector(".elpage .el");
  check("Crystal › Elements shows the 118 elements on a periodic table", (await p.$$(".elpage .pt .el")).length === 118 && (await p.textContent(".side .modsw")).includes("Crystal"));
  await p.click('.elpage .el:has(b:text-is("Cu"))');
  const card = await p.textContent(".elcard");
  check("clicking an element opens its card with values, sources and materials", card.includes("Copper") && card.includes("FCC") && card.includes("8,960 kg/m³") && card.includes("Standard atomic weights") && (await p.$$(".elcard .elmats button")).length > 0, card.slice(0, 300));
  await p.fill(".elfind", "Og"); await p.press(".elfind", "Enter");
  check("an unknown value is a gap, never zero", (await p.textContent(".elcard")).includes("Oganesson") && (await p.textContent(".elcard")).includes("No reliable value known"));
  await p.selectOption(".eltools select", "density");
  check("colouring by a property shades the table and marks the gaps", (await p.$$(".elpage .el.gap")).length >= 15 && (await p.textContent(".eltools")).includes("log scale"));
  await p.fill(".compin", "Al2(SO4)3"); await p.press(".compin", "Enter"); await p.waitForTimeout(200);
  check("composition works out a formula's molar mass", (await p.textContent(".compbox")).includes("342.13"), await p.textContent(".compbox"));
  await p.selectOption(".comprow select", "wt"); await p.fill(".compin", "Fe C 0.8"); await p.press(".compin", "Enter"); await p.waitForTimeout(200);
  check("weight percent turns into atom percent, with the balance", (await p.textContent(".compbox")).includes("3.61") && (await p.textContent(".compbox")).includes("99.200"), await p.textContent(".compbox"));
  await p.fill(".compin", "Fe 60 C 30"); await p.press(".compin", "Enter"); await p.waitForTimeout(200);
  check("a composition that does not add up says why", (await p.textContent(".compbox .err").catch(() => "")).includes("not 100"));
  await shot("elements");
  // Crystal › Calculators: from an element's card, a structure, spacings, angles, peaks, projection.
  await p.click('.elpage .el:has(b:text-is("Al"))'); await p.click(".elcard .tocalc"); await p.waitForSelector(".calcpage .cfacts");
  const facts = await p.textContent(".calcpage .cfacts");
  check("an element opens in the calculators with its structure and density", facts.includes("Face-centred cubic") && facts.includes("Atoms per cell4") && /2,69\d kg/.test(facts), facts);
  const pk = await p.$$eval(".calcpage .peaks tbody tr", rs => rs.map(r => r.textContent));
  check("the X-ray pattern of FCC aluminium starts (111), (200) at 38.5° and 44.7°", pk[0].startsWith("(111)8") && pk[0].includes("38.50") && pk[1].startsWith("(200)6") && pk[1].includes("44.7"), pk.slice(0, 2).join(" | "));
  await p.fill(".calcpage .h1 input >> nth=2", "1"); await p.press(".calcpage .h1 input >> nth=2", "Tab"); await p.waitForTimeout(200);
  check("a plane's spacing and the angle between planes", (await p.textContent(".calcpage")).includes("2.3380") && (await p.textContent(".calcpage")).includes("54.736"), await p.textContent(".calcpage .cform"));
  check("the stereographic projection draws the poles", (await p.$$(".calcpage .stereo circle")).length > 20);
  await p.selectOption(".calcpage .csrc", "rocksalt"); await p.waitForTimeout(150);
  await p.fill(".calcpage .ca", "5.64"); await p.press(".calcpage .ca", "Tab"); await p.fill(".calcpage .sp1", "Na"); await p.press(".calcpage .sp1", "Tab"); await p.fill(".calcpage .sp2", "Cl"); await p.press(".calcpage .sp2", "Tab"); await p.waitForTimeout(250);
  check("a two-element structure (rock salt) works out its density", (await p.textContent(".calcpage .cdens")).includes("2,16"), await p.textContent(".calcpage .cdens"));
  await p.fill(".calcpage .lam", "0"); await p.press(".calcpage .lam", "Tab"); await p.waitForTimeout(200);
  check("a wrong wavelength says why", (await p.textContent(".calcpage .err").catch(() => "")).includes("wavelength"));
  await p.fill(".calcpage .lam", "1.5418"); await p.press(".calcpage .lam", "Tab"); await p.waitForTimeout(150);
  await shot("calculators");
  // Testing › Tensile: import a machine export, results, the example, leaving one out, true curves.
  await goTo(p, "Tensile");
  check("Testing › Tensile starts empty and says how to begin", (await p.textContent(".tenspage .tempty")).includes("Import"));
  await p.click(".tenspage .timport"); await p.waitForSelector(".tenspage .tadd");
  check("an export's heading lines are skipped and its columns guessed", (await p.textContent(".tadd")).includes("after 3 lines of heading") && (await p.$eval(".tadd .txcol", s => s.selectedOptions[0].textContent)) === "Extension (mm)" && (await p.$eval(".tadd .tycol", s => s.selectedOptions[0].textContent)) === "Load (kN)", await p.textContent(".tadd"));
  await p.fill(".tadd .td", "5"); await p.press(".tadd .td", "Tab"); await p.fill(".tadd .tl0", "25"); await p.press(".tadd .tl0", "Tab");
  await p.click(".tadd .tok"); await p.waitForSelector(".tenspage .tres");
  const row = await p.$$eval(".tenspage .tres tbody tr:first-child td", t => t.map(x => x.textContent));
  check("the specimen's results come out of its curve (E 200 GPa, n 0.18)", row[1].startsWith("199.") || row[1].startsWith("200."), row.join(" | "));
  check("…and n is the one the curve was made with", row[7] === "0.180", row.join(" | "));
  await p.click(".tenspage .texample"); await p.waitForTimeout(300);
  check("the made-up example adds three specimens, named so", (await p.$$(".tenspage .tspec tbody tr")).length === 4 && (await p.$$eval(".tenspage .tspec input:not([type=checkbox])", x => x.map(i => i.value).join())).includes("made-up data"), String((await p.$$(".tenspage .tspec tbody tr")).length));
  const sd0 = await p.textContent(".tenspage .tres tr.stat:nth-of-type(6)");
  await p.click(".tenspage .tspec tbody tr:first-child input[type=checkbox]"); await p.waitForTimeout(300);
  check("leaving a specimen out changes the statistics", (await p.textContent(".tenspage .tres tr.stat:nth-of-type(6)")) !== sd0 && !!(await p.$(".tenspage .tres tr.off")));
  await p.click(".tenspage .ttrue"); await p.waitForTimeout(150);
  check("true stress and strain can be drawn", (await p.textContent(".tenspage .tcurve")).includes("True stress") && (await p.$$(".tenspage .tcurve polyline")).length === 3);
  await p.click(".tenspage .ttrue");
  await shot("tensile");
  await goTo(p, "Browse");
  check("each module shows only its pages, and its ribbon tabs", !(await p.$('.side button[title="Charts"]')) && !(await p.$('.tabs button:text-is("Chart")')) && !!(await p.$('.tabs button:text-is("Data")')));
  await goTo(p, "Charts");
  await p.waitForTimeout(300);
  check("your material is on the chart", !!(await p.$('.cbox ellipse.bub[data-id="u-1"]')));

  // 6b. 0.1.1: design goals, units, bar charts, search, Sources, "?" help, Home tiles.
  await goTo(p, "Charts");
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
  await goTo(p, "Stages");
  await p.waitForTimeout(200);
  await p.getByText("By weighted properties").click();
  await p.waitForTimeout(400);
  check("weighted ranking ranks by the weighted properties", (await state()).results.indexName.startsWith("Weighted: 50 % Density (low), 50 % Price (low)") && (await p.textContent(".stagespage")).includes("lower is better"));
  await p.getByText("By the index of a selection line").click();
  await p.waitForTimeout(300);
  await goTo(p, "Charts");
  await p.waitForTimeout(300);

  // Indices and formulas: the builder finds an index, plotting it makes a chart; a library index adds its formula; Expression Axis plots a formula.
  await goTo(p, "Index");
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
  await goTo(p, "Index");
  await p.waitForTimeout(300);
  await p.fill(".indexpage input", "light spring");
  await p.waitForTimeout(200);
  await p.locator(".indexpage tr.link", { hasText: "Light spring" }).getByText("Plot").click();
  await p.waitForTimeout(600);
  s0 = await state();
  check("a library index adds the formula property it needs", s0.project.formulas.some(f => f.name === "Elastic energy per volume" && f.expr === "[sy]^2/[E]") && (await p.textContent(".cbox svg")).includes("Elastic energy per volume") && /\(Elastic energy per volume\)\/ρ|Elastic energy per volume\/ρ/.test(await p.textContent(".cbox svg")));
  await p.click('.ribbon button:has-text("Chart")').catch(() => {});
  await tabTo(p, "Chart").catch(() => {});
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
  await goTo(p, "Results");
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
  await goTo(p, "Results");
  await p.waitForTimeout(200);
  await p.click('.ptools button:has-text("To a spreadsheet")');
  await p.waitForTimeout(300);
  check("To a spreadsheet saves the results as .xlsx", (await p.evaluate(() => window.__calls.some(c => c[0] === "export_results" && c[1].fmt === "xlsx"))) && (await p.textContent("#toast").catch(() => "")).includes("results.xlsx"));
  await goTo(p, "Charts");
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
  await goTo(p, "Report");
  await p.waitForTimeout(500);
  const rep = await p.textContent(".report");
  const nch = (await state()).project.charts.length;
  check("the report holds the requirements, stages, charts, shortlist, comparison and datasheets", ["1 · Requirements", "2 · Selection stages", "3 · Property charts", "4 · Shortlist", "5 · Comparison", "8 · Datasheets of the candidates", "Sources"].every(h => rep.includes(h)) && (await p.$$eval(".report figure.rfig", e => e.length)) === nch);
  await shot("16-report");

  // Your own data: import from a CSV with the columns guessed, open another database and close it.
  await goTo(p, "Data");
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
  await goTo(p, "Charts");
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
  await goTo(p, "Sources");
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
  check("Home: a tile per module, then the design problems", ["Library", "Select", "Eco", "Make", "Microstructure", "Crystal", "Testing", "More modules"].every((t, i) => tiles[i] === t) && at("Blank selection") > at("More modules"), JSON.stringify(tiles));
  await p.click(".bs aside .back");
  await p.waitForTimeout(200);

  // 7. Ribbon fits without scrolling; planned commands explain when they arrive.
  for (const w of [1280, 1600]) {
    await p.setViewportSize({ width: w, height: 900 });
    for (const t of ["Home", "Select", "Chart", "Data", "Report", "View", "Help"]) {
      await tabTo(p, t);
      await p.waitForTimeout(80);
      const over = await p.$eval(".ribbon", e => e.scrollWidth - e.clientWidth);
      if (over > 1) check(`ribbon ${t} fits at ${w}px`, false, `${over}px over`);
    }
  }
  check("every ribbon tab fits at 1280 and 1600 px", true);
  await p.setViewportSize({ width: 1440, height: 900 });
  // Eco audit: start one, a part, its phases; a lighter copy and where it breaks even; greener substitutes.
  await tabTo(p, "Report");
  await p.click('.ribbon button[title^="Energy and CO₂"]');
  await p.waitForTimeout(200);
  check("Eco Audit opens the eco page", !!(await p.$(".ecopage")));
  await p.click('.ecopage button:has-text("Start an audit")');
  await p.waitForTimeout(300);
  await p.click('.ecopage button:has-text("+ Part")');
  await p.waitForTimeout(300);
  const prow = '.ecoparts tbody tr:last-child';
  await p.selectOption(`${prow} td:nth-child(2) select`, "steel-1020");
  await p.waitForTimeout(250);
  await p.fill(`${prow} td:nth-child(3) input`, "10"); await p.press(`${prow} td:nth-child(3) input`, "Tab");
  await p.waitForTimeout(250);
  const kpi = await p.textContent(".ecokpi");
  check("the audit totals CO₂ and energy as ranges", /\d/.test(kpi) && kpi.includes("kg CO₂") && kpi.includes("–"), kpi);
  check("each life phase has its bar", (await p.$$eval(".ecocharts > div:first-child svg rect", e => e.length)) === 5);
  check("the largest phase is named with what helps", (await p.textContent(".ecodom")).includes("Material"));
  await p.click('.ecopage label:has-text("Carried in a vehicle") input');
  await p.waitForTimeout(250);
  const useNum = '.ecopage .panel:has(h2:has-text("Use")) input[type=number]';
  await p.fill(useNum, "20000"); await p.press(useNum, "Tab");
  await p.waitForTimeout(250);
  await p.click('.otabs button:has-text("Copy")');
  await p.waitForTimeout(300);
  await p.selectOption(`${prow} td:nth-child(2) select`, "al-6061-t6");
  await p.waitForTimeout(250);
  await p.fill(`${prow} td:nth-child(3) input`, "5"); await p.press(`${prow} td:nth-child(3) input`, "Tab");
  await p.waitForTimeout(300);
  const side = await p.textContent(".ecopage").catch(() => "");
  check("two options break even after some years", /breaks even after [\d.]+ years/.test(side), side.slice(-400));
  check("the break-even chart draws a line per option", (await p.$$eval(".ecogrid svg line[stroke^='#']", e => e.length)) >= 2);
  await shot("10-eco");
  await p.click('.ecoparts button.lnk:has-text("greener?")');
  await p.waitForTimeout(400);
  const gd = await p.textContent(".dlg").catch(() => "");
  check("greener substitutes say their basis", gd.includes("Greener substitutes") && gd.includes("Basis"), gd.slice(0, 300));
  await p.keyboard.press("Escape");
  // Process selection: a cast aluminium part; reasons for the others; the cost chart; records.
  await p.click('.ribbon button[title^="Which processes can make"]');
  await p.waitForTimeout(300);
  check("Process Selection opens its page", !!(await p.$(".procpage")));
  await p.selectOption(".procpage .panel:first-child select", "al-a356-t6");
  await p.waitForTimeout(250);
  await p.selectOption(".procpage select.pshape", "3d-hollow");
  await p.waitForTimeout(250);
  const why = await p.textContent(".proctable");
  check("processes that cannot make the part say why", why.includes("does not work") && why.includes("cannot make"), why.slice(0, 300));
  check("the cost chart draws a curve per process", (await p.$$eval(".procpage svg polyline", e => e.length)) >= 2);
  check("the cheapest process is named by batch size", /Cheapest:.+ parts/.test(await p.textContent(".procpage")));
  check("joins and finishes are listed", (await p.textContent(".procpage")).includes("Anodising"));
  await shot("11-process");
  await p.click('.ribbon button[title="Every process record"]');
  await p.waitForTimeout(250);
  check("the process list shows every record", (await p.$$(".page table.tbl tbody tr")).length >= 50);
  await p.click('.page button.lnk:has-text("Die casting")');
  await p.waitForTimeout(200);
  check("a process record opens", (await p.textContent(".dlg")).includes("Economic batch"));
  await p.keyboard.press("Escape");
  await goTo(p, "Report");
  await p.waitForTimeout(300);
  const rep2 = await p.textContent(".report");
  check("the report adds the eco audit and the manufacture", rep2.includes("6 · Eco audit") && rep2.includes("7 · Manufacture") && rep2.includes("Investment casting"), rep2.slice(0, 200));

  // 8. Every view prints; the chart prints as a vector page.
  for (const [v, want] of [["Charts", ".ppage svg.chartsvg"], ["Datasheet", ".pflow .sheetpage"], ["Results", ".pflow .ptable"]]) {
    await goTo(p, v);
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
  await p.click('.ribbon button[title^="The Datum Atlas Guide"]');
  await p.waitForTimeout(200);
  check("Help › Guide offers the PDF", (await p.textContent(".dlg")).includes("Datum Atlas Guide (PDF)"));
  await p.click(".dlg .guidecard");
  await p.waitForTimeout(300);
  check("the guide is downloaded, checked and opened", (await p.textContent("#toast").catch(() => "")).includes("Datum Atlas Guide.pdf") && !(await p.$(".dlg")));
  await p.click('.ribbon button[title^="Connect an AI assistant"]');
  await p.waitForTimeout(200);
  const ad = await p.textContent(".dlg");
  check("Help shows how to connect an AI assistant", ad.includes("datum-atlas-cli-") && ad.includes('"mcp"') && ad.includes("mcpServers"), ad.slice(0, 200));
  await p.click('.dlg button:has-text("Copy the settings")');
  await p.waitForTimeout(150);
  check("the assistant settings can be copied", (await p.evaluate(() => window.__calls.filter(c => c[0] === "copy_text").pop()?.[1]?.text ?? "")).includes('"args": [\n        "mcp"'));
  await p.keyboard.press("Escape");
  await p.click(".tabs .file"); await p.click(".bs aside button:has-text('Options')");
  await p.waitForTimeout(200);
  const rows = await p.$$eval(".opt", os => os.map(o => Math.abs(o.children[1].getBoundingClientRect().left - os[0].children[1].getBoundingClientRect().left)));
  check("Options controls line up", rows.length > 6 && Math.max(...rows) < 2, JSON.stringify(rows));
  // No commercial product names anywhere in the window (FAMILY §13): every page, ribbon tab,
  // dialog and ? card, with tooltips. Formats the app reads or writes (.xlsx, CSV, SVG…) are fine.
  const PRODUCTS = /\b(Excel|Microsoft|PowerPoint|Word document|MS Office|MS Project|Granta|EduPack|CES Selector|Ansys|MatWeb|Total ?Materia|MakeItFrom|Matmatch|MaterialHub|AZoM|Wikipedia|Knovel|SpringerMaterials|SolidWorks|AutoCAD|CATIA|MATLAB|Google Sheets|LibreOffice|Apple Numbers|Siemens|Altair|OriginPro|Tableau|Power BI)\b/;
  const texts = [];
  const grab = async () => texts.push(await p.evaluate(() => document.body.innerText + " " + [...document.querySelectorAll("[title],[placeholder]")].map(e => (e.getAttribute("title") ?? "") + " " + (e.getAttribute("placeholder") ?? "")).join(" ")));
  await p.keyboard.press("Escape"); await p.waitForTimeout(150);
  if (await p.$(".bs")) { await p.click(".bs aside .back"); await p.waitForTimeout(150); }
  for (const v of Object.keys(PAGE_MOD).filter(k => k !== "Eco")) { await goTo(p, v); await p.waitForTimeout(150); await grab(); }
  for (const t of await p.$$eval(".tabs button:not(.file)", bs => bs.map(b => b.textContent.trim()).filter(Boolean))) { await p.click(`.tabs button:has-text("${t}")`); await p.waitForTimeout(100); await grab(); }
  await p.click('.tabs button:has-text("Help")');
  for (const t of ["The Datum Atlas Guide", "Getting Started", "Bubble, envelope", "Connect an AI assistant", "Every tool explained"]) {
    await p.click(`.ribbon button[title^="${t}"]`); await p.waitForTimeout(200); await grab();
    if (t === "Every tool explained") {
      const n = await p.$$eval(".menu button", b => b.length);
      for (let k = 0; k < n; k++) { if (k) { await p.click('.ribbon button[title^="Every tool explained"]'); await p.waitForTimeout(100); } await p.click(`.menu button >> nth=${k}`); await p.waitForTimeout(100); await grab(); await p.keyboard.press("Escape"); }
    } else await p.keyboard.press("Escape");
  }
  for (const b of ["Home", "Roadmap", "Options", "Updates", "Export", "Print"]) { await p.click(".tabs .file"); await p.waitForTimeout(100); const btn = await p.$(`.bs aside button:has-text("${b}")`); if (btn) { await btn.click(); await p.waitForTimeout(200); await grab(); } await p.keyboard.press("Escape"); await p.waitForTimeout(100); }
  const hit = texts.join("\n").match(PRODUCTS);
  check(`no commercial product names in the window (${texts.length} views read)`, !hit && texts.length > 40, hit ? hit[0] + " in: " + texts.find(t => PRODUCTS.test(t)).match(new RegExp(".{0,60}" + hit[0] + ".{0,60}"))[0] : String(texts.length));
  check("no script errors", errors.length === 0, errors.join(" | "));

  await b.close(); srv.close(); eng.kill();
  console.log(failed ? `${failed} failed` : "all passed");
  process.exit(failed ? 1 : 0);
})().catch(e => { console.error(e); eng.kill(); process.exit(1); });
