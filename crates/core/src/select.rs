//! Selection: limit, chart and tree stages combined (a material passes when it passes every active
//! stage), each failure with its reason, and the ranking of the materials that pass by the
//! performance index of a chart's selection line.
use crate::db::{Db, SEP};
use crate::geometry::{in_polygon, mean, span, t};
use crate::model::*;
use crate::units::{fmt, range};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub name: String,
    pub family: String,
    pub pass: bool,
    /// Failures (for a material that fails) or notes such as "no data, kept".
    pub reasons: Vec<String>,
    /// Performance index of the ranking chart (stored units), and index ÷ the best one.
    pub index: Option<f64>,
    pub ratio: Option<f64>,
    pub rank: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub kind: String,
    pub name: String,
    pub chart: u32,
    pub pass: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Results {
    pub total: usize,
    pub pass: usize,
    pub stages: Vec<Stage>,
    pub rows: Vec<Row>,
    /// "E½/ρ" for the ranking line, "Weighted: …" for a weighted ranking, or "" when nothing ranks.
    pub index_name: String,
    /// The chart and the line that rank (0 when the ranking is weighted or nothing ranks).
    pub rank_chart: u32,
    pub rank_line: usize,
    /// For every selection line: the index it selects and its value on the line.
    pub lines: Vec<LineVal>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineVal {
    pub chart: u32,
    /// Which selection of the chart this line is.
    pub k: usize,
    pub name: String,
    pub value: f64,
}

/// The index a line of slope n on axes (x, y) selects: Y^(1/n)/X for n > 0, Y^(1/|n|)·X for n < 0.
pub fn index_name(c: &Chart, n: f64) -> String {
    let (x, y) = (prop(&c.x).map_or("X", |p| p.sym), prop(&c.y).map_or("Y", |p| p.sym));
    if !(c.log_x && c.log_y) { return format!("{y} − {}·{x}", fmt(n)); }
    let pow = |k: f64| [(1.0, ""), (2.0, "½"), (3.0, "⅓"), (4.0, "¼"), (1.5, "⅔"), (0.5, "²"), (1.0 / 3.0, "³")].iter().find(|(v, _)| (v - k).abs() < 1e-9).map_or_else(|| format!("^(1/{})", fmt(k)), |(_, s)| s.to_string());
    let x = if x.chars().count() > 2 { format!("({x})") } else { x.into() };
    if n.abs() < 1e-9 { y.into() } else if n > 0.0 { format!("{y}{}/{x}", pow(n)) } else { format!("{y}{}·{x}", pow(-n)) }
}

/// The y value (stored units) at x where a line of slope n on the chart has index `value`: the
/// inverse of `index_at`. None when the value cannot be on this chart (zero or below on log axes).
pub fn y_for(c: &Chart, n: f64, value: f64, x: f64) -> Option<f64> {
    if !(c.log_x && c.log_y) { return Some(value + n * t(x, c.log_x)?).filter(|v| v.is_finite()).and_then(|v| if c.log_y { Some(10f64.powf(v)) } else { Some(v) }); }
    let (lx, lv) = (t(x, true)?, t(value, true)?);
    let ly = if n.abs() < 1e-9 { lv } else if n > 0.0 { n * (lv + lx) } else { -n * (lv - lx) };
    Some(10f64.powf(ly)).filter(|v| v.is_finite() && *v > 0.0)
}

/// The index value at axis-space point (cx, cy), as a value in stored units.
fn index_at(c: &Chart, n: f64, cx: f64, cy: f64) -> f64 {
    if !(c.log_x && c.log_y) { return cy - n * cx; }
    let l = if n.abs() < 1e-9 { cy } else if n > 0.0 { cy / n - cx } else { cy / -n + cx };
    10f64.powf(l)
}

fn label(p: &Property, lo: f64, hi: f64) -> String {
    format!("{} {} {}", p.name, range(lo, hi), p.unit).trim_end().into()
}

/// Checks one limit; Ok(None) passes, Ok(Some(note)) passes with a note, Err(reason) fails.
fn limit(m: &Material, l: &Limit, st: &Stages) -> Result<Option<String>, String> {
    let Some(p) = prop(&l.prop) else { return Ok(None) };
    let Some(v) = m.props.get(&l.prop) else {
        let why = format!("no data for {}", p.name.to_lowercase());
        return if st.missing == "flag" { Ok(Some(format!("{why} (kept)"))) } else { Err(why) };
    };
    match v {
        Val::Range(lo, hi, _) | Val::RangeT(lo, hi, _, _) => {
            let x = if let Val::RangeT(_, _, _, t) = v { *t } else { mean(*lo, *hi, p.log) };
            let all = st.rule == "all";
            if let Some(min) = l.min {
                if (all && *lo < min) || (!all && x < min) { return Err(format!("{} {} {}", label(p, *lo, *hi), if all && x >= min { "is partly below" } else { "<" }, format!("{} {}", fmt(min), p.unit).trim_end())); }
            }
            if let Some(max) = l.max {
                if (all && *hi > max) || (!all && x > max) { return Err(format!("{} {} {}", label(p, *lo, *hi), if all && x <= max { "is partly above" } else { ">" }, format!("{} {}", fmt(max), p.unit).trim_end())); }
            }
            Ok(None)
        }
        Val::Flag(b, _) => match l.flag { Some(want) if want != *b => Err(format!("{}: {}", p.name, if *b { "yes" } else { "no" })), _ => Ok(None) },
        Val::Rating(r, _) => {
            let rank = |s: &str| RATINGS.iter().position(|x| *x == s).unwrap_or(0);
            match &l.rating { Some(want) if rank(r) < rank(want) => Err(format!("{}: {r} (needs {want} or better)", p.name)), _ => Ok(None) }
        }
    }
}

/// The material's point on a chart in axis space, or why it has none.
fn point(m: &Material, c: &Chart) -> Result<[f64; 2], String> {
    let get = |k: &str, log: bool| {
        let name = prop(k).map_or(k, |p| p.name).to_lowercase();
        let (lo, hi) = m.props.get(k).and_then(Val::range).ok_or_else(|| format!("not on the chart “{}”: no {name}", chart_name(c)))?;
        span(lo, hi, log).map(|s| s.0).ok_or_else(|| format!("not on the chart “{}”: {name} is not above zero", chart_name(c)))
    };
    Ok([get(&c.x, c.log_x)?, get(&c.y, c.log_y)?])
}

/// Does a point pass the chart's selection? Err gives the reason it fails.
fn on_chart(c: &Chart, s: &ChartSel, p: [f64; 2]) -> Result<(), String> {
    let (lx, ly) = (|v: f64| t(v, c.log_x).unwrap_or(f64::NAN), |v: f64| t(v, c.log_y).unwrap_or(f64::NAN));
    match s {
        ChartSel::Line { slope, x, y, above } => {
            let d = p[1] - (ly(*y) + slope * (p[0] - lx(*x)));
            if (*above && d >= 0.0) || (!*above && d <= 0.0) { Ok(()) } else { Err(format!("{} the selection line on “{}”", if *above { "below" } else { "above" }, chart_name(c))) }
        }
        ChartSel::Box { x0, x1, y0, y1 } => {
            let inx = p[0] >= lx(x0.min(*x1)) && p[0] <= lx(x0.max(*x1)) && p[1] >= ly(y0.min(*y1)) && p[1] <= ly(y0.max(*y1));
            if inx { Ok(()) } else { Err(format!("outside the box on “{}”", chart_name(c))) }
        }
        ChartSel::Poly { pts } => {
            let poly: Vec<[f64; 2]> = pts.iter().map(|q| [lx(q[0]), ly(q[1])]).collect();
            if in_polygon(&poly, p) { Ok(()) } else { Err(format!("outside the shape on “{}”", chart_name(c))) }
        }
    }
}

/// A material's value of a property for ranking: its typical value, else the mean of its range
/// (geometric for log properties).
fn mid(m: &Material, key: &str) -> Option<f64> {
    let p = prop(key)?;
    match m.props.get(key)? { Val::RangeT(_, _, _, t) => Some(*t), Val::Range(lo, hi, _) => Some(mean(*lo, *hi, p.log)), _ => None }.filter(|v| v.is_finite())
}

/// Weighted scores (0–1) of the rows that pass: each property scaled so the best passing material
/// scores 1 (a ratio for log properties, low to high for linear ones), then averaged by weight.
fn weighted(db: &Db, rows: &[Row], ws: &[Weight]) -> Vec<Option<f64>> {
    let total: f64 = ws.iter().map(|w| w.w).sum();
    let cols: Vec<Vec<Option<f64>>> = ws.iter().map(|w| rows.iter().map(|r| db.get(&r.id).and_then(|m| mid(m, &w.prop))).collect()).collect();
    let score = |i: usize| -> Option<f64> {
        if total <= 0.0 { return None; }
        let mut s = 0.0;
        for (w, col) in ws.iter().zip(&cols) {
            let v = col[i]?;
            let pass: Vec<f64> = col.iter().zip(rows).filter(|(_, r)| r.pass).filter_map(|(v, _)| *v).collect();
            let (lo, hi) = pass.iter().fold((f64::MAX, f64::MIN), |a, &v| (a.0.min(v), a.1.max(v)));
            let log = prop(&w.prop).is_some_and(|p| p.log) && lo > 0.0 && v > 0.0;
            let n = if log { if w.high { v / hi } else { lo / v } } else if hi > lo { if w.high { (v - lo) / (hi - lo) } else { (hi - v) / (hi - lo) } } else { 1.0 };
            s += w.w * n.clamp(0.0, 1.0);
        }
        Some(s / total)
    };
    (0..rows.len()).map(|i| if rows[i].pass { score(i) } else { None }).collect()
}

/// "Weighted: 60 % Density (low), 40 % Price (low)".
fn weights_name(ws: &[Weight]) -> String {
    let total: f64 = ws.iter().map(|w| w.w).sum();
    let parts: Vec<String> = ws.iter().map(|w| format!("{} % {} ({})", fmt(100.0 * w.w / total), prop(&w.prop).map_or(w.prop.as_str(), |p| p.name), if w.high { "high" } else { "low" })).collect();
    format!("Weighted: {}", parts.join(", "))
}

pub fn run(db: &Db, pr: &Project) -> Results {
    let st = &pr.stages;
    let charts: Vec<&Chart> = pr.charts.iter().filter(|c| !c.sels.is_empty() && c.kind != "bar").collect();
    // The ranking line: the chosen one, else the first line of any chart.
    let line_of = |c: &Chart, k: usize| match c.sels.get(k) { Some(ChartSel::Line { slope, above, .. }) => Some((*slope, *above)), _ => None };
    let chosen = charts.iter().find(|c| c.id == st.rank_chart).and_then(|c| line_of(c, st.rank_line).map(|l| (*c, st.rank_line, l)));
    let first = || charts.iter().find_map(|c| c.sels.iter().position(|s| matches!(s, ChartSel::Line { .. })).and_then(|k| line_of(c, k).map(|l| (*c, k, l))));
    let by_weight = st.ranking == "weighted" && !st.weights.is_empty();
    let rank = if by_weight { None } else { chosen.or_else(first) };
    let mut stages: Vec<Stage> = vec![];
    if !st.limits.is_empty() { stages.push(Stage { kind: "limit".into(), name: format!("Limits ({})", st.limits.len()), chart: 0, pass: 0 }); }
    for c in &charts { stages.push(Stage { kind: "chart".into(), name: c.name.clone(), chart: c.id, pass: 0 }); }
    if !st.exclude.is_empty() { stages.push(Stage { kind: "tree".into(), name: format!("Families left out ({})", st.exclude.len()), chart: 0, pass: 0 }); }
    let mut rows: Vec<Row> = db.materials.iter().map(|m| {
        let (mut fails, mut notes, mut k) = (vec![], vec![], 0);
        if !st.limits.is_empty() {
            let mut ok = true;
            for l in &st.limits { match limit(m, l, st) { Ok(Some(n)) => notes.push(n), Ok(None) => {} Err(e) => { ok = false; fails.push(e) } } }
            if ok { stages[k].pass += 1; }
            k += 1;
        }
        for c in &charts {
            match point(m, c).and_then(|p| c.sels.iter().try_for_each(|s| on_chart(c, s, p))) { Ok(()) => stages[k].pass += 1, Err(e) => fails.push(e) }
            k += 1;
        }
        if !st.exclude.is_empty() {
            let path = m.family.join(SEP);
            match st.exclude.iter().find(|e| path == **e || path.starts_with(&format!("{e}{SEP}"))) { Some(e) => fails.push(format!("{e} is left out")), None => stages[k].pass += 1 }
        }
        let index = rank.and_then(|(c, _, (slope, _))| point(m, c).ok().map(|p| index_at(c, slope, p[0], p[1]))).filter(|v| v.is_finite());
        Row { id: m.id.clone(), name: m.name.clone(), family: m.family.first().cloned().unwrap_or_default(), pass: fails.is_empty(), reasons: if fails.is_empty() { notes } else { fails }, index, ratio: None, rank: None }
    }).collect();
    if by_weight {
        let scores = weighted(db, &rows, &st.weights);
        for (r, s) in rows.iter_mut().zip(scores) {
            r.index = s;
            if r.pass && s.is_none() { r.reasons.push("not ranked: no data for a weighted property".into()) }
        }
    }
    let above = by_weight || rank.is_some_and(|(_, _, (_, a))| a);
    rows.sort_by(|a, b| b.pass.cmp(&a.pass).then_with(|| match (a.index, b.index) {
        (Some(x), Some(y)) if a.pass => if above { y.total_cmp(&x) } else { x.total_cmp(&y) },
        (Some(_), None) if a.pass => std::cmp::Ordering::Less,
        (None, Some(_)) if a.pass => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }).then_with(|| a.name.cmp(&b.name)));
    let best = rows.first().filter(|r| r.pass).and_then(|r| r.index);
    for (k, r) in rows.iter_mut().enumerate().filter(|(_, r)| r.pass) {
        r.rank = Some(k + 1);
        r.ratio = match (r.index, best) { (Some(i), Some(b)) if b != 0.0 && i != 0.0 => Some(if above { i / b } else { b / i }).filter(|v| v.is_finite()), _ => None };
    }
    let pass = rows.iter().filter(|r| r.pass).count();
    let index_name = if by_weight { weights_name(&st.weights) } else { rank.map(|(c, _, (slope, _))| index_name(c, slope)).unwrap_or_default() };
    let lines = charts.iter().flat_map(|c| c.sels.iter().enumerate().filter_map(move |(k, s)| match s {
        ChartSel::Line { slope, x, y, .. } => Some(LineVal { chart: c.id, k, name: self::index_name(c, *slope), value: index_at(c, *slope, t(*x, c.log_x)?, t(*y, c.log_y)?) }),
        _ => None,
    })).collect();
    Results { total: rows.len(), pass, stages, rows, index_name, rank_chart: rank.map_or(0, |r| r.0.id), rank_line: rank.map_or(0, |r| r.1), lines }
}
