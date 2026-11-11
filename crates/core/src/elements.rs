//! The elements: the 118 elements with their main properties (`data/elements.json`, compiled from
//! public sources; gaps where no reliable value is known), and the composition of a chemical formula
//! or an alloy given in weight or atom percent: molar mass, wt % and at %.
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    pub z: u32,
    pub symbol: String,
    pub name: String,
    /// g/mol; for a radioactive element without a standard weight, its longest-lived isotope.
    pub mass: f64,
    #[serde(default)]
    pub radioactive: bool,
    pub category: String,
    pub group: Option<u32>,
    pub period: u32,
    /// Place in the drawn table: rows 1–7, and 9–10 for the lanthanides and actinides.
    pub row: u32,
    pub col: u32,
    pub state: Option<String>,
    /// kg/m³ at 20 °C (gases at 0 °C, 1 atm).
    pub density: Option<f64>,
    /// °C at 1 atm.
    pub melt: Option<f64>,
    pub boil: Option<f64>,
    pub structure: Option<String>,
    /// Pauling scale.
    pub electroneg: Option<f64>,
    /// Lattice parameters, Å.
    pub a: Option<f64>,
    pub c: Option<f64>,
    pub note: Option<String>,
    pub src: String,
    pub mass_src: String,
}

#[derive(Deserialize)]
struct File { elements: Vec<Element> }

pub fn elements() -> &'static [Element] {
    static E: OnceLock<Vec<Element>> = OnceLock::new();
    E.get_or_init(|| serde_json::from_str::<File>(include_str!("../../../data/elements.json")).expect("data/elements.json").elements)
}

/// An element by symbol (Fe), name (iron) or atomic number (26).
pub fn find(q: &str) -> Option<&'static Element> {
    let q = q.trim();
    let e = elements();
    if let Ok(z) = q.parse::<u32>() { return e.iter().find(|x| x.z == z) }
    e.iter().find(|x| x.symbol == q).or_else(|| e.iter().find(|x| x.symbol.eq_ignore_ascii_case(q) || x.name.eq_ignore_ascii_case(q) || (q.eq_ignore_ascii_case("aluminum") && x.z == 13) || (q.eq_ignore_ascii_case("cesium") && x.z == 55) || (q.eq_ignore_ascii_case("sulphur") && x.z == 16)))
}

#[derive(Debug, Clone, Serialize)]
pub struct Part { pub symbol: String, pub name: String, pub atoms: f64, pub at: f64, pub wt: f64 }

/// A composition: molar mass of the formula unit (g/mol; for percentages, of the average atom) and
/// each element's atom and weight percent.
#[derive(Debug, Clone, Serialize)]
pub struct Comp { pub text: String, pub by: String, pub mass: f64, pub parts: Vec<Part> }

/// Parse "Fe2O3", "Al2(SO4)3" or "CuSO4·5H2O" into element counts (in order of first appearance).
fn formula(s: &str) -> Result<Vec<(&'static Element, f64)>, String> {
    let c: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0;
    // A number at i (or 1).
    let num = |i: &mut usize| -> f64 {
        let st = *i;
        while *i < c.len() && (c[*i].is_ascii_digit() || c[*i] == '.') { *i += 1 }
        c[st..*i].iter().collect::<String>().parse().unwrap_or(1.0)
    };
    let mut stack: Vec<Vec<(&'static Element, f64)>> = vec![vec![]];
    let add = |v: &mut Vec<(&'static Element, f64)>, e: &'static Element, n: f64| match v.iter_mut().find(|x| x.0.z == e.z) { Some(x) => x.1 += n, None => v.push((e, n)) };
    let mut mult = 1.0; // a leading number after "·" (5H2O)
    while i < c.len() {
        match c[i] {
            '(' | '[' => { stack.push(vec![]); i += 1 }
            ')' | ']' => {
                i += 1;
                let n = num(&mut i);
                let g = stack.pop().filter(|_| !stack.is_empty()).ok_or("A closing bracket without an opening one.")?;
                let top = stack.last_mut().unwrap();
                for (e, k) in g { add(top, e, k * n) }
            }
            '·' | '*' if stack.len() == 1 => { i += 1; mult = num(&mut i) }
            ch if ch.is_ascii_uppercase() => {
                let st = i;
                i += 1;
                while i < c.len() && c[i].is_ascii_lowercase() { i += 1 }
                let sym: String = c[st..i].iter().collect();
                let e = elements().iter().find(|x| x.symbol == sym).ok_or(format!("{sym} is not an element symbol."))?;
                let n = num(&mut i) * mult;
                add(stack.last_mut().unwrap(), e, n);
            }
            ch => return Err(format!("“{ch}” is not part of a formula (write symbols, counts and brackets, e.g. Al2(SO4)3).")),
        }
    }
    if stack.len() != 1 { return Err("A bracket is not closed.".into()) }
    let v = stack.pop().unwrap();
    if v.is_empty() { return Err("Write a formula such as Fe2O3, or percentages such as Al Cu 4.4 Mg 1.5.".into()) }
    if v.iter().any(|x| x.1.is_nan() || x.1 <= 0.0) { return Err("Every count must be more than zero.".into()) }
    Ok(v)
}

/// Pairs "Cu 4.4 Mg 1.5 Al" (symbol, then a percent; one symbol without a number is the balance).
fn percents(s: &str) -> Result<Vec<(&'static Element, f64)>, String> {
    let toks: Vec<&str> = s.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '%' || c == '=' || c == ':').filter(|t| !t.is_empty()).collect();
    let mut v: Vec<(&'static Element, Option<f64>)> = vec![];
    let mut k = 0;
    while k < toks.len() {
        let e = find(toks[k]).filter(|_| toks[k].parse::<f64>().is_err()).ok_or(format!("{} is not an element.", toks[k]))?;
        if v.iter().any(|x| x.0.z == e.z) { return Err(format!("{} is given twice.", e.symbol)) }
        let n = toks.get(k + 1).and_then(|t| t.parse::<f64>().ok());
        if n.is_some_and(|n| !(0.0..=100.0).contains(&n)) { return Err(format!("{}: a percent is between 0 and 100.", e.symbol)) }
        v.push((e, n));
        k += if n.is_some() { 2 } else { 1 };
    }
    let given: f64 = v.iter().filter_map(|x| x.1).sum();
    match v.iter().filter(|x| x.1.is_none()).count() {
        0 if v.is_empty() => Err("Write each element and its percent, e.g. Al Cu 4.4 Mg 1.5 (the one without a number is the balance).".into()),
        0 if (given - 100.0).abs() > 0.5 => Err(format!("The percentages add up to {given:.4}, not 100; leave one element without a number to make it the balance.")),
        0 => Ok(v.into_iter().map(|(e, n)| (e, n.unwrap())).collect()),
        1 if given <= 100.0 => Ok(v.into_iter().map(|(e, n)| (e, n.unwrap_or(100.0 - given))).collect()),
        1 => Err(format!("The percentages given add up to {given:.4}, more than 100.")),
        _ => Err("Only one element can be the balance (without a number).".into()),
    }
}

/// The composition of `text`, read as a formula (`by` = "formula"), weight percent ("wt") or atom
/// percent ("at").
pub fn composition(text: &str, by: &str) -> Result<Comp, String> {
    let v = match by { "wt" | "at" => percents(text)?, _ => formula(text)? };
    let atoms: Vec<f64> = v.iter().map(|(e, n)| if by == "wt" { n / e.mass } else { *n }).collect();
    let na: f64 = atoms.iter().sum();
    let mw: f64 = v.iter().zip(&atoms).map(|((e, _), a)| a * e.mass).sum();
    let parts = v.iter().zip(&atoms).map(|((e, _), a)| Part { symbol: e.symbol.clone(), name: e.name.clone(), atoms: if by == "formula" { *a } else { a / na * 100.0 }, at: a / na * 100.0, wt: a * e.mass / mw * 100.0 }).collect();
    Ok(Comp { text: text.trim().into(), by: if by == "wt" || by == "at" { by.into() } else { "formula".into() }, mass: if by == "formula" { mw } else { mw / na }, parts })
}

/// Problems with the data: positions, order, sources, impossible values.
pub fn check(sources: &[crate::Source]) -> Vec<String> {
    let e = elements();
    let mut bad = vec![];
    if e.len() != 118 { bad.push(format!("{} elements, not 118", e.len())) }
    let mut seen = std::collections::HashSet::new();
    for (k, x) in e.iter().enumerate() {
        let w = |m: &str| format!("{} {}: {m}", x.z, x.symbol);
        if x.z as usize != k + 1 { bad.push(w("out of order")) }
        if !seen.insert((x.row, x.col)) { bad.push(w("shares a place in the table")) }
        if !(1..=18).contains(&x.col) || !(1..=10).contains(&x.row) { bad.push(w("outside the table")) }
        if x.mass.is_nan() || x.mass <= 0.0 { bad.push(w("no mass")) }
        for s in [&x.src, &x.mass_src] { if !sources.iter().any(|q| &q.id == s) { bad.push(w(&format!("source {s} is not in sources.json"))) } }
        for (n, v) in [("density", x.density), ("electronegativity", x.electroneg), ("a", x.a), ("c", x.c)] { if v.is_some_and(|v| v.is_nan() || v <= 0.0) { bad.push(w(&format!("{n} must be more than zero (a gap is left out, never 0)"))) } }
        if let (Some(m), Some(b)) = (x.melt, x.boil) { if m >= b { bad.push(w("melts above its boiling point")) } }
        if x.melt.is_some_and(|m| m < -273.15) || x.boil.is_some_and(|b| b < -273.15) { bad.push(w("below absolute zero")) }
    }
    bad
}
