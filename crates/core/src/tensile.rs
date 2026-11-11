//! Tensile tests: specimens with their load–extension or stress–strain data (saved in the project),
//! and what follows from each curve, after the usual test-standard definitions: modulus E from a
//! straight-line fit, 0.2 % proof stress Rp0.2, tensile strength Rm, uniform elongation Ag,
//! elongation after fracture A, the work to fracture, and the strain-hardening exponent n and
//! strength coefficient K (σ = K εⁿ on true stress and true plastic strain). Then the mean,
//! standard deviation and range over the specimens. A value that cannot be worked out is a gap.
use crate::export::{Cell, Table};
use serde::{Deserialize, Serialize};

pub const MAX_SPECIMENS: usize = 24;
pub const MAX_POINTS: usize = 200_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Specimen {
    pub name: String,
    /// What the first column holds: "ext" (extension, mm), "strain" (mm/mm) or "strain%".
    pub x: String,
    /// What the second column holds: "N", "kN" (load) or "MPa" (stress).
    pub y: String,
    /// Cross-section: "rect" (w × t), "round" (diameter d) or "area" (given).
    pub shape: String,
    pub w: f64,
    pub t: f64,
    pub d: f64,
    pub area: f64,
    /// Gauge length, mm (for extension).
    pub l0: f64,
    pub on: bool,
    pub data: Vec<[f64; 2]>,
}
impl Default for Specimen {
    fn default() -> Self { Specimen { name: String::new(), x: "ext".into(), y: "N".into(), shape: "round".into(), w: 0.0, t: 0.0, d: 6.0, area: 0.0, l0: 30.0, on: true, data: vec![] } }
}

/// The tensile tests of a project and how the modulus is fitted (a window of stress, as fractions
/// of the tensile strength, on the loading part of the curve).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tensile { pub specimens: Vec<Specimen>, pub fit: [f64; 2], pub material: String }
impl Default for Tensile { fn default() -> Self { Tensile { specimens: vec![], fit: [0.1, 0.4], material: String::new() } } }

impl Specimen {
    /// Cross-section area, mm².
    pub fn a0(&self) -> f64 { match self.shape.as_str() { "rect" => self.w * self.t, "round" => std::f64::consts::PI * self.d * self.d / 4.0, _ => self.area } }
    /// Engineering strain (mm/mm) and stress (MPa), in the order recorded.
    pub fn curve(&self) -> Result<Vec<[f64; 2]>, String> {
        let a = self.a0();
        if self.y != "MPa" && !(a.is_finite() && a > 0.0) { return Err(format!("{}: give the cross-section to turn load into stress.", self.name)) }
        if self.x == "ext" && !(self.l0.is_finite() && self.l0 > 0.0) { return Err(format!("{}: give the gauge length to turn extension into strain.", self.name)) }
        Ok(self.data.iter().filter(|p| p[0].is_finite() && p[1].is_finite()).map(|p| {
            let e = match self.x.as_str() { "ext" => p[0] / self.l0, "strain%" => p[0] / 100.0, _ => p[0] };
            let s = match self.y.as_str() { "kN" => p[1] * 1000.0 / a, "MPa" => p[1], _ => p[1] / a };
            [e, s]
        }).collect())
    }
}

impl Tensile {
    pub fn validate(&self) -> Result<(), String> {
        if self.specimens.len() > MAX_SPECIMENS { return Err(format!("Up to {MAX_SPECIMENS} specimens in one project.")) }
        if !(0.0..1.0).contains(&self.fit[0]) || !(self.fit[0] < self.fit[1] && self.fit[1] <= 1.0) { return Err("The modulus window is two fractions of the tensile strength, the first smaller, e.g. 0.1 to 0.4.".into()) }
        for s in &self.specimens {
            if s.data.len() > MAX_POINTS { return Err(format!("{}: up to {MAX_POINTS} points.", s.name)) }
            if [s.w, s.t, s.d, s.area, s.l0].iter().any(|v| !(v.is_finite() && *v >= 0.0)) { return Err(format!("{}: sizes are numbers of zero or more.", s.name)) }
            if !["ext", "strain", "strain%"].contains(&s.x.as_str()) || !["N", "kN", "MPa"].contains(&s.y.as_str()) || !["rect", "round", "area"].contains(&s.shape.as_str()) { return Err(format!("{}: unknown column kind or shape.", s.name)) }
        }
        Ok(())
    }
}

/// One specimen's results. Strengths in MPa, modulus in GPa, elongations in %, work in MJ/m³.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Res {
    pub name: String, pub on: bool, pub points: usize, pub area: f64,
    pub e: Option<f64>, pub rp02: Option<f64>, pub rm: Option<f64>, pub ag: Option<f64>, pub a: Option<f64>, pub work: Option<f64>, pub n: Option<f64>, pub k: Option<f64>,
    /// Strain offset where the fitted line meets zero stress (slack and seating at the start).
    pub toe: f64,
    /// The curve to draw (strain %, MPa), thinned to about 600 points; the true curve up to Ag.
    pub curve: Vec<[f64; 2]>, pub truec: Vec<[f64; 2]>,
    /// Points on the curve: [strain %, MPa] of Rp0.2, Rm and fracture.
    pub marks: Vec<(String, [f64; 2])>,
    pub notes: Vec<String>,
}

/// Mean, standard deviation (n − 1), lowest and highest over the specimens that are on.
#[derive(Debug, Clone, Serialize)]
pub struct Stat { pub key: &'static str, pub name: &'static str, pub unit: &'static str, pub n: usize, pub mean: Option<f64>, pub sd: Option<f64>, pub min: Option<f64>, pub max: Option<f64> }

#[derive(Debug, Clone, Default, Serialize)]
pub struct Out { pub rows: Vec<Res>, pub stats: Vec<Stat> }

pub const RESULTS: &[(&str, &str, &str)] = &[("e", "Young's modulus E", "GPa"), ("rp02", "0.2 % proof stress Rp0.2", "MPa"), ("rm", "Tensile strength Rm", "MPa"), ("ag", "Uniform elongation Ag", "%"),
    ("a", "Elongation after fracture A", "%"), ("work", "Work to fracture", "MJ/m³"), ("n", "Strain-hardening exponent n", ""), ("k", "Strength coefficient K", "MPa")];

fn get(r: &Res, k: &str) -> Option<f64> { match k { "e" => r.e, "rp02" => r.rp02, "rm" => r.rm, "ag" => r.ag, "a" => r.a, "work" => r.work, "n" => r.n, "k" => r.k, _ => None } }

/// Least squares y = m x + b.
fn line(p: &[(f64, f64)]) -> Option<(f64, f64)> {
    let n = p.len() as f64;
    if p.len() < 3 { return None }
    let (sx, sy) = p.iter().fold((0.0, 0.0), |a, q| (a.0 + q.0, a.1 + q.1));
    let (mx, my) = (sx / n, sy / n);
    let (sxx, sxy) = p.iter().fold((0.0, 0.0), |a, q| (a.0 + (q.0 - mx).powi(2), a.1 + (q.0 - mx) * (q.1 - my)));
    (sxx > 0.0).then(|| { let m = sxy / sxx; (m, my - m * mx) })
}

fn thin(v: impl Iterator<Item = [f64; 2]> + Clone, n: usize) -> Vec<[f64; 2]> {
    let len = v.clone().count();
    let step = len.div_ceil(n).max(1);
    let mut out: Vec<[f64; 2]> = v.clone().step_by(step).collect();
    if let Some(l) = v.last() { if out.last() != Some(&l) { out.push(l) } }
    out
}

/// The results of one specimen.
pub fn analyse(s: &Specimen, fit: [f64; 2]) -> Res {
    let mut r = Res { name: s.name.clone(), on: s.on, points: s.data.len(), area: s.a0(), ..Default::default() };
    let c = match s.curve() { Ok(c) => c, Err(e) => { r.notes.push(e); return r } };
    if c.len() < 10 { r.notes.push("Fewer than 10 points: nothing to work out.".into()); return r }
    let (iu, rm) = c.iter().enumerate().fold((0, f64::MIN), |a, (i, p)| if p[1] > a.1 { (i, p[1]) } else { a });
    if rm <= 0.0 { r.notes.push("The stress never rises above zero.".into()); return r }
    r.rm = Some(rm);
    // Modulus: the loading part up to the first point above the window's top.
    let top = c.iter().position(|p| p[1] > fit[1] * rm).unwrap_or(iu);
    let win: Vec<(f64, f64)> = c[..=top.min(iu)].iter().filter(|p| p[1] >= fit[0] * rm && p[1] <= fit[1] * rm).map(|p| (p[0], p[1])).collect();
    let fitted = line(&win).filter(|(m, _)| *m > 0.0);
    if s.x == "ext" { r.notes.push("Strain is from extension over the gauge length: if that is the machine's crosshead, the modulus comes out low (the machine stretches too). Use an extensometer for E.".into()) }
    // Fracture: the last point still carrying a tenth of the tensile strength.
    let jf = (iu..c.len()).rev().find(|&i| c[i][1] >= 0.1 * rm).unwrap_or(iu);
    r.work = Some(c[..=jf].windows(2).map(|w| (w[1][0] - w[0][0]) * (w[1][1] + w[0][1]) / 2.0).sum());
    let mut marks = vec![];
    if let Some((m, b)) = fitted {
        let e0 = -b / m;
        r.toe = e0;
        r.e = Some(m / 1000.0);
        let pl = |p: &[f64; 2]| p[0] - e0 - p[1] / m;
        // Rp0.2: where the curve meets the line of slope E offset by 0.2 % strain.
        let f = |p: &[f64; 2]| p[1] - m * (p[0] - e0 - 0.002);
        let st = c.iter().position(|p| p[1] >= fit[0] * rm).unwrap_or(0);
        if let Some(i) = (st.max(1)..=iu).find(|&i| f(&c[i]) <= 0.0 && f(&c[i - 1]) > 0.0) {
            let (p, q) = (c[i - 1], c[i]);
            let t = f(&p) / (f(&p) - f(&q));
            let x = [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])];
            r.rp02 = Some(x[1]);
            marks.push(("Rp0.2".to_string(), [x[0] * 100.0, x[1]]));
        } else { r.notes.push("The curve does not cross the 0.2 % offset line before the maximum: no proof stress.".into()) }
        r.ag = Some((pl(&c[iu]) * 100.0).max(0.0));
        r.a = Some((pl(&c[jf]) * 100.0).max(0.0));
        // n and K: true stress against true plastic strain, from 0.2 % plastic strain to Ag.
        let pts: Vec<(f64, f64)> = c[..=iu].iter().filter_map(|p| {
            let e = p[0] - e0;
            let (st, et) = (p[1] * (1.0 + e), (1.0 + e).ln());
            let ep = et - st / m;
            (ep >= 0.002 && st > 0.0 && pl(p) >= 0.002).then(|| (ep.ln(), st.ln()))
        }).collect();
        match line(&pts) {
            Some((n, lk)) if pts.len() >= 5 && n > 0.0 && n < 1.0 => { r.n = Some(n); r.k = Some(lk.exp()) }
            _ => r.notes.push("Too little plastic strain before the maximum to fit n and K (at least 5 points between 0.2 % plastic strain and Ag).".into()),
        }
        r.truec = thin(c[..=iu].iter().map(|p| { let e = p[0] - e0; [(1.0 + e).ln() * 100.0, p[1] * (1.0 + e)] }).filter(|p| p[0] >= 0.0), 300);
    } else { r.notes.push(format!("No straight part between {:.0} % and {:.0} % of Rm to fit the modulus: change the window.", fit[0] * 100.0, fit[1] * 100.0)) }
    marks.push(("Rm".into(), [c[iu][0] * 100.0, rm]));
    marks.push(("Fracture".into(), [c[jf][0] * 100.0, c[jf][1]]));
    r.marks = marks;
    r.curve = thin(c[..=jf.min(c.len() - 1)].iter().map(|p| [p[0] * 100.0, p[1]]), 600);
    r
}

/// Every specimen's results and the statistics over those that are on.
pub fn run(t: &Tensile) -> Out {
    let rows: Vec<Res> = t.specimens.iter().map(|s| analyse(s, t.fit)).collect();
    let stats = RESULTS.iter().map(|&(key, name, unit)| {
        let v: Vec<f64> = rows.iter().filter(|r| r.on).filter_map(|r| get(r, key)).collect();
        let n = v.len();
        let mean = (n > 0).then(|| v.iter().sum::<f64>() / n as f64);
        let sd = mean.filter(|_| n > 1).map(|m| (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt());
        Stat { key, name, unit, n, mean, sd, min: v.iter().copied().reduce(f64::min), max: v.iter().copied().reduce(f64::max) }
    }).collect();
    Out { rows, stats }
}

/// The results as a table for a spreadsheet.
pub fn table(o: &Out) -> Table {
    let mut head = vec!["Specimen".to_string()];
    head.extend(RESULTS.iter().map(|(_, n, u)| if u.is_empty() { n.to_string() } else { format!("{n} ({u})") }));
    head.push("Notes".into());
    let num = |v: Option<f64>| v.map_or(Cell::Empty, Cell::Num);
    let mut rows: Vec<Vec<Cell>> = o.rows.iter().map(|r| { let mut x = vec![Cell::Text(format!("{}{}", r.name, if r.on { "" } else { " (left out)" }))]; x.extend(RESULTS.iter().map(|(k, _, _)| num(get(r, k)))); x.push(Cell::Text(r.notes.join(" "))); x }).collect();
    for (lab, f) in [("Mean", 0), ("Standard deviation", 1), ("Lowest", 2), ("Highest", 3)] {
        let mut x = vec![Cell::Text(lab.into())];
        x.extend(o.stats.iter().map(|s| num([s.mean, s.sd, s.min, s.max][f])));
        x.push(Cell::Empty);
        rows.push(x);
    }
    Table { head, rows }
}

/// Columns read from a file: the heading row (the row before the first row of numbers) and the numbers.
#[derive(Debug, Clone, Serialize)]
pub struct Cols { pub head: Vec<String>, pub rows: Vec<Vec<Option<f64>>>, pub x: usize, pub y: usize, pub xk: String, pub yk: String, pub skipped: usize }

fn num(s: &str) -> Option<f64> {
    let t = s.trim();
    let t = if t.contains(',') && !t.contains('.') { t.replace(',', ".") } else { t.replace(',', "") };
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Read a test machine's export: header lines are skipped, the headings are the last text row before
/// the numbers; the columns for strain or extension and for stress or load are guessed from them.
pub fn columns(raw: &[Vec<String>]) -> Result<Cols, String> {
    let first = raw.iter().position(|r| r.iter().filter(|c| num(c).is_some()).count() >= 2).ok_or("No rows with two numbers or more.")?;
    let width = raw[first..].iter().map(Vec::len).max().unwrap_or(0);
    let head: Vec<String> = (0..width).map(|k| first.checked_sub(1).and_then(|i| raw[i].get(k)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or(format!("Column {}", k + 1))).collect();
    let rows: Vec<Vec<Option<f64>>> = raw[first..].iter().map(|r| (0..width).map(|k| r.get(k).and_then(|c| num(c))).collect()).collect();
    let low: Vec<String> = head.iter().map(|h| h.to_lowercase()).collect();
    let find = |words: &[&str]| low.iter().position(|h| words.iter().any(|w| h.contains(w)));
    let y = find(&["stress", "load", "force", "kraft", "mpa"]).unwrap_or(width.min(2) - 1);
    let x = find(&["strain", "ext", "elong", "displac", "position", "dehnung", "mm"]).filter(|&x| x != y).unwrap_or(if y == 0 { 1 } else { 0 });
    let xk = if low[x].contains("strain") || low[x].contains("dehnung") { if low[x].contains('%') { "strain%" } else { "strain" } } else { "ext" };
    let yk = if low[y].contains("mpa") || low[y].contains("n/mm") || low[y].contains("stress") { "MPa" } else if low[y].contains("kn") { "kN" } else { "N" };
    Ok(Cols { head, skipped: first, rows, x, y, xk: xk.into(), yk: yk.into() })
}

/// The points of two columns, leaving out rows where either is not a number.
pub fn pick(c: &Cols, x: usize, y: usize) -> Vec<[f64; 2]> { c.rows.iter().filter_map(|r| Some([(*r.get(x)?)?, (*r.get(y)?)?])).collect() }

/// Made-up example data (clearly named so): three specimens of a steel-like curve with a little noise,
/// for trying the page. Not a measurement of any material.
pub fn example() -> Tensile {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut rnd = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5 };
    let specimens = [(780.0f64, 0.20f64, 0.30f64), (800.0, 0.19, 0.28), (770.0, 0.21, 0.31)].iter().enumerate().map(|(k, &(kk, n, ef))| {
        let (e_mod, d, l0) = (205_000.0, 6.0, 30.0);
        let a = std::f64::consts::PI * d * d / 4.0;
        let eu = n.exp() - 1.0;
        let s0 = kk * 0.002f64.powf(n);
        let true_at = |e: f64| { let et = (1.0 + e).ln(); let (mut lo, mut hi) = (0.0, 3000.0); for _ in 0..60 { let s = (lo + hi) / 2.0; if s / e_mod + if s < s0 { 0.0 } else { (s / kk).powf(1.0 / n) } > et { hi = s } else { lo = s } } lo };
        let su = true_at(eu) / (1.0 + eu);
        let mut data = vec![];
        let steps = 900;
        for i in 0..=steps {
            let e = ef * (i as f64 / steps as f64).powi(2); // dense at the start, where the modulus is fitted
            let s = if e <= eu { true_at(e) / (1.0 + e) } else { su * (1.0 - 0.45 * ((e - eu) / (ef - eu)).powi(2)) };
            data.push([e * l0 * (1.0 + 0.002 * rnd()), (s * a + 15.0 * rnd()).max(0.0)]);
        }
        data.push([ef * l0 * 1.001, 0.0]);
        Specimen { name: format!("Example {} (made-up data)", k + 1), x: "ext".into(), y: "N".into(), shape: "round".into(), d, l0, data, ..Default::default() }
    }).collect();
    Tensile { specimens, fit: [0.1, 0.4], material: "Example steel (made-up data)".into() }
}
