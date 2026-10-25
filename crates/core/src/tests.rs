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
    d.materials.push(d.materials[1].clone());
    let errs = db::check(&d, &[]).join("\n");
    for want in ["low 300 > high 200", "source \"nowhere\" not found", "must be above zero", "unknown property bogus", "duplicate id", "duplicate designation"] {
        assert!(errs.contains(want), "missing {want:?} in\n{errs}");
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
    p.add_chart(Chart { select: sel, ..Default::default() });
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
    doc.apply(Op::Select { chart: id, select: Some(ChartSel::Line { slope: 2.0, x: 1000.0, y: 20.0, above: true }) }).unwrap();
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
    assert_eq!(crate::goals::GOALS.len(), 22);
    let mut p = crate::templates::build(&d, Some("blank")).unwrap();
    p.stages.limits.push(Limit { prop: "price".into(), max: Some(50.0), ..Default::default() });
    let cid = p.charts[0].id;
    for g in crate::goals::GOALS {
        let mut doc = Doc::new(p.clone());
        doc.apply(crate::goals::apply(&d, &doc.p, cid, g.id).unwrap()).unwrap();
        let c = &doc.p.charts[0];
        assert_eq!((c.x.as_str(), c.y.as_str(), c.goal.as_str()), (g.x, g.y, g.id));
        let r = run(&d, &doc.p);
        assert!(r.pass > 0, "{}: nothing passes", g.id);
        // Your own limit stays; the goal's are added and marked.
        assert!(doc.p.stages.limits.iter().any(|l| l.prop == "price" && !l.goal));
        assert_eq!(doc.p.stages.limits.iter().filter(|l| l.goal).count(), g.limits.len(), "{}", g.id);
        // The ranking follows the index computed by hand from the geometric means.
        let gm = |m: &Material, k: &str| { let (a, b) = m.props[k].range().unwrap(); (a * b).sqrt() };
        let ranked: Vec<_> = r.rows.iter().filter(|x| x.pass).take(3).collect();
        let idx = |id: &str| { let m = d.get(id).unwrap(); if g.slope == 0.0 { gm(m, g.y) } else { gm(m, g.y).powf(1.0 / g.slope) / gm(m, g.x) } };
        for w in ranked.windows(2) {
            let (a, b) = (idx(&w[0].id), idx(&w[1].id));
            assert!(if g.above { a >= b * (1.0 - 1e-9) } else { a <= b * (1.0 + 1e-9) }, "{}: {} then {}", g.id, w[0].name, w[1].name);
        }
    }
    // Explore removes the line and the goal's limits.
    let mut doc = Doc::new(p.clone());
    doc.apply(crate::goals::apply(&d, &doc.p, cid, "spring").unwrap()).unwrap();
    doc.apply(crate::goals::apply(&d, &doc.p, cid, "").unwrap()).unwrap();
    assert!(doc.p.charts[0].select.is_none() && doc.p.charts[0].goal.is_empty() && doc.p.stages.limits.iter().all(|l| !l.goal));
    let r = run(&d, &{ let mut q = p.clone(); q.charts[0].x = "cv".into(); q.charts[0].select = Some(ChartSel::Line { slope: 2.0, x: 1e4, y: 10.0, above: true }); q });
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
