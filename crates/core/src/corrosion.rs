//! Corrosion: galvanic couples from the galvanic series in seawater, E–pH (Pourbaix) diagrams
//! worked out from standard Gibbs energies of formation, and corrosion kinetics (mass change
//! against time fitted to the usual laws, rates, Arrhenius, allowance). Data: data/corrosion.json,
//! every value with its source.
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Galvanic { pub id: String, pub name: String, pub min: f64, pub max: f64, pub note: String, pub src: String }

/// A species of a metal for E–pH diagrams: `n` metal atoms, `o` oxygen, `h` hydrogen, charge `z`,
/// ΔfG° (kJ/mol, 298.15 K). Solids have activity 1; dissolved species the activity chosen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Species { pub metal: String, pub name: String, pub n: f64, pub o: f64, pub h: f64, pub z: f64, pub dg: f64, pub solid: bool, pub src: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metal { pub id: String, pub name: String }

#[derive(Debug, Deserialize)]
pub struct Data { pub galvanic: Vec<Galvanic>, pub metals: Vec<Metal>, pub species: Vec<Species>, pub water: f64, #[serde(default)] pub notes: Vec<String> }

pub fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../../../data/corrosion.json")).unwrap_or_else(|e| panic!("data/corrosion.json: {e}")))
}

/// Faraday's constant (C/mol), the gas constant (J/(mol·K)) and 25 °C.
const F: f64 = 96485.332;
const RG: f64 = 8.314462;
const T0: f64 = 298.15;
/// The saturated calomel electrode against the standard hydrogen electrode, 25 °C (V).
pub const SCE: f64 = 0.241;

// ---------- galvanic couples ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Couple { pub anode: String, pub cathode: String, pub de: f64, pub de_min: f64, pub de_max: f64, pub risk: String, pub notes: Vec<String> }

/// Two metals in contact in one electrolyte, each with its corrosion potential (a range, V, any one
/// reference): which corrodes (the more negative), the difference of the middles of the ranges and
/// its spread, and a plain risk word. `area` is cathode area / anode area (0: not known).
pub fn couple(x: &Galvanic, y: &Galvanic, area: f64) -> Result<Couple, String> {
    for g in [x, y] { if !(g.min.is_finite() && g.max.is_finite() && g.min <= g.max && g.min > -5.0 && g.max < 5.0) { return Err(format!("{}: type its potential as a number of volts (lowest, then highest).", if g.name.is_empty() { "A metal" } else { &g.name })) } }
    let mid = |q: &Galvanic| (q.min + q.max) / 2.0;
    let (an, ca) = if mid(x) <= mid(y) { (x, y) } else { (y, x) };
    let de = mid(ca) - mid(an);
    let (lo, hi) = ((ca.min - an.max).max(0.0), ca.max - an.min);
    let risk = if de < 0.05 { "Low" } else if de < 0.15 { "Moderate in seawater; low in a dry or controlled place" } else if de < 0.25 { "High in seawater; moderate indoors" } else { "Severe: avoid the contact, or insulate the two" };
    let mut notes = vec![];
    if x.id == y.id && !x.id.is_empty() { notes.push("The same metal on both sides: no galvanic couple.".into()) }
    if area > 1.0 { notes.push(format!("A cathode {} times the anode's area concentrates the attack on the small anode: keep the anode (the {}) the larger part, or coat the cathode.", (area * 1000.0).round() / 1000.0, an.name.to_lowercase())) }
    else if area > 0.0 && area < 1.0 { notes.push("A large anode and a small cathode spread the attack thinly: the better way round.".into()) }
    if !an.note.is_empty() { notes.push(format!("{}: {}", an.name, an.note)) }
    if !ca.note.is_empty() { notes.push(format!("{}: {}", ca.name, ca.note)) }
    Ok(Couple { anode: an.id.clone(), cathode: ca.id.clone(), de, de_min: lo, de_max: hi, risk: risk.into(), notes })
}

// ---------- E–pH diagrams ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PbAsk { pub metal: String, pub log_a: f64, pub ph: [f64; 2], pub e: [f64; 2], pub nx: usize, pub ny: usize }
impl Default for PbAsk { fn default() -> Self { Self { metal: "fe".into(), log_a: -6.0, ph: [-2.0, 16.0], e: [-2.0, 2.0], nx: 360, ny: 320 } } }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region { pub name: String, pub kind: String, pub share: f64, pub cx: f64, pub cy: f64 }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagram { pub nx: usize, pub ny: usize, pub ph: [f64; 2], pub e: [f64; 2],
    /// Per cell, row by row from the top (highest E): the index into `regions`.
    pub cells: Vec<u8>, pub regions: Vec<Region>, pub notes: Vec<String>,
    /// The water lines at both ends of the pH range: [pH, oxygen line, hydrogen line] (V vs SHE).
    pub water: [[f64; 3]; 2] }

impl Species {
    /// Electrons released when `n` metal atoms become this species with water: n M + o H2O → species + (2o − h) H⁺ + e e⁻.
    pub fn electrons(&self) -> f64 { self.z + 2.0 * self.o - self.h }
    /// ΔG (J) per metal atom of forming this species from the metal at potential `e` (V vs SHE) and `ph`.
    pub fn g(&self, e: f64, ph: f64, log_a: f64, water: f64) -> f64 {
        let k = RG * T0 * std::f64::consts::LN_10;
        let act = if self.solid { 0.0 } else { k * log_a };
        (self.dg * 1000.0 - self.o * water * 1000.0 + act - (2.0 * self.o - self.h) * k * ph - self.electrons() * F * e) / self.n
    }
    fn kind(&self) -> &'static str { if !self.solid { "corrosion" } else if self.o == 0.0 && self.h == 0.0 && self.z == 0.0 { "immunity" } else { "passivity" } }
}

/// The species with the lowest Gibbs energy per metal atom at each point of the E–pH plane:
/// the metal (immunity), a solid oxide or hydroxide (passivity) or a dissolved ion (corrosion).
pub fn pourbaix(q: &PbAsk) -> Result<Diagram, String> {
    let d = data();
    if !(q.log_a >= -12.0 && q.log_a <= 1.0) { return Err("Choose an activity of dissolved metal between 10⁻¹² and 10.".into()) }
    if !(q.ph[1] > q.ph[0] && q.e[1] > q.e[0]) || [q.ph[0], q.ph[1], q.e[0], q.e[1]].iter().any(|v| !v.is_finite()) { return Err("The ranges need a lower and a higher value.".into()) }
    let (nx, ny) = (q.nx.clamp(20, 800), q.ny.clamp(20, 800));
    let mut sp: Vec<Species> = d.species.iter().filter(|s| s.metal == q.metal).cloned().collect();
    if sp.is_empty() { return Err(format!("No data for “{}”.", q.metal)) }
    sp.insert(0, Species { metal: q.metal.clone(), name: d.metals.iter().find(|m| m.id == q.metal).map_or(q.metal.clone(), |m| m.name.clone()), n: 1.0, o: 0.0, h: 0.0, z: 0.0, dg: 0.0, solid: true, src: String::new() });
    if sp.len() > 250 { return Err("Too many species.".into()) }
    let mut cells = vec![0u8; nx * ny];
    let mut sums = vec![(0usize, 0.0, 0.0); sp.len()];
    for j in 0..ny {
        let e = q.e[1] - (j as f64 + 0.5) / ny as f64 * (q.e[1] - q.e[0]);
        for i in 0..nx {
            let ph = q.ph[0] + (i as f64 + 0.5) / nx as f64 * (q.ph[1] - q.ph[0]);
            let k = (0..sp.len()).min_by(|&a, &b| sp[a].g(e, ph, q.log_a, d.water).total_cmp(&sp[b].g(e, ph, q.log_a, d.water))).unwrap_or(0);
            cells[j * nx + i] = k as u8;
            let s = &mut sums[k]; s.0 += 1; s.1 += ph; s.2 += e;
        }
    }
    // Only the species that appear, renumbered.
    let mut map = vec![u8::MAX; sp.len()];
    let mut regions = vec![];
    for (k, s) in sums.iter().enumerate() {
        if s.0 == 0 { continue }
        map[k] = regions.len() as u8;
        regions.push(Region { name: sp[k].name.clone(), kind: sp[k].kind().into(), share: s.0 as f64 / (nx * ny) as f64 * 100.0, cx: s.1 / s.0 as f64, cy: s.2 / s.0 as f64 });
    }
    for c in &mut cells { *c = map[*c as usize] }
    let notes = vec![format!("25 °C, dissolved metal at {} mol/L (as activity); potentials against the standard hydrogen electrode (SHE; the saturated calomel electrode is {SCE} V above it).", fmt_conc(q.log_a)),
        "Between the dashed lines water is stable; above the upper one it gives off oxygen, below the lower one hydrogen.".into(),
        "A diagram says what is stable, not how fast: a passive film protects only if it is dense and adherent, and chloride can break it.".into()];
    let w = |ph: f64| { let (o, h) = water_lines(ph); [ph, o, h] };
    Ok(Diagram { nx, ny, ph: q.ph, e: q.e, cells, regions, notes, water: [w(q.ph[0]), w(q.ph[1])] })
}

/// 10⁻⁶ and the like; 1 for 10⁰.
fn fmt_conc(log_a: f64) -> String {
    if log_a == 0.0 { return "1".into() }
    let sup: String = format!("{}", log_a.round() as i64).chars().map(|c| match c { '-' => '⁻', d => "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().nth(d.to_digit(10).unwrap_or(0) as usize).unwrap_or(d) }).collect();
    format!("10{sup}")
}

/// The water lines at 25 °C (V vs SHE): oxygen (upper) and hydrogen (lower), at pH `ph`.
pub fn water_lines(ph: f64) -> (f64, f64) { let k = RG * T0 * std::f64::consts::LN_10 / F; (1.229 - k * ph, -k * ph) }

// ---------- kinetics ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fit { pub law: String, pub formula: String, pub k: f64, pub b: f64, pub r2: f64, pub unit: String, pub curve: Vec<[f64; 2]> }

/// Mass change (mg/cm², gain positive or loss given as a positive number) against time (h), fitted
/// to the linear (Δm = k·t), parabolic (Δm² = kp·t), logarithmic (Δm = a + k·log₁₀ t) and power
/// (Δm = k·tⁿ) laws, best first. R² is on Δm for every law, so they compare.
pub fn kinetics(pts: &[[f64; 2]]) -> Result<Vec<Fit>, String> {
    let p: Vec<[f64; 2]> = pts.iter().copied().filter(|q| q[0].is_finite() && q[1].is_finite() && q[0] > 0.0).collect();
    if p.len() < 3 { return Err("Type at least three points with a time above zero.".into()) }
    let mean = p.iter().map(|q| q[1]).sum::<f64>() / p.len() as f64;
    let sst = p.iter().map(|q| (q[1] - mean).powi(2)).sum::<f64>();
    let r2 = |f: &dyn Fn(f64) -> f64| if sst > 0.0 { 1.0 - p.iter().map(|q| (q[1] - f(q[0])).powi(2)).sum::<f64>() / sst } else { f64::NAN };
    let tmax = p.iter().map(|q| q[0]).fold(0.0, f64::max);
    let curve = |f: &dyn Fn(f64) -> f64| (1..=80).map(|k| { let t = tmax * k as f64 / 80.0; [t, f(t)] }).collect::<Vec<_>>();
    let mut out = vec![];
    let lin = p.iter().map(|q| q[0] * q[1]).sum::<f64>() / p.iter().map(|q| q[0] * q[0]).sum::<f64>();
    let f = |t: f64| lin * t;
    out.push(Fit { law: "Linear".into(), formula: "Δm = k·t".into(), k: lin, b: 0.0, r2: r2(&f), unit: "mg/(cm²·h)".into(), curve: curve(&f) });
    if p.iter().all(|q| q[1] >= 0.0) {
        let kp = p.iter().map(|q| q[0] * q[1] * q[1]).sum::<f64>() / p.iter().map(|q| q[0] * q[0]).sum::<f64>();
        let f = |t: f64| (kp * t).max(0.0).sqrt();
        out.push(Fit { law: "Parabolic".into(), formula: "Δm² = kp·t".into(), k: kp, b: 0.0, r2: r2(&f), unit: "mg²/(cm⁴·h)".into(), curve: curve(&f) });
    }
    let (a, k) = line(&p.iter().map(|q| [q[0].log10(), q[1]]).collect::<Vec<_>>());
    let f = |t: f64| a + k * t.log10();
    out.push(Fit { law: "Logarithmic".into(), formula: "Δm = a + k·log₁₀ t".into(), k, b: a, r2: r2(&f), unit: "mg/cm² per decade".into(), curve: curve(&f) });
    if p.iter().all(|q| q[1] > 0.0) {
        let (la, n) = line(&p.iter().map(|q| [q[0].ln(), q[1].ln()]).collect::<Vec<_>>());
        let f = |t: f64| la.exp() * t.powf(n);
        out.push(Fit { law: "Power".into(), formula: "Δm = k·tⁿ".into(), k: la.exp(), b: n, r2: r2(&f), unit: "mg/cm² at 1 h; n".into(), curve: curve(&f) });
    }
    out.sort_by(|a, b| b.r2.total_cmp(&a.r2));
    Ok(out)
}

/// Least squares y = a + b·x.
fn line(p: &[[f64; 2]]) -> (f64, f64) {
    let n = p.len() as f64;
    let (sx, sy) = (p.iter().map(|q| q[0]).sum::<f64>(), p.iter().map(|q| q[1]).sum::<f64>());
    let sxx = p.iter().map(|q| q[0] * q[0]).sum::<f64>() - sx * sx / n;
    let sxy = p.iter().map(|q| q[0] * q[1]).sum::<f64>() - sx * sy / n;
    let b = if sxx.abs() > 0.0 { sxy / sxx } else { 0.0 };
    ((sy - b * sx) / n, b)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Arrhenius { pub q: f64, pub a: f64, pub r2: f64 }

/// Rate constants `k` at temperatures (°C): k = A·exp(−Q/RT); Q in kJ/mol.
pub fn arrhenius(pts: &[[f64; 2]]) -> Result<Arrhenius, String> {
    let p: Vec<[f64; 2]> = pts.iter().filter(|q| q[1] > 0.0 && q[0] > -273.15 && q[0].is_finite() && q[1].is_finite()).map(|q| [1.0 / (q[0] + 273.15), q[1].ln()]).collect();
    if p.len() < 2 || p.iter().all(|q| (q[0] - p[0][0]).abs() < 1e-12) { return Err("Type rate constants above zero at two temperatures or more.".into()) }
    let (a, b) = line(&p);
    let my = p.iter().map(|q| q[1]).sum::<f64>() / p.len() as f64;
    let sst = p.iter().map(|q| (q[1] - my).powi(2)).sum::<f64>();
    let r2 = if sst > 0.0 && p.len() > 2 { 1.0 - p.iter().map(|q| (q[1] - a - b * q[0]).powi(2)).sum::<f64>() / sst } else { 1.0 };
    Ok(Arrhenius { q: -b * RG / 1000.0, a: a.exp(), r2 })
}

/// Corrosion rate (mm/year) from a mass loss `w` (g) of a coupon of area `area` (cm²) and density
/// `rho` (g/cm³) over `hours`: thickness lost w/(ρ·A) cm, times 8760 h a year and 10 mm per cm.
/// What the corrosion pages need to start: the metals with E–pH data, the notes on the data.
pub fn info() -> serde_json::Value { serde_json::json!({ "metals": data().metals, "notes": data().notes, "sce": SCE }) }

/// The kinetics page in one answer: the fits of the mass-change data, the Arrhenius fit of the rate
/// constants, the corrosion rate from a mass loss, and the allowance for a design life.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct KinAsk { pub pts: Vec<[f64; 2]>, pub arr: Vec<[f64; 2]>, pub w: f64, pub area: f64, pub rho: f64, pub hours: f64, pub years: f64, pub factor: f64 }

pub fn kinetics_page(q: &KinAsk) -> serde_json::Value {
    let rate = rate_mm_y(q.w, q.area, q.rho, q.hours);
    let allowance = rate.filter(|_| q.years > 0.0).map(|r| r * q.years * if q.factor > 0.0 { q.factor } else { 1.0 });
    serde_json::json!({ "fits": kinetics(&q.pts).ok(), "fitError": kinetics(&q.pts).err(), "arrhenius": arrhenius(&q.arr).ok(), "arrError": if q.arr.is_empty() { None } else { arrhenius(&q.arr).err() }, "rate": rate, "allowance": allowance })
}

pub fn rate_mm_y(w: f64, area: f64, rho: f64, hours: f64) -> Option<f64> {
    (w >= 0.0 && area > 0.0 && rho > 0.0 && hours > 0.0).then(|| 87_600.0 * w / (rho * area * hours))
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn data_and_sources() {
        let d = data();
        let src = crate::db::builtin().sources;
        for g in &d.galvanic { assert!(g.min <= g.max && g.min > -2.0 && g.max < 0.6, "{}", g.id); assert!(src.iter().any(|s| s.id == g.src), "source {}", g.src) }
        assert!(src.iter().find(|s| s.id == "nbs-1982").is_some_and(|s| crate::db::OPEN_KINDS.contains(&s.kind.as_str())));
        for s in &d.species { assert!(s.n > 0.0 && s.dg.is_finite(), "{}", s.name); assert!(src.iter().any(|x| x.id == s.src), "source {}", s.src); assert!(d.metals.iter().any(|m| m.id == s.metal)) }
        assert!(src.iter().any(|x| d.water < 0.0 && d.species.iter().any(|s| s.src == x.id)));
    }
    #[test]
    fn couples() {
        let g = |id: &str, min: f64, max: f64| Galvanic { id: id.into(), name: id.into(), min, max, note: String::new(), src: String::new() };
        let (zn, cu) = (g("zinc", -1.03, -0.98), g("copper", -0.36, -0.28));
        let c = couple(&zn, &cu, 10.0).unwrap();
        assert_eq!(c.anode, "zinc");
        assert!((c.de - 0.685).abs() < 1e-9 && c.risk.starts_with("Severe") && c.notes.iter().any(|n| n.contains("small anode")));
        assert_eq!(couple(&cu, &zn, 1.0).unwrap().anode, "zinc");
        assert!(couple(&zn, &g("x", 0.2, 0.1), 1.0).is_err());
    }
    #[test]
    fn iron_diagram() {
        let d = pourbaix(&PbAsk::default()).unwrap();
        let at = |ph: f64, e: f64| { let i = ((ph - d.ph[0]) / (d.ph[1] - d.ph[0]) * d.nx as f64) as usize; let j = ((d.e[1] - e) / (d.e[1] - d.e[0]) * d.ny as f64) as usize; &d.regions[d.cells[j * d.nx + i] as usize] };
        // Iron is immune at very low potential, corrodes as Fe²⁺ in acid near −0.3 V, and is passive (an oxide) in neutral water at +0.5 V.
        assert_eq!(at(7.0, -1.2).kind, "immunity");
        assert_eq!(at(2.0, -0.3).name, "Fe²⁺");
        assert_eq!(at(8.0, 0.5).kind, "passivity");
        // The Fe/Fe²⁺ line at 10⁻⁶: E = −0.447 + 0.0296·(−6) ≈ −0.62 V.
        assert_eq!(at(2.0, -0.58).name, "Fe²⁺");
        assert_eq!(at(2.0, -0.66).kind, "immunity");
        assert!(pourbaix(&PbAsk { log_a: 5.0, ..Default::default() }).is_err());
        let (o, h) = water_lines(7.0);
        assert!((o - 0.815).abs() < 0.002 && (h + 0.414).abs() < 0.002);
    }
    #[test]
    fn other_metals() {
        let kind = |m: &str, ph: f64, e: f64| { let d = pourbaix(&PbAsk { metal: m.into(), ..Default::default() }).unwrap(); let i = ((ph - d.ph[0]) / (d.ph[1] - d.ph[0]) * d.nx as f64) as usize; let j = ((d.e[1] - e) / (d.e[1] - d.e[0]) * d.ny as f64) as usize; d.regions[d.cells[j * d.nx + i] as usize].clone() };
        // Copper is noble: immune at 0 V in acid; Cu²⁺ at +0.5 V in acid; an oxide in neutral water at +0.5 V.
        assert_eq!(kind("cu", 2.0, 0.0).kind, "immunity");
        assert_eq!(kind("cu", 2.0, 0.5).name, "Cu²⁺");
        assert_eq!(kind("cu", 8.0, 0.5).kind, "passivity");
        // Zinc and aluminium are amphoteric: they dissolve in acid and in strong alkali, and are passive in between (zinc about pH 9 to 11, aluminium about 4 to 8 at 10⁻⁶).
        for (m, mid) in [("zn", 9.5), ("al", 6.0)] { assert_eq!(kind(m, 2.0, -0.2).kind, "corrosion", "{m}"); assert_eq!(kind(m, mid, -0.2).kind, "passivity", "{m}"); assert_eq!(kind(m, 14.5, -0.2).kind, "corrosion", "{m}") }
        // Magnesium corrodes in neutral water and is passive (Mg(OH)₂) only in strong alkali.
        assert_eq!(kind("mg", 7.0, -1.0).name, "Mg²⁺");
        assert_eq!(kind("mg", 13.0, -1.0).kind, "passivity");
        assert_eq!(kind("ni", 2.0, 0.0).name, "Ni²⁺");
        assert_eq!(kind("ni", 10.0, 0.0).kind, "passivity");
        assert!(pourbaix(&PbAsk { metal: "cr".into(), ..Default::default() }).is_err());
    }
    #[test]
    fn kinetics_laws() {
        let par: Vec<[f64; 2]> = (1..=8).map(|k| { let t = k as f64 * 5.0; [t, (0.02 * t).sqrt()] }).collect();
        let f = kinetics(&par).unwrap();
        assert_eq!(f[0].law, "Parabolic");
        assert!((f[0].k - 0.02).abs() < 1e-9 && f[0].r2 > 0.999999);
        let lin: Vec<[f64; 2]> = (1..=6).map(|k| [k as f64, 0.3 * k as f64]).collect();
        assert_eq!(kinetics(&lin).unwrap()[0].law, "Linear");
        assert!(kinetics(&[[1.0, 1.0]]).is_err());
        let a = arrhenius(&[[600.0, 1e-4], [700.0, 1e-3]]).unwrap();
        assert!((a.q - RG * (10f64).ln() / (1.0 / 873.15 - 1.0 / 973.15) / 1000.0).abs() < 1e-9);
        assert!((rate_mm_y(0.1, 10.0, 7.87, 8760.0).unwrap() - 0.1 / (7.87 * 10.0) * 10.0).abs() < 1e-12);
        assert!(rate_mm_y(0.1, 0.0, 7.87, 1.0).is_none());
    }
}
