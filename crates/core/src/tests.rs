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
    assert_eq!(crate::goals::GOALS.len(), 47, "25 goals and 22 more indices in the library");
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
