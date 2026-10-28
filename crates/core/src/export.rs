//! The ranked results as a table for spreadsheets: CSV (UTF-8 with a byte-order mark, so
//! spreadsheet programs read ρ and σ) or an .xlsx workbook. Every material is a row: its rank, the
//! index, "× best", whether it passes and why not, and the typical value of each property the
//! selection uses (in the stored units, named in the heading).
use crate::db::Db;
use crate::geometry::mean;
use crate::model::*;
use crate::select::run;

#[derive(Debug, Clone, PartialEq)]
pub enum Cell { Text(String), Num(f64), Empty }

pub struct Table { pub head: Vec<String>, pub rows: Vec<Vec<Cell>> }

/// The properties a selection uses, in order: its charts' axes, its limits and its weights.
fn used(p: &Project) -> Vec<String> {
    let mut v: Vec<String> = vec![];
    let keys = p.charts.iter().flat_map(|c| [c.y.clone(), c.x.clone()]).chain(p.stages.limits.iter().map(|l| l.prop.clone())).chain(p.stages.weights.iter().map(|w| w.prop.clone()));
    for k in keys { if !v.contains(&k) { v.push(k) } }
    v
}

/// The results of a project as a table (the database should already hold its formulas).
pub fn results(db: &Db, p: &Project) -> Table {
    let r = run(db, p);
    let props: Vec<String> = used(p).into_iter().filter(|k| db.info(k).is_some()).collect();
    let mut head: Vec<String> = ["Rank", "Material", "Family", "Passes"].map(String::from).to_vec();
    head.push(if r.index_name.is_empty() { "Index".into() } else { r.index_name.clone() });
    head.push("× best".into());
    for k in &props { let i = db.info(k).unwrap_or_else(|| unreachable!()); head.push(if i.unit.is_empty() { i.name } else { format!("{} ({})", i.name, i.unit) }); }
    head.push("Why it fails, or notes".into());
    let num = |v: Option<f64>| v.filter(|x| x.is_finite()).map_or(Cell::Empty, Cell::Num);
    let rows = r.rows.iter().map(|x| {
        let m = db.get(&x.id);
        let mut row = vec![x.rank.map_or(Cell::Empty, |k| Cell::Num(k as f64)), Cell::Text(x.name.clone()), Cell::Text(x.family.clone()), Cell::Text(if x.pass { "Yes" } else { "No" }.into()), num(x.index), num(x.ratio)];
        for k in &props {
            let log = db.info(k).is_some_and(|i| i.log);
            row.push(match m.and_then(|m| m.props.get(k)) {
                Some(Val::RangeT(_, _, _, t)) => Cell::Num(*t),
                Some(Val::Range(lo, hi, _)) => Cell::Num(mean(*lo, *hi, log && *lo > 0.0)),
                Some(Val::Flag(b, _)) => Cell::Text(if *b { "Yes" } else { "No" }.into()),
                Some(Val::Rating(s, _)) => Cell::Text(s.clone()),
                None => Cell::Empty,
            });
        }
        row.push(Cell::Text(x.reasons.join("; ")));
        row
    }).collect();
    Table { head, rows }
}

/// CSV with quotes where needed, and a byte-order mark.
pub fn csv(t: &Table) -> String {
    let q = |s: &str| if s.contains([',', '"', '\n', '\r']) || s.starts_with(['=', '+', '-', '@']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let cell = |c: &Cell| match c { Cell::Text(s) => q(s), Cell::Num(n) => format!("{n}"), Cell::Empty => String::new() };
    let mut out = String::from("\u{FEFF}");
    out.push_str(&t.head.iter().map(|h| q(h)).collect::<Vec<_>>().join(","));
    for r in &t.rows { out.push_str("\r\n"); out.push_str(&r.iter().map(cell).collect::<Vec<_>>().join(",")); }
    out.push_str("\r\n");
    out
}

/// An .xlsx workbook with one sheet: bold headings, the passing rows first, frozen heading row.
pub fn xlsx(t: &Table, sheet: &str) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{Format, Workbook};
    let e = |x: rust_xlsxwriter::XlsxError| x.to_string();
    let mut wb = Workbook::new();
    let head = Format::new().set_bold().set_background_color(0xEDE7F6).set_text_wrap();
    let ws = wb.add_worksheet();
    let name: String = sheet.chars().filter(|c| !"[]:*?/\\".contains(*c)).take(31).collect();
    ws.set_name(if name.trim().is_empty() { "Results" } else { name.trim() }).map_err(e)?;
    for (k, h) in t.head.iter().enumerate() { ws.write_string_with_format(0, k as u16, h, &head).map_err(e)?; }
    for (i, r) in t.rows.iter().enumerate() {
        for (k, c) in r.iter().enumerate() {
            let (row, col) = (i as u32 + 1, k as u16);
            match c { Cell::Text(s) => { ws.write_string(row, col, s).map_err(e)?; } Cell::Num(n) => { ws.write_number(row, col, *n).map_err(e)?; } Cell::Empty => {} }
        }
    }
    ws.set_freeze_panes(1, 2).map_err(e)?;
    ws.set_column_width(1, 34).map_err(e)?;
    ws.set_column_width(t.head.len().saturating_sub(1) as u16, 60).map_err(e)?;
    wb.save_to_buffer().map_err(e)
}
