//! Digital image correlation (DIC) results: a displacement field (x, y, u, v on a regular grid,
//! all in one length unit) and the strains that follow by central differences (small strain):
//! εxx = ∂u/∂x, εyy = ∂v/∂y, εxy = ½(∂u/∂y + ∂v/∂x), rotation ω = ½(∂v/∂x − ∂u/∂y), the
//! principal strains, the maximum shear and the von Mises equivalent strain (with εzz = −(εxx + εyy)).
use crate::image::b64_encode;
use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct Dic { pub name: String, pub w: usize, pub h: usize, pub x0: f64, pub y0: f64, pub dx: f64, pub dy: f64, pub u: Vec<f64>, pub v: Vec<f64>, pub notes: Vec<String> }

pub const COMPONENTS: &[(&str, &str, bool)] = &[("exx", "εxx", true), ("eyy", "εyy", true), ("exy", "εxy", true), ("e1", "ε1 (major principal)", true), ("e2", "ε2 (minor principal)", true),
    ("gmax", "Maximum shear γmax", false), ("eeq", "Equivalent (von Mises)", false), ("rot", "Rotation ω (rad)", true), ("u", "Displacement u", true), ("v", "Displacement v", true)];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary { pub name: String, pub w: usize, pub h: usize, pub dx: f64, pub dy: f64, pub points: usize, pub valid: f64, pub notes: Vec<String> }

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stat { pub key: String, pub name: String, pub mean: Option<f64>, pub sd: Option<f64>, pub min: Option<f64>, pub max: Option<f64>, pub p5: Option<f64>, pub p95: Option<f64> }

fn num(s: &str) -> Option<f64> { let t = s.trim(); let t = if t.contains(',') && !t.contains('.') { t.replace(',', ".") } else { t.to_string() }; t.parse::<f64>().ok().filter(|v| v.is_finite()) }

/// Read rows of a table: headings guessed (x, y, u or ux or displacement x, v or uy); without
/// headings the first four number columns are x, y, u, v. Points go on a regular grid.
pub fn parse(rows: &[Vec<String>], name: &str) -> Result<Dic, String> {
    let first = rows.iter().position(|r| r.iter().filter(|c| num(c).is_some()).count() >= 4).ok_or("No rows with four numbers (x, y, u, v).")?;
    let head: Vec<String> = if first > 0 { rows[first - 1].iter().map(|h| h.trim().to_lowercase()).collect() } else { vec![] };
    let find = |names: &[&str]| head.iter().position(|h| { let b = h.split(['[', '(', ' ']).next().unwrap_or("").trim(); names.contains(&b) });
    let (cx, cy, cu, cv) = match (find(&["x", "x_pos", "xpos"]), find(&["y", "y_pos", "ypos"]), find(&["u", "ux", "dx", "displacement_x", "disp_x", "u_x"]), find(&["v", "uy", "dy", "displacement_y", "disp_y", "u_y"])) {
        (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
        _ => (0, 1, 2, 3),
    };
    let pts: Vec<[f64; 4]> = rows[first..].iter().filter_map(|r| Some([num(r.get(cx)?)?, num(r.get(cy)?)?, r.get(cu).and_then(|c| num(c)).unwrap_or(f64::NAN), r.get(cv).and_then(|c| num(c)).unwrap_or(f64::NAN)])).collect();
    if pts.len() < 9 { return Err("Fewer than 9 points.".into()) }
    let spacing = |k: usize| { let mut v: Vec<f64> = pts.iter().map(|p| p[k]).collect(); v.sort_by(f64::total_cmp); v.dedup_by(|a, b| (*a - *b).abs() < 1e-9); v.windows(2).map(|w| w[1] - w[0]).fold(f64::MAX, f64::min) };
    let (dx, dy) = (spacing(0), spacing(1));
    if !(dx.is_finite() && dy.is_finite() && dx > 0.0 && dy > 0.0 && dx < f64::MAX && dy < f64::MAX) { return Err("The points are not on a grid with two directions.".into()) }
    let (x0, y0) = pts.iter().fold((f64::MAX, f64::MAX), |a, p| (a.0.min(p[0]), a.1.min(p[1])));
    let (x1, y1) = pts.iter().fold((f64::MIN, f64::MIN), |a, p| (a.0.max(p[0]), a.1.max(p[1])));
    let (wf, hf) = (((x1 - x0) / dx).round() + 1.0, ((y1 - y0) / dy).round() + 1.0);
    if wf.is_nan() || hf.is_nan() || wf * hf > 20_000_000.0 { return Err("The grid is too large.".into()) }
    let (w, h) = (wf as usize, hf as usize);
    let mut d = Dic { name: name.into(), w, h, x0, y0, dx, dy, u: vec![f64::NAN; w * h], v: vec![f64::NAN; w * h], notes: vec![] };
    for p in &pts { let (i, j) = (((p[0] - x0) / dx).round() as usize, ((p[1] - y0) / dy).round() as usize); if i < w && j < h { d.u[j * w + i] = p[2]; d.v[j * w + i] = p[3] } }
    let filled = d.u.iter().filter(|x| x.is_finite()).count();
    if filled < w * h { d.notes.push(format!("{:.1} % of the grid has no displacement (outside the correlated area, or lost); strains there are blank.", (1.0 - filled as f64 / (w * h) as f64) * 100.0)) }
    Ok(d)
}

impl Dic {
    pub fn summary(&self) -> Summary { Summary { name: self.name.clone(), w: self.w, h: self.h, dx: self.dx, dy: self.dy, points: self.w * self.h, valid: self.u.iter().filter(|x| x.is_finite()).count() as f64 / (self.w * self.h) as f64 * 100.0, notes: self.notes.clone() } }
    /// A derivative of `f` at k along x (dir 0) or y (dir 1): central where both neighbours are valid, else one-sided.
    fn d(&self, f: &[f64], k: usize, dir: usize) -> f64 {
        let (i, j) = (k % self.w, k / self.w);
        let (n, s) = if dir == 0 { (self.w, self.dx) } else { (self.h, self.dy) };
        let at = |t: usize| if dir == 0 { f[j * self.w + t] } else { f[t * self.w + i] };
        let c = if dir == 0 { i } else { j };
        let (a, b) = (if c > 0 { at(c - 1) } else { f64::NAN }, if c + 1 < n { at(c + 1) } else { f64::NAN });
        let m = f[k];
        if a.is_finite() && b.is_finite() { (b - a) / (2.0 * s) } else if b.is_finite() && m.is_finite() { (b - m) / s } else if a.is_finite() && m.is_finite() { (m - a) / s } else { f64::NAN }
    }
    /// One component at every point (NaN where it cannot be worked out).
    pub fn field(&self, key: &str) -> Vec<f64> {
        (0..self.w * self.h).map(|k| {
            if !self.u[k].is_finite() || !self.v[k].is_finite() { return f64::NAN }
            if key == "u" { return self.u[k] }
            if key == "v" { return self.v[k] }
            let (ux, uy, vx, vy) = (self.d(&self.u, k, 0), self.d(&self.u, k, 1), self.d(&self.v, k, 0), self.d(&self.v, k, 1));
            let (xx, yy, xy) = (ux, vy, 0.5 * (uy + vx));
            let (c, r) = ((xx + yy) / 2.0, (((xx - yy) / 2.0).powi(2) + xy * xy).sqrt());
            match key { "exx" => xx, "eyy" => yy, "exy" => xy, "e1" => c + r, "e2" => c - r, "gmax" => r, "rot" => 0.5 * (vx - uy),
                _ => { let zz = -(xx + yy); (2.0 / 3.0 * (xx * xx + yy * yy + zz * zz + 2.0 * xy * xy)).sqrt() } }
        }).collect()
    }
    pub fn stats(&self, key: &str) -> Stat {
        let mut v: Vec<f64> = self.field(key).into_iter().filter(|x| x.is_finite()).collect();
        v.sort_by(f64::total_cmp);
        let n = v.len();
        let pc = |p: f64| (n > 0).then(|| v[((n - 1) as f64 * p).round() as usize]);
        let mean = (n > 0).then(|| v.iter().sum::<f64>() / n as f64);
        Stat { key: key.into(), name: COMPONENTS.iter().find(|c| c.0 == key).map_or(key, |c| c.1).into(), mean, sd: mean.filter(|_| n > 1).map(|m| (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()), min: v.first().copied(), max: v.last().copied(), p5: pc(0.05), p95: pc(0.95) }
    }
    /// A picture of a component: diverging blue–white–red for signed ones (symmetric about 0), white to violet to amber for the others,
    /// with the colour range from the 1st to the 99th percentile unless `range` is given. Returns (RGB base64, [lo, hi]).
    pub fn draw(&self, key: &str, range: Option<[f64; 2]>) -> (String, [f64; 2]) {
        let signed = COMPONENTS.iter().find(|c| c.0 == key).is_none_or(|c| c.2);
        paint(&self.field(key), signed, range)
    }
    /// The values held in `u`, coloured as an unsigned or signed component.
    fn draw_raw(&self, unsigned: bool) -> (String, [f64; 2]) { paint(&self.u, !unsigned, None) }
}

fn paint(f: &[f64], signed: bool, range: Option<[f64; 2]>) -> (String, [f64; 2]) {
    {
        let r = range.unwrap_or_else(|| {
            let mut v: Vec<f64> = f.iter().copied().filter(|x| x.is_finite()).collect();
            v.sort_by(f64::total_cmp);
            if v.is_empty() { return [0.0, 1.0] }
            let (lo, hi) = (v[(v.len() - 1) / 100], v[(v.len() - 1) * 99 / 100]);
            if signed { let m = lo.abs().max(hi.abs()).max(1e-12); [-m, m] } else { [lo.max(0.0), hi.max(lo + 1e-12)] }
        });
        let mut out = Vec::with_capacity(f.len() * 3);
        for x in f {
            let c = if !x.is_finite() { [70, 70, 70] } else {
                let t = ((x - r[0]) / (r[1] - r[0])).clamp(0.0, 1.0);
                if signed { if t < 0.5 { let s = t * 2.0; [(40.0 + 215.0 * s) as u8, (90.0 + 165.0 * s) as u8, (200.0 + 55.0 * s) as u8] } else { let s = (t - 0.5) * 2.0; [255, (255.0 - 205.0 * s) as u8, (255.0 - 215.0 * s) as u8] } }
                else if t < 0.5 { let s = t * 2.0; [(255.0 - 147.0 * s) as u8, (255.0 - 181.0 * s) as u8, (255.0 - 73.0 * s) as u8] } else { let s = (t - 0.5) * 2.0; [(108.0 + 134.0 * s) as u8, (74.0 + 104.0 * s) as u8, (182.0 - 131.0 * s) as u8] }
            };
            out.extend_from_slice(&c);
        }
        (b64_encode(&out), r)
    }
}

impl Dic {
    /// Values along a line between two points given in grid units (bilinear), `n` samples: [distance in field units, value].
    pub fn profile(&self, key: &str, a: [f64; 2], b: [f64; 2], n: usize) -> Vec<[f64; 2]> {
        let f = self.field(key);
        let n = n.clamp(2, 2000);
        let len = ((b[0] - a[0]) * self.dx).hypot((b[1] - a[1]) * self.dy);
        (0..n).map(|s| {
            let t = s as f64 / (n - 1) as f64;
            let (x, y) = (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]));
            let (i, j) = (x.floor().clamp(0.0, (self.w - 1) as f64) as usize, y.floor().clamp(0.0, (self.h - 1) as f64) as usize);
            let (i1, j1) = ((i + 1).min(self.w - 1), (j + 1).min(self.h - 1));
            let (fx, fy) = ((x - i as f64).clamp(0.0, 1.0), (y - j as f64).clamp(0.0, 1.0));
            let g = |ii: usize, jj: usize| f[jj * self.w + ii];
            let v = g(i, j) * (1.0 - fx) * (1.0 - fy) + g(i1, j) * fx * (1.0 - fy) + g(i, j1) * (1.0 - fx) * fy + g(i1, j1) * fx * fy;
            [t * len, v]
        }).collect()
    }
}

/// A made-up displacement field (named so): 2 % tension along x with a gentle waviness and two
/// shear bands at ±45°, on a 160 × 120 grid at 2 µm.
pub fn example() -> Dic {
    let (w, h, s) = (160usize, 120usize, 2.0);
    let mut d = Dic { name: "Example (made-up field)".into(), w, h, x0: 0.0, y0: 0.0, dx: s, dy: s, u: vec![0.0; w * h], v: vec![0.0; w * h], notes: vec!["A made-up displacement field: 2 % tension along x, waviness and two shear bands; not a measurement.".into()] };
    let mut seed = 0x51ED_2701_7B4A_9C33u64;
    let mut r = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5 };
    let band = |t: f64, c: f64| 0.5 * (1.0 + ((t - c) / 8.0).tanh()); // a smoothed step across a band
    for j in 0..h { for i in 0..w {
        let (x, y) = (i as f64 * s, j as f64 * s);
        let k = j * w + i;
        d.u[k] = 0.02 * x + 0.3 * (x / 50.0).sin() * (y / 40.0).cos() + 1.2 * band(x - y, 40.0) + 0.002 * r();
        d.v[k] = -0.01 * y + 0.8 * band(x + y, 260.0) + 0.002 * r();
    }}
    d
}

/// How to lay a DIC field over an EBSD map, and what to measure per grain.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct LinkAsk {
    /// Strain component (see COMPONENTS).
    pub key: String,
    /// True: stretch the field over the map's area. False: use coordinates, x_map = x_field × scale + offset.
    pub fit: bool,
    pub offset: [f64; 2],
    pub scale: f64,
    pub grains: crate::ebsd::GrainAsk,
}
impl Default for LinkAsk { fn default() -> Self { LinkAsk { key: "eeq".into(), fit: true, offset: [0.0, 0.0], scale: 1.0, grains: Default::default() } } }

#[derive(Debug, Clone, Serialize)]
pub struct GrainStrain { pub id: u32, pub points: usize, pub ecd: f64, pub mean: f64, pub sd: f64, pub schmid: f64, pub ipf: [u8; 3], pub cx: f64, pub cy: f64,
    /// Slip traces on the surface: (angle from X in degrees, the largest Schmid factor on that plane), strongest first.
    pub traces: Vec<(f64, f64)> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Linked { pub w: usize, pub h: usize, pub rgb: String, pub range: [f64; 2], pub grains: Vec<GrainStrain>, pub r_schmid: Option<f64>, pub covered: f64, pub notes: Vec<String> }

/// Strain by grain: the field sampled at each EBSD point, averaged over each grain, with the grain's Schmid factor and slip traces.
pub fn by_grain(d: &Dic, m: &crate::ebsd::Ebsd, q: &LinkAsk) -> Result<Linked, String> {
    use crate::orient;
    if !(q.scale.is_finite() && q.scale > 0.0 && q.offset.iter().all(|v| v.is_finite())) { return Err("The scale is above zero and the offset is two numbers.".into()) }
    let f = d.field(&q.key);
    let (ids, _) = m.grains(&q.grains);
    let (we, he) = (m.w, m.h);
    // The field's value at each map point (nearest grid point).
    let at: Vec<f64> = (0..we * he).map(|k| {
        let (i, j) = ((k % we) as f64, (k / we) as f64);
        let (gi, gj) = if q.fit { (i * (d.w - 1) as f64 / (we - 1).max(1) as f64, j * (d.h - 1) as f64 / (he - 1).max(1) as f64) }
            else { (((i * m.step - q.offset[0]) / q.scale - d.x0) / d.dx, ((j * m.step - q.offset[1]) / q.scale - d.y0) / d.dy) };
        let (ii, jj) = (gi.round(), gj.round());
        if ii < 0.0 || jj < 0.0 || ii >= d.w as f64 || jj >= d.h as f64 { f64::NAN } else { f[jj as usize * d.w + ii as usize] }
    }).collect();
    let covered = at.iter().filter(|v| v.is_finite()).count() as f64 / (we * he) as f64 * 100.0;
    // Per grain.
    let n = ids.iter().copied().max().unwrap_or(0) as usize;
    let mut acc = vec![(0usize, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0usize); n + 1]; // points, Σv, Σv², Σx, Σy, first point
    for k in 0..we * he {
        let g = ids[k] as usize;
        if g == 0 { continue }
        let a = &mut acc[g];
        if a.0 == 0 { a.5 = k }
        a.3 += (k % we) as f64; a.4 += (k / we) as f64;
        if at[k].is_finite() { a.0 += 1; a.1 += at[k]; a.2 += at[k] * at[k] }
    }
    let load = match q.grains.load.as_str() { "y" => [0.0, 1.0, 0.0], "z" => [0.0, 0.0, 1.0], _ => [1.0, 0.0, 0.0] };
    let cub = orient::slip_systems(if q.grains.slip == "bcc" { "bcc" } else { "fcc" });
    let hex = [orient::slip_systems("basal"), orient::slip_systems("prismatic")].concat();
    let mut all = vec![0usize; n + 1];
    for &g in &ids { all[g as usize] += 1 }
    let mut rows = vec![];
    for g in 1..=n {
        let a = acc[g];
        if a.0 < 3 { continue }
        let k = a.5;
        let gm = m.g(k);
        let ph = m.phase[k];
        let sys = if m.crystal(ph) == "hex" { &hex } else { &cub };
        // Traces: each distinct slip plane's line on the surface (normal Z), with its best Schmid factor.
        let mut tr: Vec<(f64, f64)> = vec![];
        let gt = orient::tr(&gm);
        for (pn, pd) in sys {
            let ns = orient::apply(&gt, *pn);
            let ang = (-ns[0]).atan2(ns[1]).to_degrees().rem_euclid(180.0);
            let c = orient::apply(&gm, load);
            let sf = ((c[0] * pn[0] + c[1] * pn[1] + c[2] * pn[2]) * (c[0] * pd[0] + c[1] * pd[1] + c[2] * pd[2])).abs();
            match tr.iter_mut().find(|t| (t.0 - ang).abs() < 0.5 || (t.0 - ang).abs() > 179.5) { Some(t) => t.1 = t.1.max(sf), None => tr.push((ang, sf)) }
        }
        tr.sort_by(|x, y| y.1.total_cmp(&x.1));
        let mean = a.1 / a.0 as f64;
        rows.push(GrainStrain { id: g as u32, points: all[g], ecd: 2.0 * (all[g] as f64 * m.step * m.step / std::f64::consts::PI).sqrt(), mean, sd: (a.2 / a.0 as f64 - mean * mean).max(0.0).sqrt(),
            schmid: orient::schmid(&gm, load, sys), ipf: orient::ipf(&gm, [0.0, 0.0, 1.0], m.crystal(ph)), cx: a.3 / all[g] as f64, cy: a.4 / all[g] as f64, traces: tr });
    }
    // Pearson r of grain strain against Schmid factor.
    let r_schmid = (rows.len() > 2).then(|| {
        let nn = rows.len() as f64;
        let (mx, my) = (rows.iter().map(|r| r.schmid).sum::<f64>() / nn, rows.iter().map(|r| r.mean).sum::<f64>() / nn);
        let (sxy, sxx, syy) = rows.iter().fold((0.0, 0.0, 0.0), |a, r| (a.0 + (r.schmid - mx) * (r.mean - my), a.1 + (r.schmid - mx).powi(2), a.2 + (r.mean - my).powi(2)));
        sxy / (sxx * syy).sqrt()
    }).filter(|v| v.is_finite());
    // The picture: the field on the map's grid, grain boundaries black.
    let (pic, range) = draw_values(&at, &q.key);
    let mut px = crate::image::b64_decode(&pic)?;
    for k in 0..we * he { let (i, j) = (k % we, k / we); if (i + 1 < we && ids[k + 1] != ids[k]) || (j + 1 < he && ids[k + we] != ids[k]) { px[3 * k..3 * k + 3].copy_from_slice(&[0, 0, 0]) } }
    rows.sort_by(|x, y| y.mean.total_cmp(&x.mean));
    let mut notes = vec![];
    if covered < 90.0 { notes.push(format!("The field covers {covered:.0} % of the map: check the scale and offset.")) }
    Ok(Linked { w: we, h: he, rgb: b64_encode(&px), range, grains: rows, r_schmid, covered, notes })
}

/// Colour values as `Dic::draw` does, for any grid.
fn draw_values(f: &[f64], key: &str) -> (String, [f64; 2]) {
    let d = Dic { name: String::new(), w: f.len(), h: 1, x0: 0.0, y0: 0.0, dx: 1.0, dy: 1.0, u: f.to_vec(), v: vec![0.0; f.len()], notes: vec![] };
    // "eeq" stands for every unsigned component (same scale), "u" for the signed ones; values are passed straight through.
    let unsigned = COMPONENTS.iter().find(|c| c.0 == key).is_some_and(|c| !c.2);
    d.draw_raw(unsigned)
}

/// A made-up field over an EBSD map (named so): each grain stretched in proportion to its Schmid
/// factor for a load along X (3 % on average), so strain by grain follows the Schmid factor.
pub fn example_on(m: &crate::ebsd::Ebsd) -> Dic {
    use crate::orient;
    let sys = orient::slip_systems("fcc");
    let mut e: Vec<f64> = (0..m.w * m.h).map(|k| if m.phase[k] > 0 { 0.03 * orient::schmid(&m.g(k), [1.0, 0.0, 0.0], &sys) / 0.45 } else { 0.0 }).collect();
    let (w, h) = (m.w, m.h);
    // Soften the steps at the boundaries a little (three 3 × 3 averages), as a real field would be.
    for _ in 0..3 {
        let s0 = e.clone();
        for j in 0..h { for i in 0..w {
            let (mut t, mut n) = (0.0, 0.0);
            for jj in j.saturating_sub(1)..=(j + 1).min(h - 1) { for ii in i.saturating_sub(1)..=(i + 1).min(w - 1) { t += s0[jj * w + ii]; n += 1.0 } }
            e[j * w + i] = t / n;
        }}
    }
    let mut d = Dic { name: "Example (made-up field over the map)".into(), w, h, x0: 0.0, y0: 0.0, dx: m.step, dy: m.step, u: vec![0.0; w * h], v: vec![0.0; w * h], notes: vec!["A made-up displacement field over the open EBSD map: each grain strains in proportion to its Schmid factor; not a measurement.".into()] };
    for j in 0..h { let mut acc = 0.0; for i in 0..w { let k = j * w + i; acc += e[k] * m.step; d.u[k] = acc } }
    for i in 0..w { let mut acc = 0.0; for j in 0..h { let k = j * w + i; acc -= 0.5 * e[k] * m.step; d.v[k] = acc } }
    d
}
