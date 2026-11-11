//! datum-atlas-cli: Datum Atlas without the window. Read the materials database, run a selection
//! project's stages, eco audit and process selection, and change a project only through the
//! engine's named operations (`datum-atlas-cli ops`), the same ones the window sends.
use datum_atlas_core::{chart, db::{self, Db}, doc::{Doc, Op}, eco, export, goals, model::*, ops, process, select, templates};
use serde_json::{json, Value};
use std::path::Path;

const HELP: &str = "datum-atlas-cli: Datum Atlas on the command line

  datum-atlas-cli search TEXT [--family F]     materials whose name, designation or uses match
  datum-atlas-cli material ID|NAME             a datasheet: every value with its unit and source
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
  datum-atlas-cli substitutes FILE ID [--tol 0.1]   materials that do the job of ID with less CO₂
  datum-atlas-cli export FILE OUT              save as .datlas, or the results as .xlsx or .csv (OUT.eco.xlsx: the eco audit)

FILE is a .datlas project. --db FILE.datdb adds a database of your own (more than once is fine).
Values are in the stored SI units the properties list shows. Nothing is saved unless you say --save or export.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
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

fn load(path: &str) -> Result<Doc, String> { Doc::open(Path::new(path)) }

/// The arguments that are not flags or flag values.
fn plain(args: &[String]) -> Vec<&str> {
    let mut out = vec![];
    let mut skip = false;
    for a in args.iter().skip(1) {
        if skip { skip = false; continue }
        if a.starts_with("--") { skip = !matches!(a.as_str(), "--csv"); continue }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("datum-atlas-cli-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        d
    }
    fn a(v: &[&str]) -> Result<String, String> { run(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>()) }

    #[test]
    fn the_command_line_reads_the_database() {
        assert!(a(&["search", "6061"]).is_ok_and(|t| t.contains("al-6061-t6")));
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
