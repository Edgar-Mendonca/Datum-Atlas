//! Starting points on the File page: a blank selection and classic design problems, each with
//! its chart, index line and limits already set.
use crate::model::*;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tpl {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    /// Slope of the index line drawn on the tile (None: no line).
    pub slope: Option<f64>,
}

const LIST: &[(&str, &str, &str)] = &[
    ("blank", "Blank selection", "Young's modulus against density, every material, no stages yet."),
    ("beam", "Light, stiff beam", "Index E^½/ρ: a line of slope 2 on the modulus–density chart."),
    ("tie", "Light, strong tie", "Index σy/ρ: a line of slope 1 on the strength–density chart."),
    ("panel", "Light, stiff panel", "Index E^⅓/ρ: a line of slope 3 on the modulus–density chart."),
    ("spring", "Springs", "Index σy²/E: most elastic energy per volume, a line of slope ½."),
    ("hot", "Hot, stiff and light", "Max service temperature at least 200 °C, then E^½/ρ."),
    ("tough", "Damage tolerant", "Fracture toughness against yield strength: tough and strong."),
];

pub fn list() -> Vec<Tpl> {
    let slope = |id: &str| match id { "beam" | "hot" => Some(2.0), "tie" => Some(1.0), "panel" => Some(3.0), "spring" => Some(0.5), _ => None };
    LIST.iter().map(|(id, name, desc)| Tpl { id, name, desc, slope: slope(id) }).collect()
}

fn chart(x: &str, y: &str, select: Option<ChartSel>) -> Chart {
    Chart { x: x.into(), y: y.into(), guides: if x == "rho" && y == "E" { vec![1.0, 2.0, 3.0] } else { vec![] }, select, ..Default::default() }
}
fn line(slope: f64, x: f64, y: f64) -> Option<ChartSel> {
    Some(ChartSel::Line { slope, x, y, above: true })
}

/// A new project from a template ("blank" or None for the blank one).
pub fn build(id: Option<&str>) -> Option<Project> {
    let id = id.unwrap_or("blank");
    let name = LIST.iter().find(|t| t.0 == id)?.1;
    let mut p = Project::new(if id == "blank" { "Selection1" } else { name });
    let c = match id {
        "beam" => chart("rho", "E", line(2.0, 1000.0, 10.0)),
        "tie" => chart("rho", "sy", line(1.0, 1000.0, 100.0)),
        "panel" => chart("rho", "E", line(3.0, 1000.0, 1.5)),
        "spring" => chart("E", "sy", line(0.5, 10.0, 100.0)),
        "hot" => { p.stages.limits.push(Limit { prop: "tmax".into(), min: Some(200.0), ..Default::default() }); chart("rho", "E", line(2.0, 1000.0, 10.0)) }
        "tough" => chart("sy", "kic", None),
        _ => chart("rho", "E", None),
    };
    p.add_chart(c);
    p.notes = match id {
        "blank" => String::new(),
        _ => LIST.iter().find(|t| t.0 == id).map(|t| t.2.to_string()).unwrap_or_default(),
    };
    Some(p)
}
