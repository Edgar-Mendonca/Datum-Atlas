//! Compare: two to six materials side by side. For every property: each material's typical value,
//! which is highest and lowest, and how far apart they are; and for the radar chart, each value
//! placed between the lowest and highest of the whole database (on a log scale for log properties).
use crate::db::Db;
use crate::geometry::mean;
use crate::model::*;
use serde::Serialize;

/// The properties on a new radar chart.
pub const RADAR: &[&str] = &["rho", "E", "sy", "kic", "tmax", "lam", "price", "co2"];
pub const MAX: usize = 6;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub key: String,
    /// Typical values (stored units), None where a material has no value.
    pub vals: Vec<Option<f64>>,
    /// Which material has the highest and the lowest (when at least two have a value and they differ).
    pub hi: Option<usize>,
    pub lo: Option<usize>,
    /// Highest ÷ lowest, for properties above zero.
    pub spread: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Radar {
    pub keys: Vec<String>,
    /// For each material, each property between 0 (lowest in the database) and 1 (highest).
    pub vals: Vec<Vec<Option<f64>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compare {
    pub ids: Vec<String>,
    pub rows: Vec<Row>,
    pub radar: Radar,
}

fn typical(db: &Db, m: &Material, k: &str) -> Option<f64> {
    let log = db.info(k).is_some_and(|i| i.log);
    match m.props.get(k)? { Val::RangeT(_, _, _, t) => Some(*t), Val::Range(lo, hi, _) => Some(mean(*lo, *hi, log && *lo > 0.0)), _ => None }.filter(|v| v.is_finite())
}

pub fn build(db: &Db, p: &Project) -> Compare {
    let mats: Vec<&Material> = p.compare.iter().filter_map(|id| db.get(id)).take(MAX).collect();
    let mut keys: Vec<String> = PROPS.iter().filter(|q| q.kind == "range").map(|q| q.key.to_string()).collect();
    keys.extend(db.formulas.iter().map(Formula::key));
    let rows = keys.iter().filter(|k| mats.iter().any(|m| m.props.contains_key(*k))).map(|k| {
        let vals: Vec<Option<f64>> = mats.iter().map(|m| typical(db, m, k)).collect();
        let have: Vec<(usize, f64)> = vals.iter().enumerate().filter_map(|(i, v)| v.map(|v| (i, v))).collect();
        let hi = have.iter().max_by(|a, b| a.1.total_cmp(&b.1)).copied();
        let lo = have.iter().min_by(|a, b| a.1.total_cmp(&b.1)).copied();
        let differ = have.len() >= 2 && hi.zip(lo).is_some_and(|(h, l)| h.1 != l.1);
        Row { key: k.clone(), spread: hi.zip(lo).filter(|(h, l)| differ && l.1 > 0.0 && h.1 > 0.0).map(|(h, l)| h.1 / l.1), hi: hi.filter(|_| differ).map(|h| h.0), lo: lo.filter(|_| differ).map(|l| l.0), vals }
    }).collect();
    let rkeys: Vec<String> = if p.radar.is_empty() { RADAR.iter().map(|s| s.to_string()).collect() } else { p.radar.clone() };
    let rkeys: Vec<String> = rkeys.into_iter().filter(|k| db.info(k).is_some()).collect();
    // The scale of each radar axis: the lowest and highest typical value in the whole database.
    let scale: Vec<Option<(f64, f64, bool)>> = rkeys.iter().map(|k| {
        let log = db.info(k).is_some_and(|i| i.log);
        let all: Vec<f64> = db.materials.iter().filter_map(|m| typical(db, m, k)).filter(|v| !log || *v > 0.0).map(|v| if log { v.log10() } else { v }).collect();
        let (lo, hi) = all.iter().fold((f64::MAX, f64::MIN), |a, &v| (a.0.min(v), a.1.max(v)));
        (hi > lo).then_some((lo, hi, log))
    }).collect();
    let vals = mats.iter().map(|m| rkeys.iter().zip(&scale).map(|(k, s)| {
        let (lo, hi, log) = (*s)?;
        let v = typical(db, m, k)?;
        if log && v <= 0.0 { return None; }
        Some((((if log { v.log10() } else { v }) - lo) / (hi - lo)).clamp(0.0, 1.0))
    }).collect()).collect();
    Compare { ids: mats.iter().map(|m| m.id.clone()).collect(), rows, radar: Radar { keys: rkeys, vals } }
}
