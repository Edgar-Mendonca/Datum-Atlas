//! Design goals: a function, objective and constraint give a performance index, and the index is
//! a straight line on a property chart. Picking a goal sets the chart's axes, draws the selection
//! line so the best fifth of the materials pass, and adds the limits the goal suggests (marked
//! as the goal's, so a new goal replaces them and never touches your own).
use crate::db::Db;
use crate::doc::Op;
use crate::geometry::span;
use crate::model::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub id: &'static str,
    pub group: &'static str,
    pub name: &'static str,
    /// The index to maximise (or minimise), as it is written.
    pub index: &'static str,
    pub x: &'static str,
    pub y: &'static str,
    pub slope: f64,
    /// Materials above the line pass (false: below, for indices to minimise).
    pub above: bool,
    pub function: &'static str,
    pub constraint: &'static str,
    pub objective: &'static str,
    pub free: &'static str,
    /// Why the index is what it is, in two or three sentences.
    pub why: &'static str,
    /// Suggested limits: (property, min, max).
    pub limits: &'static [(&'static str, Option<f64>, Option<f64>)],
    /// A second constraint needs a second chart: its axes, line and index. The goal adds that
    /// chart (marked with the goal's id and "/2") and the shortlist is what passes both.
    pub also: Option<Also>,
    /// Formula properties the index needs on an axis (name, formula, unit); applying it adds them.
    pub formulas: &'static [(&'static str, &'static str, &'static str)],
    /// In the index library only (not in the Goal list beside the chart).
    pub library: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Also {
    pub x: &'static str,
    pub y: &'static str,
    pub slope: f64,
    pub above: bool,
    pub index: &'static str,
    pub name: &'static str,
}

const TOUGH: &[(&str, Option<f64>, Option<f64>)] = &[("kic", Some(1.0), None)];
const STRONG: &[(&str, Option<f64>, Option<f64>)] = &[("kic", Some(15.0), None)];

#[allow(clippy::too_many_arguments)]
const fn g(id: &'static str, group: &'static str, name: &'static str, index: &'static str, x: &'static str, y: &'static str, slope: f64, above: bool,
    function: &'static str, constraint: &'static str, objective: &'static str, free: &'static str, why: &'static str, limits: &'static [(&'static str, Option<f64>, Option<f64>)]) -> Goal {
    Goal { id, group, name, index, x, y, slope, above, function, constraint, objective, free, why, limits, also: None, formulas: &[], library: false }
}

/// An index of the library (Index page and builder), with the formula properties it needs.
#[allow(clippy::too_many_arguments)]
const fn lib(id: &'static str, group: &'static str, name: &'static str, index: &'static str, x: &'static str, y: &'static str, slope: f64, above: bool,
    function: &'static str, constraint: &'static str, objective: &'static str, free: &'static str, why: &'static str, formulas: &'static [(&'static str, &'static str, &'static str)]) -> Goal {
    Goal { id, group, name, index, x, y, slope, above, function, constraint, objective, free, why, limits: &[], also: None, formulas, library: true }
}

/// A goal with a second constraint on a second chart.
#[allow(clippy::too_many_arguments)]
const fn g2(id: &'static str, name: &'static str, index: &'static str, x: &'static str, y: &'static str, slope: f64, also: Also,
    function: &'static str, constraint: &'static str, why: &'static str, limits: &'static [(&'static str, Option<f64>, Option<f64>)]) -> Goal {
    Goal { id, group: "Two constraints", name, index, x, y, slope, above: true, function, constraint, objective: "Minimum mass", free: "Size of the section", why, limits, also: Some(also), formulas: &[], library: false }
}

pub const GOALS: &[Goal] = &[
    g("tie-stiff", "Minimum mass", "Light, stiff tie rod", "E/ρ", "rho", "E", 1.0, true, "Tie rod in tension", "Length and stiffness fixed", "Minimum mass", "Cross-section area",
      "Stiffness S = EA/L fixes the area A = SL/E, so the mass m = ALρ = SL²·ρ/E. The lightest tie has the highest E/ρ: a line of slope 1 on the modulus–density chart.", TOUGH),
    g("beam-stiff", "Minimum mass", "Light, stiff beam", "E½/ρ", "rho", "E", 2.0, true, "Beam in bending", "Length and bending stiffness fixed", "Minimum mass", "Size of the (square) section",
      "Bending stiffness grows with EI and I ∝ A², so A ∝ (S/E)^½ and the mass ∝ ρ/E^½. Maximise E½/ρ: a line of slope 2.", TOUGH),
    g("panel-stiff", "Minimum mass", "Light, stiff panel", "E⅓/ρ", "rho", "E", 3.0, true, "Panel (plate) in bending", "Length, width and stiffness fixed", "Minimum mass", "Thickness",
      "A panel's stiffness grows with Et³, so t ∝ E^(−⅓) and the mass ∝ ρ/E^⅓. Maximise E⅓/ρ: a line of slope 3.", TOUGH),
    g("tie-strong", "Minimum mass", "Light, strong tie rod", "σy/ρ", "rho", "sy", 1.0, true, "Tie rod in tension", "Length and load fixed, no yielding", "Minimum mass", "Cross-section area",
      "The load F must stay below σyA, so A = F/σy and the mass = FLρ/σy. Maximise σy/ρ: a line of slope 1 on the strength–density chart.", STRONG),
    g("beam-strong", "Minimum mass", "Light, strong beam", "σy⅔/ρ", "rho", "sy", 1.5, true, "Beam in bending", "Length and load fixed, no yielding", "Minimum mass", "Size of the (square) section",
      "The bending moment is limited by σy·Z with Z ∝ A^(3/2), so A ∝ σy^(−⅔) and the mass ∝ ρ/σy^⅔. Maximise σy⅔/ρ: a line of slope 1.5.", STRONG),
    g("panel-strong", "Minimum mass", "Light, strong panel", "σy½/ρ", "rho", "sy", 2.0, true, "Panel (plate) in bending", "Length, width and load fixed, no yielding", "Minimum mass", "Thickness",
      "A panel's strength grows with σy·t², so t ∝ σy^(−½) and the mass ∝ ρ/σy^½. Maximise σy½/ρ: a line of slope 2.", STRONG),
    g("column", "Minimum mass", "Light, stiff column (buckling)", "E½/ρ", "rho", "E", 2.0, true, "Column in compression", "Length and buckling load fixed", "Minimum mass", "Cross-section area",
      "The Euler buckling load grows with EI and I ∝ A², exactly as for a stiff beam, so the same index E½/ρ (slope 2) gives the lightest column.", TOUGH),
    g("shaft-stiff", "Minimum mass", "Light, stiff shaft (torsion)", "E½/ρ", "rho", "E", 2.0, true, "Solid shaft in torsion", "Length and torsional stiffness fixed", "Minimum mass", "Diameter",
      "Torsional stiffness grows with GJ and J ∝ A², so the index is G½/ρ. For most materials G ≈ 3E/8, so E½/ρ ranks them the same way: a line of slope 2.", STRONG),
    g("beam-cheap", "Minimum cost", "Cheap, stiff beam", "E½/(Cₘρ)", "cv", "E", 2.0, true, "Beam in bending", "Length and bending stiffness fixed", "Minimum material cost", "Size of the section",
      "As for the light stiff beam, but the cost is mass × price per kilogram, so ρ becomes Cₘρ, the price per volume. Maximise E½/(Cₘρ): slope 2 on modulus against price per volume.", TOUGH),
    g("tie-cheap", "Minimum cost", "Cheap, strong tie rod", "σy/(Cₘρ)", "cv", "sy", 1.0, true, "Tie rod in tension", "Length and load fixed, no yielding", "Minimum material cost", "Cross-section area",
      "As for the light strong tie, with the price per volume Cₘρ in place of density. Maximise σy/(Cₘρ): slope 1.", STRONG),
    g("panel-cheap", "Minimum cost", "Cheap, stiff panel", "E⅓/(Cₘρ)", "cv", "E", 3.0, true, "Panel in bending", "Length, width and stiffness fixed", "Minimum material cost", "Thickness",
      "As for the light stiff panel, with the price per volume Cₘρ in place of density. Maximise E⅓/(Cₘρ): slope 3.", TOUGH),
    g("beam-carbon", "Minimum carbon", "Low-carbon stiff beam", "E½/(CO₂·ρ)", "co2v", "E", 2.0, true, "Beam in bending", "Length and bending stiffness fixed", "Minimum CO₂ from making the material", "Size of the section",
      "The CO₂ is mass × CO₂ per kilogram, so ρ becomes CO₂·ρ, the CO₂ per volume. Maximise E½/(CO₂·ρ): slope 2.", TOUGH),
    g("tie-carbon", "Minimum carbon", "Low-carbon strong tie rod", "σy/(CO₂·ρ)", "co2v", "sy", 1.0, true, "Tie rod in tension", "Length and load fixed, no yielding", "Minimum CO₂ from making the material", "Cross-section area",
      "As for the strong tie, with the CO₂ per volume in place of density. Maximise σy/(CO₂·ρ): slope 1.", STRONG),
    g("spring", "Elastic design", "Spring that stores the most energy", "σy²/E", "E", "sy", 0.5, true, "Spring", "No yielding", "Maximum elastic energy per volume", "The material",
      "The energy stored per volume up to yield is σy²/2E. On the strength–modulus chart that is a line of slope ½. Brittle materials make poor springs, hence the toughness limit.", &[("kic", Some(15.0), None)]),
    g("hinge", "Elastic design", "Flexible elastic hinge", "σy/E", "E", "sy", 1.0, true, "Elastic hinge or snap fit", "Must bend far without yielding", "Maximum elastic strain", "The material",
      "The largest strain before yield is σy/E, so the most flexible hinge maximises σy/E: a line of slope 1 on the strength–modulus chart.", &[("elong", Some(2.0), None)]),
    g("yield-first", "Damage tolerance", "Tough part that yields before it breaks", "KIc/σy", "sy", "kic", 1.0, true, "Part with a possible crack", "Must yield before a crack grows", "Largest tolerable crack", "The material",
      "A crack of size a grows when K = σ√(πa) reaches KIc. Yielding first needs KIc/σy large: the tolerable crack ∝ (KIc/σy)². A line of slope 1 on the toughness–strength chart.", &[("sy", Some(50.0), None)]),
    g("leak-first", "Damage tolerance", "Pressure vessel that leaks before it breaks", "KIc²/σy", "sy", "kic", 0.5, true, "Thin-walled pressure vessel", "A through-crack must leak before it runs", "Safe failure", "Wall thickness",
      "A crack as long as the wall is thick must be stable at the design stress, which with t ∝ 1/σy gives KIc²/σy. A line of slope ½ on the toughness–strength chart.", &[("sy", Some(100.0), None)]),
    g("tie-tough", "Damage tolerance", "Light, damage-tolerant tie", "KIc/ρ", "rho", "kic", 1.0, true, "Tie or skin with possible cracks", "A given crack must not grow at the design load", "Minimum mass", "Cross-section area",
      "The load a cracked part carries is ∝ KIc·A, so A ∝ 1/KIc and the mass ∝ ρ/KIc. Maximise KIc/ρ: a line of slope 1 on the toughness–density chart.", &[]),
    g("precision", "Thermal", "Precision part with minimal thermal distortion", "λ/α", "alpha", "lam", 1.0, true, "Precision device under heat flow", "Small distortion", "Minimum thermal distortion", "The material",
      "A heat flux sets a temperature gradient ∝ 1/λ and the distortion follows α times that gradient. Maximise λ/α: a line of slope 1 on conductivity against expansion.", &[]),
    g("insulation", "Thermal", "Thermal insulation", "λ (minimum)", "rho", "lam", 0.0, false, "Insulating wall or layer", "Thickness fixed, steady heat flow", "Minimum heat loss", "The material",
      "Through a wall of fixed thickness the heat flow is proportional to λ. The best insulators have the lowest conductivity: a horizontal line, and the materials below it pass.", &[("tmax", Some(60.0), None)]),
    g("heatsink", "Thermal", "Heat sink or spreader", "λ (maximum)", "rho", "lam", 0.0, true, "Heat sink", "Shape fixed", "Maximum heat flow", "The material",
      "Heat flow through a fixed shape is proportional to λ. The best heat sinks have the highest conductivity: a horizontal line, and the materials above it pass.", &[("tmax", Some(100.0), None)]),
    g("shock", "Thermal", "Resists thermal shock", "σts/(Eα)", "rho", "shock", 0.0, true, "Part heated or quenched quickly", "Must not crack on a sudden temperature change", "Largest temperature jump", "The material",
      "A sudden change ΔT in a constrained surface gives a stress EαΔT; it cracks when that reaches the strength. The largest safe jump is σts/(Eα), the thermal shock resistance: a horizontal line, and the materials above it pass.", &[]),
    g2("beam-stiff-strong", "Light beam, stiff and strong", "E½/ρ and σy⅔/ρ", "rho", "E", 2.0,
      Also { x: "rho", y: "sy", slope: 1.5, above: true, index: "σy⅔/ρ", name: "Strength of the light beam" },
      "Beam in bending", "Bending stiffness and load fixed, no yielding",
      "Each constraint gives its own index: stiffness gives E½/ρ, strength gives σy⅔/ρ. The beam must meet both, so it must pass both lines: the stiffness line on the modulus–density chart and the strength line on the strength–density chart.", TOUGH),
    g2("panel-stiff-strong", "Light panel, stiff and strong", "E⅓/ρ and σy½/ρ", "rho", "E", 3.0,
      Also { x: "rho", y: "sy", slope: 2.0, above: true, index: "σy½/ρ", name: "Strength of the light panel" },
      "Panel (plate) in bending", "Stiffness and load fixed, no yielding",
      "Stiffness gives E⅓/ρ and strength gives σy½/ρ. The panel must meet both, so it must pass both lines, one on each chart.", TOUGH),
    g2("tie-stiff-tough", "Light tie, stiff and damage tolerant", "E/ρ and KIc/ρ", "rho", "E", 1.0,
      Also { x: "rho", y: "kic", slope: 1.0, above: true, index: "KIc/ρ", name: "Toughness of the light tie" },
      "Tie rod or skin in tension", "Stiffness fixed; a given crack must not grow",
      "Stiffness gives E/ρ and a crack that must not grow gives KIc/ρ. The tie must pass both lines: stiffness on the modulus–density chart, toughness on the toughness–density chart.", &[]),
    // ---- the index library: more functions, objectives and constraints ----
    lib("shaft-strong", "Minimum mass", "Light, strong shaft (torsion)", "σy⅔/ρ", "rho", "sy", 1.5, true, "Solid shaft in torsion", "Length and torque fixed, no yielding", "Minimum mass", "Diameter",
      "The torque a shaft carries before yielding is ∝ σy·d³ ∝ σy·A^(3/2), so A ∝ σy^(−⅔) and the mass ∝ ρ/σy^⅔. Maximise σy⅔/ρ: slope 1.5 on the strength–density chart.", &[]),
    lib("vessel-light", "Minimum mass", "Light pressure vessel", "σy/ρ", "rho", "sy", 1.0, true, "Thin-walled pressure vessel", "Pressure and size fixed, no yielding", "Minimum mass", "Wall thickness",
      "The wall stress pR/t must stay below σy, so t = pR/σy and the mass of the wall ∝ R²t·ρ ∝ ρ/σy. Maximise σy/ρ: slope 1.", &[]),
    lib("flywheel", "Minimum mass", "Flywheel storing the most energy", "σy/ρ", "rho", "sy", 1.0, true, "Flywheel or spinning disc", "Must not burst", "Maximum kinetic energy per mass", "Speed",
      "The energy stored is ∝ ρω²r², and so is the largest stress. The energy per kilogram at the bursting speed is ∝ σy/ρ. Maximise σy/ρ: slope 1.", &[]),
    lib("column-crush", "Minimum mass", "Light short column (crushing)", "σc/ρ", "rho", "sc", 1.0, true, "Short column in compression", "Load fixed, no crushing", "Minimum mass", "Cross-section area",
      "A short column fails by crushing when F/A reaches σc, so A = F/σc and the mass ∝ ρ/σc. Maximise σc/ρ: slope 1 on compressive strength against density.", &[]),
    lib("plate-buckle", "Minimum mass", "Light plate in compression (buckling)", "E⅓/ρ", "rho", "E", 3.0, true, "Plate loaded in its plane", "Load fixed, no buckling", "Minimum mass", "Thickness",
      "A plate buckles at a load ∝ Et³, so t ∝ E^(−⅓) and the mass ∝ ρ/E^⅓, the same index as the stiff panel. Maximise E⅓/ρ: slope 3.", &[]),
    lib("spring-light", "Minimum mass", "Light spring", "σy²/(Eρ)", "rho", "f:Elastic energy per volume", 1.0, true, "Spring", "Store a given energy without yielding", "Minimum mass", "Size of the spring",
      "The energy per volume up to yield is σy²/2E, so the volume needed ∝ E/σy² and the mass ∝ ρE/σy². Plot σy²/E (a formula property) against density: maximise σy²/(Eρ), a line of slope 1.", &[("Elastic energy per volume", "[sy]^2/[E]", "kJ/m³")]),
    lib("diaphragm", "Elastic design", "Elastic diaphragm (largest deflection)", "σy^1.5/E", "E", "sy", 2.0 / 3.0, true, "Diaphragm or membrane", "Must deflect far without yielding", "Maximum deflection", "The material",
      "A thin diaphragm deflects with the cube of the stress over the modulus, at a fixed thickness its largest elastic deflection is ∝ σy^(3/2)/E. A line of slope ⅔ on the strength–modulus chart.", &[]),
    lib("seal", "Elastic design", "Soft elastic seal", "E (minimum)", "rho", "E", 0.0, false, "Seal or gasket", "Must conform to rough surfaces", "Maximum conformity", "The material",
      "A seal presses into the surface roughness with a contact pressure ∝ E; the softer it is, the better it seals at a given clamping force. The lowest modulus wins: a horizontal line, below passes.", &[]),
    lib("tie-stiff-cheap", "Minimum cost", "Cheap, stiff tie rod", "E/(Cₘρ)", "cv", "E", 1.0, true, "Tie rod in tension", "Length and stiffness fixed", "Minimum material cost", "Cross-section area",
      "As for the light stiff tie, with the price per volume Cₘρ in place of density. Maximise E/(Cₘρ): slope 1.", &[]),
    lib("beam-strong-cheap", "Minimum cost", "Cheap, strong beam", "σy⅔/(Cₘρ)", "cv", "sy", 1.5, true, "Beam in bending", "Length and load fixed, no yielding", "Minimum material cost", "Size of the section",
      "As for the light strong beam, with the price per volume Cₘρ in place of density. Maximise σy⅔/(Cₘρ): slope 1.5.", &[]),
    lib("panel-strong-cheap", "Minimum cost", "Cheap, strong panel", "σy½/(Cₘρ)", "cv", "sy", 2.0, true, "Panel in bending", "Length, width and load fixed, no yielding", "Minimum material cost", "Thickness",
      "As for the light strong panel, with the price per volume Cₘρ in place of density. Maximise σy½/(Cₘρ): slope 2.", &[]),
    lib("tie-stiff-carbon", "Minimum carbon", "Low-carbon stiff tie rod", "E/(CO₂·ρ)", "co2v", "E", 1.0, true, "Tie rod in tension", "Length and stiffness fixed", "Minimum CO₂ from making the material", "Cross-section area",
      "As for the light stiff tie, with the CO₂ per volume in place of density. Maximise E/(CO₂·ρ): slope 1.", &[]),
    lib("beam-strong-carbon", "Minimum carbon", "Low-carbon strong beam", "σy⅔/(CO₂·ρ)", "co2v", "sy", 1.5, true, "Beam in bending", "Length and load fixed, no yielding", "Minimum CO₂ from making the material", "Size of the section",
      "As for the light strong beam, with the CO₂ per volume in place of density. Maximise σy⅔/(CO₂·ρ): slope 1.5.", &[]),
    lib("panel-stiff-carbon", "Minimum carbon", "Low-carbon stiff panel", "E⅓/(CO₂·ρ)", "co2v", "E", 3.0, true, "Panel in bending", "Length, width and stiffness fixed", "Minimum CO₂ from making the material", "Thickness",
      "As for the light stiff panel, with the CO₂ per volume in place of density. Maximise E⅓/(CO₂·ρ): slope 3.", &[]),
    lib("beam-stiff-energy", "Minimum energy", "Low-energy stiff beam", "E½/(Hₘρ)", "eev", "E", 2.0, true, "Beam in bending", "Length and bending stiffness fixed", "Minimum energy to make the material", "Size of the section",
      "The embodied energy is mass × energy per kilogram, so ρ becomes Hₘρ, the energy per volume. Maximise E½/(Hₘρ): slope 2.", &[]),
    lib("tie-strong-energy", "Minimum energy", "Low-energy strong tie rod", "σy/(Hₘρ)", "eev", "sy", 1.0, true, "Tie rod in tension", "Length and load fixed, no yielding", "Minimum energy to make the material", "Cross-section area",
      "As for the light strong tie, with the energy per volume Hₘρ in place of density. Maximise σy/(Hₘρ): slope 1.", &[]),
    lib("fatigue-light", "Damage tolerance", "Light part under cyclic load", "σe/ρ", "rho", "fat", 1.0, true, "Tie or link under repeated load", "Must survive 10⁷ cycles", "Minimum mass", "Cross-section area",
      "Under a load that repeats many times the stress must stay below the fatigue strength σe, so A = F/σe and the mass ∝ ρ/σe. Maximise σe/ρ: slope 1.", &[]),
    lib("heat-exchanger", "Thermal", "Heat-exchanger tube", "σy·λ", "sy", "lam", -1.0, true, "Tube carrying a hot fluid under pressure", "Must hold the pressure", "Maximum heat flow", "Wall thickness",
      "The heat flow through the wall is ∝ λ/t and the pressure sets t ∝ 1/σy, so the heat flow ∝ λσy. On conductivity against strength the index is a line of slope −1, above passes.", &[]),
    lib("diffusivity-fast", "Thermal", "Heats through quickly", "a (maximum)", "rho", "diff", 0.0, true, "Part that must reach temperature fast", "Shape fixed", "Shortest heating time", "The material",
      "The time for heat to soak through a thickness x is ∝ x²/a, where a = λ/(ρcₚ) is the thermal diffusivity. The highest diffusivity heats through fastest: a horizontal line, above passes.", &[]),
    lib("diffusivity-slow", "Thermal", "Heats through slowly", "a (minimum)", "rho", "diff", 0.0, false, "Wall that must keep heat out for a while", "Thickness fixed", "Longest time before the far side warms", "The material",
      "The time for heat to soak through is ∝ x²/a, so the lowest thermal diffusivity keeps heat out longest: a horizontal line, below passes.", &[]),
    lib("heat-storage", "Thermal", "Stores the most heat per volume", "ρcₚ (maximum)", "rho", "f:Heat capacity per volume", 0.0, true, "Thermal store", "Volume fixed", "Maximum heat stored per degree", "The material",
      "The heat stored per degree in a fixed volume is ρcₚ, the heat capacity per volume (a formula property). The highest wins: a horizontal line, above passes.", &[("Heat capacity per volume", "[rho]*[cp]/1000", "kJ/(m³·K)")]),
    lib("conductor-light", "Electrical", "Light electrical conductor", "ρe·ρ (minimum)", "rho", "res", -1.0, false, "Cable or busbar", "Length and resistance fixed", "Minimum mass", "Cross-section area",
      "The resistance is ρeL/A, so A ∝ ρe and the mass ∝ ρeρ. Minimise ρe·ρ: a line of slope −1 on resistivity against density, below passes.", &[]),
];

pub fn get(id: &str) -> Option<&'static Goal> {
    GOALS.iter().find(|g| g.id == id)
}

/// The line of `slope` on axes (x, y) placed through the median x so the best fifth of the
/// materials pass (or the lowest fifth, for a line "below").
fn place(db: &Db, x: &str, y: &str, lx: bool, ly: bool, slope: f64, above: bool) -> Option<ChartSel> {
    let at = |m: &Material, k: &str, log: bool| { let (lo, hi) = m.props.get(k)?.range()?; span(lo, hi, log).map(|s| s.0) };
    let pts: Vec<(f64, f64)> = db.materials.iter().filter_map(|m| Some((at(m, x, lx)?, at(m, y, ly)?))).collect();
    if pts.is_empty() { return None; }
    let mut xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
    xs.sort_by(f64::total_cmp);
    let mut ks: Vec<f64> = pts.iter().map(|p| p.1 - slope * p.0).collect();
    ks.sort_by(f64::total_cmp);
    let q = ks[((ks.len() as f64 * if above { 0.8 } else { 0.2 }) as usize).min(ks.len() - 1)];
    let c = xs[xs.len() / 2];
    let v = |u: f64, log: bool| if log { 10f64.powf(u) } else { u };
    Some(ChartSel::Line { slope, x: v(c, lx), y: v(q + slope * c, ly), above })
}

fn log_of(k: &str) -> bool { prop(k).is_none_or(|p| p.log) }

/// The edits that apply a goal to a chart ("" = explore: no line, the goal's limits removed). A
/// goal with two constraints also adds its second chart; the second chart of the chart's old goal
/// is removed.
pub fn apply(db: &Db, p: &Project, chart: u32, id: &str) -> Result<Op, String> {
    let mut c = p.chart(chart).ok_or("That chart no longer exists.")?.clone();
    let mut st = p.stages.clone();
    st.limits.retain(|l| !l.goal);
    let g = get(id);
    if g.is_none() && !id.is_empty() { return Err(format!("There is no design goal “{id}”.")); }
    let old = format!("{}/2", c.goal.trim_end_matches("/2"));
    let mut ops: Vec<Op> = p.charts.iter().filter(|x| x.id != chart && !c.goal.is_empty() && x.goal == old).map(|x| Op::DelChart { id: x.id }).collect();
    let Some(g) = g else {
        c.goal.clear();
        c.sels.clear();
        ops.insert(0, Op::SetChart { chart: Box::new(c) });
        ops.push(Op::SetStages { stages: st });
        return Ok(Op::Batch(ops));
    };
    // Formula properties the index needs are added to the project (kept if already there).
    let mut fs = p.formulas.clone();
    for (name, expr, unit) in g.formulas {
        if !fs.iter().any(|f| f.name == *name) { fs.push(Formula { name: (*name).into(), expr: (*expr).into(), unit: (*unit).into(), log: true }); }
    }
    let fdb = db.with_formulas(&fs);
    let db = &fdb;
    if fs != p.formulas { ops.insert(0, Op::SetFormulas { formulas: fs }); }
    let (lx, ly) = (log_of(g.x), log_of(g.y));
    c.kind = "bubble".into();
    (c.x, c.y, c.log_x, c.log_y) = (g.x.into(), g.y.into(), lx, ly);
    c.goal = g.id.into();
    c.name = g.name.into();
    c.sels = vec![place(db, g.x, g.y, lx, ly, g.slope, g.above).ok_or("No material has both values for this goal.")?];
    let at = ops.iter().position(|o| !matches!(o, Op::SetFormulas { .. })).unwrap_or(ops.len());
    ops.insert(at, Op::SetChart { chart: Box::new(c) });
    if let Some(a) = &g.also {
        let (ax, ay) = (log_of(a.x), log_of(a.y));
        let sel = place(db, a.x, a.y, ax, ay, a.slope, a.above).ok_or("No material has both values for this goal.")?;
        ops.push(Op::AddChart { chart: Box::new(Chart { x: a.x.into(), y: a.y.into(), log_x: ax, log_y: ay, goal: format!("{}/2", g.id), name: a.name.into(), sels: vec![sel], guides: vec![1.0, 2.0, 3.0], ..Default::default() }) });
    }
    for (k, min, max) in g.limits {
        st.limits.push(Limit { prop: (*k).into(), min: *min, max: *max, goal: true, ..Default::default() });
    }
    (st.rank_chart, st.rank_line, st.ranking) = (chart, 0, "line".into());
    ops.push(Op::SetStages { stages: st });
    Ok(Op::Batch(ops))
}
