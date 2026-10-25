//! A property chart for the window: every material with both values as a bubble, a hull per
//! family (or class), the axis extents, and the materials left out with the reason. Coordinates
//! are in axis space (log10 on log axes), so the window only maps them to pixels.
use crate::db::Db;
use crate::geometry::{self, span};
use crate::model::*;
use serde::Serialize;

type Pair = (f64, f64);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bubble {
    pub id: String,
    pub name: String,
    /// Index into `families`.
    pub fam: usize,
    pub class: String,
    pub c: [f64; 2],
    pub r: [f64; 2],
    /// The ranges in stored units, for the tooltip.
    pub x: [f64; 2],
    pub y: [f64; 2],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hull {
    pub name: String,
    pub fam: usize,
    pub pts: Vec<[f64; 2]>,
    /// Where to put its label (over the centre of its bubbles, at the top edge), and whether to label it (the biggest
    /// group of a family or class is labelled, small outliers are not).
    pub at: [f64; 2],
    pub label: bool,
    /// The materials inside.
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Axis {
    pub key: String,
    pub name: String,
    pub sym: String,
    pub unit: String,
    pub log: bool,
    /// Extent in axis space with a margin: [min, max].
    pub lim: [f64; 2],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartOut {
    pub id: u32,
    pub x: Axis,
    pub y: Axis,
    pub families: Vec<String>,
    pub bubbles: Vec<Bubble>,
    pub hulls: Vec<Hull>,
    pub missing: Vec<(String, String)>,
    /// Bar charts: each family's span on the x axis, for its label.
    pub bands: Vec<Band>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Band {
    pub name: String,
    pub fam: usize,
    pub from: f64,
    pub to: f64,
}

/// A bar chart of one property (`c.y`): a floating bar per material from low to high, grouped by
/// family and sorted within it. Bars reuse `Bubble` (c = centre, r = half-width and half-height).
fn bars(db: &Db, c: &Chart) -> ChartOut {
    let families: Vec<String> = FAMILIES.iter().map(|s| s.to_string()).collect();
    let (mut bubbles, mut missing, mut bands) = (vec![], vec![], vec![]);
    let mut slot = 0.0;
    for (k, fam) in FAMILIES.iter().enumerate() {
        if c.hidden.iter().any(|h| h == fam) { continue; }
        let mut mine: Vec<(&Material, Pair, Pair)> = vec![];
        for m in db.materials.iter().filter(|m| m.family.first().map(String::as_str) == Some(*fam)) {
            let name = prop(&c.y).map_or(c.y.as_str(), |p| p.name).to_lowercase();
            match m.props.get(&c.y).and_then(Val::range) {
                Some(r) => match span(r.0, r.1, c.log_y) { Some(sp) => mine.push((m, r, sp)), None => missing.push((m.id.clone(), format!("{name} is not above zero, so it cannot go on a log axis"))) },
                None => missing.push((m.id.clone(), format!("no {name}"))),
            }
        }
        if mine.is_empty() { continue; }
        mine.sort_by(|a, b| a.2 .0.total_cmp(&b.2 .0));
        let from = slot;
        for (m, r, sp) in mine {
            bubbles.push(Bubble { id: m.id.clone(), name: m.name.clone(), fam: k, class: m.family.get(1).cloned().unwrap_or_default(), c: [slot + 0.5, sp.0], r: [0.38, sp.1], x: [0.0, 0.0], y: [r.0, r.1] });
            slot += 1.0;
        }
        bands.push(Band { name: fam.to_string(), fam: k, from, to: slot });
        slot += 1.5;
    }
    let (lo, hi) = bubbles.iter().fold((f64::MAX, f64::MIN), |(a, b), q| (a.min(q.c[1] - q.r[1]), b.max(q.c[1] + q.r[1])));
    let (lo, hi) = if lo > hi { (0.0, 1.0) } else { (lo, hi) };
    let w = (hi - lo).max(if c.log_y { 1.0 } else { hi.abs().max(1.0) * 0.1 });
    let x = Axis { key: "family".into(), name: "Materials, by family".into(), sym: String::new(), unit: String::new(), log: false, lim: [-1.0, slot.max(1.0)] };
    ChartOut { id: c.id, x, y: axis(&c.y, c.log_y, [lo - w * 0.06, hi + w * 0.06]), families, bubbles, hulls: vec![], missing, bands }
}

fn axis(key: &str, log: bool, lim: [f64; 2]) -> Axis {
    let p = prop(key);
    Axis { key: key.into(), name: p.map_or(key, |p| p.name).into(), sym: p.map_or("", |p| p.sym).into(), unit: p.map_or("", |p| p.unit).into(), log, lim }
}

/// Lays out a chart. Families hidden on the chart are left out (not listed as missing).
pub fn build(db: &Db, c: &Chart) -> ChartOut {
    if c.kind == "bar" { return bars(db, c); }
    let families: Vec<String> = FAMILIES.iter().map(|s| s.to_string()).collect();
    let (mut bubbles, mut missing) = (vec![], vec![]);
    for m in &db.materials {
        let fam = m.family.first().cloned().unwrap_or_default();
        if c.hidden.contains(&fam) { continue; }
        let get = |k: &str, log: bool| -> Result<(Pair, Pair), String> {
            let p = prop(k).map_or(k, |p| p.name).to_lowercase();
            let (lo, hi) = m.props.get(k).and_then(Val::range).ok_or_else(|| format!("no {p}"))?;
            Ok(((lo, hi), span(lo, hi, log).ok_or_else(|| format!("{p} is not above zero, so it cannot go on a log axis"))?))
        };
        match (get(&c.x, c.log_x), get(&c.y, c.log_y)) {
            (Ok((xr, (cx, rx))), Ok((yr, (cy, ry)))) => bubbles.push(Bubble {
                id: m.id.clone(), name: m.name.clone(), fam: FAMILIES.iter().position(|f| *f == fam).unwrap_or(0),
                class: m.family.get(1).cloned().unwrap_or_default(), c: [cx, cy], r: [rx, ry], x: [xr.0, xr.1], y: [yr.0, yr.1],
            }),
            (Err(e), _) | (_, Err(e)) => missing.push((m.id.clone(), e)),
        }
    }
    let ext = |k: usize| {
        let (lo, hi) = bubbles.iter().fold((f64::MAX, f64::MIN), |(a, b), q: &Bubble| (a.min(q.c[k] - q.r[k]), b.max(q.c[k] + q.r[k])));
        if lo > hi { return [0.0, 1.0]; }
        let w = (hi - lo).max(if [c.log_x, c.log_y][k] { 1.0 } else { hi.abs().max(1.0) * 0.1 });
        [lo - w * 0.06, hi + w * 0.06]
    };
    let (lx, ly) = (ext(0), ext(1));
    let scale = [lx[1] - lx[0], ly[1] - ly[0]];
    // Bubbles of single values still need a visible size: at least 0.5 % of the axis.
    for b in &mut bubbles {
        b.r = [b.r[0].max(scale[0] * 0.005), b.r[1].max(scale[1] * 0.005)];
    }
    let mut hulls = vec![];
    if c.hull != "none" {
        let level = if c.hull == "class" { 1 } else { 0 };
        let mut groups: Vec<(String, usize)> = vec![];
        for b in &bubbles {
            let m = db.get(&b.id).unwrap();
            let name = m.family.get(level).or(m.family.first()).cloned().unwrap_or_default();
            if !groups.iter().any(|g| g.0 == name) { groups.push((name, b.fam)); }
        }
        for (name, fam) in groups {
            let mine: Vec<&Bubble> = bubbles.iter().filter(|b| db.get(&b.id).and_then(|m| m.family.get(level).or(m.family.first())) == Some(&name)).collect();
            let shapes: Vec<([f64; 2], [f64; 2])> = mine.iter().map(|b| (b.c, b.r)).collect();
            let mut parts = geometry::clusters(&shapes, scale, 0.05);
            parts.sort_by_key(|p| std::cmp::Reverse(p.len()));
            for (k, part) in parts.iter().enumerate() {
                let n = part.len() as f64;
                let sh: Vec<_> = part.iter().map(|&i| shapes[i]).collect();
                let pts = geometry::hull(&sh, 0.018, scale);
                // The label sits on the top edge of the envelope, centred over its bubbles.
                let at = [part.iter().map(|&i| mine[i].c[0]).sum::<f64>() / n, pts.iter().map(|p| p[1]).fold(f64::MIN, f64::max)];
                hulls.push(Hull { name: name.clone(), fam, pts, at, label: k == 0, ids: part.iter().map(|&i| mine[i].id.clone()).collect() });
            }
        }
    }
    ChartOut { id: c.id, x: axis(&c.x, c.log_x, lx), y: axis(&c.y, c.log_y, ly), families, bubbles, hulls, missing, bands: vec![] }
}
