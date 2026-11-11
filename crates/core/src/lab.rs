//! Hardness, fatigue and creep test data (saved in the project) and what follows from them:
//! hardness statistics with a confidence interval of the mean; an S–N curve fitted with Basquin's
//! law σa = σf′ (2N)^b; and creep rupture data on a Larson–Miller plot, P = T (C + log10 t), with the
//! constant C given or found from the data. Then a material of your own from the results of all the
//! tests. Written from the textbook formulas; a value that cannot be worked out is a gap.
use crate::{model::Material, tensile, Val};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCALES: &[&str] = &["HV", "HB", "HK", "HRC", "HRB", "HR15N", "Shore A", "Shore D"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HardSet { pub name: String, pub scale: String, pub values: Vec<f64> }
impl Default for HardSet { fn default() -> Self { HardSet { name: String::new(), scale: "HV".into(), values: vec![] } } }

/// A fatigue test: stress amplitude (MPa), cycles to failure, and whether it ran out unbroken.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FatPoint { pub s: f64, pub n: f64, #[serde(default)] pub runout: bool }

/// A creep rupture test: stress (MPa), temperature (°C), time to rupture (hours).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CreepPoint { pub s: f64, pub t: f64, pub h: f64 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lab {
    pub hardness: Vec<HardSet>,
    pub fatigue: Vec<FatPoint>,
    /// Cycles at which to give the fatigue strength.
    pub fat_at: f64,
    pub creep: Vec<CreepPoint>,
    /// The Larson–Miller constant; 0 finds the one that fits the data best.
    pub lmp_c: f64,
    /// A question for the creep fit: [temperature °C, hours] → the stress to rupture.
    pub creep_at: [f64; 2],
}
impl Default for Lab { fn default() -> Self { Lab { hardness: vec![], fatigue: vec![], fat_at: 1e7, creep: vec![], lmp_c: 20.0, creep_at: [600.0, 100_000.0] } } }

const MAX: usize = 5000;
impl Lab {
    pub fn validate(&self) -> Result<(), String> {
        if self.hardness.len() > 50 || self.hardness.iter().any(|h| h.values.len() > MAX) || self.fatigue.len() > MAX || self.creep.len() > MAX { return Err(format!("Up to 50 hardness sets and {MAX} readings or points of each kind.")) }
        if self.hardness.iter().any(|h| !SCALES.contains(&h.scale.as_str()) || h.values.iter().any(|v| !(v.is_finite() && *v >= 0.0))) { return Err("Hardness readings are numbers of zero or more, on a known scale.".into()) }
        if self.fatigue.iter().any(|p| !(p.s.is_finite() && p.s > 0.0 && p.n.is_finite() && p.n >= 1.0)) { return Err("A fatigue point is a stress amplitude above zero and at least one cycle.".into()) }
        if self.creep.iter().any(|p| !(p.s.is_finite() && p.s > 0.0 && p.t.is_finite() && p.t > -273.15 && p.h.is_finite() && p.h > 0.0)) { return Err("A creep point is a stress above zero, a temperature above absolute zero and a time above zero.".into()) }
        if !(self.fat_at.is_finite() && self.fat_at >= 1.0) || !(self.lmp_c.is_finite() && (self.lmp_c == 0.0 || (5.0..=60.0).contains(&self.lmp_c))) { return Err("Cycles are at least one; the Larson–Miller constant is between 5 and 60, or 0 to find it.".into()) }
        if !(self.creep_at.iter().all(|v| v.is_finite()) && self.creep_at[0] > -273.15 && self.creep_at[1] > 0.0) { return Err("The creep question is a temperature and a time above zero.".into()) }
        Ok(())
    }
}

/// Two-sided 95 % Student t for n − 1 degrees of freedom.
fn t95(df: usize) -> f64 {
    const T: [f64; 30] = [12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056, 2.052, 2.048, 2.045, 2.042];
    if df == 0 { f64::NAN } else if df <= 30 { T[df - 1] } else { 1.96 + 2.4 / df as f64 }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardOut { pub name: String, pub scale: String, pub n: usize, pub mean: Option<f64>, pub sd: Option<f64>, pub min: Option<f64>, pub max: Option<f64>, pub cov: Option<f64>, pub ci: Option<[f64; 2]>,
    /// Yield strength by Tabor's relation σy ≈ H/3 (H in MPa), for metals on the Vickers scale only.
    pub tabor: Option<f64> }

pub fn hardness(h: &HardSet) -> HardOut {
    let v = &h.values;
    let n = v.len();
    let mean = (n > 0).then(|| v.iter().sum::<f64>() / n as f64);
    let sd = mean.filter(|_| n > 1).map(|m| (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt());
    HardOut { name: h.name.clone(), scale: h.scale.clone(), n, mean, sd, min: v.iter().copied().reduce(f64::min), max: v.iter().copied().reduce(f64::max),
        cov: mean.zip(sd).filter(|(m, _)| *m > 0.0).map(|(m, s)| s / m * 100.0), ci: mean.zip(sd).map(|(m, s)| { let w = t95(n - 1) * s / (n as f64).sqrt(); [m - w, m + w] }),
        tabor: mean.filter(|_| h.scale == "HV").map(|m| m * 9.806_65 / 3.0) }
}

/// Least squares y = a + b x with R².
fn fit(p: &[(f64, f64)]) -> Option<(f64, f64, f64)> {
    let n = p.len() as f64;
    if p.len() < 3 { return None }
    let (mx, my) = (p.iter().map(|q| q.0).sum::<f64>() / n, p.iter().map(|q| q.1).sum::<f64>() / n);
    let sxx: f64 = p.iter().map(|q| (q.0 - mx).powi(2)).sum();
    let sxy: f64 = p.iter().map(|q| (q.0 - mx) * (q.1 - my)).sum();
    let syy: f64 = p.iter().map(|q| (q.1 - my).powi(2)).sum();
    (sxx > 0.0).then(|| { let b = sxy / sxx; (my - b * mx, b, if syy > 0.0 { (sxy * sxy / (sxx * syy)).min(1.0) } else { 1.0 }) })
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FatOut { pub points: usize, pub broken: usize, pub sf: Option<f64>, pub b: Option<f64>, pub r2: Option<f64>, pub at: f64, pub strength: Option<f64>, pub note: String }

/// Basquin's law on the broken specimens: log σa = log σf′ + b log 2N.
pub fn fatigue(l: &Lab) -> FatOut {
    let pts: Vec<(f64, f64)> = l.fatigue.iter().filter(|p| !p.runout).map(|p| ((2.0 * p.n).log10(), p.s.log10())).collect();
    let mut o = FatOut { points: l.fatigue.len(), broken: pts.len(), at: l.fat_at, ..Default::default() };
    match fit(&pts) {
        Some((a, b, r2)) if b < 0.0 => {
            (o.sf, o.b, o.r2) = (Some(10f64.powf(a)), Some(b), Some(r2));
            o.strength = Some(10f64.powf(a + b * (2.0 * l.fat_at).log10()));
            let nmax = l.fatigue.iter().filter(|p| !p.runout).map(|p| p.n).fold(0.0, f64::max);
            if l.fat_at > nmax * 1.5 { o.note = format!("The strength at {:.0e} cycles is beyond the longest broken test ({nmax:.2e} cycles): the line is carried past the data, and many steels level off at an endurance limit.", l.fat_at) }
        }
        Some(_) => o.note = "The stress does not fall with life in these points: no Basquin fit.".into(),
        None => o.note = "At least three broken specimens at different lives are needed for the fit; run-outs are shown but not fitted.".into(),
    }
    o
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreepOut { pub points: usize, pub c: Option<f64>, pub found: bool,
    /// log10 σ = a0 + a1 P (P in thousands), with R².
    pub a: Option<[f64; 2]>, pub r2: Option<f64>, pub p: Vec<[f64; 2]>, pub stress: Option<f64>, pub p_at: Option<f64>, pub note: String }

fn lmp(c: f64, p: &CreepPoint) -> f64 { (p.t + 273.15) * (c + p.h.log10()) / 1000.0 }

pub fn creep(l: &Lab) -> CreepOut {
    let mut o = CreepOut { points: l.creep.len(), ..Default::default() };
    let with = |c: f64| fit(&l.creep.iter().map(|p| (lmp(c, p), p.s.log10())).collect::<Vec<_>>());
    let c = if l.lmp_c == 0.0 {
        o.found = true;
        (50..=600).map(|k| k as f64 / 10.0).filter_map(|c| with(c).map(|f| (c, f.2))).max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
    } else { Some(l.lmp_c) };
    let Some(c) = c else { o.note = "At least three creep tests are needed for the fit.".into(); return o };
    o.c = Some(c);
    o.p = l.creep.iter().map(|p| [lmp(c, p), p.s]).collect();
    match with(c) {
        Some((a0, a1, r2)) if a1 < 0.0 => {
            (o.a, o.r2) = (Some([a0, a1]), Some(r2));
            let pa = (l.creep_at[0] + 273.15) * (c + l.creep_at[1].log10()) / 1000.0;
            o.p_at = Some(pa);
            o.stress = Some(10f64.powf(a0 + a1 * pa));
            let (lo, hi) = o.p.iter().fold((f64::MAX, f64::MIN), |a, q| (a.0.min(q[0]), a.1.max(q[0])));
            if pa < lo || pa > hi { o.note = "The question lies outside the range of the tests on the Larson–Miller parameter: the line is carried past the data.".into() }
        }
        Some(_) => o.note = "The stress does not fall as the parameter rises in these points: no fit.".into(),
        None => o.note = "At least three creep tests at different conditions are needed for the fit.".into(),
    }
    o
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Out { pub hardness: Vec<HardOut>, pub fatigue: FatOut, pub creep: CreepOut }

pub fn run(l: &Lab) -> Out { Out { hardness: l.hardness.iter().map(hardness).collect(), fatigue: fatigue(l), creep: creep(l) } }

/// A material of your own from the project's test results: each value the range over the specimens
/// (or the fit), its source to be named when it is saved. `src` is the source id the values carry.
pub fn to_material(t: &tensile::Out, l: &Out, name: &str, src: &str) -> Material {
    let mut props: BTreeMap<String, Val> = BTreeMap::new();
    let mut put = |k: &str, v: Vec<f64>| { if let (Some(lo), Some(hi)) = (v.iter().copied().reduce(f64::min), v.iter().copied().reduce(f64::max)) { props.insert(k.into(), Val::Range(lo, hi, src.into())); } };
    let on: Vec<&tensile::Res> = t.rows.iter().filter(|r| r.on).collect();
    put("E", on.iter().filter_map(|r| r.e).collect());
    put("sy", on.iter().filter_map(|r| r.rp02).collect());
    put("ts", on.iter().filter_map(|r| r.rm).collect());
    put("elong", on.iter().filter_map(|r| r.a).collect());
    put("hv", l.hardness.iter().filter(|h| h.scale == "HV").flat_map(|h| [h.min, h.max]).flatten().collect());
    if l.fatigue.at == 1e7 { put("fat", l.fatigue.strength.into_iter().collect()) }
    let mut notes = vec![format!("From the tests in a Datum Atlas project: {} tensile specimen(s)", on.len())];
    if !l.hardness.is_empty() { notes.push(format!("{} hardness set(s)", l.hardness.len())) }
    if l.fatigue.strength.is_some() && l.fatigue.at == 1e7 { notes.push("fatigue strength at 10⁷ cycles from the Basquin fit".into()) }
    Material { id: String::new(), name: name.into(), family: vec!["Metals".into()], props, notes: notes.join(", ") + ". Values are the range over the specimens.", status: "user".into(), ..Default::default() }
}

/// Made-up example data (named so), to try the page: hardness readings, an S–N set and creep tests.
pub fn example() -> Lab {
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5 };
    let hv: Vec<f64> = (0..10).map(|_| (245.0 + 12.0 * r()).round()).collect();
    let mut fatigue: Vec<FatPoint> = [450.0, 420.0, 390.0, 360.0, 330.0, 300.0, 280.0].iter().flat_map(|&s| [0, 1].map(|_| { let n = 0.5 * (s / 1100.0f64).powf(1.0 / -0.085) * 10f64.powf(0.15 * r()); FatPoint { s, n: n.round(), runout: false } })).collect();
    fatigue.push(FatPoint { s: 250.0, n: 1e7, runout: true });
    // Creep: log10 σ = 6.0 − 0.18 P with C = 20 (P in thousands), with a little scatter in life.
    let creep = [(300.0, 550.0), (220.0, 550.0), (160.0, 600.0), (120.0, 600.0), (80.0, 650.0), (55.0, 650.0)].iter().map(|&(s, t): &(f64, f64)| {
        let p = (s.log10() - 6.0) / -0.18;
        let h = 10f64.powf(p * 1000.0 / (t + 273.15) - 20.0 + 0.1 * r());
        CreepPoint { s, t, h: (h * 10.0).round() / 10.0 }
    }).collect();
    Lab { hardness: vec![HardSet { name: "Example (made-up data)".into(), scale: "HV".into(), values: hv }], fatigue, creep, ..Default::default() }
}
