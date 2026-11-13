//! Synthetic 2D microstructures: a map of grains generated from settings (mean grain size, size
//! spread, elongation, a second phase, pores), with what it really contains measured back. The
//! grains are a Laguerre (power) tessellation: each pixel goes to the seed with the least
//! |x − s|² − r², so bigger weights make bigger grains; elongation stretches the distance. Seeded,
//! so the same settings always give the same map.
use crate::image::b64_encode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Ask {
    pub w: usize, pub h: usize,
    /// µm per pixel.
    pub step: f64,
    /// Mean equivalent grain diameter, µm.
    pub size: f64,
    /// Spread of grain sizes (0 even, 1 very mixed): the log-normal σ of the seed weights.
    pub spread: f64,
    /// Grain length over width (1 equiaxed; above 1 elongated along x).
    pub aspect: f64,
    /// Area fraction of a second phase (0–0.9), given to whole grains.
    pub phase2: f64,
    /// Area fraction of pores (0–0.3), small round holes on grain boundaries.
    pub pores: f64,
    pub seed: u64,
    /// Crystal symmetry: "cubic" or "hex".
    pub crystal: String,
    /// Texture: "random", a component of `orient::COMPONENTS` (cubic), "fibre111", "fibre100", "fibre110" (cubic) or "basal" (hex).
    pub texture: String,
    /// Share of grains given the texture (the rest random), 0–1.
    pub share: f64,
    /// Scatter about the ideal orientation, degrees (σ).
    pub scatter: f64,
    /// The sample direction the IPF colours show: "z" (normal), "x" or "y".
    pub dir: String,
}
impl Default for Ask { fn default() -> Self { Ask { w: 400, h: 300, step: 1.0, size: 25.0, spread: 0.3, aspect: 1.0, phase2: 0.0, pores: 0.0, seed: 1, crystal: "cubic".into(), texture: "random".into(), share: 1.0, scatter: 8.0, dir: "z".into() } } }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    pub w: usize, pub h: usize, pub step: f64,
    /// Grain number of each pixel (u16, little-endian, base64); pores are 0xFFFF.
    pub labels: String,
    /// Phase of each grain (1 or 2), by grain number.
    pub phase: Vec<u8>,
    pub grains: usize,
    /// Measured back from the map: mean equivalent diameter (µm, grains not on the edge), its spread, aspect ratio, phase 2 and pore fractions (%).
    pub mean_ecd: Option<f64>, pub sd_ecd: Option<f64>, pub mean_aspect: Option<f64>, pub phase2: f64, pub pores: f64,
    /// A grey picture of the map (boundaries dark, phase 2 darker, pores black), base64, for image analysis.
    pub grey: String,
    /// Bunge Euler angles (degrees) and IPF colour of each grain.
    pub euler: Vec<[f64; 3]>, pub ipf: Vec<[u8; 3]>,
    /// Measured back: % of grains within 15° of the ideal texture; % of boundary length with a misorientation under 15°; mean misorientation across boundaries.
    pub textured: Option<f64>, pub low_angle: f64, pub mean_mis: f64,
}

/// The ideal of a texture: an orientation (component) or a crystal direction along Z (fibre).
enum Ideal { None, Comp(crate::orient::M3), Fibre([f64; 3]) }
fn ideal(t: &str) -> Ideal {
    use crate::orient::*;
    match t {
        "fibre111" => Ideal::Fibre([1.0, 1.0, 1.0]), "fibre100" => Ideal::Fibre([0.0, 0.0, 1.0]), "fibre110" => Ideal::Fibre([1.0, 1.0, 0.0]), "basal" => Ideal::Fibre([0.0, 0.0, 1.0]),
        c => COMPONENTS.iter().find(|x| x.0 == c).map_or(Ideal::None, |x| Ideal::Comp(from_euler(x.2))),
    }
}

pub struct Rng(pub u64);
impl Rng {
    pub fn f(&mut self) -> f64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; (self.0 >> 11) as f64 / (1u64 << 53) as f64 }
    /// A standard normal number (Box–Muller).
    pub fn n(&mut self) -> f64 { let (a, b) = (self.f().max(1e-12), self.f()); (-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos() }
}

pub fn check(q: &Ask) -> Result<(), String> {
    if !["cubic", "hex"].contains(&q.crystal.as_str()) || !["z", "x", "y"].contains(&q.dir.as_str()) { return Err("The crystal is cubic or hexagonal; the colour direction x, y or z.".into()) }
    let fits = match q.texture.as_str() { "random" => true, "basal" => q.crystal == "hex", t => q.crystal == "cubic" && !matches!(ideal(t), Ideal::None) };
    if !fits { return Err(format!("No texture “{}” for a {} crystal.", q.texture, if q.crystal == "hex" { "hexagonal" } else { "cubic" })) }
    if !(q.share.is_finite() && (0.0..=1.0).contains(&q.share)) || !(q.scatter.is_finite() && (0.0..=90.0).contains(&q.scatter)) { return Err("Share 0–1 and scatter 0–90°.".into()) }
    if !(16..=1200).contains(&q.w) || !(16..=1200).contains(&q.h) { return Err("The map is 16 to 1,200 pixels each way.".into()) }
    let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
    if !ok(q.step, 1e-3, 1e4) || !ok(q.size, 1e-3, 1e6) { return Err("Step and grain size are lengths above zero.".into()) }
    if q.size / q.step < 3.0 { return Err("Grains must be at least 3 pixels across: make the step smaller or the grains bigger.".into()) }
    if !ok(q.spread, 0.0, 1.5) || !ok(q.aspect, 1.0, 10.0) || !ok(q.phase2, 0.0, 0.9) || !ok(q.pores, 0.0, 0.3) { return Err("Spread 0–1.5, aspect ratio 1–10, second phase 0–0.9 and pores 0–0.3.".into()) }
    let n = (q.w * q.h) as f64 / (std::f64::consts::FRAC_PI_4 * (q.size / q.step).powi(2));
    if n > 60_000.0 { return Err("Too many grains for one map (more than 60,000): make them bigger.".into()) }
    Ok(())
}

/// Generate the map.
pub fn generate(q: &Ask) -> Result<Map, String> {
    check(q)?;
    let (w, h) = (q.w, q.h);
    let mut r = Rng(q.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    // Grain count from the mean size; seeds spread over the map; weights log-normal.
    let d = q.size / q.step; // mean diameter in pixels
    let n = (((w * h) as f64 / (std::f64::consts::FRAC_PI_4 * d * d)).round() as usize).max(1);
    let sx = q.aspect.sqrt(); // stretch x by √a and squeeze y by √a: same area, length/width = a
    let seeds: Vec<(f64, f64, f64)> = (0..n).map(|_| { let rad = d / 2.0 * (q.spread * r.n() - q.spread * q.spread / 2.0).exp(); (r.f() * w as f64, r.f() * h as f64, rad * rad * 0.9) }).collect();
    // Buckets for a quick nearest search.
    let cell = (d * 1.5).max(4.0);
    let (bw, bh) = ((w as f64 / cell).ceil() as usize + 1, (h as f64 / cell).ceil() as usize + 1);
    let mut buckets = vec![vec![]; bw * bh];
    for (i, s) in seeds.iter().enumerate() { buckets[(s.1 / cell) as usize * bw + (s.0 / cell) as usize].push(i) }
    let reach = (2.0 + 2.0 * q.spread + q.aspect.sqrt()).ceil() as isize;
    let mut lab = vec![0u16; w * h];
    for y in 0..h { for x in 0..w {
        let (cx, cy) = ((x as f64 / cell) as isize, (y as f64 / cell) as isize);
        let mut best = (f64::MAX, 0usize);
        for by in (cy - reach).max(0)..=(cy + reach).min(bh as isize - 1) { for bx in (cx - reach).max(0)..=(cx + reach).min(bw as isize - 1) {
            for &i in &buckets[by as usize * bw + bx as usize] {
                let s = seeds[i];
                let (dx, dy) = ((x as f64 + 0.5 - s.0) / sx, (y as f64 + 0.5 - s.1) * sx);
                let v = dx * dx + dy * dy - s.2;
                if v < best.0 { best = (v, i) }
            }
        }}
        lab[y * w + x] = best.1 as u16;
    }}
    // Areas, then the second phase given to whole grains until its fraction is reached.
    let mut area = vec![0usize; n];
    for &l in &lab { area[l as usize] += 1 }
    let mut phase = vec![1u8; n];
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() { let j = (r.f() * (i + 1) as f64) as usize; order.swap(i, j.min(i)) }
    let (total, mut got) = ((w * h) as f64, 0.0);
    for &i in &order {
        if got / total >= q.phase2 { break }
        if area[i] > 0 { phase[i] = 2; got += area[i] as f64 }
    }
    // Pores: small round holes on boundaries until their fraction is reached.
    let is_b = |lab: &[u16], i: usize| { let (x, y) = (i % w, i / w); (x + 1 < w && lab[i + 1] != lab[i]) || (y + 1 < h && lab[i + w] != lab[i]) };
    let pr = (d * 0.12).max(1.0);
    let mut holes = 0usize;
    let mut tries = 0;
    while (holes as f64) < q.pores * total && tries < 200_000 {
        tries += 1;
        let i = (r.f() * total) as usize % (w * h);
        if !is_b(&lab, i) { continue }
        let (px, py) = ((i % w) as f64, (i / w) as f64);
        let rad = pr * (0.6 + 0.8 * r.f());
        let (x0, x1, y0, y1) = ((px - rad).max(0.0) as usize, ((px + rad) as usize).min(w - 1), (py - rad).max(0.0) as usize, ((py + rad) as usize).min(h - 1));
        for yy in y0..=y1 { for xx in x0..=x1 { if (xx as f64 - px).powi(2) + (yy as f64 - py).powi(2) <= rad * rad { let k = yy * w + xx; if lab[k] != u16::MAX { lab[k] = u16::MAX; holes += 1 } } } }
    }
    // Measure it back: grains not touching the edge.
    let mut a2 = vec![0usize; n];
    let mut edge = vec![false; n];
    let mut mom = vec![[0.0f64; 5]; n];
    for (k, &l) in lab.iter().enumerate() {
        if l == u16::MAX { continue }
        let (x, y) = ((k % w) as f64, (k / w) as f64);
        let i = l as usize;
        a2[i] += 1;
        let m = &mut mom[i];
        m[0] += x; m[1] += y; m[2] += x * x; m[3] += y * y; m[4] += x * y;
        if k % w == 0 || k / w == 0 || k % w == w - 1 || k / w == h - 1 { edge[i] = true }
    }
    let inner: Vec<usize> = (0..n).filter(|&i| !edge[i] && a2[i] >= 3).collect();
    let ecd: Vec<f64> = inner.iter().map(|&i| 2.0 * (a2[i] as f64 / std::f64::consts::PI).sqrt() * q.step).collect();
    let asp: Vec<f64> = inner.iter().map(|&i| {
        let (c, m) = (a2[i] as f64, mom[i]);
        let (mx, my) = (m[0] / c, m[1] / c);
        let (vxx, vyy, vxy) = (m[2] / c - mx * mx + 1.0 / 12.0, m[3] / c - my * my + 1.0 / 12.0, m[4] / c - mx * my);
        let (a, b) = ((vxx + vyy) / 2.0, (((vxx - vyy) / 2.0).powi(2) + vxy * vxy).sqrt());
        ((a + b) / (a - b).max(1e-9)).sqrt()
    }).collect();
    let mean = |v: &[f64]| (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64);
    let me = mean(&ecd);
    let sd = me.filter(|_| ecd.len() > 1).map(|m| (ecd.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (ecd.len() - 1) as f64).sqrt());
    let p2 = lab.iter().filter(|&&l| l != u16::MAX && phase[l as usize] == 2).count() as f64 / total * 100.0;
    let grey: Vec<u8> = (0..w * h).map(|k| {
        let l = lab[k];
        if l == u16::MAX { return 15 }
        if is_b(&lab, k) { return 45 }
        let base = if phase[l as usize] == 2 { 110.0 } else { 185.0 };
        (base + 30.0 * (((l as u64).wrapping_mul(2_654_435_761) % 1000) as f64 / 1000.0 - 0.5)) as u8
    }).collect();
    let bytes: Vec<u8> = lab.iter().flat_map(|l| l.to_le_bytes()).collect();
    // Orientations: a share of the grains near the texture, the rest random.
    use crate::orient::*;
    let id = ideal(&q.texture);
    let gs: Vec<M3> = (0..n).map(|_| {
        let base = if q.texture != "random" && r.f() < q.share {
            match &id { Ideal::Comp(m) => Some(*m), Ideal::Fibre(d) => Some(fibre(*d, 360.0 * r.f())), Ideal::None => None }
        } else { None };
        match base {
            Some(m) => { let ax = [r.n(), r.n(), r.n()]; mul(&axis_angle(ax, q.scatter * r.n()), &m) }
            None => random([r.f(), r.f(), r.f()]),
        }
    }).collect();
    let sym = symmetry(&q.crystal);
    let sdir = match q.dir.as_str() { "x" => [1.0, 0.0, 0.0], "y" => [0.0, 1.0, 0.0], _ => [0.0, 0.0, 1.0] };
    let within = |g: &M3| match &id {
        Ideal::Comp(m) => misorientation(m, g, &sym) <= 15.0,
        Ideal::Fibre(d) => { let z = apply(g, [0.0, 0.0, 1.0]); let dn = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt(); sym.iter().any(|s| { let v = apply(s, z); ((v[0] * d[0] + v[1] * d[1] + v[2] * d[2]) / dn).clamp(-1.0, 1.0).acos().to_degrees() <= 15.0 }) }
        Ideal::None => false,
    };
    let textured = (!matches!(id, Ideal::None)).then(|| gs.iter().filter(|g| within(g)).count() as f64 / n as f64 * 100.0);
    // Boundaries: misorientation across each pair of neighbouring pixels in different grains.
    let mut cache = std::collections::HashMap::new();
    let (mut nb, mut low, mut sum) = (0usize, 0usize, 0.0);
    for k in 0..w * h {
        for j in [k + 1, k + w] {
            if j >= w * h || (j == k + 1 && (k + 1) % w == 0) { continue }
            let (a, b) = (lab[k], lab[j]);
            if a == b || a == u16::MAX || b == u16::MAX { continue }
            let key = (a.min(b), a.max(b));
            let m = *cache.entry(key).or_insert_with(|| misorientation(&gs[key.0 as usize], &gs[key.1 as usize], &sym));
            nb += 1; sum += m; if m < 15.0 { low += 1 }
        }
    }
    Ok(Map { w, h, step: q.step, labels: b64_encode(&bytes), phase, grains: n, mean_ecd: me, sd_ecd: sd, mean_aspect: mean(&asp), phase2: p2, pores: holes as f64 / total * 100.0, grey: b64_encode(&grey),
        euler: gs.iter().map(to_euler).collect(), ipf: gs.iter().map(|g| ipf(g, sdir, &q.crystal)).collect(), textured, low_angle: if nb > 0 { low as f64 / nb as f64 * 100.0 } else { 0.0 }, mean_mis: if nb > 0 { sum / nb as f64 } else { 0.0 } })
}

/// The map as an orientation map in text: a header, then per pixel x and y (µm), Bunge Euler angles
/// (degrees), phase (0 for a pore) and grain number. Crystal › EBSD reads it.
pub fn ebsd_text(q: &Ask) -> Result<String, String> {
    let m = generate(q)?;
    let b = crate::image::b64_decode(&m.labels)?;
    use std::fmt::Write;
    let mut s = format!("# Datum Atlas orientation map (synthetic)\n# crystal {}\n# step {}\n# columns: x y phi1 Phi phi2 phase grain\n", q.crystal, q.step);
    for (k, c) in b.chunks(2).enumerate() {
        let l = u16::from_le_bytes([c[0], c[1]]);
        let (x, y) = ((k % m.w) as f64 * m.step, (k / m.w) as f64 * m.step);
        if l == u16::MAX { let _ = writeln!(s, "{x} {y} 0 0 0 0 0"); continue }
        let e = m.euler[l as usize];
        let _ = writeln!(s, "{x} {y} {:.3} {:.3} {:.3} {} {}", e[0], e[1], e[2], m.phase[l as usize], l as usize + 1);
    }
    Ok(s)
}

/// The map as text: one line per pixel, x and y in µm, grain number (0 for a pore) and phase.
pub fn csv(q: &Ask) -> Result<String, String> {
    let m = generate(q)?;
    let b = crate::image::b64_decode(&m.labels)?;
    let mut s = String::from("x (µm),y (µm),grain,phase\n");
    use std::fmt::Write;
    for (k, c) in b.chunks(2).enumerate() {
        let l = u16::from_le_bytes([c[0], c[1]]);
        let (g, p) = if l == u16::MAX { (0, 0) } else { (l as usize + 1, m.phase[l as usize]) };
        let _ = writeln!(s, "{},{},{},{}", (k % m.w) as f64 * m.step, (k / m.w) as f64 * m.step, g, p);
    }
    Ok(s)
}
