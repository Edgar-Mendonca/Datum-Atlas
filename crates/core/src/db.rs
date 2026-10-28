//! The materials database: the built-in data (embedded from `data/*.json`) merged with the user's
//! own `.datdb`, the family tree, search, and the data checks every value must pass.
use crate::model::*;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

const BUILTIN: &[&str] = &[
    include_str!("../../../data/sources.json"),
    include_str!("../../../data/metals.json"),
    include_str!("../../../data/polymers.json"),
    include_str!("../../../data/elastomers.json"),
    include_str!("../../../data/ceramics.json"),
    include_str!("../../../data/composites.json"),
    include_str!("../../../data/foams.json"),
    include_str!("../../../data/natural.json"),
];

/// The built-in database (the files are checked by the tests, so this cannot fail in a release).
pub fn builtin() -> Database {
    let mut db = Database { format: FORMAT, name: "Datum Atlas generic materials".into(), ..Default::default() };
    for text in BUILTIN {
        let part: Database = serde_json::from_str(text).unwrap_or_else(|e| panic!("data file: {e}"));
        db.sources.extend(part.sources);
        db.materials.extend(part.materials);
    }
    db
}

/// The materials the app works with: built-in first, then the user's (marked `user`).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Db {
    pub materials: Vec<Material>,
    pub sources: Vec<Source>,
    pub user_count: usize,
}

impl Db {
    pub fn new(builtin: Database, user: Option<&Database>) -> Self {
        let mut d = Db { materials: builtin.materials, sources: builtin.sources, user_count: 0 };
        if let Some(u) = user {
            d.sources.extend(u.sources.iter().cloned());
            d.materials.extend(u.materials.iter().cloned().map(|mut m| { m.user = true; m }));
            d.user_count = u.materials.len();
        }
        d.materials.iter_mut().for_each(derive);
        d.sources.push(Source { id: DERIVED.into(), title: "Calculated by Datum Atlas from the material's other values".into(), publisher: "Datum Atlas".into(), kind: "derived".into(), licence: "MIT".into(), note: "Products use low × low and high × high; quotients use low ÷ high and high ÷ low, so the range holds every combination.".into(), ..Default::default() });
        d
    }
    pub fn get(&self, id: &str) -> Option<&Material> {
        self.materials.iter().find(|m| m.id == id)
    }
}

pub const DERIVED: &str = "derived";

/// Adds the derived properties (price, CO₂ and energy per volume, specific stiffness and
/// strength, thermal diffusivity) from the material's own ranges.
pub fn derive(m: &mut Material) {
    let r = |k: &str| m.props.get(k).and_then(Val::range);
    let mul = |a: Option<(f64, f64)>, b: Option<(f64, f64)>, k: f64| Some((a?.0 * b?.0 * k, a?.1 * b?.1 * k));
    let div = |a: Option<(f64, f64)>, b: Option<(f64, f64)>, k: f64| Some((a?.0 / b?.1 * k, a?.1 / b?.0 * k));
    let (rho, e, sy, lam, cp) = (r("rho"), r("E"), r("sy"), r("lam"), r("cp"));
    let new = [
        ("cv", mul(r("price"), rho, 1.0)),
        ("co2v", mul(r("co2"), rho, 1.0)),
        ("eev", mul(r("ee"), rho, 1.0)),
        ("es", div(e, rho, 1000.0)),
        ("ss", div(sy, rho, 1000.0)),
        ("diff", div(lam, mul(rho, cp, 1.0), 1e6)),
        ("shock", div(r("ts"), mul(e, r("alpha"), 1.0), 1000.0)),
    ];
    for (k, v) in new {
        if let Some((a, b)) = v.filter(|(a, b)| a.is_finite() && b.is_finite() && *a > 0.0) {
            m.props.insert(k.into(), Val::Range(a, b, DERIVED.into()));
        }
    }
}

/// A node of the family tree with the number of materials under it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub count: usize,
}

pub const SEP: &str = " › ";

/// The family tree, depth first, families in their usual order (FAMILIES), classes as found.
pub fn tree(mats: &[Material]) -> Vec<Node> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut order: Vec<String> = vec![];
    for m in mats {
        for d in 1..m.family.len().min(3) + 1 {
            let path = m.family[..d].join(SEP);
            if !counts.contains_key(&path) { order.push(path.clone()); }
            *counts.entry(path).or_default() += 1;
        }
    }
    let rank = |p: &str| { let top = p.split(SEP).next().unwrap_or(""); FAMILIES.iter().position(|f| *f == top).unwrap_or(FAMILIES.len()) };
    let mut out: Vec<Node> = vec![];
    let mut tops: Vec<&String> = order.iter().filter(|p| !p.contains(SEP)).collect();
    tops.sort_by_key(|p| rank(p));
    fn walk(path: &str, order: &[String], counts: &BTreeMap<String, usize>, out: &mut Vec<Node>) {
        let depth = path.matches(SEP).count();
        out.push(Node { path: path.into(), name: path.rsplit(SEP).next().unwrap_or(path).into(), depth, count: counts[path] });
        for k in order.iter().filter(|k| k.strip_prefix(path).and_then(|r| r.strip_prefix(SEP)).is_some_and(|r| !r.contains(SEP))) {
            walk(k, order, counts, out);
        }
    }
    for t in tops { walk(t, &order, &counts, &mut out); }
    out
}

/// Materials whose name, synonyms, designations, family, notes or uses contain every word of `q`.
pub fn search<'a>(mats: &'a [Material], q: &str) -> Vec<&'a Material> {
    let words: Vec<String> = q.to_lowercase().split_whitespace().map(Into::into).collect();
    mats.iter().filter(|m| {
        let hay = format!("{} {} {} {} {} {}", m.name, m.syn.join(" "), m.des.values().cloned().collect::<Vec<_>>().join(" "), m.family.join(" "), m.notes, m.uses).to_lowercase();
        words.iter().all(|w| hay.contains(w))
    }).collect()
}

/// Kinds of source the built-in data may cite: open, public or free only (the user's rule).
pub const OPEN_KINDS: &[&str] = &["handbook", "government", "open-dataset", "datasheet", "university", "textbook-typical", "estimate"];
/// Proprietary or licensed materials databases and aggregator sites: never cited, never used to check.
const BARRED: &[&str] = &["matweb", "makeitfrom", "totalmateria", "total materia", "granta", "edupack", "matmatch", "materialhub", "suppliersonline", "azom", "wikipedia", "knovel", "springermaterials", "matdat"];

/// Everything wrong with a database: unknown properties, values without a known source, low > high,
/// non-positive values on log properties, ratings not in the scale, duplicate ids or designations,
/// and values marked checked that have no value or cite only our own compilation or an estimate.
pub fn check(db: &Database, extra_sources: &[Source]) -> Vec<String> {
    let mut errs = vec![];
    let srcs: HashSet<&str> = db.sources.iter().chain(extra_sources).map(|s| s.id.as_str()).collect();
    let general: HashSet<&str> = db.sources.iter().filter(|s| matches!(s.kind.as_str(), "textbook-typical" | "estimate")).map(|s| s.id.as_str()).collect();
    let (mut ids, mut des) = (HashSet::new(), HashSet::new());
    for s in &db.sources {
        if s.title.trim().is_empty() { errs.push(format!("source {}: no title", s.id)); }
        if !OPEN_KINDS.contains(&s.kind.as_str()) { errs.push(format!("source {}: kind {:?} is not an open, public or free kind", s.id, s.kind)); }
        let text = format!("{} {} {} {}", s.title, s.author, s.publisher, s.url).to_lowercase();
        if let Some(b) = BARRED.iter().find(|b| text.contains(*b)) { errs.push(format!("source {}: {b:?} is a proprietary database or an aggregator, never a source", s.id)); }
    }
    for m in &db.materials {
        let at = |e: String| format!("{} ({}): {e}", m.name, m.id);
        if m.id.trim().is_empty() || m.name.trim().is_empty() { errs.push(at("no id or name".into())); }
        if !ids.insert(m.id.as_str()) { errs.push(at("duplicate id".into())); }
        if !FAMILIES.contains(&m.family.first().map(String::as_str).unwrap_or("")) { errs.push(at(format!("unknown family {:?}", m.family))); }
        for (sys, d) in &m.des {
            if !des.insert(format!("{sys} {d}")) { errs.push(at(format!("duplicate designation {sys} {d}"))); }
        }
        for k in &m.checked {
            match m.props.get(k) {
                None => errs.push(at(format!("{k}: marked checked but has no value"))),
                Some(v) if general.contains(v.src()) => errs.push(at(format!("{k}: marked checked but cites {:?}, not a specific source", v.src()))),
                _ => {}
            }
        }
        for (k, v) in &m.props {
            let Some(p) = prop(k) else { errs.push(at(format!("unknown property {k}"))); continue };
            if !srcs.contains(v.src()) { errs.push(at(format!("{k}: source {:?} not found", v.src()))); }
            match (p.kind, v) {
                ("range", Val::Range(lo, hi, _) | Val::RangeT(lo, hi, _, _)) => {
                    if !(lo.is_finite() && hi.is_finite()) || lo > hi { errs.push(at(format!("{k}: low {lo} > high {hi}"))); }
                    if p.log && *lo <= 0.0 { errs.push(at(format!("{k}: {lo} must be above zero"))); }
                    if let Val::RangeT(_, _, _, t) = v { if t < lo || t > hi { errs.push(at(format!("{k}: typical {t} outside the range"))); } }
                }
                ("flag", Val::Flag(..)) => {}
                ("rating", Val::Rating(r, _)) if RATINGS.contains(&r.as_str()) => {}
                _ => errs.push(at(format!("{k}: a {} value was expected", p.kind))),
            }
        }
    }
    errs
}

/// Checks a material the user is adding (Add Material): what must be there before it is saved.
pub fn check_user(m: &Material, sources: &[Source]) -> Result<(), String> {
    if m.name.trim().is_empty() { return Err("Give the material a name.".into()); }
    if !FAMILIES.contains(&m.family.first().map(String::as_str).unwrap_or("")) { return Err("Choose the family it belongs to.".into()); }
    if !m.props.contains_key("rho") { return Err("Density is needed: every chart and many indices use it.".into()); }
    if m.props.len() < 2 { return Err("Enter at least one property besides density.".into()); }
    let db = Database { format: FORMAT, name: String::new(), sources: vec![], materials: vec![m.clone()] };
    match check(&db, sources).into_iter().next() {
        Some(e) => Err(e.split_once(": ").map(|(_, e)| e.to_string()).unwrap_or(e).replace("source \"\" not found", "give a source (a book, datasheet, test report or your own measurement)")),
        None => Ok(()),
    }
}

/// Reads a user database (`.datdb`). A file that cannot be read as one is an error, never an empty
/// database (so it is never written over). Values the engine cannot use (unknown properties, low >
/// high, not a number, zero or below on a log property, a rating not on the scale) are dropped and
/// listed, so one bad value never loses a material.
pub fn read_user(text: &str) -> Result<(Database, Vec<String>), String> {
    let mut d: Database = serde_json::from_str(text.trim_start_matches('\u{FEFF}')).map_err(|e| format!("This is not a Datum Atlas database ({e})."))?;
    if d.format > FORMAT { return Err("It was saved by a newer Datum Atlas; update the app to open it.".into()); }
    d.format = FORMAT;
    let mut notes = vec![];
    d.materials.retain(|m| { let ok = !m.id.trim().is_empty() && !m.name.trim().is_empty(); if !ok { notes.push(format!("A material without an id or name ({:?}) was left out.", m.name)) } ok });
    for m in &mut d.materials {
        if !FAMILIES.contains(&m.family.first().map(String::as_str).unwrap_or("")) { notes.push(format!("{}: unknown family {:?}, kept under Metals", m.name, m.family)); m.family = vec!["Metals".into()]; }
        let name = m.name.clone();
        m.props.retain(|k, v| {
            let ok = prop(k).is_some_and(|p| match (p.kind, &*v) {
                ("range", Val::Range(lo, hi, _) | Val::RangeT(lo, hi, _, _)) => lo.is_finite() && hi.is_finite() && lo <= hi && (!p.log || *lo > 0.0),
                ("flag", Val::Flag(..)) => true,
                ("rating", Val::Rating(r, _)) => RATINGS.contains(&r.as_str()),
                _ => false,
            }) && k.as_str() != "user";
            if !ok { notes.push(format!("{name}: the value of {k} could not be used and was left out")) }
            ok
        });
        for v in m.props.values_mut() {
            if let Val::RangeT(lo, hi, s, t) = v { if !(*lo <= *t && *t <= *hi) { *v = Val::Range(*lo, *hi, std::mem::take(s)); } }
        }
        m.checked.retain(|k| m.props.contains_key(k));
    }
    Ok((d, notes))
}

/// The property list for the window, each with its imperial unit: `imperial: [unit, factor, offset]`.
pub fn props_out() -> serde_json::Value {
    serde_json::json!(PROPS.iter().map(|p| { let mut v = serde_json::json!(p); v["imperial"] = serde_json::json!(crate::units::imperial(p.unit)); v }).collect::<Vec<_>>())
}
