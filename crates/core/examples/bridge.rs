//! The engine behind the window for UI tests: one JSON request per line on stdin
//! ({"cmd": "...", "args": {...}}), one JSON answer per line on stdout ({"ok": ...} or {"err": "..."}).
//! It answers the shell's commands that need the engine, without files or the OS:
//! cargo run -q -p datum-atlas-core --example bridge
use datum_atlas_core::{chart, compare, db::{self, Db}, doc::{Doc, Op}, eco, process, goals, research::{Item, Open}, select, sheet, templates, Database, Material, Source, Val, FORMAT};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// The Python worker for graphs (DATUM_PYTHON or python3, as the app in development).
struct PyW { _child: std::process::Child, stdin: std::process::ChildStdin, out: std::io::BufReader<std::process::ChildStdout> }
fn py(w: &mut Option<PyW>, q: Value) -> Result<Value, String> {
    if w.is_none() {
        let exe = std::env::var("DATUM_PYTHON").unwrap_or_else(|_| "python3".into());
        let script = format!("{}/../../py/worker.py", env!("CARGO_MANIFEST_DIR"));
        let mut c = std::process::Command::new(&exe).arg("-X").arg("utf8").arg(&script).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().map_err(|e| format!("The graph engine (Python) could not start: {e}"))?;
        let (stdin, mut out) = (c.stdin.take().unwrap(), std::io::BufReader::new(c.stdout.take().unwrap()));
        let mut l = String::new();
        out.read_line(&mut l).map_err(|e| e.to_string())?;
        if !l.contains("ready") { return Err("The graph engine (Python) did not start.".into()) }
        *w = Some(PyW { _child: c, stdin, out });
    }
    let p = w.as_mut().unwrap();
    writeln!(p.stdin, "{q}").map_err(|e| e.to_string())?;
    let mut l = String::new();
    p.out.read_line(&mut l).map_err(|e| e.to_string())?;
    let r: Value = serde_json::from_str(&l).map_err(|e| e.to_string())?;
    if r["ok"] == true { Ok(r) } else { Err(r["error"].as_str().unwrap_or("The graph engine failed.").to_string()) }
}

struct St { py: Option<PyW>, open: Open, doc: Doc, user: Database, others: Vec<Database>, db: Db, rev: u64, db_rev: u64 }

/// A file of the UI tests, when the window asks without a path (the shell would open a dialog).
fn fixture(a: &Value, name: &str) -> String { a["path"].as_str().map(String::from).unwrap_or_else(|| format!("{}/../../tests_ui/{name}", env!("CARGO_MANIFEST_DIR"))) }
fn rebuild(s: &mut St) { s.db = Db::new(db::builtin(), Some(&s.user)).with_others(&s.others); s.db_rev += 1; }

fn keep(s: &mut St, which: &str, it: Item) -> Result<Value, String> { s.doc.apply(Op::Research { which: which.into(), item: Some(Box::new(it)) })?; Ok(out(s)) }

fn out(s: &St) -> Value {
    let db = s.db.with_formulas(&s.doc.p.formulas);
    let charts: Vec<_> = s.doc.p.charts.iter().map(|c| chart::build(&db, c)).collect();
    json!({ "project": s.doc.p.lite(), "path": null, "dirty": s.doc.dirty, "undo": s.doc.can_undo(), "redo": s.doc.can_redo(), "rev": s.rev, "charts": charts, "results": select::run(&db, &s.doc.p), "compare": compare::build(&db, &s.doc.p), "eco": eco::audit(&db, &s.doc.p.eco), "process": process::select(&db, &s.doc.p), "tensile": datum_atlas_core::tensile::run(&s.doc.p.tensile), "lab": datum_atlas_core::lab::run(&s.doc.p.lab), "dbRev": s.db_rev })
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
        "xrd_open" => { let p = fixture(a, "pattern.xy"); let pat = datum_atlas_core::xrd::parse(&std::fs::read_to_string(&p).map_err(|e| e.to_string())?, "pattern.xy")?; let it = datum_atlas_core::xrd::add(&s.doc.p.research, pat)?; keep(s, "xrd", it)? }
        "xrd_example" => { let it = datum_atlas_core::xrd::add(&s.doc.p.research, datum_atlas_core::xrd::example()?)?; keep(s, "xrd", it)? }
        "xrd_remove" => { let it = datum_atlas_core::xrd::remove(&s.doc.p.research, a["index"].as_u64().unwrap_or(0) as usize)?; s.doc.apply(Op::Research { which: "xrd".into(), item: it.map(Box::new) })?; out(s) }
        "xrd_analyse" => { let ps = datum_atlas_core::xrd::kept(&s.doc.p.research)?; let q: datum_atlas_core::xrd::Ask = serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?; json!(datum_atlas_core::xrd::analyse(ps.get(a["index"].as_u64().unwrap_or(0) as usize).ok_or("That pattern is no longer open.")?, &q)?) }
        "xrd_plot" => json!(datum_atlas_core::xrd::for_plot(&s.doc.p.research)?),
        "corrosion_info" => datum_atlas_core::corrosion::info(),
        "pourbaix" => json!(datum_atlas_core::corrosion::pourbaix(&serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?)?),
        "galvanic" => json!(datum_atlas_core::corrosion::couple(&serde_json::from_value(a["a"].clone()).map_err(|e| e.to_string())?, &serde_json::from_value(a["b"].clone()).map_err(|e| e.to_string())?, a["area"].as_f64().unwrap_or(0.0))?),
        "kinetics" => datum_atlas_core::corrosion::kinetics_page(&serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?),
        "image_keep" => { let it = Item::image(a["name"].as_str().unwrap_or(""), a["w"].as_u64().unwrap_or(0) as usize, a["h"].as_u64().unwrap_or(0) as usize, &datum_atlas_core::image::b64_decode(a["px"].as_str().unwrap_or(""))?, a["made"].as_bool().unwrap_or(false))?; keep(s, "image", it)? }
        "image_kept" => match &s.doc.p.research.image { None => Value::Null, Some(i) => { let (w, h, px) = i.image_px()?; json!({ "name": i.info["name"], "made": i.info["made"], "w": w, "h": h, "px": datum_atlas_core::image::b64_encode(&px) }) } },
        "ebsd_open" => { let p = fixture(a, "map.ang"); let m = datum_atlas_core::ebsd::parse(&std::fs::read_to_string(&p).map_err(|e| e.to_string())?, "map.ang")?; keep(s, "ebsd", Item::ebsd(&m))? }
        "ebsd_example" => keep(s, "ebsd", Item::ebsd(&datum_atlas_core::ebsd::example()?))?,
        "ebsd_draw" => { let m = s.open.ebsd(&s.doc.p.research)?; json!({ "w": m.w, "h": m.h, "rgb": m.draw(a["kind"].as_str().unwrap_or("ipf"), a["dir"].as_str().unwrap_or("z"))? }) }
        "ebsd_grains" => { let m = s.open.ebsd(&s.doc.p.research)?; let q: datum_atlas_core::ebsd::GrainAsk = serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?; let (rgb, o) = m.draw_grains(a["kind"].as_str().unwrap_or("grains"), &q)?; json!({ "w": m.w, "h": m.h, "rgb": rgb, "out": o }) }
        "ebsd_figure" => { let m = s.open.ebsd(&s.doc.p.research)?; let (ph, of) = (a["phase"].as_u64().unwrap_or(1) as u8, a["of"].as_str().unwrap_or("100")); json!(if a["kind"] == "ipf" { m.inverse_pole_figure(ph, of)? } else { m.pole_figure(ph, of, 41)? }) }
        "dic_open" => { let p = fixture(a, "field.csv"); let t = sheet::read(std::path::Path::new(&p))?; let raw: Vec<Vec<String>> = std::iter::once(t.head).chain(t.rows).collect(); let d = datum_atlas_core::dic::parse(&raw, "field.csv")?; keep(s, "dic", Item::dic(&d))? }
        "dic_example" => keep(s, "dic", Item::dic(&datum_atlas_core::dic::example()))?,
        "dic_draw" => { let d = s.open.dic(&s.doc.p.research)?; let k = a["key"].as_str().unwrap_or("exx"); let (rgb, range) = d.draw(k, None); json!({ "w": d.w, "h": d.h, "rgb": rgb, "range": range, "stats": datum_atlas_core::dic::COMPONENTS.iter().map(|c| d.stats(c.0)).collect::<Vec<_>>() }) }
        "dic_profile" => { let d = s.open.dic(&s.doc.p.research)?; let pa: [f64; 2] = serde_json::from_value(a["a"].clone()).map_err(|e| e.to_string())?; let pb: [f64; 2] = serde_json::from_value(a["b"].clone()).map_err(|e| e.to_string())?; json!(d.profile(a["key"].as_str().unwrap_or("exx"), pa, pb, 200)) }
        "dic_link" => { let (m, d) = s.open.both(&s.doc.p.research)?; json!(datum_atlas_core::dic::by_grain(d, m, &serde_json::from_value(a["ask"].clone()).map_err(|e| e.to_string())?)?) }
        "dic_link_example" => { let m = datum_atlas_core::ebsd::example()?; let d = datum_atlas_core::dic::example_on(&m); s.doc.apply(Op::Batch(vec![Op::Research { which: "ebsd".into(), item: Some(Box::new(Item::ebsd(&m))) }, Op::Research { which: "dic".into(), item: Some(Box::new(Item::dic(&d))) }]))?; out(s) }
        "graph" => py(&mut s.py, json!({ "cmd": "render", "spec": a["spec"] }))?,
        "graph_export" => { py(&mut s.py, json!({ "cmd": "export", "spec": a["spec"], "fmt": a["fmt"], "dpi": 300 }))?; json!(format!("{}.{}", a["name"].as_str().unwrap_or("graph"), a["fmt"].as_str().unwrap_or("png"))) }
        "engine" => { let v = py(&mut s.py, json!({ "cmd": "ping" })).ok(); json!({ "versions": v, "error": null }) }
        "restart_engine" => { s.py = None; py(&mut s.py, json!({ "cmd": "ping" }))? }
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
        // A project file in the temp folder: save, then open it again (the shell asks where).
        "save" => { s.doc.save(&std::env::temp_dir().join("datum-atlas-bridge.datlas"))?; let mut o = out(s); o["path"] = json!("bridge.datlas"); o }
        "open" => { s.doc = Doc::open(&a["path"].as_str().map(std::path::PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("datum-atlas-bridge.datlas")))?; out(s) }
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
    let mut s = St { py: None, open: Open::default(), doc: Doc::new(templates::build(&d, Some("blank")).unwrap()), db: d, user, others: vec![], rev: 0, db_rev: 1 };
    let stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let q: Value = serde_json::from_str(&line).unwrap_or_default();
        let r = match handle(&mut s, q["cmd"].as_str().unwrap_or(""), &q["args"]) { Ok(v) => json!({ "ok": v }), Err(e) => json!({ "err": e }) };
        let mut o = stdout.lock();
        let _ = writeln!(o, "{r}");
        let _ = o.flush();
    }
}
