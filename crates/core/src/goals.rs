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
}

const TOUGH: &[(&str, Option<f64>, Option<f64>)] = &[("kic", Some(1.0), None)];
const STRONG: &[(&str, Option<f64>, Option<f64>)] = &[("kic", Some(15.0), None)];

#[allow(clippy::too_many_arguments)]
const fn g(id: &'static str, group: &'static str, name: &'static str, index: &'static str, x: &'static str, y: &'static str, slope: f64, above: bool,
    function: &'static str, constraint: &'static str, objective: &'static str, free: &'static str, why: &'static str, limits: &'static [(&'static str, Option<f64>, Option<f64>)]) -> Goal {
    Goal { id, group, name, index, x, y, slope, above, function, constraint, objective, free, why, limits }
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
];

pub fn get(id: &str) -> Option<&'static Goal> {
    GOALS.iter().find(|g| g.id == id)
}

/// The edits that apply a goal to a chart ("" = explore: no line, the goal's limits removed).
pub fn apply(db: &Db, p: &Project, chart: u32, id: &str) -> Result<Op, String> {
    let mut c = p.chart(chart).ok_or("That chart no longer exists.")?.clone();
    let mut st = p.stages.clone();
    st.limits.retain(|l| !l.goal);
    let Some(g) = get(id) else {
        if !id.is_empty() { return Err(format!("There is no design goal “{id}”.")); }
        c.goal.clear();
        c.select = None;
        return Ok(Op::Batch(vec![Op::SetChart { chart: Box::new(c) }, Op::SetStages { stages: st }]));
    };
    let (lx, ly) = (prop(g.x).is_none_or(|p| p.log), prop(g.y).is_none_or(|p| p.log));
    c.kind = "bubble".into();
    c.x = g.x.into();
    c.y = g.y.into();
    c.log_x = lx;
    c.log_y = ly;
    c.goal = g.id.into();
    c.name = g.name.into();
    // The line passes through the middle of the materials, placed so the best fifth pass.
    let pts: Vec<(f64, f64)> = db.materials.iter().filter_map(|m| Some((span(m.props.get(g.x)?.range()?.0, m.props.get(g.x)?.range()?.1, lx)?.0, span(m.props.get(g.y)?.range()?.0, m.props.get(g.y)?.range()?.1, ly)?.0))).collect();
    if pts.is_empty() { return Err("No material has both values for this goal.".into()); }
    let mut xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
    xs.sort_by(f64::total_cmp);
    let mut ks: Vec<f64> = pts.iter().map(|p| p.1 - g.slope * p.0).collect();
    ks.sort_by(f64::total_cmp);
    let q = ks[((ks.len() as f64 * if g.above { 0.8 } else { 0.2 }) as usize).min(ks.len() - 1)];
    let x = xs[xs.len() / 2];
    let v = |u: f64, log: bool| if log { 10f64.powf(u) } else { u };
    c.select = Some(ChartSel::Line { slope: g.slope, x: v(x, lx), y: v(q + g.slope * x, ly), above: g.above });
    for (k, min, max) in g.limits {
        st.limits.push(Limit { prop: (*k).into(), min: *min, max: *max, goal: true, ..Default::default() });
    }
    st.rank_chart = chart;
    Ok(Op::Batch(vec![Op::SetChart { chart: Box::new(c) }, Op::SetStages { stages: st }]))
}
