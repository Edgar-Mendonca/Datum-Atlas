//! The engine behind the window for UI tests: one JSON request per line on stdin
//! ({"cmd": "...", "args": {...}}), one JSON answer per line on stdout ({"ok": ...} or {"err": "..."}).
//! It answers the shell's commands that need the engine, without files or the OS:
//! cargo run -q -p datum-atlas-core --example bridge
use datum_atlas_core::{chart, compare, db::{self, Db}, doc::{Doc, Op}, eco, process, goals, select, sheet, templates, Database, Material, Source, Val, FORMAT};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

struct St { ebsd: Option<datum_atlas_core::ebsd::Ebsd>, doc: Doc, user: Database, others: Vec<Database>, db: Db, rev: u64, db_rev: u64 }

/// A file of the UI tests, when the window asks without a path (the shell would open a dialog).
fn fixture(a: &Value, name: &str) -> String { a["path"].as_str().map(String::from).unwrap_or_else(|| format!("{}/../../tests_ui/{name}", env!("CARGO_MANIFEST_DIR"))) }
fn rebuild(s: &mut St) { s.db = Db::new(db::builtin(), Some(&s.user)).with_others(&s.others); s.db_rev += 1; }

fn out(s: &St) -> Value {
    let db = s.db.with_formulas(&s.doc.p.formulas);
    let charts: Vec<_> = s.doc.p.charts.iter().map(|c| chart::build(&db, c)).collect();
    json!({ "project": s.doc.p, "path": null, "dirty": s.doc.dirty, "undo": s.doc.can_undo(), "redo": s.doc.can_redo(), "rev": s.rev, "charts": charts, "results": select::run(&db, &s.doc.p), "compare": compare::build(&db, &s.doc.p), "eco": eco::audit(&db, &s.doc.p.eco), "process": process::select(&db, &s.doc.p), "tensile": datum_atlas_core::tensile::run(&s.doc.p.tensile), "lab": datum_atlas_core::lab::run(&s.doc.p.lab), "dbRev": s.db_rev })
}

fn handle(s: &mut St, cmd: &str, a: &Value) -> Result<Value, String> {
    s.rev += 1;
    Ok(match cmd {
        "state" => out(s),
        "apply" => {
            let op: Op = serde_json::from_value(a["op"].clone()).map_err(|e| e.to_string())?;
            let op = match op { Op::Goal { chart, goal } => goals::apply(&s.db, &s.doc.p, chart, &goal)?, o => o };
            s.doc.apply(op)?;
            out(s)
        }
        "history" => { if a["redo"].as_bool().unwrap_or(false) { s.doc.redo() } else { s.doc.undo() } out(s) }
        "new_project" => { s.doc = Doc::new(templates::build(&s.db, a["template"].as_str()).ok_or("Unknown template.")?); out(s) }
        "goal_list" => json!(goals::GOALS),
        "eco_data" => json!(eco::data()),
        "processes" => json!(process::processes()),
        "microstructure" => json!(datum_atlas_core::micro::draw(s.db.get(a["id"].as_str().unwrap_or("")).ok_or("That material is not in the database.")?)),
        "substitutes" => json!(eco::substitutes(&s.db.with_formulas(&s.doc.p.formulas), &s.doc.p, a["id"].as_str().unwrap_or(""), a["tol"].as_f64().unwrap_or(0.1))),
        "templates" => json!(templates::list()),
        "crystal" => json!(datum_atlas_core::crystal::answer(&serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?)?),
        "crystal_info" => json!({ "kinds": datum_atlas_core::crystal::KINDS, "tubes": datum_atlas_core::crystal::TUBES }),
        "tensile_read" => { let p = fixture(a, "tensile.csv"); let t = sheet::read(std::path::Path::new(&p))?; let raw: Vec<Vec<String>> = std::iter::once(t.head).chain(t.rows).collect(); json!({ "path": p, "name": "tensile", "cols": datum_atlas_core::tensile::columns(&raw)? }) }
        "image_analyse" => json!(datum_atlas_core::image::analyse(&serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?)?),
        "image_example" => { let (w, h, px) = datum_atlas_core::image::example_of(a["kind"].as_str().unwrap_or("particles")); json!({ "w": w, "h": h, "px": datum_atlas_core::image::b64_encode(&px) }) }
        "synth_generate" => json!(datum_atlas_core::synth::generate(&serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?)?),
        "synth_ebsd" => json!(datum_atlas_core::synth::ebsd_text(&serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?)?),
        "ipf_key" => json!(datum_atlas_core::orient::ipf_key(a["crystal"].as_str().unwrap_or("cubic"), 40)),
        "synth_csv" => json!(datum_atlas_core::synth::csv(&serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?)?),
        "ebsd_open" => { let p = fixture(a, "map.ang"); let m = datum_atlas_core::ebsd::parse(&std::fs::read_to_string(&p).map_err(|e| e.to_string())?, "map.ang")?; let v = json!(m.summary()); s.ebsd = Some(m); v }
        "ebsd_example" => { let m = datum_atlas_core::ebsd::example()?; let v = json!(m.summary()); s.ebsd = Some(m); v }
        "ebsd_draw" => { let m = s.ebsd.as_ref().ok_or("Open an EBSD map first.")?; json!({ "w": m.w, "h": m.h, "rgb": m.draw(a["kind"].as_str().unwrap_or("ipf"), a["dir"].as_str().unwrap_or("z"))? }) }
        "ebsd_grains" => { let m = s.ebsd.as_ref().ok_or("Open an EBSD map first.")?; let q: datum_atlas_core::ebsd::GrainAsk = serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?; let (rgb, o) = m.draw_grains(a["kind"].as_str().unwrap_or("grains"), &q)?; json!({ "w": m.w, "h": m.h, "rgb": rgb, "out": o }) }
        "lab_example" => json!(datum_atlas_core::lab::example()),
        "test_material" => json!(datum_atlas_core::lab::to_material(&datum_atlas_core::tensile::run(&s.doc.p.tensile), &datum_atlas_core::lab::run(&s.doc.p.lab), a["name"].as_str().unwrap_or(""), a["src"].as_str().unwrap_or(""))),
        "tensile_example" => json!(datum_atlas_core::tensile::example()),
        "elements" => json!(datum_atlas_core::elements::elements()),
        "composition" => json!(datum_atlas_core::elements::composition(a["text"].as_str().unwrap_or(""), a["by"].as_str().unwrap_or("formula"))?),
        "database" => json!({ "materials": s.db.materials, "sources": s.db.sources, "userCount": s.db.user_count, "props": db::props_out(), "tree": db::tree(&s.db.materials), "rev": s.db_rev }),
        "save_material" => {
            let mut m: Material = serde_json::from_value(a["material"].clone()).map_err(|e| e.to_string())?;
            let srcs: Vec<Source> = serde_json::from_value(a["sources"].clone()).unwrap_or_default();
            if m.id.is_empty() { m.id = format!("u-{}", s.user.materials.len() + 1); }
            s.user.sources.extend(srcs);
            db::check_user(&m, &[s.user.sources.clone(), db::builtin().sources].concat())?;
            m.status = "user".into();
            s.user.materials.retain(|x| x.id != m.id);
            s.user.materials.push(m);
            rebuild(s);
            out(s)
        }
        "delete_material" => { let id = a["id"].as_str().unwrap_or(""); s.user.materials.retain(|m| m.id != id); rebuild(s); out(s) }
        "databases" => { let mut v = vec![json!({ "name": "My materials", "path": "My materials.datdb", "count": s.user.materials.len(), "mine": true })]; v.extend(s.others.iter().enumerate().map(|(k, d)| json!({ "name": d.name, "path": format!("db{k}"), "count": d.materials.len(), "mine": false }))); json!(v) }
        "open_database" => { let (d, _) = db::read_user(&std::fs::read_to_string(fixture(a, "team.datdb")).map_err(|e| e.to_string())?)?; s.others.push(d); rebuild(s); out(s) }
        "close_database" => { let k = a["path"].as_str().and_then(|p| p.strip_prefix("db")).and_then(|k| k.parse::<usize>().ok()).unwrap_or(usize::MAX); if k < s.others.len() { s.others.remove(k); } rebuild(s); out(s) }
        "import_preview" => { let p = fixture(a, "import.csv"); let t = sheet::read(std::path::Path::new(&p))?; json!({ "path": p, "head": t.head, "rows": t.rows, "count": t.rows.len(), "guess": t.guess }) }
        "import_materials" => {
            let t = sheet::read(std::path::Path::new(a["path"].as_str().unwrap_or("")))?;
            let map: Vec<String> = serde_json::from_value(a["map"].clone()).unwrap_or_default();
            let (d, notes) = sheet::to_database(&t, &map, a["family"].as_str().unwrap_or("Metals"), a["source"].as_str().unwrap_or(""))?;
            let n0 = s.user.materials.len();
            for src in d.sources { s.user.sources.push(Source { id: format!("u-{}", src.id), ..src }); }
            for (k, mut m) in d.materials.into_iter().enumerate() {
                m.id = format!("u-imp-{}", n0 + k + 1);
                for v in m.props.values_mut() { let to = format!("u-{}", v.src()); match v { Val::Range(_, _, x) | Val::RangeT(_, _, x, _) | Val::Flag(_, x) | Val::Rating(_, x) => *x = to } }
                s.user.materials.push(m);
            }
            let added = s.user.materials.len() - n0;
            rebuild(s);
            json!({ "out": out(s), "added": added, "notes": notes })
        }
        c => return Err(format!("bridge: unknown command {c}")),
    })
}

fn main() {
    let user = Database { format: FORMAT, name: "My materials".into(), ..Default::default() };
    let d = Db::new(db::builtin(), Some(&user));
    let mut s = St { ebsd: None, doc: Doc::new(templates::build(&d, Some("blank")).unwrap()), db: d, user, others: vec![], rev: 0, db_rev: 1 };
    let stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let q: Value = serde_json::from_str(&line).unwrap_or_default();
        let r = match handle(&mut s, q["cmd"].as_str().unwrap_or(""), &q["args"]) { Ok(v) => json!({ "ok": v }), Err(e) => json!({ "err": e }) };
        let mut o = stdout.lock();
        let _ = writeln!(o, "{r}");
        let _ = o.flush();
    }
}
