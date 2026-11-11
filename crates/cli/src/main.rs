//! datum-atlas-cli: Datum Atlas without the window. Read the materials database, run a selection
//! project's stages, eco audit and process selection, and change a project only through the
//! engine's named operations (`datum-atlas-cli ops`), the same ones the window sends.
use datum_atlas_core::{chart, db::{self, Db}, doc::{Doc, Op}, crystal, eco, elements, export, goals, model::*, ops, process, select, templates};
use serde_json::{json, Value};
use std::{io::{BufRead, Write}, path::Path};

const HELP: &str = "datum-atlas-cli: Datum Atlas on the command line

  datum-atlas-cli search TEXT [--family F]     materials whose name, designation or uses match
  datum-atlas-cli material ID|NAME             a datasheet: every value with its unit and source
  datum-atlas-cli microstructure ID|NAME       its schematic microstructure as SVG (a schematic, not a micrograph)
  datum-atlas-cli element SYMBOL|NAME|Z        an element: mass, category, density, melting point, structure…
  datum-atlas-cli composition TEXT [--wt|--at] molar mass, wt % and at % of a formula (Fe2O3) or an alloy (--wt Al Cu 4.4 Mg 1.5)
  datum-atlas-cli crystal ELEMENT [--lambda Å] [--max DEG]   its structure, density and X-ray peaks (Cu Kα unless --lambda)
  datum-atlas-cli crystal --ask JSON           any structure, e.g. '{\"kind\":\"rocksalt\",\"cell\":{…},\"species\":[\"Na\",\"Cl\"],\"hkl\":[2,0,0]}'
  datum-atlas-cli props                        the property keys, names and units
  datum-atlas-cli goals                        the design goals and the index library
  datum-atlas-cli processes                    the process records
  datum-atlas-cli templates                    the starting points for a new project
  datum-atlas-cli new TEMPLATE OUT.datlas      start a project (templates: see above)
  datum-atlas-cli info FILE                    the project: charts, stages, how many pass, the best
  datum-atlas-cli results FILE [--csv]         every material ranked, with the reason each other one fails
  datum-atlas-cli chart FILE [ID]              a chart's materials as bubbles (centre and range on both axes)
  datum-atlas-cli ops                          the named operations, with an example of each
  datum-atlas-cli apply FILE --op JSON [--op JSON…] [--save OUT.datlas]
                                               change a project, e.g. --op '{\"Goal\":{\"chart\":1,\"goal\":\"tie-strong\"}}'
  datum-atlas-cli eco FILE                     the eco audit: energy and CO₂ by life phase for each option
  datum-atlas-cli process FILE                 process selection: what can make the part, and the cost
  datum-atlas-cli tensile FILE [--csv]         the project's tensile tests: E, Rp0.2, Rm, Ag, A, n, K and statistics
  datum-atlas-cli substitutes FILE ID [--tol 0.1]   materials that do the job of ID with less CO₂
  datum-atlas-cli export FILE OUT              save as .datlas, or the results as .xlsx or .csv (OUT.eco.xlsx: the eco audit)
  datum-atlas-cli mcp [--db FILE.datdb]        serve these to an AI assistant (Model Context Protocol, stdio)

FILE is a .datlas project. --db FILE.datdb adds a database of your own (more than once is fine).
Values are in the stored SI units the properties list shows. Nothing is saved unless you say --save or export.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(text) if text.is_empty() => 0,
        Ok(text) => { println!("{text}"); 0 }
        Err(e) => { eprintln!("datum-atlas-cli: {e}"); 1 }
    };
    std::process::exit(code);
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|k| args.get(k + 1)).map(String::as_str)
}
fn pretty(v: &Value) -> String { serde_json::to_string_pretty(v).unwrap_or_default() }

/// The built-in database with any `--db` files beside it.
fn database(args: &[String]) -> Result<Db, String> {
    let mut others = vec![];
    for w in args.windows(2).filter(|w| w[0] == "--db") {
        let text = std::fs::read_to_string(&w[1]).map_err(|e| format!("Could not read {}: {e}", w[1]))?;
        others.push(db::read_user(&text).map_err(|e| format!("{}: {e}", w[1]))?.0);
    }
    Ok(Db::new(db::builtin(), None).with_others(&others))
}

/// Tensile results without the curves.
fn tensile_out(o: &datum_atlas_core::tensile::Out) -> Value {
    let mut v = json!(o);
    for r in v["rows"].as_array_mut().into_iter().flatten() { if let Some(m) = r.as_object_mut() { m.remove("curve"); m.remove("truec"); } }
    v
}

/// A crystal answer without the drawing.
fn crystal_out(q: &crystal::Ask) -> Result<Value, String> {
    let mut v = json!(crystal::answer(q)?);
    if let Some(o) = v.as_object_mut() { o.remove("drawn"); o.remove("basis"); o.remove("poles"); }
    Ok(v)
}

/// How `composition` reads its text: --wt, --at, or a formula.
fn by(args: &[String]) -> &'static str { if args.iter().any(|a| a == "--wt") { "wt" } else if args.iter().any(|a| a == "--at") { "at" } else { "formula" } }

fn load(path: &str) -> Result<Doc, String> { Doc::open(Path::new(path)) }

/// The arguments that are not flags or flag values.
fn plain(args: &[String]) -> Vec<&str> {
    let mut out = vec![];
    let mut skip = false;
    for a in args.iter().skip(1) {
        if skip { skip = false; continue }
        if a.starts_with("--") { skip = !matches!(a.as_str(), "--csv" | "--wt" | "--at"); continue }
        out.push(a.as_str());
    }
    out
}

fn run(args: &[String]) -> Result<String, String> {
    let pl = plain(args);
    let arg = |k: usize, what: &str| pl.get(k).copied().ok_or_else(|| format!("Give {what}."));
    match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => Ok(HELP.into()),
        Some("--version" | "version") => Ok(format!("datum-atlas-cli {}", env!("CARGO_PKG_VERSION"))),
        Some("search") => { let d = database(args)?; Ok(pretty(&json!(search(&d, arg(0, "the text to search for")?, flag(args, "--family"))))) }
        Some("material") => { let d = database(args)?; Ok(pretty(&material(&d, arg(0, "a material id or name")?)?)) }
        Some("element") => Ok(pretty(&json!(elements::find(arg(0, "an element symbol, name or atomic number")?).ok_or("No such element.")?))),
        Some("composition") => { arg(0, "a formula or an alloy")?; Ok(pretty(&json!(elements::composition(&pl.join(" "), by(args))?))) }
        Some("crystal") => {
            let mut q: crystal::Ask = match flag(args, "--ask") { Some(j) => serde_json::from_str(j).map_err(|e| format!("Not a crystal question ({e}).")) ?, None => crystal::Ask { element: Some(arg(0, "an element, or --ask JSON")?.into()), ..Default::default() } };
            if let Some(l) = flag(args, "--lambda") { q.lambda = l.parse().map_err(|_| "--lambda is a number in Å.")? }
            if let Some(m) = flag(args, "--max") { q.max = m.parse().map_err(|_| "--max is a number of degrees.")? }
            Ok(pretty(&crystal_out(&q)?))
        }
        Some("tensile") => {
            let p = load(arg(0, "a .datlas project")?)?.p;
            let o = datum_atlas_core::tensile::run(&p.tensile);
            Ok(if args.iter().any(|a| a == "--csv") { export::csv(&datum_atlas_core::tensile::table(&o)) } else { pretty(&tensile_out(&o)) })
        }
        Some("microstructure") => { let d = database(args)?; Ok(datum_atlas_core::micro::draw(find(&d, arg(0, "a material id or name")?)?).svg) }
        Some("props") => Ok(PROPS.iter().map(|p| format!("{:<12} {:<38} {}", p.key, p.name, if p.unit.is_empty() { p.kind } else { p.unit })).collect::<Vec<_>>().join("\n")),
        Some("goals") => Ok(goals::GOALS.iter().map(|g| format!("{:<22} {:<46} {}{}", g.id, g.name, g.index, if g.library { "  (library)" } else { "" })).collect::<Vec<_>>().join("\n")),
        Some("processes") => Ok(process::processes().iter().map(|p| format!("{:<18} {:<44} {}", p.id, p.name, p.group)).collect::<Vec<_>>().join("\n")),
        Some("templates") => Ok(templates::list().iter().map(|t| format!("{:<8} {:<22} {}", t.id, t.name, t.desc)).collect::<Vec<_>>().join("\n")),
        Some("ops") => Ok(ops::catalog().iter().map(|o| format!("{}\n  {}\n  {}\n", o.name, o.about, o.example)).collect::<Vec<_>>().join("\n")),
        Some("new") => {
            let d = database(args)?;
            let id = arg(0, "a template (see `datum-atlas-cli templates`)")?;
            let p = templates::build(&d, Some(id)).ok_or(format!("No template {id}; see `datum-atlas-cli templates`."))?;
            let out = arg(1, "the file to write, like part.datlas")?;
            save(&d, &mut Doc::new(p), out)?;
            Ok(format!("Saved {out}"))
        }
        Some("info") => { let d = database(args)?; Ok(pretty(&info(&d, &load(arg(0, "the project file")?)?.p))) }
        Some("results") => {
            let d = database(args)?;
            let p = load(arg(0, "the project file")?)?.p;
            if args.iter().any(|a| a == "--csv") { return Ok(export::csv(&export::results(&d.with_formulas(&p.formulas), &p)).trim_start_matches('\u{FEFF}').trim_end().to_string()) }
            Ok(pretty(&results(&d, &p, usize::MAX)))
        }
        Some("chart") => {
            let d = database(args)?;
            let p = load(arg(0, "the project file")?)?.p;
            let id = pl.get(1).and_then(|s| s.parse().ok()).or(p.charts.first().map(|c| c.id)).ok_or("The project has no chart.")?;
            Ok(pretty(&chart_out(&d, &p, id)?))
        }
        Some("apply") => {
            let d = database(args)?;
            let mut doc = load(arg(0, "the project file")?)?;
            let list: Vec<&String> = args.windows(2).filter(|w| w[0] == "--op").map(|w| &w[1]).collect();
            if list.is_empty() { return Err("Give at least one --op JSON (see `datum-atlas-cli ops`).".into()) }
            for (k, j) in list.iter().enumerate() {
                let op: Op = serde_json::from_str(j).map_err(|e| format!("--op {}: not an operation ({e})", k + 1))?;
                ops::apply(&mut doc, &d, op).map_err(|e| format!("--op {}: {e}", k + 1))?;
            }
            if let Some(out) = flag(args, "--save") { save(&d, &mut doc, out)? }
            Ok(pretty(&info(&d, &doc.p)))
        }
        Some("eco") => { let d = database(args)?; let p = load(arg(0, "the project file")?)?.p; Ok(pretty(&json!(eco::audit(&d.with_formulas(&p.formulas), &p.eco)))) }
        Some("process") => { let d = database(args)?; let p = load(arg(0, "the project file")?)?.p; Ok(pretty(&process_out(&d, &p))) }
        Some("substitutes") => {
            let d = database(args)?;
            let p = load(arg(0, "the project file")?)?.p;
            let id = find(&d, arg(1, "a material id or name")?)?.id.clone();
            let tol = flag(args, "--tol").and_then(|t| t.parse().ok()).unwrap_or(0.1f64).clamp(0.0, 1.0);
            Ok(pretty(&json!(eco::substitutes(&d.with_formulas(&p.formulas), &p, &id, tol))))
        }
        Some("export") => {
            let d = database(args)?;
            let mut doc = load(arg(0, "the project file")?)?;
            let out = arg(1, "the file to write")?;
            save(&d, &mut doc, out)?;
            Ok(format!("Saved {out}"))
        }
        Some("mcp") => { let d = database(args)?; mcp(&d, std::io::stdin().lock(), std::io::stdout().lock()); Ok(String::new()) }
        Some(x) => Err(format!("Unknown command {x}.\n\n{HELP}")),
    }
}

/// Writes the project (.datlas) or its results (.xlsx, .csv; a name ending .eco.xlsx or .eco.csv
/// writes the eco audit). Other extensions are refused.
pub fn save(d: &Db, doc: &mut Doc, out: &str) -> Result<(), String> {
    let path = Path::new(out);
    let lower = out.to_lowercase();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let db = d.with_formulas(&doc.p.formulas);
    let table = || if lower.ends_with(".eco.xlsx") || lower.ends_with(".eco.csv") { export::eco(&eco::audit(&db, &doc.p.eco)) } else { export::results(&db, &doc.p) };
    match ext.as_str() {
        "datlas" => doc.save(path),
        "xlsx" => datum_atlas_core::doc::write_atomic(path, &export::xlsx(&table(), &doc.p.name)?),
        "csv" => datum_atlas_core::doc::write_atomic(path, export::csv(&table()).as_bytes()),
        _ => Err(format!("Datum Atlas writes .datlas, .xlsx or .csv, not \"{out}\".")),
    }
}

/// A material by id, else by a name or designation that matches only one.
pub fn find<'a>(d: &'a Db, q: &str) -> Result<&'a Material, String> {
    if let Some(m) = d.get(q) { return Ok(m) }
    let hits = db::search(&d.materials, q);
    if let Some(m) = hits.iter().find(|m| m.name.eq_ignore_ascii_case(q)) { return Ok(m) }
    match hits.len() {
        0 => Err(format!("No material matches \"{q}\".")),
        1 => Ok(hits[0]),
        n => Err(format!("{n} materials match \"{q}\": {}. Give the id.", hits.iter().take(8).map(|m| format!("{} ({})", m.name, m.id)).collect::<Vec<_>>().join(", "))),
    }
}

pub fn search(d: &Db, q: &str, family: Option<&str>) -> Vec<Value> {
    db::search(&d.materials, q).into_iter().filter(|m| family.is_none_or(|f| m.family.first().is_some_and(|x| x.eq_ignore_ascii_case(f))))
        .take(50).map(|m| json!({ "id": m.id, "name": m.name, "family": m.family.join(" › "), "designations": m.des })).collect()
}

/// A datasheet: every value with its unit, its source and whether it was checked.
pub fn material(d: &Db, q: &str) -> Result<Value, String> {
    let m = find(d, q)?;
    let src = |id: &str| d.sources.iter().find(|s| s.id == id).map_or(id.to_string(), |s| format!("{}{}", s.title, if s.url.is_empty() { String::new() } else { format!(" ({})", s.url) }));
    let vals: Vec<Value> = PROPS.iter().filter_map(|p| m.props.get(p.key).map(|v| {
        let value = match v { Val::Range(a, b, _) | Val::RangeT(a, b, _, _) => json!([a, b]), Val::Flag(b, _) => json!(b), Val::Rating(r, _) => json!(r) };
        json!({ "key": p.key, "name": p.name, "value": value, "unit": p.unit, "source": src(v.src()), "checked": m.checked.iter().any(|k| k == p.key) })
    })).collect();
    Ok(json!({ "id": m.id, "name": m.name, "family": m.family.join(" › "), "designations": m.des, "also_called": m.syn, "uses": m.uses, "notes": m.notes,
        "status": if m.user { "yours".into() } else if m.origin.is_empty() { m.status.clone() } else { format!("from {}", m.origin) }, "values": vals }))
}

pub fn results(d: &Db, p: &Project, top: usize) -> Value {
    let r = select::run(&d.with_formulas(&p.formulas), p);
    json!({ "pass": r.pass, "total": r.total, "ranked_by": r.index_name, "stages": r.stages.iter().map(|s| json!({"stage": s.name, "pass": s.pass})).collect::<Vec<_>>(),
        "materials": r.rows.iter().filter(|x| x.pass).take(top).map(|x| json!({"rank": x.rank, "id": x.id, "name": x.name, "family": x.family, "index": x.index, "times_best": x.ratio, "notes": x.reasons})).collect::<Vec<_>>(),
        "failing": if top == usize::MAX { json!(r.rows.iter().filter(|x| !x.pass).map(|x| json!({"id": x.id, "name": x.name, "why": x.reasons})).collect::<Vec<_>>()) } else { json!(r.total - r.pass) } })
}

pub fn info(d: &Db, p: &Project) -> Value {
    json!({ "name": p.name, "requirements": p.notes,
        "charts": p.charts.iter().map(|c| json!({ "id": c.id, "name": c.name, "x": c.x, "y": c.y, "goal": c.goal, "selections": c.sels.len() })).collect::<Vec<_>>(),
        "limits": p.stages.limits.iter().map(|l| json!({ "prop": l.prop, "min": l.min, "max": l.max, "flag": l.flag, "rating": l.rating, "from_goal": l.goal })).collect::<Vec<_>>(),
        "families_left_out": p.stages.exclude, "formulas": p.formulas, "compare": p.compare,
        "eco_options": p.eco.options.len(), "process_material": p.process.material, "results": results(d, p, 10) })
}

pub fn chart_out(d: &Db, p: &Project, id: u32) -> Result<Value, String> {
    let c = p.chart(id).ok_or(format!("No chart {id}; `info` lists the charts."))?;
    let o = chart::build(&d.with_formulas(&p.formulas), c);
    let fail: std::collections::HashSet<String> = select::run(&d.with_formulas(&p.formulas), p).rows.into_iter().filter(|r| !r.pass).map(|r| r.id).collect();
    Ok(json!({ "id": c.id, "name": c.name, "x": o.x, "y": o.y, "missing": o.missing.len(),
        "materials": o.bubbles.iter().map(|b| json!({ "id": b.id, "name": b.name, "family": o.families.get(b.fam).cloned().unwrap_or_default(), "x": b.x, "y": b.y, "passes": !fail.contains(&b.id) })).collect::<Vec<_>>() }))
}

pub fn process_out(d: &Db, p: &Project) -> Value {
    let o = process::select(&d.with_formulas(&p.formulas), p);
    json!({ "part": p.process, "pass": o.pass, "stages": o.stages, "notes": o.notes, "price_used": o.price,
        "processes": o.rows.iter().map(|r| json!({ "id": r.id, "name": r.name, "group": r.group, "passes": r.pass, "why_not": r.reasons, "usd_per_part": r.cost })).collect::<Vec<_>>(),
        "cheapest_by_batch": o.cheapest, "joins": o.joins, "finishes": o.finishes })
}

// ---------- MCP (Model Context Protocol) over stdio ----------

fn tools() -> Value {
    let obj = |props: Value, req: &[&str]| json!({ "type": "object", "properties": props, "required": req });
    let none = || obj(json!({}), &[]);
    let s = |d: &str| json!({ "type": "string", "description": d });
    json!([
        { "name": "search_materials", "description": "Materials whose name, designation (UNS, EN, JIS…), family or uses contain every word of the text. Returns ids for get_material.", "inputSchema": obj(json!({ "text": s("words to find"), "family": s("Metals, Polymers, Elastomers, Ceramics and glasses, Composites, Foams or Natural materials") }), &["text"]) },
        { "name": "get_material", "description": "A material's datasheet: every value (low, high) in the units of list_properties, its source and whether it was checked.", "inputSchema": obj(json!({ "id": s("a material id, or a name that matches only one") }), &["id"]) },
        { "name": "list_properties", "description": "Property keys, names and stored units (SI). Charts, limits and formulas use the keys.", "inputSchema": none() },
        { "name": "list_goals", "description": "Design goals and the index library: id, name, index, axes and the slope of its selection line.", "inputSchema": none() },
        { "name": "list_processes", "description": "Process records: what each suits and can make (mass, section, shapes, tolerance, roughness, economic batch) and indicative costs.", "inputSchema": none() },
        { "name": "get_element", "description": "One of the 118 elements: atomic mass (g/mol), category, group and period, state, density (kg/m³), melting and boiling points (°C), crystal structure, electronegativity, lattice parameters (Å), with sources; a missing value is unknown, never zero.", "inputSchema": obj(json!({ "element": s("a symbol (Fe), name (iron) or atomic number (26)") }), &["element"]) },
        { "name": "composition", "description": "Molar mass and each element's atom and weight percent, from a chemical formula (Fe2O3, Al2(SO4)3, CuSO4·5H2O) or an alloy in weight or atom percent (\"Al Cu 4.4 Mg 1.5\": the element without a number is the balance).", "inputSchema": obj(json!({ "text": s("the formula or the alloy"), "by": s("formula (default), wt or at") }), &["text"]) },
        { "name": "crystal", "description": "Crystal calculators: a structure (an element's own, or bcc, fcc, hcp, diamond, sc, rocksalt, cscl, zincblende, fluorite, or any cell) with its volume, atoms per cell and theoretical density (kg/m³); the spacing d (Å) and Bragg angle of plane hkl; angles between planes hkl and hkl2 and between directions uvw and uvw2; four-index forms for hexagonal cells; and the X-ray powder peaks (2θ, d, multiplicity, approximate intensity) for wavelength lambda (Å, default Cu Kα 1.5418) up to max degrees 2θ.", "inputSchema": obj(json!({ "element": s("an element with a simple structure (Fe, Cu, Mg, Si…); or leave out and give kind, cell and species"), "kind": s("bcc, fcc, hcp, diamond, sc, rocksalt, cscl, zincblende, fluorite or cell"), "cell": { "type": "object", "description": "a, b, c (Å), alpha, beta, gamma (degrees)" }, "species": { "type": "array", "items": { "type": "string" } }, "lambda": { "type": "number" }, "max": { "type": "number" }, "hkl": { "type": "array", "items": { "type": "integer" } }, "hkl2": { "type": "array", "items": { "type": "integer" } }, "uvw": { "type": "array", "items": { "type": "integer" } }, "uvw2": { "type": "array", "items": { "type": "integer" } } }), &[]) },
        { "name": "list_templates", "description": "Starting points for a new project.", "inputSchema": none() },
        { "name": "new_project", "description": "Start a selection project from a template (see list_templates). Returns its facts.", "inputSchema": obj(json!({ "template": s("blank, beam, tie, panel, spring, hot or tough"), "name": s("optional name") }), &["template"]) },
        { "name": "open_project", "description": "Open a .datlas project file to work on. Returns its facts.", "inputSchema": obj(json!({ "path": s("a .datlas file") }), &["path"]) },
        { "name": "project_info", "description": "The open project: charts (ids), limits, families left out, formulas, and how many materials pass with the best ten.", "inputSchema": none() },
        { "name": "list_operations", "description": "The named operations that change a project, each with an example.", "inputSchema": none() },
        { "name": "apply_operation", "description": "Change the open project with one named operation (see list_operations), e.g. {\"Goal\":{\"chart\":1,\"goal\":\"tie-strong\"}}. A failed operation changes nothing.", "inputSchema": obj(json!({ "operation": { "type": "object" } }), &["operation"]) },
        { "name": "undo", "description": "Undo the last change.", "inputSchema": none() },
        { "name": "get_results", "description": "Every material ranked by the selection (index and × best), and the reason each other one fails.", "inputSchema": obj(json!({ "top": { "type": "integer", "description": "how many passing materials to list (default all)" } }), &[]) },
        { "name": "get_chart", "description": "A chart's materials: each one's range on both axes (stored units) and whether it passes.", "inputSchema": obj(json!({ "chart": { "type": "integer" } }), &[]) },
        { "name": "tensile_results", "description": "The open project's tensile tests: for each specimen E (GPa), Rp0.2, Rm (MPa), Ag and A (%), work to fracture (MJ/m³), n and K (MPa), with notes, and the mean, standard deviation and range. Add or change specimens with the Tensile operation.", "inputSchema": none() },
        { "name": "eco_audit", "description": "The eco audit of the open project: energy (MJ) and CO₂ (kg) by life phase as ranges for each option, the largest phase, notes for gaps, break-even. Set it up with the Eco operation.", "inputSchema": none() },
        { "name": "process_selection", "description": "Process selection for the open project's part: the processes that can make it, why each other one cannot, cost per part (USD, indicative) and the cheapest by batch size, joins and finishes. Set the part with the Process operation.", "inputSchema": none() },
        { "name": "greener_substitutes", "description": "Materials that pass the open project's stages, with an index at least (1 − tolerance) of this one's, and less CO₂ for the same job.", "inputSchema": obj(json!({ "material": s("a material id"), "tolerance": { "type": "number", "description": "0 to 1, default 0.1" } }), &["material"]) },
        { "name": "save_project", "description": "Save the open project as .datlas, or its results as .xlsx or .csv (a name ending .eco.xlsx or .eco.csv saves the eco audit).", "inputSchema": obj(json!({ "path": s("the file to write") }), &["path"]) },
    ])
}

fn call(d: &Db, doc: &mut Option<Doc>, name: &str, a: &Value) -> Result<Value, String> {
    let s = |k: &str| a[k].as_str().map(String::from);
    match name {
        "search_materials" => Ok(json!(search(d, &s("text").ok_or("text is needed")?, s("family").as_deref()))),
        "get_material" => material(d, &s("id").ok_or("id is needed")?),
        "list_properties" => Ok(json!(PROPS.iter().map(|p| json!({ "key": p.key, "name": p.name, "unit": p.unit, "kind": p.kind, "group": p.group, "log": p.log, "about": p.about })).collect::<Vec<_>>())),
        "list_goals" => Ok(json!(goals::GOALS.iter().map(|g| json!({ "id": g.id, "name": g.name, "group": g.group, "index": g.index, "x": g.x, "y": g.y, "slope": g.slope, "library": g.library, "why": g.why })).collect::<Vec<_>>())),
        "list_processes" => Ok(json!(process::processes())),
        "list_templates" => Ok(json!(templates::list())),
        "crystal" => crystal_out(&serde_json::from_value(a.clone()).map_err(|e| format!("Not a crystal question ({e})."))?),
        "get_element" => Ok(json!(elements::find(&s("element").ok_or("element is needed")?).ok_or("No such element.")?)),
        "composition" => Ok(json!(elements::composition(&s("text").ok_or("text is needed")?, s("by").as_deref().unwrap_or("formula"))?)),
        "list_operations" => Ok(json!(ops::catalog())),
        "new_project" => {
            let t = s("template").unwrap_or_else(|| "blank".into());
            let mut p = templates::build(d, Some(&t)).ok_or(format!("No template {t}; see list_templates."))?;
            if let Some(n) = s("name").filter(|n| !n.trim().is_empty()) { p.name = n; }
            let v = info(d, &p);
            *doc = Some(Doc::new(p));
            Ok(v)
        }
        "open_project" => { let n = load(&s("path").ok_or("path is needed")?)?; let v = info(d, &n.p); *doc = Some(n); Ok(v) }
        _ => {
            let doc = doc.as_mut().ok_or("Open or start a project first (open_project or new_project).")?;
            match name {
                "project_info" => Ok(info(d, &doc.p)),
                "apply_operation" => {
                    let op: Op = serde_json::from_value(a["operation"].clone()).map_err(|e| format!("Not an operation ({e}). See list_operations."))?;
                    ops::apply(doc, d, op)?;
                    Ok(info(d, &doc.p))
                }
                "undo" => { doc.undo(); Ok(info(d, &doc.p)) }
                "get_results" => Ok(results(d, &doc.p, a["top"].as_u64().map_or(usize::MAX, |n| n as usize))),
                "get_chart" => { let id = a["chart"].as_u64().map(|n| n as u32).or(doc.p.charts.first().map(|c| c.id)).ok_or("The project has no chart.")?; chart_out(d, &doc.p, id) }
                "tensile_results" => Ok(tensile_out(&datum_atlas_core::tensile::run(&doc.p.tensile))),
                "eco_audit" => Ok(json!(eco::audit(&d.with_formulas(&doc.p.formulas), &doc.p.eco))),
                "process_selection" => Ok(process_out(d, &doc.p)),
                "greener_substitutes" => {
                    let id = find(d, &s("material").ok_or("material is needed")?)?.id.clone();
                    Ok(json!(eco::substitutes(&d.with_formulas(&doc.p.formulas), &doc.p, &id, a["tolerance"].as_f64().unwrap_or(0.1).clamp(0.0, 1.0))))
                }
                "save_project" => { let out = s("path").ok_or("path is needed")?; save(d, doc, &out)?; Ok(json!({ "saved": out })) }
                x => Err(format!("Unknown tool {x}.")),
            }
        }
    }
}

/// One JSON-RPC message per line in, one per line out.
fn mcp(d: &Db, input: impl BufRead, mut out: impl Write) {
    let mut doc: Option<Doc> = None;
    for line in input.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() { continue }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => { let _ = writeln!(out, "{}", json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("Parse error: {e}")}})); let _ = out.flush(); continue }
        };
        let Some(id) = msg.get("id").cloned() else { continue }; // a notification: no answer
        let p = &msg["params"];
        let result = match msg["method"].as_str().unwrap_or("") {
            "initialize" => Ok(json!({ "protocolVersion": p["protocolVersion"].as_str().unwrap_or("2025-06-18"), "capabilities": { "tools": {} },
                "serverInfo": { "name": "datum-atlas", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Datum Atlas chooses materials. Find materials with search_materials and read them with get_material (values in stored SI units, each with its source). For a selection, start or open a project, change it only with apply_operation (see list_operations; a Goal sets up a chart, its line, limits and ranking), and read get_results. eco_audit and process_selection follow the Eco and Process operations. Nothing is written to disk unless save_project is called." })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => {
                let name = p["name"].as_str().unwrap_or("");
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(d, &mut doc, name, &p["arguments"]))).unwrap_or_else(|_| Err("Internal error; the project is unchanged.".into()));
                Ok(match r {
                    Ok(v) => json!({ "content": [{ "type": "text", "text": pretty(&v) }], "isError": false }),
                    Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
                })
            }
            m => Err(json!({ "code": -32601, "message": format!("Method not found: {m}") })),
        };
        let reply = match result { Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}), Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}) };
        let _ = writeln!(out, "{reply}");
        let _ = out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("datum-atlas-cli-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        d
    }
    fn a(v: &[&str]) -> Result<String, String> { run(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>()) }

    fn session(d: &Db, lines: &[Value]) -> Vec<Value> {
        let input: String = lines.iter().map(|l| l.to_string() + "\n").collect::<String>() + "not json at all\n";
        let mut out = vec![];
        mcp(d, input.as_bytes(), &mut out);
        String::from_utf8_lossy(&out).lines().map(|l| serde_json::from_str(l).unwrap_or(Value::Null)).collect()
    }
    fn tool(id: u64, name: &str, args: Value) -> Value { json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}}) }
    fn text(r: &Value) -> Value { serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap_or("")).unwrap_or(Value::Null) }

    #[test]
    fn an_assistant_can_select_audit_and_save() {
        let d = Db::new(db::builtin(), None);
        let dir = dir("mcp");
        let path = dir.join("bracket.datlas").display().to_string();
        let r = session(&d, &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            tool(3, "get_results", json!({})),
            tool(4, "search_materials", json!({"text": "6061"})),
            tool(5, "get_material", json!({"id": "al-6061-t6"})),
            tool(6, "new_project", json!({"template": "beam", "name": "Bracket"})),
            tool(7, "apply_operation", json!({"operation": {"Goal": {"chart": 1, "goal": "tie-strong"}}})),
            tool(8, "apply_operation", json!({"operation": {"DelChart": {"id": 99}}})),
            tool(9, "get_results", json!({"top": 5})),
            tool(10, "apply_operation", json!({"operation": {"Eco": {"eco": {"grid": "eu", "options": [{"name": "Al", "years": 10, "parts": [{"name": "P", "material": "al-6061-t6", "mass": 2, "qty": 1, "process": "extrude-metal", "removed": 0, "recycled": 0, "eol": "recycle"}], "legs": [], "use": {"kind": "none"}}]}}}})),
            tool(11, "eco_audit", json!({})),
            tool(12, "process_selection", json!({})),
            tool(13, "greener_substitutes", json!({"material": "steel-1020", "tolerance": 0.5})),
            tool(14, "undo", json!({})),
            tool(15, "save_project", json!({"path": path})),
            tool(16, "save_project", json!({"path": dir.join("x.exe").display().to_string()})),
            tool(17, "nonsense", json!({})),
            json!({"jsonrpc": "2.0", "id": 18, "method": "nope"}),
        ]);
        assert_eq!(r[0]["result"]["serverInfo"]["name"], "datum-atlas");
        assert_eq!(r.len(), 19, "the notification gets no answer");
        assert_eq!(r[1]["result"]["tools"].as_array().map(Vec::len), Some(22));
        assert_eq!(r[2]["result"]["isError"], true, "results need a project");
        assert!(text(&r[3]).as_array().is_some_and(|a| a.iter().any(|m| m["id"] == "al-6061-t6")));
        assert!(text(&r[4])["values"].as_array().is_some_and(|v| v.iter().any(|x| x["key"] == "rho" && x["source"].is_string())));
        assert_eq!(text(&r[5])["name"], "Bracket");
        assert_eq!(text(&r[6])["charts"][0]["goal"], "tie-strong");
        assert_eq!(r[7]["result"]["isError"], true, "a failed operation is refused");
        assert!(text(&r[8])["materials"].as_array().is_some_and(|a| a.len() == 5));
        assert_eq!(text(&r[10])["options"][0]["dominant"], "Material");
        assert!(text(&r[11])["processes"].as_array().is_some_and(|a| !a.is_empty()));
        assert!(text(&r[12])["basis"].as_str().is_some_and(|b| b.contains("same job")));
        assert!(text(&r[13])["eco_options"] == 0, "undo takes back the audit");
        assert_eq!(r[14]["result"]["isError"], false);
        assert!(Doc::open(Path::new(&path)).is_ok_and(|x| x.p.name == "Bracket"));
        assert_eq!(r[15]["result"]["isError"], true);
        assert_eq!(r[16]["result"]["isError"], true);
        assert_eq!(r[17]["error"]["code"], -32601);
        assert_eq!(r[18]["error"]["code"], -32700);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_command_line_reads_the_database() {
        assert!(a(&["search", "6061"]).is_ok_and(|t| t.contains("al-6061-t6")));
        assert!(a(&["element", "iron"]).is_ok_and(|t| t.contains("\"symbol\": \"Fe\"") && t.contains("BCC")));
        assert!(a(&["composition", "Fe2O3"]).is_ok_and(|t| t.contains("159.6")));
        assert!(a(&["composition", "--wt", "Al", "Cu", "4.4", "Mg", "1.5"]).is_ok_and(|t| t.contains("\"by\": \"wt\"")));
        assert!(a(&["crystal", "Fe"]).is_ok_and(|t| t.contains("\"twoTheta\": 44.70") && t.contains("bcc")));
        assert!(a(&["crystal", "--ask", r#"{"kind":"rocksalt","cell":{"a":5.64,"b":5.64,"c":5.64,"alpha":90,"beta":90,"gamma":90},"species":["Na","Cl"]}"#]).is_ok_and(|t| t.contains("Rock salt")));
        assert!(a(&["crystal", "Og"]).is_err() && a(&["crystal", "--ask", "{"]).is_err());
        assert!(a(&["element", "Xx"]).is_err() && a(&["composition", "Fe2O3)"]).is_err());
        assert!(a(&["search", "steel", "--family", "Polymers"]).is_ok_and(|t| t.trim() == "[]"));
        let m = a(&["material", "al-6061-t6"]).unwrap_or_default();
        assert!(m.contains("\"rho\"") && m.contains("\"checked\": true") && m.contains("\"source\""), "{m}");
        assert!(a(&["material", "steel"]).is_err_and(|e| e.contains("Give the id")), "an ambiguous name asks for the id");
        assert!(a(&["material", "unobtainium"]).is_err());
        assert!(a(&["props"]).is_ok_and(|t| t.contains("co2_comb")));
        assert!(a(&["goals"]).is_ok_and(|t| t.contains("beam-stiff") && t.contains("(library)")));
        assert!(a(&["processes"]).is_ok_and(|t| t.contains("die-cast")));
        assert!(a(&["ops"]).is_ok_and(|t| t.contains("Goal") && t.contains("Eco")));
        assert!(a(&["nonsense"]).is_err());
    }

    #[test]
    fn the_command_line_runs_and_changes_a_project() {
        let d = dir("run");
        let f = d.join("beam.datlas").display().to_string();
        assert!(a(&["new", "beam", &f]).is_ok());
        let info = a(&["info", &f]).unwrap_or_default();
        assert!(info.contains("\"goal\": \"beam-stiff\"") && info.contains("\"pass\""), "{info}");
        assert!(a(&["results", &f]).is_ok_and(|t| t.contains("\"failing\"") && t.contains("\"why\"")));
        assert!(a(&["results", &f, "--csv"]).is_ok_and(|t| t.starts_with("Rank,Material")));
        assert!(a(&["chart", &f]).is_ok_and(|t| t.contains("\"passes\"")));
        // Change it through the named operations; a bad one changes nothing and is refused.
        let out = d.join("tie.datlas").display().to_string();
        let eco = r#"{"Eco":{"eco":{"grid":"eu","options":[{"name":"Steel","years":10,"parts":[{"name":"Bracket","material":"steel-1020","mass":2.5,"qty":1,"process":"deform","removed":0,"recycled":0,"eol":"recycle"}],"legs":[],"use":{"kind":"none"}}]}}}"#;
        let proc = r#"{"Process":{"sel":{"material":"al-a356-t6","mass":2,"section":4,"shape":"3d-hollow","tol":0.5,"rough":0,"batch":20000,"price":0,"join":"","purpose":""}}}"#;
        assert!(a(&["apply", &f, "--op", r#"{"Goal":{"chart":1,"goal":"tie-strong"}}"#, "--op", eco, "--op", proc, "--save", &out]).is_ok());
        assert!(a(&["apply", &f, "--op", "{bad"]).is_err());
        assert!(a(&["apply", &f, "--op", r#"{"DelChart":{"id":99}}"#]).is_err());
        assert!(a(&["info", &out]).is_ok_and(|t| t.contains("tie-strong")));
        assert!(a(&["eco", &out]).is_ok_and(|t| t.contains("\"dominant\": \"Material\"")));
        assert!(a(&["process", &out]).is_ok_and(|t| t.contains("investment-cast") && t.contains("cheapest_by_batch")));
        assert!(a(&["substitutes", &f, "steel-1020", "--tol", "0.5"]).is_ok_and(|t| t.contains("same job")));
        let ten = d.join("t.datlas").display().to_string();
        let op = datum_atlas_core::ops::catalog().into_iter().find(|o| o.name == "Tensile").map(|o| o.example.to_string()).unwrap_or_default();
        assert!(a(&["apply", &f, "--op", &op, "--save", &ten]).is_ok());
        assert!(a(&["tensile", &ten]).is_ok_and(|t| t.contains("\"rm\": 500") && t.contains("\"stats\"")), "{:?}", a(&["tensile", &ten]));
        assert!(a(&["tensile", &ten, "--csv"]).is_ok_and(|t| t.contains("Specimen,") && t.contains("Mean")));
        // Exports by extension only.
        for x in ["r.xlsx", "r.csv", "r.eco.xlsx", "copy.datlas"] { assert!(a(&["export", &out, &d.join(x).display().to_string()]).is_ok(), "{x}"); }
        assert!(std::fs::read_to_string(d.join("r.csv")).is_ok_and(|t| t.contains("Material")));
        assert!(a(&["export", &out, &d.join("x.exe").display().to_string()]).is_err());
        assert!(a(&["info", &d.join("missing.datlas").display().to_string()]).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_database_of_your_own_sits_beside_the_built_in_one() {
        let d = dir("db");
        let f = d.join("team.datdb");
        let _ = std::fs::write(&f, r#"{"format":1,"name":"Team","sources":[{"id":"s1","title":"Lab report","kind":"user"}],"materials":[{"id":"t1","name":"Team alloy X","family":["Metals"],"props":{"rho":[2700,2800,"s1"],"E":[70,72,"s1"]}}]}"#);
        let db = f.display().to_string();
        assert!(a(&["search", "Team alloy", "--db", &db]).is_ok_and(|t| t.contains("d1:t1")));
        assert!(a(&["material", "Team alloy X", "--db", &db]).is_ok_and(|t| t.contains("from Team") && t.contains("Lab report")));
        assert!(a(&["search", "x", "--db", &d.join("none.datdb").display().to_string()]).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
