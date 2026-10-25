//! Starting points on the File page: a blank selection and design problems. Each is a design goal
//! (goals.rs) applied to a new selection, sometimes with a limit of its own.
use crate::db::Db;
use crate::doc::Doc;
use crate::goals;
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

/// (id, goal, name, description)
const LIST: &[(&str, &str, &str, &str)] = &[
    ("blank", "", "Blank selection", "Young's modulus against density, every material, no stages yet."),
    ("beam", "beam-stiff", "Light, stiff beam", "Index E½/ρ: a line of slope 2 on the modulus–density chart."),
    ("tie", "tie-strong", "Light, strong tie", "Index σy/ρ: a line of slope 1 on the strength–density chart."),
    ("panel", "panel-stiff", "Light, stiff panel", "Index E⅓/ρ: a line of slope 3 on the modulus–density chart."),
    ("spring", "spring", "Springs", "Index σy²/E: most elastic energy per volume, a line of slope ½."),
    ("hot", "beam-stiff", "Hot, stiff and light", "Max service temperature at least 200 °C, then E½/ρ."),
    ("tough", "leak-first", "Leak before break", "Index KIc²/σy for a pressure vessel, on toughness against strength."),
];

pub fn list() -> Vec<Tpl> {
    LIST.iter().map(|(id, g, name, desc)| Tpl { id, name, desc, slope: goals::get(g).map(|g| g.slope) }).collect()
}

/// A new project from a template ("blank" or None for the blank one).
pub fn build(db: &Db, id: Option<&str>) -> Option<Project> {
    let id = id.unwrap_or("blank");
    let (_, goal, name, desc) = LIST.iter().find(|t| t.0 == id)?;
    let mut p = Project::new(if id == "blank" { "Selection1" } else { name });
    let cid = p.add_chart(Chart { guides: vec![1.0, 2.0, 3.0], ..Default::default() });
    if id != "blank" { p.notes = desc.to_string(); }
    let mut d = Doc::new(p);
    if !goal.is_empty() { d.apply(goals::apply(db, &d.p, cid, goal).ok()?).ok()?; }
    if id == "hot" { let mut st = d.p.stages.clone(); st.limits.push(Limit { prop: "tmax".into(), min: Some(200.0), ..Default::default() }); d.apply(crate::doc::Op::SetStages { stages: st }).ok()?; }
    let mut p = d.p;
    p.charts.iter_mut().for_each(|c| if c.x != "rho" || c.y != "E" { c.guides.clear(); });
    Some(p)
}
