//! Processes (`data/processes.json`) and process selection: which processes can make your part from
//! your material, and why the others cannot, stage by stage like material selection; the cost of a
//! part against batch size; which processes suit the shortlisted materials; joining and finishing.
use crate::db::{Db, SEP};
use crate::eco::R;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// A process: what it suits and what it can make, and its indicative costs. Joining and finishing
/// records use `fam` with `permanent`/`watertight` or `purposes` only.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Process {
    pub id: String,
    pub name: String,
    /// Shaping, Machining, Additive, Joining or Finishing; `kind` is finer (Casting, Moulding…).
    pub group: String,
    pub kind: String,
    /// Family paths of the materials it suits ("Metals › Non-ferrous › Aluminium alloys").
    pub fam: Vec<String>,
    /// Part mass (kg), section thickness (mm), shapes, tolerance (mm), roughness Ra (µm), economic batch.
    pub mass: R,
    pub section: R,
    pub shapes: Vec<String>,
    pub tol: R,
    pub rough: R,
    pub batch: R,
    /// Tooling (USD), rate (parts an hour), overhead (USD an hour), material wasted (fraction).
    pub tooling: R,
    pub rate: R,
    pub overhead: R,
    pub scrap: R,
    /// Its energy entry in `data/eco.json`.
    pub eco: String,
    pub permanent: bool,
    pub watertight: bool,
    pub purposes: Vec<String>,
    pub note: String,
    pub src: String,
}

#[derive(Deserialize)]
struct File { processes: Vec<Process> }

pub fn processes() -> &'static [Process] {
    static P: OnceLock<Vec<Process>> = OnceLock::new();
    P.get_or_init(|| serde_json::from_str::<File>(include_str!("../../../data/processes.json")).unwrap_or_else(|e| panic!("data/processes.json: {e}")).processes)
}
pub const SELECTABLE: &[&str] = &["Shaping", "Machining", "Additive"];
pub const SHAPES: &[&str] = &["flat", "sheet", "prismatic", "3d-solid", "3d-hollow"];

/// Does the process work this material (its family path starts with one of the process's)?
pub fn suits(p: &Process, m: &Material) -> bool {
    let path = m.family.join(SEP);
    p.fam.iter().any(|f| path == *f || path.starts_with(&format!("{f}{SEP}")))
}

/// The part to make, saved with the project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProcSel {
    pub material: String,
    /// kg, mm; tolerance (mm) and roughness (µm) needed, 0 for any; number of parts.
    pub mass: f64,
    pub section: f64,
    /// One of SHAPES, or "" for any.
    pub shape: String,
    pub tol: f64,
    pub rough: f64,
    pub batch: f64,
    /// Material price (USD/kg) for the cost model; 0 uses the material's own.
    pub price: f64,
    /// A second material to join to the first, and what a finish is for ("" for any).
    pub join: String,
    pub purpose: String,
}
impl Default for ProcSel {
    fn default() -> Self { Self { material: String::new(), mass: 1.0, section: 5.0, shape: String::new(), tol: 0.0, rough: 0.0, batch: 1000.0, price: 0.0, join: String::new(), purpose: String::new() } }
}
impl ProcSel {
    pub fn normalize(&mut self) {
        let fin = |v: &mut f64, lo: f64, hi: f64, d: f64| *v = if v.is_finite() { v.clamp(lo, hi) } else { d };
        fin(&mut self.mass, 1e-6, 1e6, 1.0); fin(&mut self.section, 0.001, 1e4, 5.0); fin(&mut self.tol, 0.0, 100.0, 0.0);
        fin(&mut self.rough, 0.0, 1000.0, 0.0); fin(&mut self.batch, 1.0, 1e9, 1000.0); fin(&mut self.price, 0.0, 1e7, 0.0);
        if !self.shape.is_empty() && !SHAPES.contains(&self.shape.as_str()) { self.shape.clear(); }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub name: String,
    pub group: String,
    pub kind: String,
    pub pass: bool,
    pub reasons: Vec<String>,
    /// Cost per part at your batch size (USD), for processes that pass all but the batch stage.
    pub cost: Option<R>,
    /// (batch size, low, high) cost per part from 1 to 10⁷ parts.
    pub curve: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Matrix { pub mats: Vec<(String, String)>, pub procs: Vec<(String, String)>, pub ok: Vec<Vec<bool>> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Out {
    pub rows: Vec<Row>,
    /// Each stage and how many processes pass it.
    pub stages: Vec<(String, usize)>,
    pub pass: usize,
    /// The cheapest process at each batch size on the curves.
    pub cheapest: Vec<(f64, String)>,
    /// The material price used (USD/kg), when known.
    pub price: Option<R>,
    pub matrix: Matrix,
    pub joins: Vec<String>,
    pub finishes: Vec<String>,
    pub notes: Vec<String>,
}

const N_PTS: usize = 29;
/// The batch sizes of the cost curves: 1 to 10⁷ in quarter decades.
pub fn batches() -> Vec<f64> { (0..N_PTS).map(|k| 10f64.powf(k as f64 * 0.25)).collect() }

/// Cost per part (USD) of `kg` of material at `cm` USD/kg, made `n` at a time: material (with what is
/// wasted) + tooling shared over the batch + overhead per part.
pub fn cost(p: &Process, kg: f64, cm: R, n: f64) -> R {
    let n = n.max(1.0);
    let part = |cm: f64, f: f64, t: f64, oh: f64, rate: f64| kg * cm / (1.0 - f.min(0.95)) + t / n + oh / rate.max(1e-6);
    [part(cm[0], p.scrap[0], p.tooling[0], p.overhead[0], p.rate[1]), part(cm[1], p.scrap[1], p.tooling[1], p.overhead[1], p.rate[0])]
}

fn shape_name(s: &str) -> &str {
    match s { "flat" => "a flat part", "sheet" => "a sheet part", "prismatic" => "a prismatic (constant section) part", "3d-solid" => "a 3-D solid", "3d-hollow" => "a 3-D hollow part", _ => s }
}

/// Process selection for the project's part.
pub fn select(db: &Db, pr: &Project) -> Out {
    let s = &pr.process;
    let mut notes = vec![];
    let m = db.get(&s.material);
    if m.is_none() { notes.push("Choose the material to keep only the processes that work it.".into()); }
    let price = if s.price > 0.0 { Some([s.price, s.price]) } else { m.and_then(|m| m.props.get("price")).and_then(Val::range).map(|(a, b)| [a, b]) };
    if m.is_some() && price.is_none() { notes.push("The material has no price: type one to see the costs.".into()); }
    let f = crate::units::fmt;
    let names = ["Material", "Part mass", "Section thickness", "Shape", "Tolerance", "Roughness", "Batch size"];
    let mut counts = [0usize; 7];
    let xs = batches();
    let mut rows: Vec<Row> = processes().iter().filter(|p| SELECTABLE.contains(&p.group.as_str())).map(|p| {
        let mut why: [Option<String>; 7] = Default::default();
        if let Some(m) = m { if !suits(p, m) { why[0] = Some(format!("does not work {}", m.family.last().map_or("this material", |x| x.as_str()).to_lowercase())); } }
        if s.mass < p.mass[0] || s.mass > p.mass[1] { why[1] = Some(format!("part mass {} kg is outside {}–{} kg", f(s.mass), f(p.mass[0]), f(p.mass[1]))); }
        if s.section < p.section[0] { why[2] = Some(format!("{} mm sections are thinner than its {} mm", f(s.section), f(p.section[0]))); }
        else if s.section > p.section[1] { why[2] = Some(format!("{} mm sections are thicker than its {} mm", f(s.section), f(p.section[1]))); }
        if !s.shape.is_empty() && !p.shapes.contains(&s.shape) { why[3] = Some(format!("cannot make {}", shape_name(&s.shape))); }
        if s.tol > 0.0 && p.tol[0] > s.tol { why[4] = Some(format!("its best tolerance is ±{} mm; you need ±{} mm", f(p.tol[0]), f(s.tol))); }
        if s.rough > 0.0 && p.rough[0] > s.rough { why[5] = Some(format!("its smoothest surface is Ra {} µm; you need {} µm", f(p.rough[0]), f(s.rough))); }
        if s.batch < p.batch[0] || s.batch > p.batch[1] { why[6] = Some(format!("{} parts is outside its economic batch of {}–{}", f(s.batch), f(p.batch[0]), f(p.batch[1]))); }
        for (k, w) in why.iter().enumerate() { if w.is_none() { counts[k] += 1; } }
        let fit = why[..6].iter().all(Option::is_none);
        let (cost_now, curve) = match (fit, price) {
            (true, Some(cm)) => (Some(cost(p, s.mass, cm, s.batch)), xs.iter().map(|&n| { let c = cost(p, s.mass, cm, n); [n, c[0], c[1]] }).collect()),
            _ => (None, vec![]),
        };
        let reasons: Vec<String> = why.into_iter().flatten().collect();
        Row { id: p.id.clone(), name: p.name.clone(), group: p.group.clone(), kind: p.kind.clone(), pass: reasons.is_empty(), reasons, cost: cost_now, curve }
    }).collect();
    // Passing first, cheapest first; then the others.
    rows.sort_by(|a, b| b.pass.cmp(&a.pass).then(a.cost.map_or(f64::MAX, |c| c[0] + c[1]).total_cmp(&b.cost.map_or(f64::MAX, |c| c[0] + c[1]))));
    let cheapest = xs.iter().enumerate().filter_map(|(k, &n)| rows.iter().filter(|r| !r.curve.is_empty()).min_by(|a, b| (a.curve[k][1] + a.curve[k][2]).total_cmp(&(b.curve[k][1] + b.curve[k][2]))).map(|r| (n, r.id.clone()))).collect();
    // Which processes work the shortlisted materials: those compared, else the best of the results.
    let ids: Vec<String> = if !pr.compare.is_empty() { pr.compare.clone() } else { crate::select::run(db, pr).rows.into_iter().filter(|r| r.pass).take(6).map(|r| r.id).collect() };
    let mats: Vec<&Material> = ids.iter().filter_map(|id| db.get(id)).collect();
    let procs: Vec<&Process> = processes().iter().filter(|p| SELECTABLE.contains(&p.group.as_str())).collect();
    let matrix = Matrix {
        mats: mats.iter().map(|m| (m.id.clone(), m.name.clone())).collect(),
        procs: procs.iter().map(|p| (p.id.clone(), p.name.clone())).collect(),
        ok: procs.iter().map(|p| mats.iter().map(|m| suits(p, m)).collect()).collect(),
    };
    let other = db.get(&s.join);
    let joins = match m {
        Some(m) => processes().iter().filter(|p| p.group == "Joining" && suits(p, m) && other.is_none_or(|o| suits(p, o))).map(|p| p.id.clone()).collect(),
        None => vec![],
    };
    let finishes = match m {
        Some(m) => processes().iter().filter(|p| p.group == "Finishing" && suits(p, m) && (s.purpose.is_empty() || p.purposes.contains(&s.purpose))).map(|p| p.id.clone()).collect(),
        None => vec![],
    };
    let pass = rows.iter().filter(|r| r.pass).count();
    Out { rows, stages: names.iter().zip(counts).map(|(n, c)| (n.to_string(), c)).collect(), pass, cheapest, price, matrix, joins, finishes, notes }
}

/// Everything wrong with the process data: unknown sources, families or eco entries, bad ranges.
pub fn check(sources: &[Source]) -> Vec<String> {
    let mut errs = vec![];
    let mut ids = std::collections::HashSet::new();
    let eco = crate::eco::data();
    let fams: Vec<String> = crate::db::builtin().materials.iter().flat_map(|m| (1..=m.family.len()).map(|k| m.family[..k].join(SEP)).collect::<Vec<_>>()).collect();
    for p in processes() {
        let at = |e: &str| format!("{} ({}): {e}", p.name, p.id);
        if !ids.insert(p.id.as_str()) { errs.push(at("duplicate id")); }
        if !sources.iter().any(|s| s.id == p.src) { errs.push(at("source not found")); }
        if p.fam.is_empty() { errs.push(at("suits no material")); }
        for f in &p.fam { if !fams.contains(f) { errs.push(at(&format!("unknown family {f:?}"))); } }
        if SELECTABLE.contains(&p.group.as_str()) {
            let ok = |r: R| r[0].is_finite() && r[1].is_finite() && 0.0 <= r[0] && r[0] <= r[1];
            if ![p.mass, p.section, p.tol, p.rough, p.batch, p.tooling, p.rate, p.overhead, p.scrap].iter().all(|r| ok(*r)) || p.rate[0] <= 0.0 || p.scrap[1] >= 1.0 { errs.push(at("a range is negative, low > high, a zero rate or all scrap")); }
            if p.shapes.is_empty() || p.shapes.iter().any(|s| !SHAPES.contains(&s.as_str())) { errs.push(at("unknown or no shapes")); }
            if !eco.processes.iter().any(|e| e.id == p.eco) { errs.push(at("unknown eco process")); }
        } else if !["Joining", "Finishing"].contains(&p.group.as_str()) { errs.push(at("unknown group")); }
    }
    errs
}
