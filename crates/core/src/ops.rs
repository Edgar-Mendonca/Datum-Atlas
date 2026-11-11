//! The named operations: every change to a selection project is one of these, whether it comes
//! from the window, the command line or an AI assistant. Each has a name, what it does, and an
//! example (the JSON to send). Charts are named by their id (`project_info` lists them).
use crate::db::Db;
use crate::doc::{Doc, Op};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize, Debug)]
pub struct OpInfo { pub name: &'static str, pub about: &'static str, pub example: Value }

pub fn catalog() -> Vec<OpInfo> {
    let o = |name, about, example| OpInfo { name, about, example };
    vec![
        o("Goal", "Apply a design goal to a chart (see the goals list): its axes, its selection line through the best materials, its limits and its ranking. \"\" goes back to exploring.", json!({"Goal": {"chart": 1, "goal": "beam-stiff"}})),
        o("AddChart", "Add a property chart: x and y are property keys (see the properties list), log scales, envelopes by \"family\", \"class\" or \"none\"; kind \"bubble\" or \"bar\".", json!({"AddChart": {"chart": {"x": "rho", "y": "sy", "logX": true, "logY": true, "hull": "family"}}})),
        o("SetChart", "Replace a chart's settings, keeping its id: axes, scales, envelopes, labels, families hidden, guide slopes, style.", json!({"SetChart": {"chart": {"id": 1, "name": "", "x": "rho", "y": "E", "logX": true, "logY": true, "hull": "class", "guides": [1, 2, 3]}}})),
        o("DelChart", "Delete a chart.", json!({"DelChart": {"id": 2}})),
        o("Select", "Replace a chart's selections: lines (slope, a point x, y in stored units, above passes or below), boxes and polygons. A material passes the chart when it passes all of them; [] removes them.", json!({"Select": {"chart": 1, "sels": [{"Line": {"slope": 2, "x": 1000, "y": 10, "above": true}}, {"Box": {"x0": 1000, "x1": 3000, "y0": 50, "y1": 250}}]}})),
        o("PassThrough", "Move selection line k of a chart, keeping its slope, so its index has this value.", json!({"PassThrough": {"chart": 1, "k": 0, "value": 0.004}})),
        o("SetStages", "Replace the selection stages: limits (min, max, a yes/no flag or a lowest rating), families left out, the rule for ranges (\"mean\" or \"all\"), missing values (\"fail\" or \"flag\"), and the ranking (\"line\", or \"weighted\" with weights).", json!({"SetStages": {"stages": {"limits": [{"prop": "tmax", "min": 150}, {"prop": "price", "max": 5}], "exclude": ["Foams"], "rule": "mean", "missing": "fail", "ranking": "line"}}})),
        o("SetFormulas", "Replace your formula properties: a name, a formula of other properties in [brackets], its unit and whether it is plotted on a log axis. Use one as \"f:name\".", json!({"SetFormulas": {"formulas": [{"name": "Spring index", "expr": "[sy]^2/[E]", "unit": "MPa", "log": true}]}})),
        o("Compare", "The materials to compare (up to six ids) and the radar chart's properties ([] for the default set).", json!({"Compare": {"ids": ["al-6061-t6", "steel-1020"], "radar": []}})),
        o("Eco", "Replace the eco audit: design options with parts (material id, mass kg, number, process, removed fraction, recycled %, end of life landfill/burn/downcycle/recycle/none), transport legs, use (none, static: kW, hours, days; mobile: vehicle, km a year) and years; the electricity grid.", json!({"Eco": {"eco": {"grid": "eu", "options": [{"name": "Steel", "years": 10, "parts": [{"name": "Bracket", "material": "steel-1020", "mass": 2.5, "qty": 4, "process": "deform", "removed": 0.1, "recycled": 30, "eol": "recycle"}], "legs": [{"mode": "truck", "km": 800}], "use": {"kind": "mobile", "vehicle": "car-petrol", "km": 15000}}]}}})),
        o("Process", "Replace the part for process selection: material id, mass kg, thinnest section mm, shape (flat, sheet, prismatic, 3d-solid, 3d-hollow or \"\"), tolerance mm and roughness µm needed (0: any), parts in the batch, material price (0: its own), a second material to join and what a finish is for.", json!({"Process": {"sel": {"material": "al-a356-t6", "mass": 2, "section": 4, "shape": "3d-hollow", "tol": 0.5, "rough": 0, "batch": 20000, "price": 0, "join": "", "purpose": "corrosion"}}})),
        o("Tensile", "Replace the tensile tests: specimens (name; x: ext mm, strain or strain%; y: N, kN or MPa; shape round with d, rect with w and t, or area; gauge length l0 mm; on; data as [x, y] pairs) and the modulus window as fractions of the tensile strength.", json!({"Tensile": {"tensile": {"specimens": [{"name": "S1", "x": "strain%", "y": "MPa", "shape": "area", "area": 1, "on": true, "data": [[0, 0], [0.05, 100], [0.1, 200], [0.15, 300], [0.3, 380], [1, 420], [5, 480], [10, 500], [15, 470], [18, 300], [18.1, 0]]}], "fit": [0.1, 0.4], "material": ""}}})),
        o("Lab", "Replace the hardness, fatigue and creep tests: hardness sets (name, scale HV, HB, HK, HRC, HRB, HR15N, Shore A or Shore D, readings), fatigue points (stress amplitude MPa, cycles, runout), the cycles for the fatigue strength, creep points (stress MPa, temperature °C, hours to rupture), the Larson–Miller constant (0 to find it) and a creep question [°C, hours].", json!({"Lab": {"lab": {"hardness": [{"name": "Plate A", "scale": "HV", "values": [212, 218, 209, 221, 215]}], "fatigue": [{"s": 400, "n": 30000}, {"s": 350, "n": 150000}, {"s": 300, "n": 900000}, {"s": 260, "n": 10000000, "runout": true}], "fat_at": 10000000, "creep": [], "lmp_c": 20, "creep_at": [600, 100000]}}})),
        o("Name", "Rename the project.", json!({"Name": {"name": "Pump bracket"}})),
        o("Notes", "Set the requirements text (what the part must do), shown in the report.", json!({"Notes": {"text": "Bracket in bending; stiffness fixed; minimise mass; below 120 °C."}})),
        o("Batch", "Several operations as one edit (one undo); if one fails, none is applied.", json!({"Batch": [{"Name": {"name": "Bracket"}}, {"Notes": {"text": "Light and stiff."}}]})),
    ]
}

/// Applies an operation to a project, turning a `Goal` into the edits it makes first (a goal needs
/// the database to place its line). A failed operation changes nothing.
pub fn apply(doc: &mut Doc, db: &Db, op: Op) -> Result<(), String> {
    let op = match op { Op::Goal { chart, goal } => crate::goals::apply(db, &doc.p, chart, &goal)?, o => o };
    doc.apply(op)
}

#[cfg(test)]
mod tests {
    use crate::{db::{self, Db}, doc::{Doc, Op}, templates};

    #[test]
    fn every_example_is_a_working_operation() {
        let d = Db::new(db::builtin(), None);
        let names: Vec<&str> = super::catalog().iter().map(|o| o.name).collect();
        let mut dedup = names.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(dedup.len(), names.len(), "names are unique");
        for info in super::catalog() {
            let op: Op = serde_json::from_value(info.example.clone()).unwrap_or_else(|e| panic!("{}: {e}", info.name));
            let mut doc = Doc::new(templates::build(&d, Some("beam")).unwrap_or_default());
            if info.name == "DelChart" { let _ = doc.apply(Op::AddChart { chart: Box::default() }); }
            super::apply(&mut doc, &d, op).unwrap_or_else(|e| panic!("{}: {e}", info.name));
            assert!(doc.dirty || info.name == "Goal", "{} changed nothing", info.name);
        }
    }
}
