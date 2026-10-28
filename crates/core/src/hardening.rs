//! Hardening tests (FAMILY §11): a failed edit changes nothing, random projects and databases never
//! panic and keep their invariants, damaged files open or give a clear error, and a large selection
//! stays fast.
use crate::{chart, db::{self, Db}, doc::{self, Doc, Op}, goals, select, templates, *};
use std::{panic, time::Instant};

/// A small deterministic random generator (no extra crate).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    fn below(&mut self, n: u64) -> u64 { self.next() % n.max(1) }
    fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T { &v[self.below(v.len() as u64) as usize] }
    /// A number from a wide, awkward spread: zero, negatives, tiny, huge.
    fn num(&mut self) -> f64 { *self.pick(&[0.0, -1.0, 1e-12, 0.5, 1.0, 2.0, 7.3, 100.0, 2700.0, 1e9, 1e300, -1e300, f64::MIN_POSITIVE]) }
}

fn data() -> Db { Db::new(db::builtin(), None) }

/// Everything the window would ask of a project: every chart laid out and the results. Checks the
/// invariants that must hold whatever the project.
fn exercise(d: &Db, p: &Project, at: &str) {
    let fd = d.with_formulas(&p.formulas);
    let d = &fd;
    for c in &p.charts {
        let o = chart::build(d, c);
        for b in &o.bubbles {
            assert!(b.c.iter().chain(&b.r).all(|v| v.is_finite()), "{at}: a bubble off the chart ({} on {:?})", b.name, chart_name(c));
        }
        for h in &o.hulls { assert!(h.pts.iter().flatten().all(|v| v.is_finite()), "{at}: an envelope off the chart"); }
        assert!(o.x.lim.iter().chain(&o.y.lim).all(|v| v.is_finite()), "{at}: axis limits not finite");
    }
    let r = select::run(d, p);
    assert!(r.pass <= r.total && r.rows.len() == r.total, "{at}: counts do not add up");
    assert!(r.stages.iter().all(|s| s.pass <= r.total), "{at}: a stage passes more than all materials");
    assert_eq!(r.rows.iter().filter(|x| x.pass).count(), r.pass, "{at}: pass count differs from the rows");
    for x in &r.rows {
        assert!(x.pass || !x.reasons.is_empty(), "{at}: {} fails without a reason", x.name);
        if let Some(v) = x.ratio { assert!(v.is_finite(), "{at}: × best not finite for {}", x.name) }
    }
}

/// A project with random charts (bubble and bar, any axes, odd lines and boxes) and random stages.
fn random_project(r: &mut Rng) -> Project {
    let mut p = Project::new("R");
    // Formulas of every kind, some that do not parse, and one that uses another.
    for _ in 0..r.below(3) {
        let (name, expr) = *r.pick(&[("Spring", "[sy]^2/[E]"), ("Odd", "[tmin]^0.5"), ("Zero", "1/([E]-[E])"), ("Broken", "[E]/"), ("Chain", "[Spring]*2"), ("Big", "exp([rho])"), ("Neg", "-[E]")]);
        if !p.formulas.iter().any(|f| f.name == name) { p.formulas.push(Formula { name: name.into(), expr: expr.into(), unit: String::new(), log: r.below(2) == 0 }); }
    }
    let mut keys: Vec<String> = PROPS.iter().map(|p| p.key.to_string()).collect();
    keys.extend(p.formulas.iter().map(Formula::key));
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    for _ in 0..1 + r.below(4) {
        let sels = (0..r.below(4)).map(|_| match r.below(3) {
            0 => ChartSel::Line { slope: r.num(), x: r.num(), y: r.num(), above: r.below(2) == 0 },
            1 => ChartSel::Box { x0: r.num(), x1: r.num(), y0: r.num(), y1: r.num() },
            _ => ChartSel::Poly { pts: (0..r.below(7)).map(|_| [r.num(), r.num()]).collect() },
        }).collect();
        let c = Chart {
            x: r.pick(&keys).to_string(), y: r.pick(&keys).to_string(), log_x: r.below(2) == 0, log_y: r.below(2) == 0,
            hull: r.pick(&["family", "class", "none", "?"]).to_string(), kind: r.pick(&["bubble", "bar", "pie"]).to_string(),
            hidden: if r.below(4) == 0 { vec!["Metals".into()] } else { vec![] }, guides: vec![1.0, r.num()], sels, ..Default::default()
        };
        p.add_chart(c);
    }
    for _ in 0..r.below(5) {
        let (a, b) = (r.num(), r.num());
        p.stages.limits.push(Limit { prop: r.pick(&keys).to_string(), min: (r.below(2) == 0).then_some(a.min(b)), max: (r.below(2) == 0).then_some(a.max(b)),
            flag: (r.below(4) == 0).then_some(true), rating: (r.below(4) == 0).then(|| r.pick(&["Poor", "Excellent", "Nonsense"]).to_string()), goal: false });
    }
    if r.below(3) == 0 { p.stages.exclude.push(r.pick(&["Metals", "Polymers", "Metals › Ferrous", "Nowhere"]).to_string()) }
    p.stages.rule = r.pick(&["mean", "all", "?"]).to_string();
    p.stages.missing = r.pick(&["fail", "flag", "?"]).to_string();
    p.stages.rank_chart = r.below(6) as u32;
    p.stages.rank_line = r.below(4) as usize;
    p.stages.ranking = r.pick(&["line", "weighted", "?"]).to_string();
    p.stages.weights = (0..r.below(4)).map(|_| Weight { prop: r.pick(&keys).to_string(), w: r.num().abs().min(10.0), high: r.below(2) == 0 }).collect();
    p.normalize();
    p
}

#[test]
fn a_failed_edit_changes_nothing() {
    let d = data();
    let mut doc = Doc::new(templates::build(&d, Some("beam")).unwrap_or_default());
    let before = doc.json();
    let id = doc.p.charts[0].id;
    let mut bad_limits = doc.p.stages.clone();
    bad_limits.limits.push(Limit { prop: "E".into(), min: Some(10.0), max: Some(1.0), ..Default::default() });
    let bad = vec![
        Op::DelChart { id: 999 },
        Op::SetChart { chart: Box::new(Chart { id: 999, ..Default::default() }) },
        Op::Select { chart: 999, sels: vec![] },
        Op::Select { chart: id, sels: vec![ChartSel::Line { slope: f64::NAN, x: 1.0, y: 1.0, above: true }] },
        Op::Select { chart: id, sels: vec![ChartSel::Line { slope: 1.0, x: f64::INFINITY, y: 1.0, above: true }] },
        Op::Select { chart: id, sels: vec![ChartSel::Poly { pts: vec![[1.0, 1.0], [2.0, 2.0]] }] },
        Op::SetStages { stages: bad_limits },
        Op::Name { name: "   ".into() },
        Op::Goal { chart: id, goal: "beam-stiff".into() },
        // A batch whose last edit fails must leave nothing of its first edits behind.
        Op::Batch(vec![Op::Name { name: "Changed".into() }, Op::Notes { text: "x".into() }, Op::DelChart { id: 999 }]),
    ];
    for o in bad {
        let name = format!("{o:?}");
        let res = panic::catch_unwind(panic::AssertUnwindSafe(|| doc.apply(o)));
        assert!(res.as_ref().is_ok_and(|x| x.is_err()), "{name} should be refused");
        assert_eq!(doc.json(), before, "a failed {name} changed the project");
        assert!(!doc.dirty, "a failed {name} marked the project changed");
    }
    // Goals for a chart or a goal that does not exist are refused before they reach the project.
    assert!(goals::apply(&d, &doc.p, 999, "beam-stiff").is_err());
    assert!(goals::apply(&d, &doc.p, id, "no-such-goal").is_err());
}

#[test]
fn random_projects_never_panic() {
    let d = data();
    let mut r = Rng(0x2545_F491_4F6C_DD1D);
    for round in 0..300 {
        let p = random_project(&mut r);
        let res = panic::catch_unwind(panic::AssertUnwindSafe(|| exercise(&d, &p, &format!("round {round}"))));
        assert!(res.is_ok(), "round {round} panicked: {}", serde_json::to_string(&p).unwrap_or_default());
        // Every goal applies to any chart of any project, and the project it makes holds.
        let mut doc = Doc::new(p);
        let chart = doc.p.charts[0].id;
        let g = r.pick(goals::GOALS).id;
        // A goal may be refused (no data for its properties yet); refused is fine, a panic is not.
        let Ok(op) = goals::apply(&d, &doc.p, chart, g) else { continue };
        doc.apply(op).unwrap_or_else(|e| panic!("round {round}: goal {g}: {e}"));
        exercise(&d, &doc.p, &format!("round {round} after goal {g}"));
    }
}

#[test]
fn random_edits_never_break_the_project() {
    let d = data();
    let mut r = Rng(0xA076_1D64_78BD_642F);
    for round in 0..40 {
        let mut doc = Doc::new(templates::build(&d, Some("beam")).unwrap_or_default());
        for step in 0..50 {
            let ids: Vec<u32> = doc.p.charts.iter().map(|c| c.id).collect();
            let id = if ids.is_empty() || r.below(10) == 0 { 999 } else { *r.pick(&ids) };
            let q = random_project(&mut r);
            let o = match r.below(8) {
                0 => Op::AddChart { chart: Box::new(q.charts[0].clone()) },
                1 => Op::DelChart { id },
                2 => Op::SetChart { chart: Box::new(Chart { id, ..q.charts[0].clone() }) },
                3 => Op::Select { chart: id, sels: q.charts[0].sels.clone() },
                4 => Op::SetStages { stages: q.stages },
                5 => Op::Name { name: r.pick(&["A", "", "  ", "\u{0}"]).to_string() },
                6 => goals::apply(&d, &doc.p, id, r.pick(goals::GOALS).id).unwrap_or(Op::Batch(vec![])),
                _ => Op::Ui { key: "k".into(), value: serde_json::json!(r.num()) },
            };
            let (before, name) = (doc.json(), format!("{o:?}"));
            let res = panic::catch_unwind(panic::AssertUnwindSafe(|| doc.apply(o)));
            let Ok(res) = res else { panic!("round {round} step {step}: {name} panicked") };
            if res.is_err() { assert_eq!(doc.json(), before, "round {round} step {step}: a failed {name} changed the project") }
            exercise(&d, &doc.p, &format!("round {round} step {step}"));
            if r.below(6) == 0 { doc.undo() }
            if r.below(12) == 0 { doc.redo() }
        }
    }
}

/// A user database with random materials and awkward values, written as a file would be.
fn random_user_db(r: &mut Rng) -> String {
    let keys: Vec<&str> = PROPS.iter().map(|p| p.key).chain(["bogus"]).collect();
    let mats: Vec<serde_json::Value> = (0..1 + r.below(6)).map(|i| {
        let mut props = serde_json::Map::new();
        for _ in 0..r.below(8) {
            let k = r.pick(&keys).to_string();
            let v = match r.below(5) {
                0 => serde_json::json!([r.num(), r.num(), "u-s1"]),
                1 => serde_json::json!([r.num(), r.num(), "u-s1", r.num()]),
                2 => serde_json::json!([r.below(2) == 0, "u-s1"]),
                3 => serde_json::json!([r.pick(&["Good", "Nonsense"]), "u-s1"]),
                _ => serde_json::json!({ "odd": true }),
            };
            props.insert(k, v);
        }
        serde_json::json!({ "id": format!("u-{i}"), "name": if r.below(8) == 0 { "" } else { "M" }, "family": [r.pick(&["Metals", "Polymers", "Nope"])], "props": props, "checked": ["E", "rho"] })
    }).collect();
    serde_json::json!({ "format": 1, "name": "Mine", "sources": [{ "id": "u-s1", "title": "Test report", "kind": "user" }], "materials": mats }).to_string()
}

#[test]
fn random_user_databases_never_panic() {
    let mut r = Rng(0xD1B5_4A32_D192_ED03);
    for round in 0..300 {
        let text = random_user_db(&mut r);
        let res = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let Ok((user, _)) = db::read_user(&text) else { return };
            let d = Db::new(db::builtin(), Some(&user));
            let mut p = random_project(&mut r);
            p.charts.push(Chart { id: 900, ..Default::default() });
            exercise(&d, &p, &format!("round {round}"));
            let _ = db::tree(&d.materials);
        }));
        assert!(res.is_ok(), "round {round} panicked on {text}");
    }
}

/// Cuts and garbles a good file many ways; the reader must answer Ok or an error, never panic.
fn survives(name: &str, good: &str, read: &dyn Fn(&str)) {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let b = good.as_bytes();
    let mut cases: Vec<String> = (0..60).map(|k| String::from_utf8_lossy(&b[..b.len() * k / 60]).into_owned()).collect();
    for _ in 0..2000 {
        let mut v = b.to_vec();
        for _ in 0..1 + r.below(8) {
            let i = r.below(v.len() as u64) as usize;
            v[i] = *r.pick(&[b'{', b'}', b'[', b']', b'"', b',', b':', b'-', b'9', b'e', 0, 0xFF]);
        }
        cases.push(String::from_utf8_lossy(&v).into_owned());
    }
    let deep = "[".repeat(100_000);
    cases.extend(["", "{", "null", "[]", "\u{FEFF}{}", r#"{"format":99}"#, r#"{"charts":[{"x":"E","y":"E","select":{"Line":{"slope":0,"x":0,"y":0,"above":true}}}]}"#, r#"{"materials":[{"id":"u-1","name":"X","props":{"rho":[1e308,1e309,"s"]}}]}"#].map(String::from));
    cases.push(deep);
    for (k, c) in cases.iter().enumerate() {
        let res = panic::catch_unwind(panic::AssertUnwindSafe(|| read(c)));
        assert!(res.is_ok(), "{name} reader panicked on case {k}: {:?}", &c[..c.len().min(120)]);
    }
}

#[test]
fn damaged_files_never_panic() {
    let d = data();
    let p = templates::build(&d, Some("tough")).unwrap_or_default();
    survives(".datlas", &serde_json::to_string_pretty(&p).unwrap_or_default(), &|t| {
        if let Ok(p) = Project::from_json(t) { exercise(&d, &p, ".datlas") }
    });
    let user = r#"{"format":1,"name":"Mine","sources":[{"id":"u-s1","title":"Test report","kind":"user"}],"materials":[{"id":"u-1","name":"Test alloy","family":["Metals"],"props":{"rho":[2700,2750,"u-s1"],"E":[70,72,"u-s1"],"recyclable":[true,"u-s1"]}}]}"#;
    survives(".datdb", user, &|t| {
        if let Ok((u, _)) = db::read_user(t) { let d = Db::new(db::builtin(), Some(&u)); exercise(&d, &templates::build(&d, Some("beam")).unwrap_or_default(), ".datdb") }
    });
}

#[test]
fn a_damaged_user_database_is_an_error_never_empty() {
    assert!(db::read_user("").is_err());
    assert!(db::read_user("{\"materials\": [").is_err());
    assert!(db::read_user(r#"{"format":2}"#).is_err());
    // One bad value is dropped and reported; the material and its other values stay.
    let (u, notes) = db::read_user(r#"{"materials":[{"id":"u-1","name":"A","family":["Metals"],"props":{"rho":[2700,2750,"s"],"E":[-5,3,"s"]},"checked":["E","rho"]}]}"#).unwrap_or_default();
    assert_eq!(u.materials.len(), 1);
    assert!(u.materials[0].props.contains_key("rho") && !u.materials[0].props.contains_key("E"));
    assert_eq!(u.materials[0].checked, vec!["rho".to_string()]);
    assert!(notes.iter().any(|n| n.contains("E")), "{notes:?}");
}

#[test]
fn saving_replaces_the_file_whole() {
    let dir = std::env::temp_dir().join(format!("datum-atlas-hardening-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let f = dir.join("p.datlas");
    std::fs::write(&f, "old").unwrap_or_default();
    doc::write_atomic(&f, b"new").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(std::fs::read_to_string(&f).unwrap_or_default(), "new");
    assert!(std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0) == 1, "the temporary file was left behind");
    assert!(doc::write_atomic(&dir.join("no/such/dir/x.datlas"), b"x").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_large_selection_stays_fast() {
    // 1,000 materials (the built-in ones repeated) and a project of 20 charts with lines and limits.
    let mut big = db::builtin();
    let base = big.materials.clone();
    for k in 1.. { if big.materials.len() >= 1000 { break } big.materials.extend(base.iter().map(|m| Material { id: format!("{}-{k}", m.id), des: Default::default(), ..m.clone() })); }
    big.materials.truncate(1000);
    let d = Db::new(big, None);
    let mut p = Project::new("Big");
    let pairs = [("rho", "E"), ("rho", "sy"), ("sy", "kic"), ("alpha", "lam"), ("price", "co2")];
    for i in 0..20 {
        let (x, y) = pairs[i % pairs.len()];
        p.add_chart(Chart { x: x.into(), y: y.into(), sels: vec![ChartSel::Line { slope: 1.0 + (i % 3) as f64, x: 1000.0, y: 10.0, above: true }], ..Default::default() });
    }
    p.stages.limits.push(Limit { prop: "tmax".into(), min: Some(100.0), ..Default::default() });
    let run = || { for c in &p.charts { let _ = chart::build(&d, c); } let _ = select::run(&d, &p); };
    run();
    let t = Instant::now();
    run();
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    // Debug builds (cargo test) are about 10–20× slower than the app.
    let limit = if cfg!(debug_assertions) { 6000.0 } else { 400.0 };
    assert!(ms < limit, "1,000 materials on 20 charts took {ms:.0} ms (limit {limit} ms)");
    eprintln!("1,000 materials on 20 charts: {ms:.0} ms");
}

#[test]
fn damaged_spreadsheets_never_panic() {
    use crate::sheet;
    let dir = std::env::temp_dir().join(format!("datum-atlas-sheets-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let t = sheet::of_database(&db::builtin());
    let (csv, xlsx) = (dir.join("good.csv"), dir.join("good.xlsx"));
    let _ = std::fs::write(&csv, crate::export::csv(&t));
    let _ = std::fs::write(&xlsx, crate::export::xlsx(&t, "Materials").unwrap_or_default());
    // The good workbook reads back with its columns guessed.
    let back = sheet::read(&xlsx).unwrap_or_else(|e| panic!("{e}"));
    assert!(back.guess.iter().any(|g| g == "rho") && back.rows.len() == db::builtin().materials.len());
    let mut r = Rng(0xBF58_476D_1CE4_E5B9);
    for (k, good) in [csv, xlsx].iter().enumerate() {
        let b = std::fs::read(good).unwrap_or_default();
        for case in 0..200 {
            let mut v = if case < 30 { b[..b.len() * case / 30].to_vec() } else { b.clone() };
            for _ in 0..if case < 30 { 0 } else { 1 + r.below(6) } {
                let i = r.below(v.len().max(1) as u64) as usize;
                if let Some(x) = v.get_mut(i) { *x = *r.pick(&[b',', b'"', b'\n', b'\r', b'0', b'-', b';', 0, 0xFF]) }
            }
            let f = dir.join(format!("bad{k}_{case}.{}", if k == 0 { "csv" } else { "xlsx" }));
            let _ = std::fs::write(&f, &v);
            let res = panic::catch_unwind(|| sheet::read(&f).map(|t| sheet::to_database(&t, &t.guess, "Metals", "x").map(|(d, _)| Db::new(db::builtin(), Some(&d)))));
            assert!(res.is_ok(), "{} case {case} panicked", f.display());
            let _ = std::fs::remove_file(&f);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
