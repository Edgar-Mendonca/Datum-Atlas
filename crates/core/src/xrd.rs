//! X-ray diffraction, measured patterns: read text exports (.xy, .csv, .dat, .txt), the instrument
//! XML format (.xrdml) and the GSAS text format (.fxye); remove the background (SNIP), smooth
//! (Savitzky–Golay), strip Kα2 (Rachinger), find and fit peaks (pseudo-Voigt, our own
//! Levenberg–Marquardt), and work out crystallite size (Scherrer) and size with microstrain
//! (Williamson–Hall). Wavelengths: NIST X-ray transition energies (direct measurements).
use serde::{Deserialize, Serialize};

/// X-ray tube anodes: Kα1 and Kα2 wavelengths (Å), NIST SRD 128, direct experimental values.
pub const TUBES: &[(&str, f64, f64)] = &[("Cu", 1.540_592_9, 1.544_427_4), ("Co", 1.788_996, 1.792_835), ("Mo", 0.709_317_15, 0.713_607),
    ("Cr", 2.289_726, 2.293_651), ("Fe", 1.936_041, 1.939_973), ("Ag", 0.559_421_78, 0.563_813_1)];
/// The Kα doublet as one wavelength: (2·Kα1 + Kα2)/3, the usual weighting.
pub fn ka(anode: &str) -> Option<f64> { TUBES.iter().find(|t| t.0.eq_ignore_ascii_case(anode)).map(|t| (2.0 * t.1 + t.2) / 3.0) }
fn tube(anode: &str) -> Option<(f64, f64)> { TUBES.iter().find(|t| t.0.eq_ignore_ascii_case(anode)).map(|t| (t.1, t.2)) }

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pattern { pub name: String, pub x: Vec<f64>, pub y: Vec<f64>, /// The anode read from the file ("Cu"…), or "".
    pub anode: String, pub notes: Vec<String> }

pub const MAX_POINTS: usize = 200_000;

fn num(s: &str) -> Option<f64> { let t = s.trim(); t.parse::<f64>().ok().or_else(|| t.replace(',', ".").parse().ok()).filter(|v: &f64| v.is_finite()) }
fn between<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> { let i = s.find(a)? + a.len(); let j = s[i..].find(b)? + i; Some(&s[i..j]) }

/// Read a pattern. `name` is the file name (its extension picks the format when the text does not).
pub fn parse(text: &str, name: &str) -> Result<Pattern, String> {
    let low = name.to_lowercase();
    let mut p = if text.contains("<xrdMeasurement") || low.ends_with(".xrdml") { xrdml(text)? }
        else if text.lines().any(|l| l.starts_with("BANK")) || low.ends_with(".fxye") { fxye(text)? }
        else { columns(text)? };
    p.name = name.rsplit(['/', '\\']).next().unwrap_or(name).to_string();
    let n = p.x.len();
    if n < 10 { return Err("Fewer than ten points: is this a diffraction pattern (2θ and counts)?".into()) }
    if n > MAX_POINTS { return Err(format!("More than {MAX_POINTS} points; cut the range in your instrument's software first.")) }
    let mut ix: Vec<usize> = (0..n).collect();
    ix.sort_by(|&a, &b| p.x[a].total_cmp(&p.x[b]));
    if ix.windows(2).any(|w| w[1] < w[0]) { p.notes.push("The points were sorted by angle.".into()) }
    (p.x, p.y) = (ix.iter().map(|&i| p.x[i]).collect(), ix.iter().map(|&i| p.y[i]).collect());
    if !(p.x[0] >= 0.0 && p.x[n - 1] <= 180.0) { return Err("The first column should be 2θ in degrees (0 to 180).".into()) }
    if p.x[n - 1] - p.x[0] <= 0.0 { return Err("All the points are at the same angle.".into()) }
    Ok(p)
}

/// Two or more columns: 2θ and counts (a third column, such as errors, is ignored); lines that are
/// not numbers (headings, comments) are skipped.
fn columns(text: &str) -> Result<Pattern, String> {
    let mut p = Pattern::default();
    let semi = text.lines().take(50).any(|l| l.contains(';'));
    for l in text.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with(['#', '!', '%', '\'', '/', '"']) { continue }
        let parts: Vec<&str> = if semi || t.contains('\t') { t.split([';', '\t']).collect() } else if t.contains(',') && !t.contains(' ') && t.matches(',').count() == 1 && !t.contains('.') { vec![t] } else { t.split([',', ' ']).filter(|s| !s.is_empty()).collect() };
        let v: Vec<f64> = parts.iter().filter_map(|s| num(s)).collect();
        if v.len() >= 2 && v.len() == parts.iter().filter(|s| !s.trim().is_empty()).count() { p.x.push(v[0]); p.y.push(v[1]); }
    }
    if p.x.is_empty() { return Err("No columns of numbers found: the file should hold 2θ and counts.".into()) }
    Ok(p)
}

/// The instrument XML format: the scan's 2θ start and end, the counts (or intensities), and the anode's Kα1.
fn xrdml(text: &str) -> Result<Pattern, String> {
    let mut p = Pattern::default();
    let pos = text.split("<positions").find(|s| s.contains("axis=\"2Theta\"")).ok_or("No 2θ positions in this .xrdml file.")?;
    let (a, b) = (between(pos, "<startPosition>", "<").and_then(num), between(pos, "<endPosition>", "<").and_then(num));
    let list = between(pos, "<listPositions>", "<");
    let data = text.split("<counts").nth(1).or_else(|| text.split("<intensities").nth(1)).and_then(|s| s.split_once('>').map(|x| x.1)).and_then(|s| s.split('<').next()).ok_or("No counts in this .xrdml file.")?;
    p.y = data.split_whitespace().filter_map(num).collect();
    let n = p.y.len();
    p.x = match (list, a, b) {
        (Some(l), _, _) => l.split_whitespace().filter_map(num).collect(),
        (None, Some(a), Some(b)) if n > 1 => (0..n).map(|i| a + (b - a) * i as f64 / (n - 1) as f64).collect(),
        _ => return Err("The .xrdml file has no 2θ range.".into()),
    };
    if p.x.len() != n { return Err("The positions and the counts do not match in length.".into()) }
    if let Some(k1) = between(text, "<kAlpha1", "</kAlpha1>").and_then(|s| s.split_once('>')).and_then(|s| num(s.1)) {
        if let Some(t) = TUBES.iter().find(|t| (t.1 - k1).abs() < 0.005) { p.anode = t.0.into() } else { p.notes.push(format!("Kα1 in the file: {k1} Å (not one of the usual anodes).")) }
    }
    Ok(p)
}

/// GSAS text: a BANK line, then 2θ (in centidegrees for FXYE), counts and errors.
fn fxye(text: &str) -> Result<Pattern, String> {
    let mut p = Pattern::default();
    let mut on = false; let mut centi = true;
    for l in text.lines() {
        if l.starts_with("BANK") { on = true; centi = !l.contains("ESD") || l.contains("FXYE"); continue }
        if !on { continue }
        let v: Vec<f64> = l.split_whitespace().filter_map(num).collect();
        if v.len() >= 2 { p.x.push(if centi { v[0] / 100.0 } else { v[0] }); p.y.push(v[1]); }
    }
    if p.x.is_empty() { return Err("No data after the BANK line.".into()) }
    Ok(p)
}

// ---------- processing ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Ask {
    /// Anode (Cu, Co, Mo, Cr, Fe, Ag) or "" with `lambda` (Å) for any other wavelength.
    pub anode: String, pub lambda: f64,
    /// Zero shift added to 2θ (degrees) and the range kept.
    pub zero: f64, pub range: [f64; 2],
    /// Savitzky–Golay window in points (0: none), the background (SNIP) width in degrees (0: none), Kα2 stripping.
    pub smooth: usize, pub bg: f64, pub strip: bool,
    /// Peaks lower than this share of the strongest (%) are not listed.
    pub min_peak: f64,
    /// Scherrer constant and the instrument's own FWHM (degrees) to take off.
    pub k: f64, pub inst: f64,
}
impl Default for Ask { fn default() -> Self { Self { anode: "Cu".into(), lambda: 0.0, zero: 0.0, range: [0.0, 180.0], smooth: 0, bg: 3.0, strip: false, min_peak: 2.0, k: 0.9, inst: 0.0 } } }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peak { pub two_theta: f64, pub d: f64, pub height: f64, pub area: f64, pub fwhm: f64, pub breadth: f64, pub eta: f64, pub size_nm: Option<f64>, pub fit_ok: bool }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wh { pub size_nm: Option<f64>, pub strain: f64, pub r2: f64, pub points: Vec<[f64; 2]>, pub line: [f64; 2] }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer { pub x: Vec<f64>, pub y: Vec<f64>, pub bg: Vec<f64>, pub net: Vec<f64>, pub lambda: f64, pub peaks: Vec<Peak>, pub wh: Option<Wh>, pub notes: Vec<String> }

/// Savitzky–Golay smoothing (quadratic), window `w` points (odd, 5 or more).
pub fn smooth(y: &[f64], w: usize) -> Vec<f64> {
    if w < 5 || y.len() < w { return y.to_vec() }
    let m = (w / 2) as i64;
    let den = ((2 * m + 3) * (2 * m + 1) * (2 * m - 1)) as f64;
    let c: Vec<f64> = (-m..=m).map(|i| (3 * (3 * m * m + 3 * m - 1) - 15 * i * i) as f64 / den).collect();
    let n = y.len() as i64;
    (0..n).map(|k| if k < m || k >= n - m { y[k as usize] } else { (-m..=m).map(|i| c[(i + m) as usize] * y[(k + i) as usize]).sum() }).collect()
}

/// SNIP background over `p` points: clipping on the doubly logarithmic scale, windows growing to `p`.
pub fn snip(y: &[f64], p: usize) -> Vec<f64> {
    let n = y.len();
    let p = p.min(n - 1).max(1);
    // The pattern mirrored beyond both ends, so the ends are clipped like the middle.
    let ext: Vec<f64> = (0..n + 2 * p).map(|i| { let j = i as isize - p as isize; y[if j < 0 { (-j) as usize } else if j >= n as isize { 2 * (n - 1) - j as usize } else { j as usize }.min(n - 1)] }).collect();
    let m = ext.len();
    let mut v: Vec<f64> = ext.iter().map(|&q| ((q.max(0.0) + 1.0).sqrt() + 1.0).ln().ln_1p()).collect();
    for k in 1..=p {
        let w = v.clone();
        for i in k..m - k { v[i] = w[i].min((w[i - k] + w[i + k]) / 2.0) }
    }
    v[p..p + n].iter().map(|&q| { let a = (q.exp() - 1.0).exp() - 1.0; a * a - 1.0 }).collect()
}

/// Rachinger Kα2 stripping on an evenly spaced pattern (`x` in degrees 2θ).
pub fn strip_ka2(x: &[f64], y: &[f64], l1: f64, l2: f64) -> Vec<f64> {
    let mut o = y.to_vec();
    let (x0, dx) = (x[0], (x[x.len() - 1] - x[0]) / (x.len() - 1) as f64);
    for i in 0..x.len() {
        let t = (x[i] / 2.0).to_radians();
        let s = l1 / l2 * t.sin();
        if s >= 1.0 { continue }
        let x1 = 2.0 * s.asin().to_degrees();
        let f = (x1 - x0) / dx;
        if f < 0.0 { continue }
        let (j, r) = (f.floor() as usize, f.fract());
        if j + 1 >= i { continue }
        o[i] -= 0.5 * (o[j] * (1.0 - r) + o[j + 1] * r);
    }
    o
}

/// The pseudo-Voigt profile: height, centre, FWHM and mixing η (0 Gaussian, 1 Lorentzian).
pub fn pv(x: f64, c: f64, h: f64, w: f64, eta: f64) -> f64 {
    let z = (x - c) / w;
    h * (eta / (1.0 + 4.0 * z * z) + (1.0 - eta) * (-4.0 * std::f64::consts::LN_2 * z * z).exp())
}
/// Its area over height (the integral breadth), in the units of `w`.
pub fn pv_breadth(w: f64, eta: f64) -> f64 { w * (eta * std::f64::consts::FRAC_PI_2 + (1.0 - eta) * (std::f64::consts::PI / (4.0 * std::f64::consts::LN_2)).sqrt()) }

/// Least squares fit of one pseudo-Voigt with a straight background to the points (Levenberg–Marquardt).
/// Parameters: [centre, height, fwhm, eta, a, b]; returns them and whether it converged.
fn fit_pv(x: &[f64], y: &[f64], p0: [f64; 6]) -> ([f64; 6], bool) {
    let model = |p: &[f64; 6], t: f64| pv(t, p[0], p[1], p[2], p[3]) + p[4] + p[5] * (t - p0[0]);
    let cost = |p: &[f64; 6]| x.iter().zip(y).map(|(&t, &v)| (v - model(p, t)).powi(2)).sum::<f64>();
    let clamp = |mut p: [f64; 6]| { p[2] = p[2].clamp(1e-4, 10.0); p[3] = p[3].clamp(0.0, 1.0); p };
    let (mut p, mut lam, mut c) = (p0, 1e-3, cost(&p0));
    let mut ok = false;
    for _ in 0..200 {
        // Numerical Jacobian, normal equations (6×6), damped.
        let mut jt = vec![[0.0; 6]; x.len()];
        for k in 0..6 {
            let h = (p[k].abs() * 1e-6).max(1e-8);
            let mut q = p; q[k] += h;
            for (i, &t) in x.iter().enumerate() { jt[i][k] = (model(&q, t) - model(&p, t)) / h }
        }
        let mut a = [[0.0; 6]; 6]; let mut g = [0.0; 6];
        for (i, &t) in x.iter().enumerate() { let r = y[i] - model(&p, t); for k in 0..6 { g[k] += jt[i][k] * r; for l in 0..6 { a[k][l] += jt[i][k] * jt[i][l] } } }
        let mut improved = false;
        for _ in 0..12 {
            let mut m = a; for (k, row) in m.iter_mut().enumerate() { row[k] = row[k] * (1.0 + lam) + 1e-12 }
            let Some(d) = solve6(m, g) else { lam *= 10.0; continue };
            let q = clamp(std::array::from_fn(|k| p[k] + d[k]));
            let cq = cost(&q);
            if cq < c { let rel = (c - cq) / c.max(1e-30); p = q; c = cq; lam = (lam / 3.0).max(1e-9); improved = true; if rel < 1e-9 { ok = true } break }
            lam *= 4.0;
        }
        if !improved || ok { ok = true; break }
    }
    (p, ok && p[1] > 0.0)
}

fn solve6(mut a: [[f64; 6]; 6], mut b: [f64; 6]) -> Option<[f64; 6]> {
    for c in 0..6 {
        let r = (c..6).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))?;
        if a[r][c].abs() < 1e-300 { return None }
        a.swap(c, r); b.swap(c, r);
        for i in c + 1..6 { let f = a[i][c] / a[c][c]; for j in c..6 { a[i][j] -= f * a[c][j] } b[i] -= f * b[c] }
    }
    let mut x = [0.0; 6];
    for i in (0..6).rev() { x[i] = (b[i] - (i + 1..6).map(|j| a[i][j] * x[j]).sum::<f64>()) / a[i][i] }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

/// Process a pattern and find, fit and measure its peaks.
pub fn analyse(p: &Pattern, q: &Ask) -> Result<Answer, String> {
    let mut notes = vec![];
    let (l1, l2) = match tube(&q.anode) { Some(t) => t, None if q.lambda > 0.05 && q.lambda < 10.0 => (q.lambda, q.lambda), None => return Err("Choose the anode, or type the wavelength in Å.".into()) };
    let strip = q.strip && l2 > l1;
    let lambda = if strip || l2 == l1 { l1 } else { (2.0 * l1 + l2) / 3.0 };
    let (lo, hi) = (q.range[0].min(q.range[1]), q.range[0].max(q.range[1]));
    let pts: Vec<(f64, f64)> = p.x.iter().zip(&p.y).map(|(&a, &b)| (a + q.zero, b)).filter(|(a, _)| *a >= lo && *a <= hi && *a > 0.0 && *a < 180.0).collect();
    if pts.len() < 10 { return Err("Fewer than ten points in the range kept.".into()) }
    let (mut x, y): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
    // Evenly spaced points are assumed by smoothing and stripping: resample when the steps vary.
    let n = x.len();
    let step = (x[n - 1] - x[0]) / (n - 1) as f64;
    let uneven = x.windows(2).any(|w| ((w[1] - w[0]) - step).abs() > 0.05 * step);
    let y = if uneven {
        notes.push("The steps were uneven; the pattern was put on an even grid.".into());
        let xs: Vec<f64> = (0..n).map(|i| x[0] + step * i as f64).collect();
        let mut j = 0;
        let ys = xs.iter().map(|&t| { while j + 2 < n && x[j + 1] < t { j += 1 } let f = ((t - x[j]) / (x[j + 1] - x[j])).clamp(0.0, 1.0); y[j] * (1.0 - f) + y[j + 1] * f }).collect();
        x = xs; ys
    } else { y };
    let ys = smooth(&y, if q.smooth >= 5 { q.smooth | 1 } else { 0 });
    let bg = if q.bg > 0.0 { snip(&ys, (q.bg / step).round().max(1.0) as usize) } else { vec![0.0; n] };
    let mut net: Vec<f64> = ys.iter().zip(&bg).map(|(a, b)| a - b).collect();
    if strip { net = strip_ka2(&x, &net, l1, l2) }
    // Peak search: local maxima of the lightly smoothed net pattern standing out of the noise.
    let s = smooth(&net, 7);
    let top = s.iter().cloned().fold(0.0, f64::max);
    if top <= 0.0 { return Ok(Answer { x, y, bg, net, lambda, peaks: vec![], wh: None, notes }) }
    let noise = { let mut d: Vec<f64> = y.windows(2).map(|w| (w[1] - w[0]).abs()).collect(); d.sort_by(f64::total_cmp); d[d.len() / 2] / 2f64.sqrt() * 1.4826 };
    let thr = (q.min_peak / 100.0 * top).max(4.0 * noise);
    // A peak must stand out from the dips on both sides (its prominence), not only from zero:
    // bumps on the tails of strong peaks and the noise on top of them do not count.
    let prominence = |i: usize| {
        let (mut l, mut lmin) = (i, s[i]);
        while l > 0 && s[l - 1] <= s[i] { l -= 1; lmin = lmin.min(s[l]) }
        let (mut r, mut rmin) = (i, s[i]);
        while r + 1 < n && s[r + 1] <= s[i] { r += 1; rmin = rmin.min(s[r]) }
        s[i] - lmin.max(rmin)
    };
    let cand: Vec<usize> = (2..n - 2).filter(|&i| s[i] > thr && s[i] >= s[i - 1] && s[i] > s[i + 1] && prominence(i) > thr).collect();
    let mut peaks = vec![];
    for (k, &i) in cand.iter().enumerate() {
        // Half-height width from the data, then a fit over ±2 widths, stopping at the neighbours.
        let half = s[i] / 2.0;
        let (mut a, mut b) = (i, i);
        while a > 0 && s[a] > half { a -= 1 }
        while b + 1 < n && s[b] > half { b += 1 }
        let w0 = ((b - a) as f64 * step).max(2.0 * step);
        let left = if k > 0 { (cand[k - 1] + i) / 2 } else { 0 };
        let right = if k + 1 < cand.len() { (cand[k + 1] + i) / 2 } else { n - 1 };
        let span = (2.0 * w0 / step).ceil() as usize;
        let (lo, hi) = (i.saturating_sub(span).max(left), (i + span).min(right));
        if hi - lo < 5 { continue }
        let (fx, fy) = (&x[lo..=hi], &net[lo..=hi]);
        let (f, ok) = fit_pv(fx, fy, [x[i], net[i].max(s[i]), w0, 0.5, 0.0, 0.0]);
        let (c, h, w, eta) = if ok && f[0] > fx[0] && f[0] < fx[fx.len() - 1] { (f[0], f[1], f[2], f[3]) } else { (x[i], s[i], w0, 0.5) };
        let th = (c / 2.0).to_radians();
        let d = lambda / (2.0 * th.sin());
        let br = pv_breadth(w, eta);
        let fs = (w * w - q.inst * q.inst).max(0.0).sqrt();
        let size = (fs > 0.0 && q.inst < w).then(|| q.k * lambda / (fs.to_radians() * th.cos()) / 10.0);
        peaks.push(Peak { two_theta: c, d, height: h, area: h * br, fwhm: w, breadth: br, eta, size_nm: size, fit_ok: ok });
    }
    if peaks.is_empty() { notes.push("No peaks stand out of the noise: lower the threshold or smooth the pattern.".into()) }
    if peaks.iter().any(|p| !p.fit_ok) { notes.push("Some peaks could not be fitted (overlapping or too narrow); their values come from the data directly.".into()) }
    if q.inst <= 0.0 && !peaks.is_empty() { notes.push("No instrument broadening was taken off: sizes are lower bounds. Measure a standard (a well-crystallised powder) and type its FWHM.".into()) }
    let wh = williamson_hall(&peaks, lambda, q.k, q.inst);
    Ok(Answer { x, y, bg, net, lambda, peaks, wh, notes })
}

/// Williamson–Hall: β·cosθ = Kλ/D + 4ε·sinθ, with β the integral breadth (radians) less the instrument's.
pub fn williamson_hall(peaks: &[Peak], lambda: f64, k: f64, inst: f64) -> Option<Wh> {
    let pts: Vec<[f64; 2]> = peaks.iter().filter(|p| p.fit_ok).filter_map(|p| {
        let th = (p.two_theta / 2.0).to_radians();
        let b = (p.breadth * p.breadth - inst * inst).max(0.0).sqrt().to_radians();
        (b > 0.0).then(|| [4.0 * th.sin(), b * th.cos()])
    }).collect();
    if pts.len() < 3 { return None }
    let n = pts.len() as f64;
    let (sx, sy) = (pts.iter().map(|q| q[0]).sum::<f64>(), pts.iter().map(|q| q[1]).sum::<f64>());
    let sxx = pts.iter().map(|q| q[0] * q[0]).sum::<f64>() - sx * sx / n;
    let sxy = pts.iter().map(|q| q[0] * q[1]).sum::<f64>() - sx * sy / n;
    if sxx <= 0.0 { return None }
    let b = sxy / sxx; let a = (sy - b * sx) / n;
    let my = sy / n;
    let sst = pts.iter().map(|q| (q[1] - my).powi(2)).sum::<f64>();
    let r2 = if sst > 0.0 { 1.0 - pts.iter().map(|q| (q[1] - a - b * q[0]).powi(2)).sum::<f64>() / sst } else { 1.0 };
    Some(Wh { size_nm: (a > 0.0).then(|| k * lambda / a / 10.0), strain: b, r2, points: pts, line: [a, b] })
}

// ---------- the patterns kept in the project ----------

use crate::research::{Item, Research};
pub const MAX_PATTERNS: usize = 12;

/// The patterns kept in the project (none when there are none).
pub fn kept(r: &Research) -> Result<Vec<Pattern>, String> { r.xrd.as_ref().map_or(Ok(vec![]), Item::to_xrd) }
fn settings(r: &Research) -> serde_json::Value { r.xrd.as_ref().map_or(serde_json::json!({}), |i| i.settings.clone()) }
/// The kept patterns with one more.
pub fn add(r: &Research, p: Pattern) -> Result<Item, String> {
    let mut v = kept(r)?;
    if v.len() >= MAX_PATTERNS { return Err(format!("Up to {MAX_PATTERNS} patterns in one project; remove one first.")) }
    v.push(p);
    Ok(Item::xrd(&v, settings(r)))
}
/// The kept patterns without pattern `k` (nothing kept when it was the last).
pub fn remove(r: &Research, k: usize) -> Result<Option<Item>, String> {
    let mut v = kept(r)?;
    if k >= v.len() { return Err("That pattern is no longer open.".into()) }
    v.remove(k);
    Ok((!v.is_empty()).then(|| Item::xrd(&v, settings(r))))
}
/// Every kept pattern for the overlay graph, thinned to at most 4000 points each (keeping the highest
/// point of each stretch, so peaks stay).
pub fn for_plot(r: &Research) -> Result<Vec<serde_json::Value>, String> {
    Ok(kept(r)?.iter().map(|p| {
        let k = p.x.len().div_ceil(4000).max(1);
        let (mut x, mut y) = (vec![], vec![]);
        for c in p.x.chunks(k).zip(p.y.chunks(k)) { let j = (0..c.1.len()).max_by(|&a, &b| c.1[a].total_cmp(&c.1[b])).unwrap_or(0); x.push(c.0[j]); y.push(c.1[j]); }
        serde_json::json!({ "name": p.name, "x": x, "y": y })
    }).collect())
}

/// A made-up pattern to try the pages: a cubic phase's peaks (from the crystal engine) broadened by
/// 40 nm crystallites and 0.1 % microstrain, Cu Kα1 and Kα2, a curved background and counting noise.
pub fn example() -> Result<Pattern, String> {
    let s = crate::crystal::build("fcc", crate::crystal::Cell { a: 4.0495, b: 4.0495, c: 4.0495, alpha: 90.0, beta: 90.0, gamma: 90.0 }, &["Al".to_string()])?;
    let (l1, l2) = tube("Cu").unwrap_or((1.5406, 1.5444));
    let lines = crate::crystal::peaks(&s, l1, 140.0)?;
    let (d_nm, eps) = (40.0, 0.001);
    let mut seed = 12345u64;
    let mut rnd = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed >> 11) as f64 / (1u64 << 53) as f64 };
    let x: Vec<f64> = (0..6001).map(|i| 20.0 + i as f64 * 0.02).collect();
    let y = x.iter().map(|&t| {
        let mut v = 60.0 + 900.0 * (-(t - 20.0) / 12.0).exp() + 15.0 * (t / 140.0);
        for pk in &lines {
            for (lam, r) in [(l1, 1.0), (l2, 0.5)] {
                let th = (lam / 2.0 / pk.d).asin();
                let c = 2.0 * th.to_degrees();
                let w = (0.9 * lam / (d_nm * 10.0 * th.cos()) + 4.0 * eps * th.tan()).to_degrees().max(0.03);
                v += pv(t, c, 80.0 * pk.intensity * r, w, 0.5);
            }
        }
        // Counting noise, roughly Poisson (the sum of uniforms).
        let z = (0..12).map(|_| rnd()).sum::<f64>() - 6.0;
        (v + z * v.sqrt()).max(0.0).round()
    }).collect();
    Ok(Pattern { name: "Example (made-up pattern)".into(), x, y, anode: "Cu".into(), notes: vec!["A made-up pattern: an FCC metal (a = 4.0495 Å) with 40 nm crystallites and 0.1 % microstrain, Cu Kα, with noise.".into()] })
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn reads_the_formats() {
        let xy = "# 2theta counts\n10.00 5\n10.02 6\n".to_string() + &(0..20).map(|i| format!("{:.2}\t{}\n", 10.04 + i as f64 * 0.02, i)).collect::<String>();
        let p = parse(&xy, "a.xy").unwrap();
        assert_eq!(p.x.len(), 22);
        let semi = (0..20).map(|i| format!("{},{};{}\n", 10 + i, 5, i * 3)).collect::<String>();
        assert_eq!(parse(&semi, "b.csv").unwrap().y[2], 6.0);
        let xml = r#"<xrdMeasurements><xrdMeasurement><usedWavelength><kAlpha1 unit="Angstrom">1.5405980</kAlpha1></usedWavelength><scan><dataPoints><positions axis="2Theta" unit="deg"><startPosition>10.0</startPosition><endPosition>29.0</endPosition></positions><counts unit="counts">1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20</counts></dataPoints></scan></xrdMeasurement></xrdMeasurements>"#;
        let p = parse(xml, "c.xrdml").unwrap();
        assert_eq!((p.x.len(), p.x[19], p.anode.as_str()), (20, 29.0, "Cu"));
        let gsas = "Title\nBANK 1 20 20 CONS 1000 2 0 0 FXYE\n".to_string() + &(0..20).map(|i| format!("{} {} 1\n", 1000 + 2 * i, i)).collect::<String>();
        let p = parse(&gsas, "d.fxye").unwrap();
        assert!((p.x[1] - 10.02).abs() < 1e-9);
        assert!(parse("hello\nworld", "e.txt").is_err());
        assert!(parse(&(0..20).map(|i| format!("{} 1\n", 500 + i)).collect::<String>(), "f.xy").is_err(), "2θ above 180");
    }
    #[test]
    fn processing_pieces() {
        // Savitzky–Golay keeps a straight line and a parabola exactly.
        let y: Vec<f64> = (0..50).map(|i| (i as f64 - 20.0).powi(2)).collect();
        let s = smooth(&y, 7);
        assert!(s.iter().zip(&y).all(|(a, b)| (a - b).abs() < 1e-9));
        // SNIP takes off a sloping background under a narrow peak and leaves the peak.
        let y: Vec<f64> = (0..400).map(|i| 100.0 + i as f64 * 0.5 + pv(i as f64, 200.0, 1000.0, 6.0, 0.3)).collect();
        let b = snip(&y, 30);
        assert!((b[50] - y[50]).abs() < 0.05 * y[50] && (y[200] - b[200]) > 900.0);
        // …and does not climb under a peak at the very end of the range.
        let z: Vec<f64> = (0..400).map(|i| 100.0 + pv(i as f64, 396.0, 1000.0, 6.0, 0.3)).collect();
        let bz = snip(&z, 30);
        assert!(bz[399] < 200.0, "{}", bz[399]);
        // Kα2 stripping removes the doublet's second peak.
        let x: Vec<f64> = (0..2000).map(|i| 60.0 + i as f64 * 0.005).collect();
        let (l1, l2) = tube("Cu").unwrap();
        let c2 = 2.0 * (l2 / l1 * (32.0f64).to_radians().sin()).asin().to_degrees();
        let y: Vec<f64> = x.iter().map(|&t| pv(t, 64.0, 100.0, 0.05, 0.5) + pv(t, c2, 50.0, 0.05, 0.5)).collect();
        let o = strip_ka2(&x, &y, l1, l2);
        let at = |t: f64| o[((t - 60.0) / 0.005).round() as usize];
        assert!((at(64.0) - 100.0).abs() < 2.0 && at(c2).abs() < 3.0, "{} {}", at(64.0), at(c2));
    }
    #[test]
    fn peaks_size_and_strain_of_the_example() {
        let p = example().unwrap();
        let a = analyse(&p, &Ask { strip: true, smooth: 0, bg: 4.0, ..Default::default() }).unwrap();
        // The FCC lines of a = 4.0495 Å: (111) at d = 2.338 Å, 2θ ≈ 38.47° with Cu Kα1.
        let p111 = a.peaks.iter().min_by(|x, y| (x.d - 2.338).abs().total_cmp(&(y.d - 2.338).abs())).unwrap();
        assert!((p111.two_theta - 38.47).abs() < 0.03 && (p111.d - 2.338).abs() < 0.002, "{p111:?}");
        assert!(a.peaks.len() >= 8 && a.peaks.len() <= 12, "{}", a.peaks.len());
        // Williamson–Hall gets back the 40 nm and 0.1 % it was made with (the profile is a 50/50 pseudo-Voigt, so within a margin).
        let wh = a.wh.unwrap();
        assert!(wh.size_nm.unwrap() > 25.0 && wh.size_nm.unwrap() < 60.0 && wh.strain > 0.0005 && wh.strain < 0.002 && wh.r2 > 0.9, "{wh:?}");
        assert!(analyse(&p, &Ask { anode: "".into(), ..Default::default() }).is_err());
        assert!((ka("Cu").unwrap() - 1.541_871).abs() < 1e-5);
    }
    #[test]
    fn kept_in_the_project() {
        let mut r = Research::default();
        for _ in 0..2 { r.xrd = Some(add(&r, example().unwrap()).unwrap()) }
        assert_eq!(kept(&r).unwrap().len(), 2);
        assert!(for_plot(&r).unwrap()[0]["x"].as_array().unwrap().len() <= 4000);
        r.xrd = remove(&r, 0).unwrap();
        assert_eq!(kept(&r).unwrap().len(), 1);
        assert!(remove(&r, 5).is_err());
        assert!(remove(&r, 0).unwrap().is_none());
    }
}
