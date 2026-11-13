use crate::chart::build;
use crate::db::{self, Db};
use crate::doc::{Doc, Op};
use crate::geometry::{ellipse, hull, inside};
use crate::model::*;
use crate::select::run;
use crate::units;

fn db() -> Db {
    Db::new(db::builtin(), None)
}

#[test]
fn builtin_data_passes_every_check() {
    let d = db::builtin();
    let errs = db::check(&d, &[]);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
    assert!(d.materials.len() >= 100, "{} materials", d.materials.len());
    for f in FAMILIES {
        assert!(d.materials.iter().any(|m| m.family[0] == *f), "no {f}");
    }
    // Every material can go on the default chart.
    assert!(d.materials.iter().all(|m| m.props.contains_key("rho")));
}

#[test]
fn checks_catch_bad_values() {
    let mut d = db::builtin();
    let m = &mut d.materials[0];
    m.props.insert("E".into(), Val::Range(300.0, 200.0, "atlas-typ".into()));
    m.props.insert("ts".into(), Val::Range(1.0, 2.0, "nowhere".into()));
    m.props.insert("rho".into(), Val::Range(0.0, 2.0, "atlas-typ".into()));
    m.props.insert("bogus".into(), Val::Flag(true, "atlas-typ".into()));
    m.checked = vec!["kic".into(), "price".into(), "cv".into()];
    d.sources.push(Source { id: "x1".into(), title: "Some property site".into(), url: "https://www.matweb.com/x".into(), kind: "datasheet".into(), ..Default::default() });
    d.sources.push(Source { id: "x2".into(), title: "A licensed book".into(), kind: "licensed".into(), ..Default::default() });
    d.materials.push(d.materials[1].clone());
    let errs = db::check(&d, &[]).join("\n");
    for want in ["low 300 > high 200", "kic: marked checked but cites \"atlas-typ\"", "price: marked checked but cites", "cv: marked checked but has no value", "is a proprietary database or an aggregator", "is not an open, public or free kind", "source \"nowhere\" not found", "must be above zero", "unknown property bogus", "duplicate id", "duplicate designation"] {
        assert!(errs.contains(want), "missing {want:?} in\n{errs}");
    }
}

#[test]
fn checked_values_cite_datasheets() {
    let d = db::builtin();
    let n: usize = d.materials.iter().map(|m| m.checked.len()).sum();
    assert!(n >= 70, "only {n} checked values");
    for m in d.materials.iter().filter(|m| !m.checked.is_empty()) {
        for k in &m.checked { assert!(d.sources.iter().any(|s| s.id == m.props[k].src() && ["datasheet", "government", "handbook"].contains(&s.kind.as_str()) && s.url.starts_with("https://")), "{} {k}", m.id); }
    }
}

#[test]
fn units_round_trip() {
    for p in PROPS.iter().filter(|p| p.kind == "range") {
        for v in [0.013, 1.0, 27.5, 7850.0, 1.9e5] {
            let (shown, _) = units::to(v, p.unit, "imperial");
            let back = units::from(shown, p.unit, "imperial");
            assert!((back - v).abs() <= 1e-9 * v.abs().max(1.0), "{} {v} → {shown} → {back}", p.key);
        }
    }
    assert!((units::to(100.0, "°C", "imperial").0 - 212.0).abs() < 1e-9);
    assert!((units::to(6.894_757_29, "MPa", "imperial").0 - 1.0).abs() < 1e-9);
    assert_eq!(units::fmt(2690.0), "2690");
    assert_eq!(units::fmt(0.0254), "0.0254");
    assert_eq!(units::fmt(23.46), "23.5");
    assert_eq!(units::range(68.0, 70.0), "68 – 70");
}

#[test]
fn bubbles_span_ranges_in_log_space() {
    let d = db();
    let c = Chart::default();
    let out = build(&d, &c);
    let b = out.bubbles.iter().find(|b| b.id == "al-6061-t6").unwrap();
    // Density 2690–2710, modulus 68–70: centre at the geometric means.
    assert!((10f64.powf(b.c[0]) - (2690.0f64 * 2710.0).sqrt()).abs() < 1e-6);
    assert!((10f64.powf(b.c[1]) - (68.0f64 * 70.0).sqrt()).abs() < 1e-9);
    assert!((b.r[1] - ((70.0f64).log10() - (68.0f64).log10()) / 2.0).abs() < 1e-12 || b.r[1] >= 0.005 * (out.y.lim[1] - out.y.lim[0]) - 1e-12);
    assert_eq!(out.bubbles.len(), d.materials.len());
    // Over seven decades of modulus, from foams to carbides.
    assert!(out.y.lim[1] - out.y.lim[0] > 6.0);
}

#[test]
fn hulls_contain_all_their_bubbles() {
    let d = db();
    for (x, y, lx, ly) in [("rho", "E", true, true), ("rho", "sy", true, true), ("sy", "kic", true, true), ("lam", "alpha", true, true), ("rho", "tmax", true, false), ("price", "co2", true, true)] {
        for level in ["family", "class"] {
            let c = Chart { x: x.into(), y: y.into(), log_x: lx, log_y: ly, hull: level.into(), ..Default::default() };
            let out = build(&d, &c);
            assert!(!out.hulls.is_empty());
            let tol = 1e-6 * (out.x.lim[1] - out.x.lim[0] + out.y.lim[1] - out.y.lim[0]);
            let lvl = if level == "class" { 1 } else { 0 };
            // Every bubble is in exactly one envelope of its own family or class.
            for b in &out.bubbles {
                let m = d.get(&b.id).unwrap();
                let hs: Vec<_> = out.hulls.iter().filter(|h| h.ids.contains(&b.id)).collect();
                assert!(hs.len() == 1 && Some(&hs[0].name) == m.family.get(lvl).or(m.family.first()), "{x}/{y} {level}: {}", b.name);
            }
            for h in &out.hulls {
                for b in out.bubbles.iter().filter(|b| h.ids.contains(&b.id)) {
                    for p in ellipse(b.c, b.r, 64) {
                        assert!(inside(&h.pts, p, tol), "{x}/{y} {level}: {} sticks out of the {} hull", b.name, h.name);
                    }
                }
            }
        }
    }
    // One bubble and two bubbles still get a closed outline.
    let one = hull(&[([0.0, 0.0], [0.1, 0.1])], 0.05, [1.0, 1.0]);
    assert!(one.len() > 10 && ellipse([0.0, 0.0], [0.1, 0.1], 32).into_iter().all(|p| inside(&one, p, 1e-9)));
}

#[test]
fn missing_and_non_positive_values_are_left_out_never_zero() {
    let d = db();
    let c = Chart { x: "rho".into(), y: "price".into(), ..Default::default() };
    let out = build(&d, &c);
    let (id, why) = out.missing.iter().find(|m| m.0 == "bone").expect("bone has no price");
    assert_eq!(id, "bone");
    assert!(why.contains("no price"));
    assert!(!out.bubbles.iter().any(|b| b.id == "bone"));
}

fn project(sel: Option<ChartSel>) -> Project {
    let mut p = Project::new("t");
    p.add_chart(Chart { sels: sel.into_iter().collect(), ..Default::default() });
    p
}

#[test]
fn index_line_passes_materials_on_the_right_side_for_slopes_1_2_3() {
    let d = db();
    for n in [1.0, 2.0, 3.0] {
        for above in [true, false] {
            let p = project(Some(ChartSel::Line { slope: n, x: 1000.0, y: 10.0, above }));
            let r = run(&d, &p);
            for row in &r.rows {
                let m = d.get(&row.id).unwrap();
                let (e, rho) = (m.props["E"].range().unwrap(), m.props["rho"].range().unwrap());
                let (e, rho) = ((e.0 * e.1).sqrt(), (rho.0 * rho.1).sqrt());
                let over = e.log10() - 1.0 >= n * (rho.log10() - 3.0);
                assert_eq!(row.pass, over == above || (e.log10() - 1.0 - n * (rho.log10() - 3.0)).abs() < 1e-12, "{} n={n} above={above}", row.name);
            }
            assert!(r.pass > 0 && r.pass < r.total);
        }
    }
    // E^½/ρ ranking: the best first, ratio 1, falling.
    let r = run(&d, &project(Some(ChartSel::Line { slope: 2.0, x: 1000.0, y: 10.0, above: true })));
    assert_eq!(r.index_name, "E½/ρ");
    // The line through ρ = 1000 kg/m³, E = 10 GPa: E^½/ρ = √10 / 1000.
    assert!((r.lines[0].value - 10f64.sqrt() / 1000.0).abs() < 1e-12);
    let ranked: Vec<_> = r.rows.iter().filter(|x| x.pass).collect();
    assert_eq!(ranked[0].ratio, Some(1.0));
    assert!(ranked.windows(2).all(|w| w[0].index >= w[1].index));
    assert!(ranked.iter().any(|x| x.id == "cfrp-ud") && !ranked.iter().any(|x| x.id == "steel-1020"));
}

#[test]
fn limits_fail_missing_values_with_a_reason_unless_flagged() {
    let d = db();
    let mut p = project(None);
    p.stages.limits.push(Limit { prop: "tmax".into(), min: Some(150.0), ..Default::default() });
    let r = run(&d, &p);
    let pe = r.rows.iter().find(|x| x.id == "pe-ld").unwrap();
    assert!(!pe.pass && pe.reasons[0].contains("Max service temperature 60 – 80 °C < 150 °C"), "{:?}", pe.reasons);
    assert!(r.rows.iter().find(|x| x.id == "ss-304").unwrap().pass);
    let bone = r.rows.iter().find(|x| x.id == "bone").unwrap();
    assert!(!bone.pass && bone.reasons[0].contains("no data"));
    p.stages.missing = "flag".into();
    let bone = run(&d, &p).rows.into_iter().find(|x| x.id == "bone").unwrap();
    assert!(bone.pass && bone.reasons[0].contains("kept"));
    // Whole-range rule: 4340 (yield 470–1600) passes a 1000 MPa minimum on its mean, not on its whole range.
    let mut p = project(None);
    p.stages.limits.push(Limit { prop: "sy".into(), min: Some(800.0), ..Default::default() });
    assert!(run(&d, &p).rows.iter().find(|x| x.id == "steel-4340").unwrap().pass);
    p.stages.rule = "all".into();
    let s = run(&d, &p).rows.into_iter().find(|x| x.id == "steel-4340").unwrap();
    assert!(!s.pass && s.reasons[0].contains("partly below"));
    // Flags: recyclable = yes.
    let mut p = project(None);
    p.stages.limits.push(Limit { prop: "recyclable".into(), flag: Some(true), ..Default::default() });
    let r = run(&d, &p);
    assert!(r.rows.iter().find(|x| x.id == "ep").is_some_and(|x| !x.pass) && r.rows.iter().find(|x| x.id == "al-6061-t6").is_some_and(|x| x.pass));
}

#[test]
fn stages_combine_and_count() {
    let d = db();
    let mut p = project(Some(ChartSel::Box { x0: 1000.0, x1: 3000.0, y0: 30.0, y1: 500.0 }));
    p.stages.limits.push(Limit { prop: "price".into(), max: Some(10.0), ..Default::default() });
    p.stages.exclude.push("Ceramics and glasses".into());
    let r = run(&d, &p);
    assert_eq!(r.stages.len(), 3);
    for row in r.rows.iter().filter(|x| x.pass) {
        let m = d.get(&row.id).unwrap();
        assert!(m.family[0] != "Ceramics and glasses", "{}", row.name);
    }
    assert!(r.rows.iter().any(|x| x.id == "al-6061-t6" && x.pass));
    assert!(r.rows.iter().any(|x| x.id == "glass-soda" && !x.pass && x.reasons.iter().any(|s| s.contains("left out"))));
    assert_eq!(r.pass, r.rows.iter().filter(|x| x.pass).count());
    assert!(r.stages.iter().all(|s| s.pass >= r.pass));
}

#[test]
fn project_edits_undo_and_round_trip() {
    let mut doc = Doc::new(crate::templates::build(&db(), Some("beam")).unwrap());
    let id = doc.p.charts[0].id;
    doc.apply(Op::Select { chart: id, sels: vec![ChartSel::Line { slope: 2.0, x: 1000.0, y: 20.0, above: true }] }).unwrap();
    doc.apply(Op::SetStages { stages: Stages { limits: vec![Limit { prop: "tmax".into(), min: Some(100.0), ..Default::default() }], ..Default::default() } }).unwrap();
    assert!(doc.apply(Op::SetStages { stages: Stages { limits: vec![Limit { prop: "E".into(), min: Some(5.0), max: Some(1.0), ..Default::default() }], ..Default::default() } }).is_err());
    let mut c = doc.p.charts[0].clone();
    c.y = "sy".into();
    doc.apply(Op::SetChart { chart: Box::new(c) }).unwrap();
    assert_eq!(doc.p.charts[0].name, "Yield strength (elastic limit) vs Density");
    doc.undo();
    assert_eq!(doc.p.charts[0].y, "E");
    let back = Project::from_json(&doc.json()).unwrap();
    assert_eq!(back, doc.p);
    assert!(Project::from_json(r#"{"format": 99}"#).is_err());
    for t in crate::templates::list() {
        let p = crate::templates::build(&db(), Some(t.id)).unwrap();
        assert_eq!(p.charts.len(), 1, "{}", t.id);
        assert!(run(&db(), &p).total >= 100);
    }
}

#[test]
fn user_materials_need_name_family_density_and_sources() {
    let mut m = Material { id: "u-1".into(), name: "My alloy".into(), family: vec!["Metals".into()], ..Default::default() };
    let src = vec![Source { id: "u-s1".into(), title: "Test report 12".into(), kind: "user".into(), ..Default::default() }];
    assert!(db::check_user(&m, &src).unwrap_err().contains("Density"));
    m.props.insert("rho".into(), Val::Range(2700.0, 2750.0, "u-s1".into()));
    m.props.insert("E".into(), Val::Range(70.0, 72.0, "".into()));
    assert!(db::check_user(&m, &src).unwrap_err().contains("give a source"));
    m.props.insert("E".into(), Val::Range(70.0, 72.0, "u-s1".into()));
    assert!(db::check_user(&m, &src).is_ok());
    let user = Database { format: FORMAT, name: "My materials".into(), sources: src, materials: vec![m] };
    let d = Db::new(db::builtin(), Some(&user));
    assert!(d.get("u-1").is_some_and(|m| m.user) && d.user_count == 1);
    assert!(build(&d, &Chart::default()).bubbles.iter().any(|b| b.id == "u-1"));
}

#[test]
fn tree_and_search() {
    let d = db();
    let t = db::tree(&d.materials);
    assert_eq!(t[0].name, "Metals");
    assert_eq!(t.iter().filter(|n| n.depth == 0).map(|n| n.name.as_str()).collect::<Vec<_>>(), FAMILIES.to_vec());
    assert_eq!(t.iter().filter(|n| n.depth == 0).map(|n| n.count).sum::<usize>(), d.materials.len());
    assert!(db::search(&d.materials, "A96061").iter().any(|m| m.id == "al-6061-t6"));
    assert!(db::search(&d.materials, "marine stainless").iter().any(|m| m.id == "ss-316"));
}

#[test]
fn derived_properties_span_every_combination() {
    let d = db();
    let m = d.get("al-6061-t6").unwrap();
    let (cv, es) = (m.props["cv"].range().unwrap(), m.props["es"].range().unwrap());
    assert!((cv.0 - 2.5 * 2690.0).abs() < 1e-9 && (cv.1 - 3.5 * 2710.0).abs() < 1e-9);
    assert!((es.0 - 68.0 / 2710.0 * 1000.0).abs() < 1e-9 && (es.1 - 70.0 / 2690.0 * 1000.0).abs() < 1e-9);
    assert_eq!(m.props["cv"].src(), db::DERIVED);
    // Diffusivity of copper is about 110 mm²/s; thermal shock of soda-lime glass is tens of degrees.
    let a = d.get("cu-c11000").unwrap().props["diff"].range().unwrap();
    assert!(a.0 > 90.0 && a.1 < 130.0, "{a:?}");
    let s = d.get("glass-soda").unwrap().props["shock"].range().unwrap();
    assert!(s.0 > 20.0 && s.1 < 200.0, "{s:?}");
    assert!(!d.get("bone").unwrap().props.contains_key("cv"), "no price, no price per volume");
}

#[test]
fn every_goal_draws_its_line_ranks_by_its_index_and_adds_its_limits() {
    let d = db();
    assert_eq!(crate::goals::GOALS.iter().filter(|g| !g.library).count(), 25);
    assert_eq!(crate::goals::GOALS.len(), 52, "25 goals and 27 more indices in the library");
    let mut p = crate::templates::build(&d, Some("blank")).unwrap();
    p.stages.limits.push(Limit { prop: "price".into(), max: Some(50.0), ..Default::default() });
    let cid = p.charts[0].id;
    // Indices whose properties have no values in the database yet (to be filled with open data).
    let waiting: [&str; 0] = [];
    for g in crate::goals::GOALS {
        let mut doc = Doc::new(p.clone());
        let op = crate::goals::apply(&d, &doc.p, cid, g.id);
        if waiting.contains(&g.id) { assert!(op.is_err(), "{} now has data: take it off the waiting list", g.id); continue; }
        doc.apply(op.unwrap_or_else(|e| panic!("{}: {e}", g.id))).unwrap();
        let c = &doc.p.charts[0];
        assert_eq!((c.x.as_str(), c.y.as_str(), c.goal.as_str()), (g.x, g.y, g.id));
        // A second constraint adds its chart with its own line; the shortlist passes both.
        assert_eq!(doc.p.charts.len(), 1 + g.also.is_some() as usize, "{}", g.id);
        if let Some(a) = &g.also {
            let c2 = &doc.p.charts[1];
            assert_eq!((c2.x.as_str(), c2.y.as_str(), c2.goal.clone(), c2.sels.len()), (a.x, a.y, format!("{}/2", g.id), 1));
        }
        let fd = d.with_formulas(&doc.p.formulas);
        assert_eq!(doc.p.formulas.len(), g.formulas.len(), "{}: the formulas it needs", g.id);
        let r = run(&fd, &doc.p);
        assert!(r.pass > 0, "{}: nothing passes", g.id);
        // Your own limit stays; the goal's are added and marked.
        assert!(doc.p.stages.limits.iter().any(|l| l.prop == "price" && !l.goal));
        assert_eq!(doc.p.stages.limits.iter().filter(|l| l.goal).count(), g.limits.len(), "{}", g.id);
        // The ranking follows the index computed by hand from the geometric means.
        let gm = |m: &Material, k: &str| { let (a, b) = m.props[k].range().unwrap(); (a * b).sqrt() };
        let ranked: Vec<_> = r.rows.iter().filter(|x| x.pass).take(3).collect();
        let idx = |id: &str| { let m = fd.get(id).unwrap(); let n = g.slope; if n == 0.0 { gm(m, g.y) } else if n > 0.0 { gm(m, g.y).powf(1.0 / n) / gm(m, g.x) } else { gm(m, g.y).powf(-1.0 / n) * gm(m, g.x) } };
        for w in ranked.windows(2) {
            let (a, b) = (idx(&w[0].id), idx(&w[1].id));
            assert!(if g.above { a >= b * (1.0 - 1e-9) } else { a <= b * (1.0 + 1e-9) }, "{}: {} then {}", g.id, w[0].name, w[1].name);
        }
    }
    // Re-applying a two-chart goal keeps one second chart; another goal removes it.
    let mut doc = Doc::new(p.clone());
    for id in ["beam-stiff-strong", "beam-stiff-strong", "tie-stiff-tough"] { doc.apply(crate::goals::apply(&d, &doc.p, cid, id).unwrap()).unwrap(); }
    assert_eq!(doc.p.charts.iter().map(|c| c.goal.as_str()).collect::<Vec<_>>(), ["tie-stiff-tough", "tie-stiff-tough/2"]);
    doc.apply(crate::goals::apply(&d, &doc.p, cid, "spring").unwrap()).unwrap();
    assert_eq!(doc.p.charts.len(), 1);
    // Explore removes the line and the goal's limits.
    let mut doc = Doc::new(p.clone());
    doc.apply(crate::goals::apply(&d, &doc.p, cid, "spring").unwrap()).unwrap();
    doc.apply(crate::goals::apply(&d, &doc.p, cid, "").unwrap()).unwrap();
    assert!(doc.p.charts[0].sels.is_empty() && doc.p.charts[0].goal.is_empty() && doc.p.stages.limits.iter().all(|l| !l.goal));
    let r = run(&d, &{ let mut q = p.clone(); q.charts[0].x = "cv".into(); q.charts[0].sels = vec![ChartSel::Line { slope: 2.0, x: 1e4, y: 10.0, above: true }]; q });
    assert_eq!(r.index_name, "E½/(Cₘρ)");
}

#[test]
fn bar_charts_group_by_family_and_sort() {
    let d = db();
    let out = build(&d, &Chart { kind: "bar".into(), y: "E".into(), ..Default::default() });
    assert_eq!(out.bubbles.len(), d.materials.len());
    assert_eq!(out.bands.len(), 7);
    let metals: Vec<_> = out.bubbles.iter().filter(|b| b.fam == 0).collect();
    assert!(metals.windows(2).all(|w| w[0].c[1] <= w[1].c[1] && w[0].c[0] < w[1].c[0]));
    assert!(out.bands.windows(2).all(|w| w[0].to < w[1].from));
}

#[test]
fn site_json_is_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let s: serde_json::Value = serde_json::from_str(include_str!("../../../docs/site.json")).unwrap();
    for k in ["id", "name", "accent", "logo", "tagline", "for"] { assert!(s[k].as_str().is_some_and(|v| !v.is_empty()), "site.json: {k}"); }
    for f in s["screenshots"].as_array().unwrap().iter().chain([&s["logo"]]) { assert!(root.join(f.as_str().unwrap()).exists(), "site.json: {f} missing"); }
    let about = s["about"].as_array().unwrap();
    assert!((1..=2).contains(&about.len()) && about.iter().all(|p| p.as_str().is_some_and(|p| !p.is_empty())), "site.json: about is one or two paragraphs");
    let steps = s["steps"].as_array().unwrap();
    assert!(steps.len() == 3 && steps.iter().all(|x| ["title", "text"].iter().all(|k| x[k].as_str().is_some_and(|v| !v.is_empty()))), "site.json: three steps with title and text");
    let ver = |v: &str| v.split('.').map(|n| n.parse::<u32>().unwrap()).collect::<Vec<_>>();
    let road = s["roadmap"].as_array().unwrap();
    for r in road {
        assert_eq!(ver(r["v"].as_str().unwrap()).len(), 3, "site.json: versions have three numbers");
        assert!(["Next", "Soon", "Later", "Goal"].contains(&r["when"].as_str().unwrap()), "site.json: when");
    }
    assert!(ver(road[0]["v"].as_str().unwrap()) > ver(env!("CARGO_PKG_VERSION")), "site.json: the roadmap still starts at a released version; update it with the release");
}

#[test]
fn bands_polygons_and_old_files() {
    let d = db();
    let mut p = Project::new("T");
    let line = |y: f64, above: bool| ChartSel::Line { slope: 1.0, x: 1000.0, y, above };
    p.add_chart(Chart { sels: vec![line(10.0, true), line(100.0, false)], ..Default::default() });
    let r = run(&d, &p);
    let e_rho = |id: &str| { let m = d.get(id).unwrap(); let g = |k: &str| { let (a, b) = m.props[k].range().unwrap(); (a * b).sqrt() }; g("E") / g("rho") };
    // A band: E/ρ between 0.01 and 0.1 (GPa per kg/m³) passes, both sides fail with a reason.
    for x in &r.rows {
        let Ok(v) = std::panic::catch_unwind(|| e_rho(&x.id)) else { continue };
        assert_eq!(x.pass, (0.01..=0.1).contains(&v), "{} at {v}", x.name);
        if !x.pass { assert!(x.reasons[0].contains("selection line")) }
    }
    assert_eq!(r.lines.len(), 2);
    assert_eq!((r.lines[1].k, r.rank_line), (1, 0));
    // A triangle on log axes: inside passes, outside fails with "outside the shape".
    let mut q = Project::new("T");
    q.add_chart(Chart { sels: vec![ChartSel::Poly { pts: vec![[1000.0, 10.0], [10000.0, 10.0], [10000.0, 1000.0]] }], ..Default::default() });
    let r = run(&d, &q);
    let steel = r.rows.iter().find(|x| x.id == "steel-1045").unwrap();
    assert!(steel.pass, "{:?}", steel.reasons);
    let foam = r.rows.iter().find(|x| x.family == "Foams").unwrap();
    assert!(!foam.pass && foam.reasons[0].contains("outside the shape"));
    // A file from 0.1.x holds one "select": it opens with that selection in the list.
    let old = r#"{"format":1,"name":"Old","charts":[{"id":1,"x":"rho","y":"E","select":{"Line":{"slope":2,"x":1000,"y":20,"above":true}}}],"nextId":2}"#;
    let p = Project::from_json(old).unwrap();
    assert_eq!(p.charts[0].sels.len(), 1);
    assert!(!serde_json::to_string(&p).unwrap().contains("\"select\""), "the old field is never written");
    assert!(p.eco.options.is_empty() && p.process == crate::process::ProcSel::default(), "an older file opens with an empty audit and part");
}

#[test]
fn weighted_ranking() {
    let d = db();
    let mut p = Project::new("T");
    p.add_chart(Chart::default());
    p.stages.limits.push(Limit { prop: "sy".into(), min: Some(200.0), ..Default::default() });
    // All weight on low density: the ranking is by density, lightest first, "× best" = ρ_best/ρ.
    p.stages.ranking = "weighted".into();
    p.stages.weights = vec![Weight { prop: "rho".into(), w: 1.0, high: false }];
    let r = run(&d, &p);
    assert!(r.index_name.starts_with("Weighted: 100 % Density (low)"), "{}", r.index_name);
    let rho = |id: &str| { let (a, b) = d.get(id).unwrap().props["rho"].range().unwrap(); (a * b).sqrt() };
    let pass: Vec<_> = r.rows.iter().filter(|x| x.pass).collect();
    for w in pass.windows(2) { assert!(rho(&w[0].id) <= rho(&w[1].id) * (1.0 + 1e-9), "{} then {}", w[0].name, w[1].name) }
    assert_eq!(pass[0].ratio, Some(1.0));
    let k = pass.len() - 1;
    assert!((pass[k].ratio.unwrap() - rho(&pass[0].id) / rho(&pass[k].id)).abs() < 1e-9);
    // Two properties: half price, half strength. Scores stay within 0–1 and the best scores 1.
    p.stages.weights = vec![Weight { prop: "price".into(), w: 1.0, high: false }, Weight { prop: "sy".into(), w: 1.0, high: true }];
    let r = run(&d, &p);
    assert!(r.rows.iter().filter_map(|x| x.index).all(|v| (0.0..=1.0).contains(&v)));
    assert_eq!(r.rows[0].ratio, Some(1.0));
    assert_eq!(r.rank_chart, 0);
}

#[test]
fn a_line_passes_through_the_index_value_typed() {
    let d = db();
    for (lx, ly, slope) in [(true, true, 2.0), (true, true, 1.0), (true, true, -1.0), (true, true, 0.0), (false, false, 2.0), (true, false, 1.0)] {
        let mut p = Project::new("T");
        let id = p.add_chart(Chart { log_x: lx, log_y: ly, sels: vec![ChartSel::Line { slope, x: 3000.0, y: 50.0, above: true }], ..Default::default() });
        let mut doc = Doc::new(p);
        let target = 0.004;
        match doc.apply(Op::PassThrough { chart: id, k: 0, value: target }) {
            Ok(()) => { let r = run(&d, &doc.p); assert!((r.lines[0].value - target).abs() <= 1e-9 * target.max(1.0), "{lx} {ly} {slope}: {}", r.lines[0].value) }
            Err(e) => assert!(lx != ly, "{lx} {ly} {slope}: {e}"),
        }
    }
    let mut doc = Doc::new(Project::new("T"));
    let id = doc.p.add_chart(Chart { sels: vec![ChartSel::Line { slope: 2.0, x: 3000.0, y: 50.0, above: true }], ..Default::default() });
    assert!(doc.apply(Op::PassThrough { chart: id, k: 0, value: -1.0 }).is_err());
    assert!(doc.apply(Op::PassThrough { chart: id, k: 3, value: 1.0 }).is_err());
}

#[test]
fn formulas_work_on_ranges() {
    use crate::formula::{eval, parse, show};
    let get = |k: &str| match k { "E" => Some((4.0, 9.0)), "rho" => Some((2.0, 4.0)), "tmin" => Some((-5.0, 3.0)), _ => None };
    let ev = |t: &str| eval(&parse(t, &[]).unwrap(), &get);
    assert_eq!(ev("[E]^0.5/[rho]"), Some((0.5, 1.5)));
    assert_eq!(ev("[Young's modulus] * 2 + 1"), Some((9.0, 19.0)));
    assert_eq!(ev("[ρ] - [E]"), Some((-7.0, 0.0)));
    assert_eq!(ev("[tmin]^2"), Some((0.0, 25.0)), "an even power of a range across zero starts at zero");
    assert_eq!(ev("sqrt([tmin])"), None, "the root of a negative value is a gap");
    assert_eq!(ev("1/[tmin]"), None, "dividing by a range that holds zero is a gap");
    assert_eq!(ev("[G]"), None, "a missing value is a gap, never zero");
    assert_eq!(ev("max([E], 2*[rho])"), Some((4.0, 9.0)));
    assert_eq!(ev("-[E] + 2e1"), Some((11.0, 16.0)));
    assert_eq!(show(&parse("[E]^0.5/[rho]", &[]).unwrap()), "E^0.5/ρ");
    for bad in ["[E", "[nothing]", "foo([E])", "sqrt()", "[E] +", "(", "[E] [rho]", "max([E])", "1..2"] {
        assert!(parse(bad, &[]).is_err(), "{bad} should not parse");
    }
    assert!(parse(&"(".repeat(500), &[]).is_err(), "deep nesting is refused, not a stack overflow");
}

#[test]
fn formula_properties_chart_limit_and_rank() {
    let d = db();
    let mut doc = Doc::new(Project::new("T"));
    let f = |n: &str, e: &str| Formula { name: n.into(), expr: e.into(), unit: String::new(), log: true };
    // Names must be new and formulas must parse; a later formula may use an earlier one.
    assert!(doc.apply(Op::SetFormulas { formulas: vec![f("Density", "[E]")] }).is_err());
    assert!(doc.apply(Op::SetFormulas { formulas: vec![f("Bad", "[E]/")] }).is_err());
    doc.apply(Op::SetFormulas { formulas: vec![f("Spring", "[sy]^2/[E]"), f("Spring per mass", "[Spring]/[rho]")] }).unwrap();
    let id = doc.p.add_chart(Chart { x: "rho".into(), y: "f:Spring".into(), sels: vec![ChartSel::Line { slope: 1.0, x: 1000.0, y: 1.0, above: true }], ..Default::default() });
    doc.p.stages.limits.push(Limit { prop: "f:Spring per mass".into(), min: Some(0.05), ..Default::default() });
    doc.p.normalize();
    let fd = d.with_formulas(&doc.p.formulas);
    let o = crate::chart::build(&fd, doc.p.chart(id).unwrap());
    assert_eq!(o.y.name, "Spring");
    assert!(o.bubbles.len() > 50, "{}", o.bubbles.len());
    let steel = fd.get("steel-1045").unwrap();
    let (sy, e) = (steel.props["sy"].range().unwrap(), steel.props["E"].range().unwrap());
    assert_eq!(steel.props["f:Spring"].range(), Some((sy.0 * sy.0 / e.1, sy.1 * sy.1 / e.0)));
    let r = run(&fd, &doc.p);
    assert_eq!(r.index_name, "Spring/ρ");
    assert!(r.rows.iter().any(|x| !x.pass && x.reasons.iter().any(|s| s.starts_with("Spring per mass") && s.contains(" < 0.05"))), "a limit on a formula fails with its value");
    assert!(r.rows.iter().any(|x| x.pass), "and some materials pass it");
    // Removing a formula sends its chart axis back to the default and drops its limit.
    doc.apply(Op::SetFormulas { formulas: vec![] }).unwrap();
    assert_eq!((doc.p.charts[0].y.as_str(), doc.p.stages.limits.len()), ("E", 0));
}

#[test]
fn results_export_to_csv_and_xlsx() {
    use crate::export::{csv, results, xlsx, Cell};
    let d = db();
    let p = crate::templates::build(&d, Some("beam")).unwrap();
    let r = run(&d, &p);
    let t = results(&d, &p);
    assert_eq!(t.rows.len(), r.total);
    assert_eq!(t.head[4], "E½/ρ");
    assert!(t.head.iter().any(|h| h == "Young's modulus (GPa)") && t.head.iter().any(|h| h == "Density (kg/m³)"));
    // The first row is the best: rank 1, × best 1; failures say why.
    assert_eq!((&t.rows[0][0], &t.rows[0][5]), (&Cell::Num(1.0), &Cell::Num(1.0)));
    let fail = t.rows.iter().find(|x| x[3] == Cell::Text("No".into())).unwrap();
    assert!(matches!(fail.last(), Some(Cell::Text(s)) if !s.is_empty()));
    let c = csv(&t);
    assert!(c.starts_with('\u{FEFF}') && c.lines().count() == r.total + 1);
    assert!(c.contains(",\"Low carbon steel, AISI 1020\","), "names with commas are quoted");
    let b = xlsx(&t, "Light, stiff beam").unwrap();
    assert!(b.starts_with(b"PK"), "an .xlsx is a zip file");
}

#[test]
fn compare_and_radar() {
    let d = db();
    let mut doc = Doc::new(Project::new("T"));
    assert!(doc.apply(Op::Compare { ids: (0..7).map(|i| format!("m{i}")).collect(), radar: vec![] }).is_err(), "up to six");
    doc.apply(Op::Compare { ids: vec!["steel-1045".into(), "al-6061-t6".into(), "steel-1045".into(), "pa66".into()], radar: vec![] }).unwrap();
    assert_eq!(doc.p.compare.len(), 3, "a material once");
    let c = crate::compare::build(&d, &doc.p);
    let rho = c.rows.iter().find(|r| r.key == "rho").unwrap();
    assert_eq!((rho.hi, rho.lo), (Some(0), Some(2)), "steel is the densest, the polymer the lightest");
    assert!(rho.spread.unwrap() > 5.0);
    assert_eq!(c.radar.keys, crate::compare::RADAR);
    assert!(c.radar.vals.iter().flatten().flatten().all(|v| (0.0..=1.0).contains(v)));
    assert!(c.radar.vals[0][0].unwrap() > c.radar.vals[2][0].unwrap(), "steel sits further out on density");
}

#[test]
fn spreadsheets_in_and_out() {
    use crate::sheet::{from_text, guess, of_database, range, to_database};
    assert_eq!(range("2700"), Some((2700.0, 2700.0)));
    assert_eq!(range("2690 – 2710"), Some((2690.0, 2710.0)));
    assert_eq!(range("-50 to 20"), Some((-50.0, 20.0)));
    assert_eq!(range("1.2e3-1.5e3"), Some((1200.0, 1500.0)));
    assert_eq!(range("7,85"), Some((7.85, 7.85)));
    assert_eq!(range("n/a"), None);
    assert_eq!(guess("Density (kg/m³)"), "rho");
    assert_eq!(guess("Yield strength (ksi)"), "sy|imperial");
    assert_eq!(guess("E"), "E");
    assert_eq!(guess("Material"), "name");
    assert_eq!(guess("Colour"), "");
    // A semicolon file with quotes, a range, an imperial column, a bad cell and a row without values.
    let t = from_text("Material;Family;Density (kg/m³);Yield strength (ksi);Recyclable;Water\n\"Alloy; A\";Metals;2690 – 2710;40;yes;Good\nResin B;polymers;1200;abc;no;\nEmpty C;Metals;;;;\n").unwrap();
    assert_eq!(t.head.len(), 6);
    assert_eq!(t.guess, ["name", "family", "rho", "sy|imperial", "recyclable", "water"]);
    let (d, notes) = to_database(&t, &t.guess, "Metals", "Test report 9").unwrap();
    assert_eq!(d.materials.len(), 2, "the row without values is left out");
    let a = &d.materials[0];
    assert_eq!((a.name.as_str(), a.family[0].as_str()), ("Alloy; A", "Metals"));
    assert_eq!(a.props["rho"].range(), Some((2690.0, 2710.0)));
    assert!((a.props["sy"].range().unwrap().0 - 275.79).abs() < 0.01, "40 ksi is 275.8 MPa");
    assert_eq!(d.materials[1].family[0], "Polymers");
    assert!(notes.iter().any(|n| n.contains("abc")) && notes.iter().any(|n| n.contains("Empty C")));
    assert_eq!(d.sources.len(), 1);
    assert!(to_database(&t, &t.guess, "Metals", "").is_err(), "a source is needed");
    // A database written out reads back the same values.
    let out = of_database(&db::builtin());
    let text = crate::export::csv(&out);
    let back = from_text(&text).unwrap();
    let (d2, _) = to_database(&back, &back.guess, "Metals", "Round trip").unwrap();
    let steel = d2.materials.iter().find(|m| m.name == "Medium carbon steel, AISI 1045").unwrap();
    assert_eq!(steel.props["E"].range(), Some((200.0, 210.0)));
    assert_eq!(d2.materials.len(), db::builtin().materials.len());
}

#[test]
fn other_databases_sit_beside_the_built_in_data() {
    let mine = Database { name: "Team data".into(), sources: vec![Source { id: "s1".into(), title: "Lab report".into(), kind: "user".into(), ..Default::default() }],
        materials: vec![Material { id: "u-1".into(), name: "Team alloy".into(), family: vec!["Metals".into()], props: [("rho".to_string(), Val::Range(2700.0, 2800.0, "s1".into())), ("E".to_string(), Val::Range(70.0, 72.0, "s1".into()))].into(), ..Default::default() }], ..Default::default() };
    let d = Db::new(db::builtin(), None).with_others(&[mine.clone(), mine]);
    let a = d.get("d1:u-1").unwrap();
    assert_eq!((a.origin.as_str(), a.props["rho"].src(), a.user), ("Team data", "d1:s1", false));
    assert!(d.get("d2:u-1").is_some(), "the same file opened twice does not clash");
    assert!(a.props.contains_key("es"), "derived properties are worked out");
    assert!(d.sources.iter().any(|s| s.id == "d2:s1"));
    assert!(db::check(&Database { materials: d.materials.iter().filter(|m| !m.origin.is_empty()).cloned().collect(), ..Default::default() }, &d.sources).is_empty());
}

#[test]
fn eco_data_passes_every_check() {
    let d = db::builtin();
    let errs = crate::eco::check(crate::eco::data(), &d.sources);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
    let mut no_eco = vec![];
    for m in &d.materials {
        let r = |k: &str| m.props.get(k).and_then(Val::range);
        if let (Some(e), Some(rc)) = (r("ee"), r("rc_ee")) { assert!(rc.1 <= e.1, "{}: recycling takes more energy than primary", m.name); }
        if let (Some(c), Some(rc)) = (r("co2"), r("rc_co2")) { assert!(rc.1 <= c.1, "{}: recycling releases more CO₂ than primary", m.name); }
        if m.props.contains_key("rc_ee") { assert!(m.props.get("recyclable").and_then(Val::flag) == Some(true), "{}: recycling values on a material that is not recyclable", m.name); }
        if m.props.contains_key("hc") { assert!(m.props.contains_key("co2_comb"), "{}: heat of combustion without its CO₂", m.name); }
        if r("ee").is_none() { no_eco.push(m.id.as_str()); }
    }
    // The gaps left on purpose: no open source gives a fair figure.
    assert_eq!(no_eco, ["be", "zr-r60804", "leather", "bone"]);
}

#[test]
fn eco_audit_works_out_each_phase() {
    use crate::eco::*;
    let d = db();
    let e = |k: &str| d.get("steel-1020").unwrap().props[k].range().unwrap();
    let (ee, co2) = (e("ee"), e("co2"));
    let part = Part { name: "Bracket".into(), material: "steel-1020".into(), mass: 10.0, ..Default::default() };
    let opt = Opt { name: "Steel".into(), parts: vec![part.clone()], legs: vec![Leg { mode: "truck".into(), km: 1000.0 }],
        use_: Use { kind: "static".into(), kw: 0.1, hours: 10.0, days: 100.0, ..Default::default() }, years: 2.0 };
    let a = audit(&d, &Eco { options: vec![opt.clone()], grid: "world".into() });
    let o = &a.options[0];
    let near = |x: f64, y: f64| (x - y).abs() < 1e-9 * y.abs().max(1.0);
    assert!(near(o.phases[0].mj[0], 10.0 * ee.0) && near(o.phases[0].mj[1], 10.0 * ee.1), "material: mass × embodied energy");
    assert!(near(o.phases[0].co2[1], 10.0 * co2.1));
    assert_eq!(o.phases[2].mj, [10.0, 16.0], "10 tonne·km by truck at 1 to 1.6 MJ");
    assert!(near(o.phases[3].co2[0], 200.0 * 0.45) && near(o.phases[3].co2[1], 200.0 * 0.5), "200 kWh on the world grid");
    assert!(near(o.phases[4].mj[0], 1.0) && near(o.phases[4].mj[1], 3.0), "landfill handling");
    let t = crate::export::eco(&a);
    assert_eq!(t.rows.len(), 5 + 2 + 1, "five phases, two totals, one part");
    assert!(crate::export::csv(&t).contains("Material") && crate::export::xlsx(&t, "Eco").is_ok());
    assert_eq!(o.dominant, "Use");
    // Recycled content lowers the material phase; recycling at end of life is a credit (negative).
    let mut rec = opt.clone();
    rec.parts[0].recycled = 100.0;
    rec.parts[0].eol = "recycle".into();
    let b = audit(&d, &Eco { options: vec![rec], grid: "world".into() });
    assert!(b.options[0].phases[0].mj[1] < o.phases[0].mj[0]);
    assert!(b.options[0].phases[4].mj[1] < 0.0 && b.options[0].net.mj[1] < b.options[0].total.mj[1]);
    // Steel cannot be burnt for energy: said, and left out (never a silent zero).
    let mut burn = opt.clone();
    burn.parts[0].eol = "burn".into();
    let c = audit(&d, &Eco { options: vec![burn], grid: "world".into() });
    assert!(c.options[0].notes.iter().any(|n| n.contains("not combustible")));
    // A material without embodied energy is a reported gap.
    let mut gap = opt;
    gap.parts[0].material = "bone".into();
    let g = audit(&d, &Eco { options: vec![gap], grid: "world".into() });
    assert!(g.options[0].notes.iter().any(|n| n.contains("gap")) && g.options[0].phases[0].mj == [0.0, 0.0]);
}

#[test]
fn eco_break_even_of_a_lighter_option() {
    use crate::eco::*;
    let d = db();
    let car = |mat: &str, kg: f64| Opt { name: mat.into(), parts: vec![Part { material: mat.into(), mass: kg, ..Default::default() }], legs: vec![],
        use_: Use { kind: "mobile".into(), vehicle: "car-petrol".into(), km: 10_000.0, ..Default::default() }, years: 15.0 };
    let a = audit(&d, &Eco { options: vec![car("steel-1020", 10.0), car("al-6061-t6", 5.0)], grid: "world".into() });
    let (s, al) = (&a.options[0], &a.options[1]);
    assert!((s.rate[0] - 2.0 * al.rate[0]).abs() < 1e-9, "half the mass, half the fuel");
    let t = a.breakeven[0].mj_years.expect("the lighter, energy-hungrier option pays back");
    assert!((t - (al.fixed[0] - s.fixed[0]) / (s.rate[0] - al.rate[0])).abs() < 1e-9 && t > 0.0);
}

#[test]
fn eco_audit_edits_are_checked() {
    use crate::eco::*;
    let mut doc = Doc::new(Project::new("t"));
    let bad = Eco { options: vec![Opt { parts: vec![Part { mass: -1.0, ..Default::default() }], ..Default::default() }], ..Default::default() };
    assert!(doc.apply(Op::Eco { eco: bad }).is_err());
    assert!(doc.apply(Op::Eco { eco: Eco { options: vec![Opt::default(); 5], ..Default::default() } }).is_err());
    // A damaged file is mended on opening: numbers in range, unknown choices back to defaults.
    let mut p = Project::new("t");
    p.eco = Eco { grid: "mars".into(), options: vec![Opt { years: f64::NAN, parts: vec![Part { recycled: 400.0, eol: "x".into(), process: "y".into(), ..Default::default() }], ..Default::default() }] };
    let mut back = p.clone();
    back.normalize();
    let o = &back.eco.options[0];
    assert_eq!((back.eco.grid.as_str(), o.parts[0].recycled, o.parts[0].eol.as_str(), o.parts[0].process.as_str()), ("world", 100.0, "landfill", "none"));
    assert!(o.years.is_finite());
}

#[test]
fn greener_substitutes_do_the_same_job_with_less_co2() {
    let d = db();
    let mut doc = Doc::new(crate::templates::build(&d, Some("beam")).unwrap());
    doc.apply(Op::SetStages { stages: Stages::default() }).unwrap();
    let s = crate::eco::substitutes(&d, &doc.p, "steel-1020", 0.5);
    assert!(s.basis.contains("same job"), "{}", s.basis);
    assert!(!s.rows.is_empty());
    assert!(s.rows.windows(2).all(|w| w[0].co2 <= w[1].co2) && s.rows.iter().all(|r| r.co2 < 1.0 && r.index.unwrap() >= 0.5));
    assert!(crate::eco::substitutes(&d, &doc.p, "nothing", 0.1).rows.is_empty());
}

#[test]
fn process_data_passes_every_check() {
    let errs = crate::process::check(&db::builtin().sources);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
    let ps = crate::process::processes();
    assert!(ps.len() >= 40, "{} processes", ps.len());
    for g in ["Shaping", "Machining", "Additive", "Joining", "Finishing"] { assert!(ps.iter().any(|p| p.group == g), "no {g}"); }
    // Every material can be made by some process, and every metal and polymer has processing ratings.
    for m in &db::builtin().materials {
        assert!(ps.iter().any(|p| crate::process::SELECTABLE.contains(&p.group.as_str()) && crate::process::suits(p, m)), "nothing makes {}", m.name);
        if ["Metals", "Polymers"].contains(&m.family[0].as_str()) { for k in ["cast", "mach", "weld"] { assert!(m.props.contains_key(k), "{}: no {k}", m.name); } }
    }
}

#[test]
fn process_selection_says_why_each_process_fails() {
    use crate::process::*;
    let d = db();
    let mut p = Project::new("t");
    p.process = ProcSel { material: "al-a356-t6".into(), mass: 2.0, section: 4.0, shape: "3d-hollow".into(), tol: 0.5, rough: 0.0, batch: 20_000.0, ..Default::default() };
    let o = select(&d, &p);
    let row = |id: &str| o.rows.iter().find(|r| r.id == id).unwrap();
    assert!(row("die-cast").reasons.iter().any(|r| r.contains("cannot make")), "die casting makes no hollow parts here");
    assert!(row("injection-mould").reasons.iter().any(|r| r.contains("does not work")));
    assert!(row("sand-cast").reasons.iter().any(|r| r.contains("tolerance")));
    assert!(row("investment-cast").pass && row("perm-mould").pass);
    assert_eq!(o.pass, o.rows.iter().filter(|r| r.pass).count());
    assert!(o.rows.iter().take_while(|r| r.pass).count() == o.pass, "passing processes come first");
    // The cost model: material + tooling shared + overhead; tooling fades as the batch grows.
    let die = processes().iter().find(|x| x.id == "die-cast").unwrap();
    let a = cost(die, 1.0, [2.0, 2.0], 10.0);
    let b = cost(die, 1.0, [2.0, 2.0], 1e6);
    assert!((a[0] - (2.0 / 0.95 + die.tooling[0] / 10.0 + die.overhead[0] / die.rate[1])).abs() < 1e-9);
    assert!(b[1] < a[0] && a[0] <= a[1]);
    let sand = row("sand-cast");
    assert!(sand.curve.is_empty() && sand.cost.is_none(), "no cost for a process that cannot make the part");
    let inv = row("investment-cast");
    assert_eq!(inv.curve.len(), batches().len());
    assert!(o.cheapest.iter().all(|(_, id)| o.rows.iter().any(|r| &r.id == id && !r.curve.is_empty())));
    // Joining aluminium to steel, and finishes that protect aluminium from corrosion.
    p.process.join = "steel-1020".into();
    p.process.purpose = "corrosion".into();
    let o = select(&d, &p);
    assert!(o.joins.contains(&"adhesive".to_string()) && o.joins.contains(&"friction-stir".to_string()) && !o.joins.contains(&"soldering".to_string()));
    assert!(o.finishes.contains(&"anodise".to_string()) && !o.finishes.contains(&"galvanise".to_string()));
    // A bad part is refused; a damaged one is mended.
    let mut doc = Doc::new(Project::new("t"));
    assert!(doc.apply(Op::Process { sel: ProcSel { mass: -1.0, ..Default::default() } }).is_err());
    let mut q = ProcSel { mass: f64::NAN, batch: 0.0, shape: "blob".into(), ..Default::default() };
    q.normalize();
    assert_eq!((q.mass, q.batch, q.shape.as_str()), (1.0, 1.0, ""));
}

#[test]
fn the_guide_and_public_text_hold_together() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let guide = std::fs::read_to_string(format!("{root}docs/guide/en.md")).unwrap();
    // Every picture the guide shows exists, and every chapter has a heading.
    for img in guide.lines().filter_map(|l| l.strip_prefix("![")).filter_map(|l| l.split_once("](")).map(|(_, r)| r.trim_end_matches(')')) {
        assert!(std::path::Path::new(&format!("{root}docs/guide/{img}")).exists(), "the guide shows {img}, which is missing");
    }
    assert!(guide.matches("\n## ").count() >= 12);
    // Public text never names a commercial product (hub FAMILY §13); file formats are named by extension.
    let names = ["Excel", "Microsoft", "PowerPoint", "Granta", "EduPack", "Ansys", "MatWeb", "Total Materia", "MakeItFrom", "Matmatch", "Wikipedia", "SolidWorks", "MATLAB", "Google Sheets", "LibreOffice"];
    for f in ["docs/guide/en.md", "README.md", "CHANGELOG.md", "docs/site.json", "docs/MEMORY.md", "SECURITY.md", "CONTRIBUTING.md"] {
        let text = std::fs::read_to_string(format!("{root}{f}")).unwrap_or_default();
        for n in names {
            let hit = text.match_indices(n).any(|(i, _)| !text[i + n.len()..].starts_with(|c: char| c.is_alphanumeric()) && !text[..i].ends_with(|c: char| c.is_alphanumeric()));
            assert!(!hit, "{f} names {n}");
        }
    }
}

#[test]
fn every_material_has_a_schematic_microstructure() {
    use crate::micro::{draw, Kind};
    let d = db::builtin();
    let mut kinds = std::collections::HashSet::new();
    for m in &d.materials {
        let a = draw(m);
        assert!(a.svg.starts_with("<svg") && a.svg.ends_with("</svg>") && a.svg.contains("SCHEMATIC"), "{}", m.id);
        assert!(!a.svg.contains("NaN") && !a.svg.contains("inf"), "{}: a number off the picture", m.id);
        assert!(a.svg.len() < 250_000, "{}: {} bytes", m.id, a.svg.len());
        assert!(a.caption.starts_with("Schematic, not a micrograph:"), "{}", m.id);
        assert_eq!(a.svg, draw(m).svg, "{}: the same material always gives the same picture", m.id);
        kinds.insert(format!("{:?}", a.kind).split([' ', '{']).next().unwrap_or("").to_string());
    }
    assert!(kinds.len() >= 22, "{kinds:?}");
    // The classes get the structures they have.
    let k = |id: &str| crate::micro::recipe(d.materials.iter().find(|m| m.id == id).unwrap()).0;
    assert!(matches!(k("steel-1020"), Kind::Pearlite { .. }) && matches!(k("ci-grey"), Kind::Graphite { shape: 0 }) && matches!(k("ci-ductile"), Kind::Graphite { shape: 2 }));
    assert!(matches!(k("al-a356-t6"), Kind::Dendrites { .. }) && matches!(k("cfrp-ud"), Kind::Fibres { .. }) && matches!(k("pp"), Kind::Spherulites { .. }) && matches!(k("pc"), Kind::Amorphous { .. }));
    assert!(matches!(k("glass-soda"), Kind::Glass) && matches!(k("pe-foam"), Kind::Foam { open: false, .. }) && matches!(k("pine-l"), Kind::Wood { hard: false }));
}

#[test]
fn the_elements_hold_together() {
    use crate::elements::{check, composition, elements, find};
    let bad = check(&db::builtin().sources);
    assert!(bad.is_empty(), "{bad:#?}");
    let e = elements();
    assert_eq!((e[25].symbol.as_str(), e[25].structure.as_deref(), e[25].density), ("Fe", Some("BCC"), Some(7874.0)));
    assert!(e.iter().filter(|x| x.z >= 104).all(|x| x.density.is_none() && x.melt.is_none()), "superheavy elements are gaps, not zeros");
    assert_eq!((find("26").unwrap().symbol.as_str(), find("aluminum").unwrap().z, find("CU").unwrap().z, find("Tungsten").unwrap().symbol.as_str()), ("Fe", 13, 29, "W"));
    assert!(find("Xx").is_none());
    // Formulas: molar mass and weight fractions.
    let c = composition("Fe2O3", "formula").unwrap();
    assert!((c.mass - 159.69).abs() < 0.01 && (c.parts[0].wt - 69.94).abs() < 0.02, "{c:?}");
    assert!((composition("Al2(SO4)3", "formula").unwrap().mass - 342.13).abs() < 0.01);
    assert!((composition("CuSO4·5H2O", "formula").unwrap().mass - 249.69).abs() < 0.02);
    assert!((composition("Ti0.5Al0.5", "formula").unwrap().parts[1].at - 50.0).abs() < 1e-9);
    // Alloys: Fe–0.8 wt % C is about 3.6 at % C; the element without a number is the balance.
    let s = composition("Fe C 0.8", "wt").unwrap();
    assert!((s.parts[0].wt - 99.2).abs() < 1e-9 && (s.parts[1].at - 3.614).abs() < 0.005, "{s:?}");
    let b = composition("Ni 50 Ti 50", "at").unwrap();
    assert!((b.parts[0].wt - 55.07).abs() < 0.02, "{b:?}");
    for (t, by) in [("Fe2O3)", "formula"), ("(Fe", "formula"), ("Qq2", "formula"), ("", "formula"), ("Fe0", "formula"), ("Fe 60 C 30", "wt"), ("Fe C", "wt"), ("Fe 120", "wt"), ("Fe 50 Fe 50", "at"), ("Fe 80 Ni 30 Cr", "wt")] {
        assert!(composition(t, by).is_err(), "{t} ({by}) should be refused");
    }
}

#[test]
fn crystal_calculators_match_the_textbook() {
    use crate::crystal::*;
    let fe = of_element("Fe").unwrap();
    assert_eq!((fe.kind.as_str(), fe.system.as_str(), fe.atoms.len()), ("bcc", "cubic", 2));
    assert!((fe.cell.d([1, 1, 0]) - 2.0269).abs() < 1e-3 && (fe.density() - 7874.0).abs() < 15.0, "{}", fe.density());
    // Iron with Cu Kα: (110), (200), (211) at about 44.7°, 65.0°, 82.3°; no (100) or (111).
    let p = peaks(&fe, 1.5406, 90.0).unwrap();
    let at: Vec<_> = p.iter().map(|x| (x.hkl, (x.two_theta * 10.0).round() / 10.0, x.mult)).collect();
    assert_eq!(at, vec![([1, 1, 0], 44.7, 12), ([2, 0, 0], 65.0, 6), ([2, 1, 1], 82.3, 24)], "{at:?}");
    assert!((p[0].intensity - 100.0).abs() < 1e-9);
    // FCC: unmixed indices only; diamond also loses (200); hexagonal (0001) is missing, (0002) is there.
    let al = of_element("Al").unwrap();
    let hk: Vec<_> = peaks(&al, 1.5406, 85.0).unwrap().iter().map(|x| x.hkl).collect();
    assert_eq!(hk, vec![[1, 1, 1], [2, 0, 0], [2, 2, 0], [3, 1, 1], [2, 2, 2]]);
    assert!((peaks(&al, 1.5406, 40.0).unwrap()[0].two_theta - 38.47).abs() < 0.02);
    let si: Vec<_> = peaks(&of_element("Si").unwrap(), 1.5406, 80.0).unwrap().iter().map(|x| x.hkl).collect();
    assert_eq!(si, vec![[1, 1, 1], [2, 2, 0], [3, 1, 1], [4, 0, 0], [3, 3, 1]]);
    let mg = of_element("Mg").unwrap();
    assert_eq!(mg.system, "hexagonal");
    let mp = peaks(&mg, 1.5406, 40.0).unwrap();
    assert!(mp.iter().any(|x| x.hkil == Some([0, 0, 0, 2])) && !mp.iter().any(|x| x.hkl == [0, 0, 1]), "{mp:?}");
    assert!((mg.density() - 1738.0).abs() < 25.0, "{}", mg.density());
    // Rock salt: both species; angles; Miller–Bravais.
    let nacl = build("rocksalt", Cell::cubic(5.64), &["Na".into(), "Cl".into()]).unwrap();
    assert!((nacl.density() - 2165.0).abs() < 20.0 && nacl.atoms.len() == 8, "{}", nacl.density());
    let c = Cell::cubic(1.0);
    assert!((crate::crystal::answer(&Ask { kind: "cell".into(), cell: c, hkl: [1, 0, 0], hkl2: [1, 1, 1], ..Default::default() }).unwrap().angle_planes.unwrap() - 54.7356).abs() < 1e-3);
    let w = crate::crystal::answer(&Ask { element: Some("tungsten".into()), ..Default::default() }).unwrap();
    assert!(w.structure.kind == "bcc" && (w.density - 19250.0).abs() < 100.0 && w.bragg.is_some(), "{}", w.density);
    assert!(crate::crystal::answer(&Ask { element: Some("Og".into()), ..Default::default() }).is_err());
    assert_eq!((dir4([1, 0, 0]), dir4([0, 0, 1]), plane4([1, 0, 1])), ([2, -1, -1, 0], [0, 0, 0, 1], [1, 0, -1, 1]));
    // Stereographic projection down [001]: (001) at the centre, (100) on the circle, (111) at 0.732 / √2… from it.
    let s = stereo(&c, [0, 0, 1], 1).unwrap();
    let at = |h: [i32; 3]| s.iter().find(|p| p.hkl == h).map(|p| (p.x * p.x + p.y * p.y).sqrt());
    assert!(at([0, 0, 1]).unwrap() < 1e-9 && (at([1, 0, 0]).unwrap() - 1.0).abs() < 1e-9 && (at([1, 1, 1]).unwrap() - (54.7356f64.to_radians() / 2.0).tan()).abs() < 1e-6 && at([0, 0, -1]).is_none());
    // Refusals.
    assert!(build("fcc", Cell::cubic(-1.0), &["Cu".into()]).is_err() && build("rocksalt", Cell::cubic(5.0), &["Na".into()]).is_err() && build("nope", c, &["Fe".into()]).is_err());
    assert!(build("cell", Cell { a: 1.0, b: 1.0, c: 1.0, alpha: 10.0, beta: 10.0, gamma: 100.0 }, &["Fe".into()]).is_err());
    assert!(peaks(&fe, 0.0, 90.0).is_err() && stereo(&c, [0, 0, 0], 2).is_err());
    let tri = build("cell", Cell { a: 5.0, b: 6.0, c: 7.0, alpha: 80.0, beta: 95.0, gamma: 105.0 }, &["C".into()]).unwrap();
    assert_eq!(tri.system, "triclinic");
}

#[test]
fn tensile_results_match_a_known_curve() {
    use crate::tensile::*;
    // Hollomon steel: E = 200 GPa, σ = 800 εp^0.2 in true terms; necking from the Considère strain.
    let (e_mod, k, n) = (200_000.0f64, 800.0f64, 0.2f64);
    // Elastic up to the stress at 0.2 % plastic strain (a short plateau), Hollomon above it.
    let s0 = k * 0.002f64.powf(n);
    let tr = |e: f64| { let et = (1.0 + e).ln(); let (mut lo, mut hi) = (0.0, 3000.0); for _ in 0..80 { let s = (lo + hi) / 2.0; if s / e_mod + if s < s0 { 0.0 } else { (s / k).powf(1.0 / n) } > et { hi = s } else { lo = s } } lo };
    let eu = n.exp() - 1.0;
    let su = tr(eu) / (1.0 + eu);
    let mut data: Vec<[f64; 2]> = (0..=6000).map(|i| { let e = 0.30 * i as f64 / 6000.0; [e, if e <= eu { tr(e) / (1.0 + e) } else { su * (1.0 - 0.5 * ((e - eu) / (0.30 - eu)).powi(2)) }] }).collect();
    data.push([0.3001, 0.0]);
    let s = Specimen { name: "H".into(), x: "strain".into(), y: "MPa".into(), shape: "area".into(), area: 1.0, data: data.clone(), ..Default::default() };
    let r = analyse(&s, [0.1, 0.4]);
    let near = |v: Option<f64>, x: f64, tol: f64| v.is_some_and(|v| (v - x).abs() <= tol);
    assert!(near(r.e, 200.0, 0.5) && near(r.rm, su, 0.1) && near(r.n, 0.2, 0.005) && near(r.k, 800.0, 8.0), "{r:?}");
    assert!(near(r.rp02, 800.0 * 0.002f64.powf(0.2), 3.0) && near(r.ag, (eu - su / e_mod) * 100.0, 0.05) && near(r.a, (0.30 - 0.5 * su / e_mod) * 100.0, 0.3), "{:?} {:?} {:?}", r.rp02, r.ag, r.a);
    assert!(r.work.is_some_and(|w| w > 100.0 && w < 150.0) && r.toe.abs() < 1e-6 && r.notes.is_empty(), "{:?} {:?}", r.work, r.notes);
    // Slack at the start moves the curve, not the results; kN and extension convert.
    let shifted = Specimen { x: "ext".into(), y: "kN".into(), shape: "round".into(), d: 10.0, l0: 50.0, data: data.iter().map(|p| [(p[0] + 0.001) * 50.0, p[1] * std::f64::consts::PI * 25.0 / 1000.0]).collect(), ..s.clone() };
    let q = analyse(&shifted, [0.1, 0.4]);
    assert!(near(q.e, 200.0, 0.5) && near(Some(q.toe), 0.001, 1e-6) && near(q.ag, r.ag.unwrap(), 1e-6) && near(q.rp02, r.rp02.unwrap(), 1e-6) && q.notes.len() == 1, "{q:?}");
    // Statistics, gaps and refusals.
    let t = Tensile { specimens: vec![s.clone(), s.clone(), Specimen { on: false, ..shifted.clone() }, Specimen { area: 0.0, y: "N".into(), ..s.clone() }], ..Default::default() };
    let o = run(&t);
    let rm = o.stats.iter().find(|x| x.key == "rm").unwrap();
    assert!(rm.n == 2 && rm.sd == Some(0.0) && near(rm.mean, su, 0.1) && o.rows[3].rm.is_none() && !o.rows[3].notes.is_empty());
    assert_eq!(crate::tensile::table(&o).rows.len(), 4 + 4);
    assert!(Tensile { fit: [0.5, 0.2], ..Default::default() }.validate().is_err() && Tensile { specimens: vec![Specimen { l0: -1.0, ..Default::default() }], ..Default::default() }.validate().is_err());
    // The example is made-up data, named so, and works out.
    let ex = run(&example());
    assert!(ex.rows.len() == 3 && ex.rows.iter().all(|r| r.name.contains("made-up") && near(r.e, 205.0, 3.0) && r.n.is_some() && r.rp02.is_some()), "{:?}", ex.rows.iter().map(|r| (r.e, r.n, &r.notes)).collect::<Vec<_>>());
    // A machine export: header lines, headings, decimal commas, kN.
    let raw: Vec<Vec<String>> = [vec!["Test report"], vec!["Operator", "A"], vec![], vec!["Time (s)", "Extension (mm)", "Load (kN)"], vec!["0", "0,000", "0,01"], vec!["0,1", "0,010", "1,5"], vec!["0,2", "x", "2,9"]].iter().map(|r| r.iter().map(|c| c.to_string()).collect()).collect();
    let c = columns(&raw).unwrap();
    assert!(c.x == 1 && c.y == 2 && c.xk == "ext" && c.yk == "kN" && c.skipped == 4 && c.head[2] == "Load (kN)", "{c:?}");
    assert_eq!(pick(&c, c.x, c.y), vec![[0.0, 0.01], [0.01, 1.5]]);
    assert!(columns(&[vec!["a".to_string(), "b".to_string()]]).is_err());
}

#[test]
fn hardness_fatigue_and_creep_match_their_formulas() {
    use crate::lab::*;
    let near = |v: Option<f64>, x: f64, tol: f64| v.is_some_and(|v| (v - x).abs() <= tol);
    let h = hardness(&HardSet { name: "A".into(), scale: "HV".into(), values: vec![200.0, 210.0, 220.0] });
    assert!(near(h.mean, 210.0, 1e-9) && near(h.sd, 10.0, 1e-9) && near(h.cov, 4.7619, 1e-3) && near(h.tabor, 686.47, 0.01), "{h:?}");
    assert!(h.ci.is_some_and(|c| (c[0] - 185.157).abs() < 0.01 && (c[1] - 234.843).abs() < 0.01), "{h:?}");
    assert!(hardness(&HardSet { scale: "HRC".into(), values: vec![40.0], ..Default::default() }).tabor.is_none());
    assert!(hardness(&HardSet::default()).mean.is_none());
    // Basquin: σa = 1000 (2N)^−0.1 comes back; a run-out is not fitted.
    let pts: Vec<FatPoint> = [500.0, 450.0, 400.0, 350.0, 300.0].iter().map(|&s: &f64| FatPoint { s, n: 0.5 * (s / 1000.0).powf(-10.0), runout: false }).chain([FatPoint { s: 150.0, n: 1e7, runout: true }]).collect();
    let mut l = Lab { fatigue: pts, ..Default::default() };
    let f = fatigue(&l);
    assert!(near(f.sf, 1000.0, 1e-6) && near(f.b, -0.1, 1e-9) && near(f.r2, 1.0, 1e-9) && near(f.strength, 1000.0 * 2e7f64.powf(-0.1), 1e-6) && f.broken == 5 && !f.note.is_empty(), "{f:?}");
    // Larson–Miller: log σ = 6 − 0.18 P (C = 20) comes back, and C is found when asked.
    l.creep = [(300.0, 550.0), (220.0, 550.0), (160.0, 600.0), (120.0, 600.0), (80.0, 650.0), (55.0, 650.0)].iter().map(|&(s, t): &(f64, f64)| CreepPoint { s, t, h: 10f64.powf((s.log10() - 6.0) / -0.18 * 1000.0 / (t + 273.15) - 20.0) }).collect();
    let c = creep(&l);
    assert!(c.a.is_some_and(|a| (a[0] - 6.0).abs() < 1e-6 && (a[1] + 0.18).abs() < 1e-8) && near(c.stress, 10f64.powf(6.0 - 0.18 * 873.15 * 25.0 / 1000.0), 1e-6), "{c:?}");
    l.lmp_c = 0.0;
    let c = creep(&l);
    assert!(c.found && near(c.c, 20.0, 0.051), "{c:?}");
    // A material of your own from the example tests, with ranges and no zeros.
    let t = crate::tensile::run(&crate::tensile::example());
    let o = run(&example());
    assert!(o.fatigue.strength.is_some() && o.creep.stress.is_some() && o.hardness[0].mean.is_some(), "{:?} {:?}", o.fatigue, o.creep);
    let m = to_material(&t, &o, "My steel", "u-src");
    for k in ["E", "sy", "ts", "elong", "hv", "fat"] { assert!(matches!(m.props.get(k), Some(Val::Range(lo, hi, s)) if *lo > 0.0 && hi >= lo && s == "u-src"), "{k}: {:?}", m.props.get(k)); }
    assert!(Lab { lmp_c: 2.0, ..Default::default() }.validate().is_err() && Lab { fatigue: vec![FatPoint { s: -1.0, n: 10.0, runout: false }], ..Default::default() }.validate().is_err() && Lab { hardness: vec![HardSet { scale: "XX".into(), ..Default::default() }], ..Default::default() }.validate().is_err());
    assert!(example().validate().is_ok());
}

#[test]
fn image_analysis_measures_known_particles() {
    use crate::image::*;
    // Ten dark discs of radius 10 on a bright field, one cut by the edge.
    let (w, h) = (300usize, 200usize);
    let mut px = vec![200u8; w * h];
    let centres: Vec<(f64, f64)> = (0..10).map(|k| (30.0 + 55.0 * (k % 5) as f64, 50.0 + 90.0 * (k / 5) as f64)).collect();
    let all = centres.iter().copied().chain([(2.0, 100.0)]);
    for (cx, cy) in all { for y in 0..h { for x in 0..w { if (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2) <= 100.0 { px[y * w + x] = 50 } } } }
    let q = Ask { w, h, px: b64_encode(&px), scale: 0.5, ..Default::default() };
    let a = analyse(&q).unwrap();
    assert!(a.threshold >= 50 && a.threshold < 200 && a.stats.count == 10, "{} {}", a.threshold, a.stats.count);
    assert!(a.stats.mean.is_some_and(|d| (d - 10.0).abs() < 0.3) && a.stats.circ.is_some_and(|c| c > 0.85) && a.stats.aspect.is_some_and(|r| r < 1.1), "{:?}", a.stats);
    let on = px.iter().filter(|&&v| v == 50).count() as f64 / (w * h) as f64 * 100.0;
    assert!((a.fraction - on).abs() < 1e-9 && a.unit == "µm" && a.particles.len() == 11 && a.particles.iter().filter(|p| p.edge).count() == 1);
    assert!(a.stats.density.is_some_and(|n| (n - 10.0 / (300.0 * 200.0 * 0.25e-6)).abs() < 1e-6));
    let m = b64_decode(&a.mask).unwrap();
    assert!(m.len() == w * h && m.contains(&1) && m.contains(&2));
    // Bright phase, a region, smoothing and the edge rule change what they should.
    let inv: Vec<u8> = px.iter().map(|v| 255 - v).collect();
    let b = analyse(&Ask { px: b64_encode(&inv), dark: false, edge: false, smooth: 1, ..q.clone() }).unwrap();
    assert!(b.stats.count == 11, "{}", b.stats.count);
    let c = analyse(&Ask { roi: [0, 0, 150, 200], scale: 0.0, ..q.clone() }).unwrap();
    assert!(c.unit == "px" && c.stats.count < 10 && !c.notes.is_empty());
    assert_eq!(otsu(&{ let mut hh = vec![0u32; 256]; hh[40] = 100; hh[200] = 300; hh }), 40);
    assert!(analyse(&Ask { w: 10, h: 10, px: b64_encode(&[0; 50]), ..Default::default() }).is_err() && analyse(&Ask { roi: [5, 5, 6, 6], ..q.clone() }).is_err() && b64_decode("@@").is_err());
    assert_eq!(b64_decode(&b64_encode(b"Datum")).unwrap(), b"Datum");
    // The made-up example: particles are found, the data bar is left out by the region.
    let (ew, eh, ep) = example();
    let e = analyse(&Ask { w: ew, h: eh, px: b64_encode(&ep), roi: [0, 0, ew, 440], smooth: 1, scale: 0.5, ..Default::default() }).unwrap();
    assert!(e.stats.count >= 50 && e.stats.count <= 60 && e.stats.mean.is_some_and(|d| d > 6.0 && d < 14.0), "{:?}", e.stats);
}

#[test]
fn grain_size_nodularity_and_coating_thickness() {
    use crate::image::*;
    let near = |v: Option<f64>, x: f64, tol: f64| v.is_some_and(|v| (v - x).abs() <= tol);
    // A square grid of 20 µm grains with one-pixel dark boundaries: mean intercept 20 µm, G = 8.0.
    let (w, h) = (200usize, 200usize);
    let px: Vec<u8> = (0..w * h).map(|i| if (i % w) % 20 == 0 || (i / w) % 20 == 0 { 30 } else { 200 }).collect();
    let q = Ask { w, h, px: b64_encode(&px), scale: 1.0, mode: "grains".into(), ..Default::default() };
    let g = analyse(&q).unwrap().grains.unwrap();
    assert!(g.intercepts == 200 && near(g.mean_intercept, 20.0, 1e-9) && near(g.g_intercept, 8.0, 0.01) && g.segs.len() == 20, "{g:?}");
    assert!(g.inside == 81 && g.on_edge == 19 && near(g.mean_ecd, 2.0 * (361.0f64 / std::f64::consts::PI).sqrt(), 1e-6) && g.g_area.is_some(), "{g:?}");
    // Coating: a bright layer exactly 30 px thick.
    let px: Vec<u8> = (0..w * h).map(|i| if (50..80).contains(&(i / w)) { 220 } else { 40 }).collect();
    let c = analyse(&Ask { w, h, px: b64_encode(&px), scale: 2.0, dark: false, mode: "coating".into(), ..Default::default() }).unwrap().coating.unwrap();
    assert!(near(c.mean, 60.0, 1e-9) && near(c.sd, 0.0, 1e-9) && c.columns == 200, "{c:?}");
    // Nodularity on the made-up image: round nodules and flakes are told apart.
    let (ew, eh, ep) = example_of("nodular");
    let n = analyse(&Ask { w: ew, h: eh, px: b64_encode(&ep), roi: [0, 0, ew, 440], smooth: 1, scale: 0.5, mode: "nodularity".into(), ..Default::default() }).unwrap().nodularity.unwrap();
    assert!(n.nodules > 25 && n.others > 10 && n.by_count.is_some_and(|v| v > 50.0 && v < 80.0), "{n:?}");
    // The made-up grain image gives a grain count of the right order (90 seeds).
    let (gw, gh, gp) = example_of("grains");
    let e = analyse(&Ask { w: gw, h: gh, px: b64_encode(&gp), scale: 0.5, mode: "grains".into(), ..Default::default() }).unwrap().grains.unwrap();
    assert!(e.inside + e.on_edge > 70 && e.inside + e.on_edge < 110 && e.mean_intercept.is_some(), "{e:?}");
}

#[test]
fn synthetic_structures_have_what_was_asked() {
    use crate::synth::*;
    let a = generate(&Ask::default()).unwrap();
    assert!(a.mean_ecd.is_some_and(|d| d > 25.0 * 0.75 && d < 25.0 * 1.15) && a.mean_aspect.is_some_and(|r| r < 2.0), "{:?} {:?}", a.mean_ecd, a.mean_aspect);
    assert_eq!(a.labels, generate(&Ask::default()).unwrap().labels, "the same settings give the same map");
    assert_ne!(a.labels, generate(&Ask { seed: 2, ..Ask::default() }).unwrap().labels);
    let wide = generate(&Ask { spread: 0.0, ..Ask::default() }).unwrap();
    let mixed = generate(&Ask { spread: 0.8, ..Ask::default() }).unwrap();
    assert!(mixed.sd_ecd.unwrap() / mixed.mean_ecd.unwrap() > wide.sd_ecd.unwrap() / wide.mean_ecd.unwrap(), "a bigger spread gives more mixed sizes");
    let long = generate(&Ask { aspect: 3.0, ..Ask::default() }).unwrap();
    assert!(long.mean_aspect.is_some_and(|r| r > 3.0), "{:?}", long.mean_aspect);
    let two = generate(&Ask { phase2: 0.3, pores: 0.04, ..Ask::default() }).unwrap();
    assert!(two.phase2 > 25.0 && two.phase2 < 40.0 && two.pores > 3.0 && two.pores < 6.0, "{} {}", two.phase2, two.pores);
    assert!(csv(&Ask { w: 20, h: 16, size: 5.0, ..Ask::default() }).is_ok_and(|s| s.lines().count() == 1 + 320 && s.starts_with("x (µm)")));
    for bad in [Ask { w: 5, ..Ask::default() }, Ask { size: 1.0, ..Ask::default() }, Ask { aspect: 0.5, ..Ask::default() }, Ask { step: f64::NAN, ..Ask::default() }, Ask { w: 1200, h: 1200, size: 3.0, ..Ask::default() }] { assert!(generate(&bad).is_err()) }
    // The grey picture measures back through image analysis.
    let g = crate::image::analyse(&crate::image::Ask { w: a.w, h: a.h, px: a.grey.clone(), scale: a.step, mode: "grains".into(), ..Default::default() }).unwrap().grains.unwrap();
    assert!(g.mean_ecd.is_some_and(|d| d > 15.0 && d < 32.0), "{g:?}");
}

#[test]
fn orientations_match_the_textbook() {
    use crate::orient::*;
    for e in [[10.0, 20.0, 30.0], [200.0, 75.0, 310.0], [35.26, 45.0, 0.0]] { let b = to_euler(&from_euler(e)); assert!((0..3).all(|i| (b[i] - e[i]).abs() < 1e-6), "{e:?} → {b:?}") }
    let (cub, hex) = (symmetry("cubic"), symmetry("hex"));
    assert_eq!((cub.len(), hex.len()), (24, 12));
    for m in cub.iter().chain(&hex) { let p = mul(m, &tr(m)); assert!((0..3).all(|i| (0..3).all(|j| (p[i][j] - if i == j { 1.0 } else { 0.0 }).abs() < 1e-9))) }
    let e = |a| from_euler(a);
    let (cube, goss, rcube) = (e([0.0, 0.0, 0.0]), e([0.0, 45.0, 0.0]), e([45.0, 0.0, 0.0]));
    assert!((misorientation(&cube, &goss, &cub) - 45.0).abs() < 1e-6 && (misorientation(&cube, &rcube, &cub) - 45.0).abs() < 1e-6);
    let twin = mul(&axis_angle([1.0, 1.0, 1.0], 60.0), &goss);
    assert!((misorientation(&goss, &twin, &cub) - 60.0).abs() < 1e-6, "Σ3 twin is 60°");
    assert!(misorientation(&cube, &mul(&cub[5], &cube), &cub) < 1e-6, "a symmetry rotation is no misorientation");
    // Random pairs: no angle above 62.8° and a mean near 40.7° (the cubic random distribution).
    let mut s = crate::synth::Rng(77);
    let angs: Vec<f64> = (0..3000).map(|_| { let a = random([s.f(), s.f(), s.f()]); let b = random([s.f(), s.f(), s.f()]); misorientation(&a, &b, &cub) }).collect();
    let mean = angs.iter().sum::<f64>() / angs.len() as f64;
    assert!(angs.iter().all(|&a| a <= 62.81) && (mean - 40.7).abs() < 1.0, "{mean}");
    // IPF colours: Cube's normal is [001] red, Goss's is [011] green, a <111> fibre is blue.
    assert_eq!(ipf(&cube, [0.0, 0.0, 1.0], "cubic"), [255, 0, 0]);
    assert_eq!(ipf(&goss, [0.0, 0.0, 1.0], "cubic"), [0, 255, 0]);
    assert_eq!(ipf(&fibre([1.0, 1.0, 1.0], 33.0), [0.0, 0.0, 1.0], "cubic"), [0, 0, 255]);
    assert_eq!(ipf(&cube, [0.0, 0.0, 1.0], "hex"), [255, 0, 0]);
    assert!(ipf_key("cubic", 20).len() > 50 && ipf_key("hex", 20).len() > 50);
}

#[test]
fn synthetic_textures_come_back() {
    use crate::synth::*;
    let rnd = generate(&Ask::default()).unwrap();
    assert!(rnd.textured.is_none() && (rnd.mean_mis - 40.7).abs() < 3.0 && rnd.low_angle < 8.0, "{} {}", rnd.mean_mis, rnd.low_angle);
    let cube = generate(&Ask { texture: "cube".into(), share: 1.0, scatter: 5.0, ..Ask::default() }).unwrap();
    assert!(cube.textured.is_some_and(|t| t > 90.0) && cube.low_angle > 60.0 && cube.ipf.iter().filter(|c| c[0] > 200).count() * 10 > cube.grains * 8, "{:?} {}", cube.textured, cube.low_angle);
    let half = generate(&Ask { texture: "goss".into(), share: 0.5, scatter: 5.0, ..Ask::default() }).unwrap();
    assert!(half.textured.is_some_and(|t| t > 40.0 && t < 62.0), "{:?}", half.textured);
    let fib = generate(&Ask { texture: "fibre111".into(), scatter: 5.0, ..Ask::default() }).unwrap();
    assert!(fib.textured.is_some_and(|t| t > 90.0) && fib.ipf.iter().filter(|c| c[2] > 200).count() * 10 > fib.grains * 8);
    let mg = generate(&Ask { crystal: "hex".into(), texture: "basal".into(), scatter: 5.0, ..Ask::default() }).unwrap();
    assert!(mg.textured.is_some_and(|t| t > 90.0) && mg.ipf.iter().filter(|c| c[0] > 200).count() * 10 > mg.grains * 8);
    assert!(generate(&Ask { crystal: "hex".into(), texture: "cube".into(), ..Ask::default() }).is_err() && generate(&Ask { texture: "nope".into(), ..Ask::default() }).is_err());
    let t = ebsd_text(&Ask { w: 20, h: 16, size: 5.0, ..Ask::default() }).unwrap();
    assert!(t.starts_with("# Datum Atlas orientation map") && t.lines().filter(|l| !l.starts_with('#')).count() == 320);
}
