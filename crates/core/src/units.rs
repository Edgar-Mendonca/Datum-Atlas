//! Units: every value is stored in one SI-based unit per property (see `PROPS`); the imperial
//! set converts exactly (`to`/`from` are inverses, temperatures are affine). Formatting with
//! significant figures for tables, datasheets and tooltips.

/// The imperial unit for a stored unit, with factor and offset: shown = stored × k + c.
pub fn imperial(unit: &str) -> Option<(&'static str, f64, f64)> {
    Some(match unit {
        "kg/m³" => ("lb/in³", 1.0 / 27_679.904_7, 0.0),
        "GPa" => ("10⁶ psi", 1.0 / 6.894_757_29, 0.0),
        "MPa" => ("ksi", 1.0 / 6.894_757_29, 0.0),
        "MPa·m½" => ("ksi·in½", 1.0 / 1.098_843, 0.0),
        "°C" => ("°F", 1.8, 32.0),
        "W/(m·K)" => ("BTU/(h·ft·°F)", 1.0 / 1.730_734_6, 0.0),
        "µstrain/°C" => ("µstrain/°F", 1.0 / 1.8, 0.0),
        "J/(kg·K)" => ("BTU/(lb·°F)", 1.0 / 4_186.8, 0.0),
        "USD/kg" => ("USD/lb", 0.453_592_37, 0.0),
        "MJ/kg" => ("BTU/lb", 429.922_614, 0.0),
        "kg/kg" => ("lb/lb", 1.0, 0.0),
        "µΩ·cm" => ("µΩ·cm", 1.0, 0.0),
        "USD/m³" => ("USD/ft³", 0.028_316_846_6, 0.0),
        "kg CO₂/m³" => ("lb CO₂/ft³", 0.062_427_96, 0.0),
        "MJ/m³" => ("BTU/ft³", 26.839_19, 0.0),
        _ => return None,
    })
}

/// Stored value → value in `system` ("si" or "imperial"), with the unit shown.
pub fn to(v: f64, unit: &'static str, system: &str) -> (f64, &'static str) {
    match (system, imperial(unit)) {
        ("imperial", Some((u, k, c))) => (v * k + c, u),
        _ => (v, unit),
    }
}

/// A value typed in `system` → stored unit.
pub fn from(v: f64, unit: &str, system: &str) -> f64 {
    match (system, imperial(unit)) {
        ("imperial", Some((_, k, c))) => (v - c) / k,
        _ => v,
    }
}

/// Three significant figures, without trailing zeros; scientific for very small or large values.
pub fn fmt(v: f64) -> String {
    if !v.is_finite() {
        return "–".into();
    }
    if v == 0.0 {
        return "0".into();
    }
    let a = v.abs();
    if !(1e-3..1e6).contains(&a) {
        let e = a.log10().floor() as i32;
        let m = v / 10f64.powi(e);
        return format!("{}e{e}", trim(&format!("{m:.2}")));
    }
    let d = (2 - a.log10().floor() as i32).max(0) as usize;
    trim(&format!("{v:.d$}"))
}

fn trim(s: &str) -> String {
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').into() } else { s.into() }
}

/// "2690 – 2710" (or one value when low = high).
pub fn range(lo: f64, hi: f64) -> String {
    if fmt(lo) == fmt(hi) { fmt(lo) } else { format!("{} – {}", fmt(lo), fmt(hi)) }
}
