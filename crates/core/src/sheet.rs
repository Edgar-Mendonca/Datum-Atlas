//! Spreadsheets in and out of a materials database. Reading a CSV (the separator is guessed) or the
//! first sheet of a workbook follows Datum Plan's table reader. Each column is matched to a property
//! by its heading (key, name or symbol; a unit in brackets such as "(ksi)" is converted from
//! imperial). A cell holds one value ("2700") or a range ("2690 – 2710"); a blank cell is a gap,
//! never zero. A database is written back out the same way, so it can be read in again.
use crate::db::read_user;
use crate::export::{self, Cell, Table as Out};
use crate::model::*;
use crate::units;
use calamine::{open_workbook_auto, Data, Reader};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Table {
    pub head: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// The field each column most likely holds: "name", "family", "class", "designation", "notes",
    /// "uses", "source", a property key (with "|imperial" when its heading names an imperial unit), or "".
    pub guess: Vec<String>,
}

fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let first = text.trim_start_matches('\u{feff}').lines().next().unwrap_or("");
    let sep = [',', ';', '\t'].into_iter().max_by_key(|c| first.matches(*c).count()).unwrap_or(',');
    let (mut rows, mut row, mut cell, mut q) = (vec![], vec![], String::new(), false);
    let mut it = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if q && it.peek() == Some(&'"') => { cell.push('"'); it.next(); }
            '"' => q = !q,
            c if c == sep && !q => row.push(std::mem::take(&mut cell)),
            '\n' if !q => { row.push(std::mem::take(&mut cell)); rows.push(std::mem::take(&mut row)); }
            '\r' if !q => {}
            c => cell.push(c),
        }
    }
    if !cell.is_empty() || !row.is_empty() { row.push(cell); rows.push(row); }
    rows
}

fn cell(d: &Data) -> String {
    match d {
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// Reads a CSV text (for tests and pasted tables) into a table with guesses.
pub fn from_text(text: &str) -> Result<Table, String> { finish(csv_rows(text)) }

/// Reads a .csv or .txt as text, anything else as a workbook (its first sheet).
pub fn read(path: &Path) -> Result<Table, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let rows = if ext == "csv" || ext == "txt" {
        csv_rows(&String::from_utf8_lossy(&std::fs::read(path).map_err(|e| e.to_string())?))
    } else {
        let mut wb = open_workbook_auto(path).map_err(|e| format!("Could not read the workbook: {e}"))?;
        let range = wb.worksheet_range_at(0).ok_or("The workbook has no sheets.")?.map_err(|e| e.to_string())?;
        range.rows().map(|r| r.iter().map(cell).collect()).collect()
    };
    finish(rows)
}

fn finish(mut rows: Vec<Vec<String>>) -> Result<Table, String> {
    rows.retain(|r| r.iter().any(|c| !c.trim().is_empty()));
    if rows.is_empty() { return Err("The file has no rows.".into()); }
    let head: Vec<String> = rows.remove(0).into_iter().map(|h| h.trim().to_string()).collect();
    let guess = head.iter().map(|h| guess(h)).collect();
    Ok(Table { head, rows, guess })
}

/// The field a heading names: "Density (kg/m³)" → "rho"; "Yield strength (ksi)" → "sy|imperial".
pub fn guess(h: &str) -> String {
    let (words, unit) = match h.split_once('(') { Some((a, b)) => (a.trim(), b.trim_end_matches(')').trim()), None => (h.trim(), "") };
    let w = words.to_lowercase();
    for (f, names) in [("name", &["name", "material", "material name", "grade"][..]), ("family", &["family"]), ("class", &["class", "subclass", "type"]), ("designation", &["designation", "uns", "en", "astm", "iso", "standard"]),
        ("notes", &["notes", "note", "condition", "comments"]), ("uses", &["uses", "applications", "typical uses"]), ("source", &["source", "reference", "ref"])] {
        if names.contains(&w.as_str()) { return f.into(); }
    }
    let Some(p) = PROPS.iter().find(|p| p.key.to_lowercase() == w || p.name.to_lowercase() == w || (!p.sym.is_empty() && p.sym.to_lowercase() == w)
        || p.name.to_lowercase().split(" (").next() == Some(w.as_str()) || p.name.to_lowercase().split(',').next() == Some(w.as_str())) else { return String::new() };
    match units::imperial(p.unit) { Some((iu, _, _)) if !unit.is_empty() && unit == iu => format!("{}|imperial", p.key), _ => p.key.into() }
}

/// One value or a range from a cell: "2700", "2690 – 2710", "-50 to 20", "1.2e9". None if no number.
pub fn range(s: &str) -> Option<(f64, f64)> {
    let t = s.replace(['–', '—'], " - ").replace(" to ", " - ").replace(',', ".");
    let mut nums = vec![];
    let mut cur = String::new();
    let flush = |cur: &mut String, nums: &mut Vec<f64>| { if let Ok(v) = cur.parse::<f64>() { nums.push(v) } cur.clear(); };
    let cs: Vec<char> = t.chars().collect();
    for (i, &c) in cs.iter().enumerate() {
        let sign = c == '-' && cur.is_empty() && cs.get(i + 1).is_some_and(|n| n.is_ascii_digit() || *n == '.') && (i == 0 || cs[i - 1] == ' ');
        if c.is_ascii_digit() || c == '.' || sign || ((c == 'e' || c == 'E') && !cur.is_empty()) || ((c == '-' || c == '+') && cur.ends_with(['e', 'E'])) { cur.push(c) } else { flush(&mut cur, &mut nums) }
    }
    flush(&mut cur, &mut nums);
    match nums[..] { [v] => Some((v, v)), [a, b] => Some((a.min(b), a.max(b))), _ => None }.filter(|r| r.0.is_finite() && r.1.is_finite())
}

/// Materials from a table: `map[c]` is the field of column c (as `guess` gives). Rows without a name
/// are skipped. `family` is used when no family column says which. Every value cites one source: the
/// row's source column, else `source`. Values the engine cannot use are dropped and listed.
pub fn to_database(t: &Table, map: &[String], family: &str, source: &str) -> Result<(Database, Vec<String>), String> {
    let col = |f: &str| map.iter().position(|m| m == f);
    let name = col("name").ok_or("Choose which column holds the material names.")?;
    if source.trim().is_empty() && col("source").is_none() { return Err("Say where the values come from: a datasheet, a test report, a book…".into()); }
    let mut d = Database { format: FORMAT, name: "Imported".into(), ..Default::default() };
    let mut notes = vec![];
    let src_of = |title: &str, d: &mut Database| -> String {
        let title = if title.trim().is_empty() { source.trim() } else { title.trim() };
        if let Some(s) = d.sources.iter().find(|s| s.title == title) { return s.id.clone(); }
        let id = format!("imp-s{}", d.sources.len() + 1);
        d.sources.push(Source { id: id.clone(), title: title.into(), kind: "user".into(), ..Default::default() });
        id
    };
    for (r, row) in t.rows.iter().enumerate() {
        let get = |c: usize| row.get(c).map(|s| s.trim()).unwrap_or("");
        let nm = get(name);
        if nm.is_empty() { continue; }
        let src = src_of(col("source").map_or("", get), &mut d);
        let fam_cell = col("family").map_or("", get);
        let fam = FAMILIES.iter().find(|f| f.eq_ignore_ascii_case(fam_cell)).or_else(|| FAMILIES.iter().find(|f| f.eq_ignore_ascii_case(family))).unwrap_or(&FAMILIES[0]);
        let mut m = Material { id: format!("imp-{}", r + 1), name: nm.into(), family: vec![fam.to_string()], status: "user".into(), ..Default::default() };
        if let Some(c) = col("class").map(get).filter(|c| !c.is_empty()) { m.family.extend(c.split('›').map(|s| s.trim().to_string())); }
        if let Some(c) = col("designation").map(get).filter(|c| !c.is_empty()) { m.des.insert("Code".into(), c.into()); }
        m.notes = col("notes").map_or("", get).into();
        m.uses = col("uses").map_or("", get).into();
        for (c, f) in map.iter().enumerate() {
            let (key, imp) = match f.split_once('|') { Some((k, _)) => (k, true), None => (f.as_str(), false) };
            let (Some(p), text) = (prop(key), get(c)) else { continue };
            if text.is_empty() { continue; }
            let v = match p.kind {
                "range" => range(text).map(|(lo, hi)| { let s = |v: f64| if imp { units::from(v, p.unit, "imperial") } else { v }; let (a, b) = (s(lo), s(hi)); Val::Range(a.min(b), a.max(b), src.clone()) }),
                "flag" => match text.to_lowercase().as_str() { "yes" | "true" | "1" | "y" => Some(Val::Flag(true, src.clone())), "no" | "false" | "0" | "n" => Some(Val::Flag(false, src.clone())), _ => None },
                _ => RATINGS.iter().find(|x| x.eq_ignore_ascii_case(text)).map(|x| Val::Rating(x.to_string(), src.clone())),
            };
            match v { Some(v) => { m.props.insert(key.into(), v); } None => notes.push(format!("{nm}: “{text}” is not a value of {}", p.name.to_lowercase())) }
        }
        if m.props.is_empty() { notes.push(format!("{nm}: no values, left out")); continue; }
        if !m.props.contains_key("rho") { notes.push(format!("{nm}: no density, so it is missing from most charts")); }
        d.materials.push(m);
    }
    if d.materials.is_empty() { return Err("No row has both a name and a value.".into()); }
    // The same checks as a database file: values the engine cannot use are dropped and listed.
    let (d, more) = read_user(&serde_json::to_string(&d).map_err(|e| e.to_string())?)?;
    notes.extend(more);
    Ok((d, notes))
}

/// A database as a table that `to_database` reads back: name, family, class, designation, each
/// property ("low – high" in stored units) and the source of the first value.
pub fn of_database(d: &Database) -> Out {
    let keys: Vec<&Property> = PROPS.iter().filter(|p| p.group != "Derived" && d.materials.iter().any(|m| m.props.contains_key(p.key))).collect();
    let mut head: Vec<String> = ["Name", "Family", "Class", "Designation"].map(String::from).to_vec();
    head.extend(keys.iter().map(|p| if p.unit.is_empty() { p.name.to_string() } else { format!("{} ({})", p.name, p.unit) }));
    head.extend(["Notes", "Uses", "Source"].map(String::from));
    let rows = d.materials.iter().map(|m| {
        let mut r = vec![Cell::Text(m.name.clone()), Cell::Text(m.family.first().cloned().unwrap_or_default()), Cell::Text(m.family[1.min(m.family.len())..].join(" › ")), Cell::Text(m.des.values().cloned().collect::<Vec<_>>().join(", "))];
        r.extend(keys.iter().map(|p| match m.props.get(p.key) {
            Some(Val::Range(lo, hi, _) | Val::RangeT(lo, hi, _, _)) => Cell::Text(if lo == hi { units::fmt(*lo) } else { format!("{} – {}", units::fmt(*lo), units::fmt(*hi)) }),
            Some(Val::Flag(b, _)) => Cell::Text(if *b { "Yes" } else { "No" }.into()),
            Some(Val::Rating(s, _)) => Cell::Text(s.clone()),
            None => Cell::Empty,
        }));
        let src = m.props.values().next().map(|v| v.src().to_string()).unwrap_or_default();
        r.extend([Cell::Text(m.notes.clone()), Cell::Text(m.uses.clone()), Cell::Text(d.sources.iter().find(|s| s.id == src).map_or(src, |s| s.title.clone()))]);
        r
    }).collect();
    Out { head, rows }
}

/// The table as CSV or .xlsx bytes.
pub fn bytes(t: &Out, fmt: &str, name: &str) -> Result<Vec<u8>, String> {
    if fmt == "csv" { Ok(export::csv(t).into_bytes()) } else { export::xlsx(t, name) }
}
