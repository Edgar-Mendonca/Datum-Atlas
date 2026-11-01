//! The eco audit: energy and CO₂ of a product through its life (material, manufacture, transport,
//! use, end of life) for up to four design options, from the materials' own values and the public
//! factors in `data/eco.json`. Every figure is a range (low, high); a value that is missing is
//! reported in the notes and left out, never counted as zero. Also greener substitutes: materials
//! that do the same job with less CO₂.
use crate::db::Db;
use crate::geometry::mean;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// A range: low, high.
pub type R = [f64; 2];
const Z: R = [0.0, 0.0];
fn add(a: R, b: R) -> R { [a[0] + b[0], a[1] + b[1]] }
fn mul(a: R, b: R) -> R { [a[0] * b[0], a[1] * b[1]] }
fn sc(a: R, k: f64) -> R { [a[0] * k, a[1] * k] }
fn neg(a: R) -> R { [-a[1], -a[0]] }
pub fn mid(a: R) -> f64 { (a[0] + a[1]) / 2.0 }

/// A factor of `data/eco.json`: energy (MJ) and CO₂ (kg) per unit (kg, tonne·km…), or electricity (kWh).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Factor { pub id: String, pub name: String, pub mj: R, pub co2: R, pub kwh: Option<R>, pub src: String, pub note: String }

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EolData { pub collect: R, pub landfill_mj: R, pub landfill_co2: R, pub burn_eff: R, pub downcycle: R, pub src: String, pub note: String }

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Data { pub grid_mj: R, pub grid_mj_note: String, pub processes: Vec<Factor>, pub machining: Factor, pub transport: Vec<Factor>, pub vehicles: Vec<Factor>, pub grid: Vec<Factor>, pub eol: EolData }

/// The built-in factors (checked by the tests).
pub fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../../../data/eco.json")).unwrap_or_else(|e| panic!("data/eco.json: {e}")))
}

// ---------------- the audit in a project ----------------

/// The eco audit of a project: design options and the electricity grid they use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Eco { pub options: Vec<Opt>, pub grid: String }
impl Default for Eco { fn default() -> Self { Self { options: vec![], grid: "world".into() } } }

/// One version of the product: its parts, how it travels, how it is used, for how many years.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Opt { pub name: String, pub parts: Vec<Part>, pub legs: Vec<Leg>, #[serde(rename = "use")] pub use_: Use, pub years: f64 }
impl Default for Opt { fn default() -> Self { Self { name: "Option 1".into(), parts: vec![], legs: vec![], use_: Use::default(), years: 10.0 } } }

/// A part: material, finished mass (kg) and how many, its process, the mass machined away (as a
/// fraction of the finished mass), its recycled content (%), and its end of life.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Part { pub name: String, pub material: String, pub mass: f64, pub qty: f64, pub process: String, pub removed: f64, pub recycled: f64, pub eol: String }
impl Default for Part { fn default() -> Self { Self { name: "Part".into(), material: String::new(), mass: 1.0, qty: 1.0, process: "none".into(), removed: 0.0, recycled: 0.0, eol: "landfill".into() } } }

/// A transport leg: mode (`data().transport`) and distance (km).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Leg { pub mode: String, pub km: f64 }

/// The use phase: "none"; "static" (power kW, hours a day, days a year); or "mobile" (a vehicle of
/// `data().vehicles` carrying the product `km` a year).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Use { pub kind: String, pub kw: f64, pub hours: f64, pub days: f64, pub vehicle: String, pub km: f64 }
impl Default for Use { fn default() -> Self { Self { kind: "none".into(), kw: 0.0, hours: 0.0, days: 365.0, vehicle: "car-petrol".into(), km: 0.0 } } }

pub const EOL: &[&str] = &["landfill", "burn", "downcycle", "recycle", "none"];
pub const MAX_OPTS: usize = 4;
pub const MAX_PARTS: usize = 60;

impl Eco {
    /// Keeps every number finite and in its range, at most four options, known choices only.
    pub fn normalize(&mut self) {
        let d = data();
        let fin = |v: &mut f64, lo: f64, hi: f64| *v = if v.is_finite() { v.clamp(lo, hi) } else { lo };
        if !d.grid.iter().any(|g| g.id == self.grid) { self.grid = "world".into(); }
        self.options.truncate(MAX_OPTS);
        for o in &mut self.options {
            fin(&mut o.years, 0.0, 200.0);
            o.parts.truncate(MAX_PARTS);
            o.legs.truncate(20);
            for p in &mut o.parts {
                fin(&mut p.mass, 0.0, 1e7); fin(&mut p.qty, 0.0, 1e6); fin(&mut p.removed, 0.0, 20.0); fin(&mut p.recycled, 0.0, 100.0);
                if !d.processes.iter().any(|f| f.id == p.process) { p.process = "none".into(); }
                if !EOL.contains(&p.eol.as_str()) { p.eol = "landfill".into(); }
            }
            for l in &mut o.legs { fin(&mut l.km, 0.0, 1e6); if !d.transport.iter().any(|f| f.id == l.mode) { l.mode = "truck".into(); } }
            let u = &mut o.use_;
            if !["none", "static", "mobile"].contains(&u.kind.as_str()) { u.kind = "none".into(); }
            fin(&mut u.kw, 0.0, 1e5); fin(&mut u.hours, 0.0, 24.0); fin(&mut u.days, 0.0, 366.0); fin(&mut u.km, 0.0, 1e7);
            if !d.vehicles.iter().any(|f| f.id == u.vehicle) { u.vehicle = "car-petrol".into(); }
        }
    }
}

// ---------------- results ----------------

pub const PHASES: [&str; 5] = ["Material", "Manufacture", "Transport", "Use", "End of life"];

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Val2 { pub mj: R, pub co2: R }
impl Val2 {
    fn add(self, o: Val2) -> Val2 { Val2 { mj: add(self.mj, o.mj), co2: add(self.co2, o.co2) } }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartOut { pub name: String, pub material: String, pub mass: f64, pub material_v: Val2, pub make: Val2, pub eol: Val2 }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptOut {
    pub name: String,
    /// The product's mass, kg.
    pub mass: f64,
    /// Material, manufacture, transport, use, end of life.
    pub phases: Vec<Val2>,
    /// The first four phases; and with the end-of-life credit.
    pub total: Val2,
    pub net: Val2,
    /// Middle values for the break-even chart: everything but use and end of life, and use per year.
    pub fixed: [f64; 2],
    pub rate: [f64; 2],
    /// The phase with the most CO₂, and what usually helps.
    pub dominant: String,
    pub advice: String,
    pub parts: Vec<PartOut>,
    pub notes: Vec<String>,
}

/// Where two options cost the same over the years: option `b` against option `a` (the first).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakeven { pub a: usize, pub b: usize, pub mj_years: Option<f64>, pub co2_years: Option<f64> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Audit { pub options: Vec<OptOut>, pub breakeven: Vec<Breakeven>, pub grid: String }

fn range(m: &Material, k: &str) -> Option<R> { m.props.get(k).and_then(Val::range).map(|(a, b)| [a, b]) }
fn flag(m: &Material, k: &str) -> bool { m.props.get(k).and_then(Val::flag).unwrap_or(false) }

const ADVICE: [&str; 4] = [
    "Use less material (a lighter design, a better index), more recycled content, or a material with less embodied CO₂: see Greener substitutes.",
    "Choose a process with less energy per kilogram, cut scrap and machining, and make it with low-carbon electricity.",
    "Ship by sea or rail rather than by air or road, source closer, and pack lighter.",
    "For a product that moves, cut its mass; for one that is powered, cut its power and standby, and use cleaner electricity.",
];

fn find<'a>(list: &'a [Factor], id: &str) -> Option<&'a Factor> { list.iter().find(|f| f.id == id) }

/// Works out every option of the audit.
pub fn audit(db: &Db, e: &Eco) -> Audit {
    let d = data();
    let grid = find(&d.grid, &e.grid).or_else(|| d.grid.first());
    let gco2 = grid.map_or(Z, |g| g.co2);
    let elec = |kwh: R| Val2 { mj: mul(kwh, d.grid_mj), co2: mul(kwh, gco2) };
    let options: Vec<OptOut> = e.options.iter().map(|o| {
        let mut notes = vec![];
        let mut ph = [Val2::default(); 5];
        let mut parts = vec![];
        let mut mass = 0.0;
        for p in &o.parts {
            let m_fin = p.mass * p.qty;
            mass += m_fin;
            let Some(mat) = db.get(&p.material) else {
                notes.push(format!("{}: no material chosen; left out.", p.name));
                continue;
            };
            let m_in = m_fin * (1.0 + p.removed);
            // Material: primary and recycled content mixed.
            let f = p.recycled / 100.0;
            let (ee, co2) = (range(mat, "ee"), range(mat, "co2"));
            let (rce, rcc) = (range(mat, "rc_ee"), range(mat, "rc_co2"));
            let mut mv = Val2::default();
            match (ee, co2) {
                (Some(ee), Some(co2)) => {
                    let (rce, rcc) = match (rce, rcc) {
                        (Some(a), Some(b)) => (a, b),
                        _ => { if f > 0.0 { notes.push(format!("{}: {} has no value for recycled material; counted as primary.", p.name, mat.name)); } (ee, co2) }
                    };
                    mv = Val2 { mj: sc(add(sc(ee, 1.0 - f), sc(rce, f)), m_in), co2: sc(add(sc(co2, 1.0 - f), sc(rcc, f)), m_in) };
                }
                _ => notes.push(format!("{}: {} has no embodied energy or CO₂; its material phase is a gap, left out.", p.name, mat.name)),
            }
            // Manufacture: the shaping process, and machining of the mass removed.
            let pr = find(&d.processes, &p.process).cloned().unwrap_or_default();
            let mut make = Val2 { mj: sc(pr.mj, m_fin), co2: sc(pr.co2, m_fin) };
            if p.removed > 0.0 { make = make.add(Val2 { mj: sc(d.machining.mj, m_fin * p.removed), co2: sc(d.machining.co2, m_fin * p.removed) }); }
            // End of life.
            let ev = &d.eol;
            let eol = match p.eol.as_str() {
                "landfill" => Val2 { mj: sc(ev.landfill_mj, m_fin), co2: sc(ev.landfill_co2, m_fin) },
                "burn" => match (flag(mat, "combust"), range(mat, "hc"), range(mat, "co2_comb")) {
                    (true, Some(hc), Some(cc)) => {
                        let rec = sc(mul(hc, ev.burn_eff), m_fin);
                        let saved = mul(sc(rec, 1.0 / 3.6), gco2);
                        Val2 { mj: neg(rec), co2: [cc[0] * m_fin - saved[1], cc[1] * m_fin - saved[0]] }
                    }
                    _ => { notes.push(format!("{}: {} is not combustible for energy recovery (or has no heat of combustion); end of life left out.", p.name, mat.name)); Val2::default() }
                },
                "downcycle" => match (flag(mat, "downcycle"), ee, co2) {
                    (true, Some(ee), Some(co2)) => { let k = mul(ev.downcycle, ev.collect); Val2 { mj: neg(sc(mul(ee, k), m_fin)), co2: neg(sc(mul(co2, k), m_fin)) } }
                    _ => { notes.push(format!("{}: {} cannot be downcycled (or has no embodied energy); end of life left out.", p.name, mat.name)); Val2::default() }
                },
                "recycle" => match (flag(mat, "recyclable"), ee, co2, rce, rcc) {
                    (true, Some(ee), Some(co2), Some(re), Some(rc)) => {
                        let save = |a: R, b: R| [(a[0] - b[1]).max(0.0), (a[1] - b[0]).max(0.0)];
                        Val2 { mj: neg(sc(mul(save(ee, re), ev.collect), m_fin)), co2: neg(sc(mul(save(co2, rc), ev.collect), m_fin)) }
                    }
                    _ => { notes.push(format!("{}: {} is not recyclable (or has no recycling values); end of life left out.", p.name, mat.name)); Val2::default() }
                },
                _ => Val2::default(),
            };
            ph[0] = ph[0].add(mv);
            ph[1] = ph[1].add(make);
            ph[4] = ph[4].add(eol);
            parts.push(PartOut { name: p.name.clone(), material: mat.name.clone(), mass: m_fin, material_v: mv, make, eol });
        }
        // Transport of the whole product.
        for l in &o.legs {
            if let Some(f) = find(&d.transport, &l.mode) { let tkm = mass / 1000.0 * l.km; ph[2] = ph[2].add(Val2 { mj: sc(f.mj, tkm), co2: sc(f.co2, tkm) }); }
        }
        // Use.
        let u = &o.use_;
        let per_year = match u.kind.as_str() {
            "static" => elec([u.kw * u.hours * u.days; 2]),
            "mobile" => match find(&d.vehicles, &u.vehicle) {
                Some(v) => { let tkm = mass / 1000.0 * u.km; match v.kwh { Some(k) => elec(sc(k, tkm)), None => Val2 { mj: sc(v.mj, tkm), co2: sc(v.co2, tkm) } } }
                None => Val2::default(),
            },
            _ => Val2::default(),
        };
        ph[3] = Val2 { mj: sc(per_year.mj, o.years), co2: sc(per_year.co2, o.years) };
        let total = ph[..4].iter().fold(Val2::default(), |a, b| a.add(*b));
        let net = total.add(ph[4]);
        let k = (0..4).max_by(|&a, &b| mid(ph[a].co2).total_cmp(&mid(ph[b].co2))).unwrap_or(0);
        let fixed = ph[..3].iter().fold(Val2::default(), |a, b| a.add(*b));
        OptOut {
            name: o.name.clone(), mass, phases: ph.to_vec(), total, net,
            fixed: [mid(fixed.mj), mid(fixed.co2)], rate: [mid(per_year.mj), mid(per_year.co2)],
            dominant: if mid(total.co2) > 0.0 { PHASES[k].into() } else { String::new() },
            advice: if mid(total.co2) > 0.0 { ADVICE[k].into() } else { String::new() },
            parts, notes,
        }
    }).collect();
    let cross = |fa: f64, ra: f64, fb: f64, rb: f64| { let t = (fb - fa) / (ra - rb); (t.is_finite() && t > 0.0).then_some(t) };
    let breakeven = options.iter().enumerate().skip(1).map(|(b, ob)| {
        let oa = &options[0];
        Breakeven { a: 0, b, mj_years: cross(oa.fixed[0], oa.rate[0], ob.fixed[0], ob.rate[0]), co2_years: cross(oa.fixed[1], oa.rate[1], ob.fixed[1], ob.rate[1]) }
    }).collect();
    Audit { options, breakeven, grid: grid.map_or(String::new(), |g| g.name.clone()) }
}

// ---------------- greener substitutes ----------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sub {
    pub id: String,
    pub name: String,
    pub family: String,
    /// CO₂ for the same job (or per kilogram), as a multiple of the material's own.
    pub co2: f64,
    /// Its index as a multiple of the material's (when something ranks).
    pub index: Option<f64>,
    /// What you give up: property and ratio (price per job higher; others lower), when 20 % worse or more.
    pub gives: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subs { pub of: String, pub basis: String, pub rows: Vec<Sub>, pub note: String }

fn typ(m: &Material, k: &str) -> Option<f64> { let (a, b) = m.props.get(k)?.range()?; Some(mean(a, b, prop(k).is_some_and(|p| p.log))) }

/// Materials that pass the same stages, with an index at least (1 − `tol`) of this material's, and
/// less CO₂ for the same job. With a minimum-mass index (one with ρ), the job's mass scales as
/// 1/index, so CO₂ per job ∝ CO₂/index; with no index, CO₂ per kilogram is compared.
pub fn substitutes(db: &Db, p: &Project, id: &str, tol: f64) -> Subs {
    let Some(me) = db.get(id) else { return Subs { of: String::new(), basis: String::new(), rows: vec![], note: "That material is not in the database.".into() } };
    let r = crate::select::run(db, p);
    let base = r.rows.iter().find(|x| x.id == id).and_then(|x| x.index);
    let name = &r.index_name;
    let per_job = base.is_some() && name.contains('ρ') && !name.contains("CO₂") && !name.contains("Cₘ") && !name.contains("Hₘ");
    let basis = if per_job { format!("CO₂ for the same job: a lighter material needs less of it (per unit of {name})") } else if base.is_some() { format!("CO₂ per kilogram; the index {name} is not a minimum-mass index") } else { "CO₂ per kilogram; set a selection line to compare for the same job".into() };
    let Some(c0) = typ(me, "co2") else { return Subs { of: me.name.clone(), basis, rows: vec![], note: format!("{} has no CO₂ value, so nothing can be compared.", me.name) } };
    let job = |ix: Option<f64>| if per_job { base.zip(ix).map(|(b, i)| b / i) } else { Some(1.0) };
    let p0 = typ(me, "price");
    let mut rows: Vec<Sub> = r.rows.iter().filter(|x| x.pass && x.id != id).filter_map(|x| {
        if let (Some(b), Some(i)) = (base, x.index) { if i < b * (1.0 - tol) { return None; } } else if base.is_some() { return None; }
        let m = db.get(&x.id)?;
        let j = job(x.index)?;
        let co2 = typ(m, "co2")? * j / c0;
        if co2 >= 1.0 { return None; }
        let mut gives = vec![];
        if let (Some(a), Some(b)) = (p0, typ(m, "price")) { let k = b * j / a; if k >= 1.2 { gives.push(("price".to_string(), k)); } }
        for k in ["tmax", "kic", "elong", "sy"] {
            if let (Some(a), Some(b)) = (typ(me, k), typ(m, k)) { if a > 0.0 && b / a <= 0.8 { gives.push((k.to_string(), b / a)); } }
        }
        Some(Sub { id: m.id.clone(), name: m.name.clone(), family: m.family.first().cloned().unwrap_or_default(), co2, index: base.zip(x.index).map(|(b, i)| i / b), gives })
    }).collect();
    rows.sort_by(|a, b| a.co2.total_cmp(&b.co2));
    rows.truncate(20);
    let note = if rows.is_empty() { "No material that passes the same stages does this job with less CO₂.".into() } else { String::new() };
    Subs { of: me.name.clone(), basis, rows, note }
}

/// Everything wrong with the factors: unknown sources, low above high, negative values, duplicate ids.
pub fn check(d: &Data, sources: &[Source]) -> Vec<String> {
    let mut errs = vec![];
    let known = |s: &str| sources.iter().any(|x| x.id == s);
    let ok = |r: R| r[0].is_finite() && r[1].is_finite() && 0.0 <= r[0] && r[0] <= r[1];
    let all: Vec<(&str, &Factor)> = d.processes.iter().map(|f| ("process", f)).chain([("machining", &d.machining)]).chain(d.transport.iter().map(|f| ("transport", f))).chain(d.vehicles.iter().map(|f| ("vehicle", f))).chain(d.grid.iter().map(|f| ("grid", f))).collect();
    for (kind, f) in &all {
        if !known(&f.src) { errs.push(format!("{kind} {}: source {:?} not found", f.id, f.src)); }
        if !ok(f.mj) || !ok(f.co2) || f.kwh.is_some_and(|k| !ok(k)) { errs.push(format!("{kind} {}: a range is negative or low > high", f.id)); }
        if f.name.trim().is_empty() { errs.push(format!("{kind} {}: no name", f.id)); }
    }
    for list in [&d.processes, &d.transport, &d.vehicles, &d.grid] {
        let mut ids: Vec<&str> = list.iter().map(|f| f.id.as_str()).collect();
        ids.sort();
        if ids.windows(2).any(|w| w[0] == w[1]) { errs.push("duplicate factor ids".into()); }
    }
    let e = &d.eol;
    if !known(&e.src) { errs.push("end of life: source not found".into()); }
    for r in [e.collect, e.burn_eff, e.downcycle] { if !(ok(r) && r[1] <= 1.0) { errs.push("end of life: a share must be between 0 and 1".into()); } }
    if !ok(e.landfill_mj) || !ok(e.landfill_co2) || !ok(d.grid_mj) { errs.push("end of life or grid energy: bad range".into()); }
    errs
}
