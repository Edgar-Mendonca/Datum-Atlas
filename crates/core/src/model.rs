//! Materials, properties, sources and databases (`data/*.json`, `.datdb`), and the selection
//! project (`.datlas`): property charts and selection stages.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FORMAT: u32 = 1;

/// One property value: a range (low, high, source; optionally a typical value), a yes/no flag
/// or a rating, each with its source id. Stored as short arrays: `[2690, 2710, "mil5j"]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Val {
    RangeT(f64, f64, String, f64),
    Range(f64, f64, String),
    Flag(bool, String),
    Rating(String, String),
}

impl Val {
    pub fn range(&self) -> Option<(f64, f64)> {
        match self { Val::Range(a, b, _) | Val::RangeT(a, b, _, _) => Some((*a, *b)), _ => None }
    }
    pub fn src(&self) -> &str {
        match self { Val::Range(_, _, s) | Val::RangeT(_, _, s, _) | Val::Flag(_, s) | Val::Rating(_, s) => s }
    }
    pub fn flag(&self) -> Option<bool> {
        match self { Val::Flag(b, _) => Some(*b), _ => None }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Material {
    pub id: String,
    pub name: String,
    /// Family, class, subclass…: ["Metals", "Non-ferrous", "Aluminium alloys", "Wrought"].
    pub family: Vec<String>,
    pub syn: Vec<String>,
    /// Standard designations by system: UNS, EN, ASTM, ISO, JIS.
    pub des: BTreeMap<String, String>,
    pub props: BTreeMap<String, Val>,
    pub notes: String,
    pub uses: String,
    /// "draft" until every value has been checked against its source, then "checked".
    pub status: String,
    /// Set when loaded from the user's own database (not saved).
    #[serde(skip_deserializing)]
    pub user: bool,
}

/// Where values come from. `kind`: handbook, government, open-dataset, datasheet, textbook-typical,
/// estimate, user.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub title: String,
    pub author: String,
    pub publisher: String,
    pub year: String,
    pub url: String,
    pub licence: String,
    pub kind: String,
    pub note: String,
}

/// A database file: the built-in data (`data/*.json`) or a user's `.datdb`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Database {
    pub format: u32,
    pub name: String,
    pub sources: Vec<Source>,
    pub materials: Vec<Material>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    pub key: &'static str,
    pub name: &'static str,
    pub sym: &'static str,
    pub group: &'static str,
    pub unit: &'static str,
    /// range, flag or rating.
    pub kind: &'static str,
    /// Charts use a log axis by default.
    pub log: bool,
    pub about: &'static str,
}

const fn p(key: &'static str, name: &'static str, sym: &'static str, group: &'static str, unit: &'static str, log: bool, about: &'static str) -> Property {
    Property { key, name, sym, group, unit, kind: "range", log, about }
}
const fn f(key: &'static str, name: &'static str, group: &'static str, about: &'static str) -> Property {
    Property { key, name, sym: "", group, unit: "", kind: "flag", log: false, about }
}
const fn r(key: &'static str, name: &'static str, group: &'static str, about: &'static str) -> Property {
    Property { key, name, sym: "", group, unit: "", kind: "rating", log: false, about }
}

/// The property list, in datasheet order. Values are stored in these units (SI-based).
pub const PROPS: &[Property] = &[
    p("rho", "Density", "ρ", "General", "kg/m³", true, "Mass per unit volume."),
    p("price", "Price", "Cm", "General", "USD/kg", true, "Typical bulk price per kilogram (the year is in the source)."),
    p("E", "Young's modulus", "E", "Mechanical", "GPa", true, "Stiffness in tension: stress divided by elastic strain."),
    p("G", "Shear modulus", "G", "Mechanical", "GPa", true, "Stiffness in shear."),
    p("nu", "Poisson's ratio", "ν", "Mechanical", "", false, "Lateral contraction divided by axial strain."),
    p("sy", "Yield strength (elastic limit)", "σy", "Mechanical", "MPa", true, "Stress at which permanent deformation starts (0.2 % proof stress for metals)."),
    p("ts", "Tensile strength", "σts", "Mechanical", "MPa", true, "Highest nominal stress in a tensile test (flexural strength for brittle materials, see the notes)."),
    p("sc", "Compressive strength", "σc", "Mechanical", "MPa", true, "Failure stress in compression; the strength that matters for ceramics, stone and concrete."),
    p("elong", "Elongation", "ε", "Mechanical", "% strain", true, "Strain at fracture in a tensile test: a measure of ductility."),
    p("hv", "Hardness (Vickers)", "H", "Mechanical", "HV", true, "Resistance to indentation."),
    p("fat", "Fatigue strength at 10⁷ cycles", "σe", "Mechanical", "MPa", true, "Stress amplitude a part survives for ten million cycles."),
    p("kic", "Fracture toughness", "KIc", "Mechanical", "MPa·m½", true, "Resistance to the growth of a crack."),
    p("eta", "Loss coefficient", "η", "Mechanical", "", true, "Fraction of vibration energy lost per cycle (damping)."),
    p("tm", "Melting point", "Tm", "Thermal", "°C", false, "Melting temperature (or the solidus for alloys)."),
    p("tg", "Glass temperature", "Tg", "Thermal", "°C", false, "Where an amorphous polymer or glass turns from glassy to rubbery."),
    p("tmax", "Max service temperature", "Tmax", "Thermal", "°C", false, "Highest temperature for long use without unacceptable loss of properties."),
    p("tmin", "Min service temperature", "Tmin", "Thermal", "°C", false, "Lowest temperature for use without becoming brittle."),
    p("lam", "Thermal conductivity", "λ", "Thermal", "W/(m·K)", true, "Rate of heat flow through the material."),
    p("alpha", "Thermal expansion", "α", "Thermal", "µstrain/°C", true, "Strain per degree of temperature change (10⁻⁶/°C)."),
    p("cp", "Specific heat", "cp", "Thermal", "J/(kg·K)", true, "Energy to heat one kilogram by one degree."),
    p("res", "Electrical resistivity", "ρe", "Electrical", "µΩ·cm", true, "Resistance to electrical current."),
    p("eps", "Dielectric constant", "εr", "Electrical", "", true, "Relative permittivity."),
    r("transp", "Transparency", "Optical", "Opaque, translucent, transparent or optical quality."),
    r("water", "Durability: fresh water", "Durability", "Resistance to fresh water."),
    r("acids", "Durability: weak acids", "Durability", "Resistance to weak acids."),
    r("alkalis", "Durability: weak alkalis", "Durability", "Resistance to weak alkalis."),
    r("uv", "Durability: UV light", "Durability", "Resistance to sunlight and UV."),
    r("flam", "Flammability", "Durability", "How readily it burns."),
    r("cast", "Castability", "Processing", "How well it casts."),
    r("mach", "Machinability", "Processing", "How easily it is machined."),
    r("weld", "Weldability", "Processing", "How well it welds."),
    p("co2", "CO₂ footprint, primary production", "CO₂", "Eco", "kg/kg", true, "Carbon dioxide (equivalent) released to make one kilogram from raw materials."),
    p("ee", "Embodied energy, primary production", "Hm", "Eco", "MJ/kg", true, "Energy used to make one kilogram from raw materials."),
    f("recyclable", "Recyclable", "Eco", "Can be recycled into the same kind of material."),
];

pub const FAMILIES: &[&str] = &["Metals", "Polymers", "Elastomers", "Ceramics and glasses", "Composites", "Foams", "Natural materials"];
pub const RATINGS: &[&str] = &["Unacceptable", "Poor", "Acceptable", "Good", "Excellent"];

pub fn prop(key: &str) -> Option<&'static Property> {
    PROPS.iter().find(|p| p.key == key)
}

// ---------------- the selection project (.datlas) ----------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Project {
    pub format: u32,
    pub name: String,
    /// What the part must do, for the report.
    pub notes: String,
    pub charts: Vec<Chart>,
    pub stages: Stages,
    pub next_id: u32,
    /// Window settings saved with the file (page setup, open chart…).
    pub ui: serde_json::Value,
}

impl Default for Project {
    fn default() -> Self {
        Self { format: FORMAT, name: "Selection1".into(), notes: String::new(), charts: vec![], stages: Stages::default(), next_id: 1, ui: serde_json::json!({}) }
    }
}

/// A property chart: two properties, their scales, and an optional selection on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Chart {
    pub id: u32,
    pub name: String,
    pub x: String,
    pub y: String,
    pub log_x: bool,
    pub log_y: bool,
    /// Envelopes around "family" or "class", or "none".
    pub hull: String,
    pub labels: bool,
    /// Families hidden on this chart.
    pub hidden: Vec<String>,
    /// Slopes of the guide lines shown (e.g. 1, 2, 3).
    pub guides: Vec<f64>,
    pub select: Option<ChartSel>,
}

impl Default for Chart {
    fn default() -> Self {
        Self { id: 0, name: String::new(), x: "rho".into(), y: "E".into(), log_x: true, log_y: true, hull: "family".into(), labels: false, hidden: vec![], guides: vec![], select: None }
    }
}

/// A chart stage: materials above (or below) a line of slope `slope` through (x, y), or inside a box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChartSel {
    Line { slope: f64, x: f64, y: f64, above: bool },
    Box { x0: f64, x1: f64, y0: f64, y1: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Stages {
    pub limits: Vec<Limit>,
    /// Tree stage: family paths left out ("Metals" or "Metals › Ferrous").
    pub exclude: Vec<String>,
    /// "mean": a range passes on its mean (geometric for log properties); "all": the whole range must pass.
    pub rule: String,
    /// "fail": materials without the value fail; "flag": they pass, marked "no data".
    pub missing: String,
    /// The chart whose index line ranks the results (0: the first chart with a line).
    pub rank_chart: u32,
}

impl Default for Stages {
    fn default() -> Self {
        Self { limits: vec![], exclude: vec![], rule: "mean".into(), missing: "fail".into(), rank_chart: 0 }
    }
}

/// A limit: min and/or max for a range property, or a required yes/no, or a lowest rating.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Limit {
    pub prop: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub flag: Option<bool>,
    pub rating: Option<String>,
}

impl Project {
    pub fn new(name: &str) -> Self {
        Self { name: name.into(), ..Default::default() }
    }
    pub fn chart(&self, id: u32) -> Option<&Chart> {
        self.charts.iter().find(|c| c.id == id)
    }
    pub fn take_id(&mut self) -> u32 {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        id
    }
    pub fn add_chart(&mut self, mut c: Chart) -> u32 {
        c.id = self.take_id();
        if c.name.is_empty() {
            c.name = chart_name(&c);
        }
        let id = c.id;
        self.charts.push(c);
        id
    }
    pub fn normalize(&mut self) {
        let top = self.charts.iter().map(|c| c.id).max().unwrap_or(0);
        self.next_id = self.next_id.max(top + 1);
        self.stages.limits.retain(|l| prop(&l.prop).is_some());
        for c in &mut self.charts {
            if prop(&c.x).is_none() { c.x = "rho".into(); }
            if prop(&c.y).is_none() { c.y = "E".into(); }
            c.guides.retain(|s| s.is_finite());
        }
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut p: Project = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if p.format > FORMAT {
            return Err("it was saved by a newer Datum Atlas; update the app to open it".into());
        }
        p.format = FORMAT;
        p.normalize();
        Ok(p)
    }
}

/// "Young's modulus vs Density".
pub fn chart_name(c: &Chart) -> String {
    format!("{} vs {}", prop(&c.y).map_or("?", |p| p.name), prop(&c.x).map_or("?", |p| p.name))
}
