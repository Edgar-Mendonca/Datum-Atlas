//! Selection: limit, chart and tree stages combined (a material passes when it passes every active
//! stage), each failure with its reason, and the ranking of the materials that pass by the
//! performance index of a chart's selection line.
use crate::db::{Db, SEP};
use crate::geometry::{mean, span, t};
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
    /// "E^½/ρ" for the ranking chart's line, or "" when nothing ranks.
    pub index_name: String,
    pub rank_chart: u32,
    /// For every chart with a selection line: the index it selects and its value on the line.
    pub lines: Vec<LineVal>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineVal {
    pub chart: u32,
    pub name: String,
    pub value: f64,
}

/// The index a line of slope n on axes (x, y) selects: Y^(1/n)/X for n > 0, Y^(1/|n|)·X for n < 0.
pub fn index_name(c: &Chart, n: f64) -> String {
    let (x, y) = (prop(&c.x).map_or("X", |p| p.sym), prop(&c.y).map_or("Y", |p| p.sym));
    if !(c.log_x && c.log_y) { return format!("{y} − {}·{x}", fmt(n)); }
    let pow = |k: f64| if (k - 1.0).abs() < 1e-9 { String::new() } else if (k - 2.0).abs() < 1e-9 { "^½".into() } else if (k - 0.5).abs() < 1e-9 { "²".into() } else if (k - 3.0).abs() < 1e-9 { "^⅓".into() } else { format!("^(1/{})", fmt(k)) };
    if n.abs() < 1e-9 { y.into() } else if n > 0.0 { format!("{y}{}/{x}", pow(n)) } else { format!("{y}{}·{x}", pow(-n)) }
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
    }
}

pub fn run(db: &Db, pr: &Project) -> Results {
    let st = &pr.stages;
    let charts: Vec<&Chart> = pr.charts.iter().filter(|c| c.select.is_some()).collect();
    let rank_chart = charts.iter().find(|c| c.id == st.rank_chart).or_else(|| charts.iter().find(|c| matches!(c.select, Some(ChartSel::Line { .. })))).copied()
        .filter(|c| matches!(c.select, Some(ChartSel::Line { .. })));
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
            match point(m, c).and_then(|p| on_chart(c, c.select.as_ref().unwrap(), p)) { Ok(()) => stages[k].pass += 1, Err(e) => fails.push(e) }
            k += 1;
        }
        if !st.exclude.is_empty() {
            let path = m.family.join(SEP);
            match st.exclude.iter().find(|e| path == **e || path.starts_with(&format!("{e}{SEP}"))) { Some(e) => fails.push(format!("{e} is left out")), None => stages[k].pass += 1 }
        }
        let index = rank_chart.and_then(|c| match c.select { Some(ChartSel::Line { slope, .. }) => point(m, c).ok().map(|p| index_at(c, slope, p[0], p[1])), _ => None });
        Row { id: m.id.clone(), name: m.name.clone(), family: m.family.first().cloned().unwrap_or_default(), pass: fails.is_empty(), reasons: if fails.is_empty() { notes } else { fails }, index, ratio: None, rank: None }
    }).collect();
    let above = matches!(rank_chart.and_then(|c| c.select.as_ref()), Some(ChartSel::Line { above: true, .. }));
    rows.sort_by(|a, b| b.pass.cmp(&a.pass).then_with(|| match (a.index, b.index) {
        (Some(x), Some(y)) if a.pass => if above { y.total_cmp(&x) } else { x.total_cmp(&y) },
        _ => std::cmp::Ordering::Equal,
    }).then_with(|| a.name.cmp(&b.name)));
    let best = rows.first().filter(|r| r.pass).and_then(|r| r.index);
    for (k, r) in rows.iter_mut().enumerate().filter(|(_, r)| r.pass) {
        r.rank = Some(k + 1);
        r.ratio = match (r.index, best) { (Some(i), Some(b)) if b != 0.0 => Some(if above { i / b } else { b / i }), _ => None };
    }
    let pass = rows.iter().filter(|r| r.pass).count();
    let index_name = rank_chart.map(|c| match c.select { Some(ChartSel::Line { slope, .. }) => index_name(c, slope), _ => String::new() }).unwrap_or_default();
    let lines = charts.iter().filter_map(|c| match c.select {
        Some(ChartSel::Line { slope, x, y, .. }) => Some(LineVal { chart: c.id, name: self::index_name(c, slope), value: index_at(c, slope, t(x, c.log_x)?, t(y, c.log_y)?) }),
        _ => None,
    }).collect();
    Results { total: rows.len(), pass, stages, rows, index_name, rank_chart: rank_chart.map_or(0, |c| c.id), lines }
}
