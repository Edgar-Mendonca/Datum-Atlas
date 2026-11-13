//! Image analysis of micrographs: a grey image (one byte per pixel, decoded by the window), a scale,
//! a region of interest, optional smoothing, a threshold (given, or found by Otsu's method) that
//! splits one phase from the rest; then the phase's area fraction (phase fraction, porosity) and its
//! separate particles, each with area, equivalent circle diameter, perimeter, circularity and aspect
//! ratio, and their statistics. Written from the textbook methods.
use serde::{Deserialize, Serialize};

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut n) = (0u32, 0);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        let v = B64.iter().position(|&x| x == c).ok_or("The image data is damaged.")? as u32;
        acc = acc << 6 | v;
        n += 6;
        if n >= 8 { n -= 8; out.push((acc >> n) as u8); }
    }
    Ok(out)
}
pub fn b64_encode(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let v = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 { s.push(if k <= c.len() { B64[(v >> (18 - 6 * k) & 63) as usize] as char } else { '=' }); }
    }
    s
}

pub const MAX_PIXELS: usize = 16_000_000;

/// What to measure. `px` is the grey image, base64, `w` × `h` bytes.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Ask {
    pub w: usize, pub h: usize, pub px: String,
    /// µm per pixel; 0 when the image is not calibrated (results in pixels).
    pub scale: f64,
    /// Region measured: [x0, y0, x1, y1] in pixels (x1, y1 exclusive); all zero for the whole image.
    pub roi: [usize; 4],
    /// Passes of a 3 × 3 average before the threshold.
    pub smooth: u8,
    /// The threshold (0–255), or none to find it by Otsu's method.
    pub threshold: Option<u8>,
    /// The phase is darker than the threshold (pores, graphite, dark particles); else brighter.
    pub dark: bool,
    /// Smallest particle counted, in pixels.
    pub min_area: usize,
    /// Leave out particles that touch the edge of the region (their size is not known).
    pub edge: bool,
}
impl Default for Ask { fn default() -> Self { Ask { w: 0, h: 0, px: String::new(), scale: 0.0, roi: [0; 4], smooth: 0, threshold: None, dark: true, min_area: 4, edge: true } } }

#[derive(Debug, Clone, Serialize)]
pub struct Particle { pub area: f64, pub ecd: f64, pub perim: f64, pub circ: f64, pub aspect: f64, pub cx: f64, pub cy: f64, pub edge: bool }

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats { pub count: usize, pub density: Option<f64>, pub mean: Option<f64>, pub sd: Option<f64>, pub d10: Option<f64>, pub d50: Option<f64>, pub d90: Option<f64>, pub aspect: Option<f64>, pub circ: Option<f64>, pub mean_area: Option<f64> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub threshold: u8, pub otsu: u8, pub hist: Vec<u32>,
    /// Area fraction of the phase in the region, %.
    pub fraction: f64,
    /// "µm" when calibrated, else "px".
    pub unit: String,
    pub particles: Vec<Particle>, pub stats: Stats,
    /// Equivalent diameters in bins: [from, to, count].
    pub bins: Vec<[f64; 3]>,
    /// Per pixel: 0 the rest, 1 a counted particle, 2 phase not counted (small or on the edge), 3 outside the region; base64.
    pub mask: String,
    pub notes: Vec<String>,
}

/// Otsu's threshold: the grey level that best separates the histogram into two classes.
pub fn otsu(hist: &[u32]) -> u8 {
    let total: f64 = hist.iter().map(|&c| c as f64).sum();
    let sum: f64 = hist.iter().enumerate().map(|(i, &c)| i as f64 * c as f64).sum();
    let (mut wb, mut sb, mut best, mut t) = (0.0, 0.0, -1.0, 0u8);
    for (i, &c) in hist.iter().enumerate() {
        wb += c as f64;
        if wb == 0.0 { continue }
        let wf = total - wb;
        if wf == 0.0 { break }
        sb += i as f64 * c as f64;
        let (mb, mf) = (sb / wb, (sum - sb) / wf);
        let v = wb * wf * (mb - mf).powi(2);
        if v > best { best = v; t = i as u8 }
    }
    t
}

fn pct(v: &[f64], p: f64) -> Option<f64> {
    if v.is_empty() { return None }
    let x = p * (v.len() - 1) as f64;
    let (i, f) = (x.floor() as usize, x.fract());
    Some(v[i] + f * (v[(i + 1).min(v.len() - 1)] - v[i]))
}

pub fn analyse(q: &Ask) -> Result<Answer, String> {
    let (w, h) = (q.w, q.h);
    if w == 0 || h == 0 || w * h > MAX_PIXELS { return Err(format!("The image must have pixels, and at most {} million.", MAX_PIXELS / 1_000_000)) }
    let mut g = b64_decode(&q.px)?;
    if g.len() != w * h { return Err("The image data does not match its size.".into()) }
    if !(q.scale.is_finite() && (0.0..=1e5).contains(&q.scale)) { return Err("The scale is a number of µm per pixel, from 0 to 100,000.".into()) }
    let [x0, y0, mut x1, mut y1] = q.roi;
    if x1 == 0 { x1 = w }
    if y1 == 0 { y1 = h }
    let (x1, y1) = (x1.min(w), y1.min(h));
    if x0 + 2 > x1 || y0 + 2 > y1 { return Err("The region to measure is empty.".into()) }
    for _ in 0..q.smooth.min(10) {
        let s = g.clone();
        for y in 0..h { for x in 0..w {
            let (mut t, mut n) = (0u32, 0u32);
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) { for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) { t += s[yy * w + xx] as u32; n += 1 } }
            g[y * w + x] = (t / n) as u8;
        }}
    }
    let mut hist = vec![0u32; 256];
    for y in y0..y1 { for x in x0..x1 { hist[g[y * w + x] as usize] += 1 } }
    let o = otsu(&hist);
    let t = q.threshold.unwrap_or(o);
    let inside = |x: usize, y: usize| x >= x0 && x < x1 && y >= y0 && y < y1;
    let phase = |v: u8| if q.dark { v <= t } else { v > t };
    let mut mask = vec![3u8; w * h];
    let mut on = 0usize;
    for y in y0..y1 { for x in x0..x1 { let i = y * w + x; let p = phase(g[i]); mask[i] = if p { 2 } else { 0 }; on += p as usize } }
    let n_roi = (x1 - x0) * (y1 - y0);
    // Particles: 8-connected groups of phase pixels.
    let (sc, unit) = if q.scale > 0.0 { (q.scale, "µm") } else { (1.0, "px") };
    let mut seen = vec![false; w * h];
    let mut parts = vec![];
    let mut stack = vec![];
    let mut members: Vec<usize> = vec![];
    for y in y0..y1 { for x in x0..x1 {
        let i = y * w + x;
        if mask[i] != 2 || seen[i] { continue }
        members.clear();
        stack.push(i);
        seen[i] = true;
        let mut edge = false;
        while let Some(j) = stack.pop() {
            members.push(j);
            let (jx, jy) = (j % w, j / w);
            if jx == x0 || jy == y0 || jx + 1 == x1 || jy + 1 == y1 { edge = true }
            for dy in -1i64..=1 { for dx in -1i64..=1 {
                let (nx, ny) = (jx as i64 + dx, jy as i64 + dy);
                if (dx, dy) == (0, 0) || nx < 0 || ny < 0 { continue }
                let (nx, ny) = (nx as usize, ny as usize);
                if !inside(nx, ny) { continue }
                let k = ny * w + nx;
                if mask[k] == 2 && !seen[k] { seen[k] = true; stack.push(k) }
            }}
        }
        let a = members.len();
        // Perimeter: pixel edges facing the rest, × π/4 to correct the staircase (Crofton).
        let mut edges = 0usize;
        let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for &j in &members {
            let (jx, jy) = (j % w, j / w);
            for (nx, ny) in [(jx as i64 - 1, jy as i64), (jx as i64 + 1, jy as i64), (jx as i64, jy as i64 - 1), (jx as i64, jy as i64 + 1)] {
                let out = nx < 0 || ny < 0 || !inside(nx as usize, ny as usize) || !phase(g[ny as usize * w + nx as usize]);
                edges += out as usize;
            }
            let (fx, fy) = (jx as f64, jy as f64);
            sx += fx; sy += fy; sxx += fx * fx; syy += fy * fy; sxy += fx * fy;
        }
        let n = a as f64;
        let (cx, cy) = (sx / n, sy / n);
        let (vxx, vyy, vxy) = (sxx / n - cx * cx + 1.0 / 12.0, syy / n - cy * cy + 1.0 / 12.0, sxy / n - cx * cy);
        let (m, d) = ((vxx + vyy) / 2.0, (((vxx - vyy) / 2.0).powi(2) + vxy * vxy).sqrt());
        let aspect = ((m + d) / (m - d).max(1e-9)).sqrt();
        let counted = a >= q.min_area.max(1) && !(q.edge && edge);
        if counted { for &j in &members { mask[j] = 1 } }
        if a < q.min_area.max(1) { continue }
        let area = n * sc * sc;
        let perim = edges as f64 * std::f64::consts::FRAC_PI_4 * sc;
        parts.push(Particle { area, ecd: 2.0 * (area / std::f64::consts::PI).sqrt(), perim, circ: (4.0 * std::f64::consts::PI * area / (perim * perim)).min(1.0), aspect, cx: cx * sc, cy: cy * sc, edge });
    }}
    let use_: Vec<&Particle> = parts.iter().filter(|p| !(q.edge && p.edge)).collect();
    let mut d: Vec<f64> = use_.iter().map(|p| p.ecd).collect();
    d.sort_by(f64::total_cmp);
    let k = d.len();
    let mean = (k > 0).then(|| d.iter().sum::<f64>() / k as f64);
    let avg = |f: &dyn Fn(&Particle) -> f64| (k > 0).then(|| use_.iter().map(|p| f(p)).sum::<f64>() / k as f64);
    let stats = Stats { count: k, density: (q.scale > 0.0).then(|| k as f64 / (n_roi as f64 * q.scale * q.scale * 1e-6)),
        mean, sd: mean.filter(|_| k > 1).map(|m| (d.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (k - 1) as f64).sqrt()),
        d10: pct(&d, 0.1), d50: pct(&d, 0.5), d90: pct(&d, 0.9), aspect: avg(&|p| p.aspect), circ: avg(&|p| p.circ), mean_area: avg(&|p| p.area) };
    let bins = match (d.first(), d.last()) {
        (Some(&lo), Some(&hi)) if hi > lo => { let nb = 12usize; let wd = (hi - lo) / nb as f64; (0..nb).map(|b| { let a = lo + b as f64 * wd; [a, a + wd, d.iter().filter(|&&x| x >= a && (x < a + wd || (b == nb - 1 && x <= hi))).count() as f64] }).collect() }
        (Some(&lo), _) => vec![[lo, lo, k as f64]],
        _ => vec![],
    };
    let mut notes = vec![];
    if q.scale <= 0.0 { notes.push("Not calibrated: sizes are in pixels. Set the scale from the scale bar for µm.".into()) }
    if on as f64 > 0.6 * n_roi as f64 { notes.push("The phase covers most of the region: check that the right phase (dark or bright) is chosen.".into()) }
    if parts.len() > 20_000 { notes.push("Very many particles: the threshold may be catching noise; smooth or raise the smallest size.".into()) }
    parts.truncate(5000);
    Ok(Answer { threshold: t, otsu: o, hist, fraction: on as f64 / n_roi as f64 * 100.0, unit: unit.into(), particles: parts, stats, bins, mask: b64_encode(&mask), notes })
}

/// A made-up test image (named so where it is used): dark round particles of known sizes on a
/// noisy bright background, with a black data bar along the bottom. Returns (w, h, grey pixels).
pub fn example() -> (usize, usize, Vec<u8>) {
    let (w, h) = (640usize, 480usize);
    let mut s = 0x1234_5678_9abc_def1u64;
    let mut r = move || { s ^= s << 13; s ^= s >> 7; s ^= s << 17; (s >> 11) as f64 / (1u64 << 53) as f64 };
    let mut px: Vec<u8> = (0..w * h).map(|_| 0u8).collect();
    for v in px.iter_mut() { *v = (190.0 + 24.0 * (r() - 0.5)) as u8 }
    let mut discs: Vec<(f64, f64, f64)> = vec![];
    while discs.len() < 60 {
        let (x, y, rad) = (r() * w as f64, r() * 420.0, 4.0 + 10.0 * r());
        if discs.iter().all(|d| ((d.0 - x).powi(2) + (d.1 - y).powi(2)).sqrt() > d.2 + rad + 3.0) { discs.push((x, y, rad)) }
    }
    for (cx, cy, rad) in discs { for y in 0..h { for x in 0..w { if ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)).sqrt() <= rad { px[y * w + x] = (60.0 + 24.0 * (r() - 0.5)) as u8 } } } }
    for y in 440..h { for x in 0..w { px[y * w + x] = if (20..120).contains(&x) && (455..460).contains(&y) { 255 } else { 0 } } }
    (w, h, px)
}
