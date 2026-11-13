//! EBSD orientation maps: read the common text exports (`.ang`, `.ctf`) and Datum Atlas's own
//! orientation map text, put the points on a square grid, and draw maps: orientation (IPF colour of
//! a sample direction), phase, and pattern quality or confidence. Unindexed points stay empty.
use crate::{image::b64_encode, orient};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Phase { pub name: String, pub crystal: String }

/// A map on a square grid. Per point: Bunge Euler angles (degrees), phase (0 = not indexed),
/// quality (image quality or band contrast) and confidence (confidence index or 1/MAD), NaN when absent.
#[derive(Debug, Clone, Default)]
pub struct Ebsd {
    pub name: String, pub format: String, pub w: usize, pub h: usize, pub step: f64,
    pub euler: Vec<[f32; 3]>, pub phase: Vec<u8>, pub quality: Vec<f32>, pub conf: Vec<f32>,
    pub phases: Vec<Phase>, pub quality_name: String, pub conf_name: String, pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary { pub name: String, pub format: String, pub w: usize, pub h: usize, pub step: f64, pub points: usize, pub indexed: f64,
    pub phases: Vec<(Phase, f64)>, pub quality_name: String, pub conf_name: String, pub notes: Vec<String> }

pub const MAX_POINTS: usize = 6_000_000;

fn laue(code: &str) -> &'static str { match code.trim() { "11" | "43" | "432" | "m-3m" | "m3m" | "cubic" => "cubic", "9" | "62" | "6" | "622" | "6/mmm" | "hex" | "hexagonal" => "hex", _ => "" } }

/// Read an export. `name` is the file name (its extension picks the format when the text does not).
pub fn parse(text: &str, name: &str) -> Result<Ebsd, String> {
    let low = name.to_lowercase();
    let ctf = low.ends_with(".ctf") || text.lines().take(3).any(|l| l.starts_with("Channel Text File"));
    let ours = text.starts_with("# Datum Atlas orientation map");
    let mut phases: Vec<Phase> = vec![];
    let mut pts: Vec<(f64, f64, [f32; 3], u8, f32, f32)> = vec![];
    let mut notes = vec![];
    let (mut step, mut ystep) = (0.0, 0.0);
    let (format, qn, cn);
    let num = |s: &str| s.trim().parse::<f64>().ok();
    if ctf {
        (format, qn, cn) = ("ctf", "Band contrast", "Band slope");
        let mut lines = text.lines();
        let mut nph = 0;
        for l in lines.by_ref() {
            let f: Vec<&str> = l.split('\t').collect();
            match f[0].trim() {
                "XStep" => step = f.get(1).and_then(|v| num(v)).unwrap_or(0.0),
                "YStep" => ystep = f.get(1).and_then(|v| num(v)).unwrap_or(0.0),
                "Phases" => nph = f.get(1).and_then(|v| v.trim().parse().ok()).unwrap_or(0),
                "Phase" if f.len() > 3 && f[1].trim() == "X" => break,
                _ if nph > phases.len() && f.len() >= 4 && f[0].contains(';') => phases.push(Phase { name: f[2].trim().into(), crystal: laue(f[3]).into() }),
                _ => {}
            }
        }
        for l in lines {
            let f: Vec<f64> = l.split('\t').filter_map(num).collect();
            if f.len() < 9 { continue }
            let ph = f[0] as u8;
            pts.push((f[1], f[2], [f[5] as f32, f[6] as f32, f[7] as f32], if f[4] != 0.0 { 0 } else { ph }, f.get(9).copied().unwrap_or(f64::NAN) as f32, f.get(10).copied().unwrap_or(f64::NAN) as f32));
            if pts.len() > MAX_POINTS { return Err(format!("More than {} million points.", MAX_POINTS / 1_000_000)) }
        }
    } else if ours {
        (format, qn, cn) = ("datum", "", "");
        let mut crystal = "cubic".to_string();
        for l in text.lines() {
            if let Some(r) = l.strip_prefix('#') {
                let r = r.trim();
                if let Some(c) = r.strip_prefix("crystal") { crystal = laue(c).to_string() }
                if let Some(s) = r.strip_prefix("step") { step = num(s).unwrap_or(0.0) }
                continue;
            }
            let f: Vec<f64> = l.split_whitespace().filter_map(num).collect();
            if f.len() < 6 { continue }
            pts.push((f[0], f[1], [f[2] as f32, f[3] as f32, f[4] as f32], f[5] as u8, f32::NAN, f32::NAN));
            if pts.len() > MAX_POINTS { return Err(format!("More than {} million points.", MAX_POINTS / 1_000_000)) }
        }
        let n = pts.iter().map(|p| p.3).max().unwrap_or(1).max(1);
        phases = (1..=n).map(|k| Phase { name: format!("Phase {k}"), crystal: crystal.clone() }).collect();
        ystep = step;
    } else {
        (format, qn, cn) = ("ang", "Image quality", "Confidence index");
        let mut hexgrid = false;
        for l in text.lines() {
            if let Some(r) = l.strip_prefix('#') {
                let r = r.trim();
                let (k, v) = r.split_once(|c: char| c == ':' || c.is_whitespace()).unwrap_or((r, ""));
                match k.trim().to_lowercase().as_str() {
                    "phase" => phases.push(Phase { name: String::new(), crystal: String::new() }),
                    "materialname" => if let Some(p) = phases.last_mut() { p.name = v.trim().into() },
                    "symmetry" => if let Some(p) = phases.last_mut() { p.crystal = laue(v).into() },
                    "xstep" => step = num(v).unwrap_or(0.0),
                    "ystep" => ystep = num(v).unwrap_or(0.0),
                    "grid" => hexgrid = v.to_lowercase().contains("hex"),
                    _ => {}
                }
                continue;
            }
            let f: Vec<f64> = l.split_whitespace().filter_map(num).collect();
            if f.len() < 8 { continue }
            let e = [f[0].to_degrees() as f32, f[1].to_degrees() as f32, f[2].to_degrees() as f32];
            let unindexed = f[6] < 0.0 || (f[0] > 12.0 && f[1] > 12.0);
            let ph = if unindexed { 0 } else { (f[7] as u8).max(1) };
            pts.push((f[3], f[4], e, ph, f[5] as f32, f[6] as f32));
            if pts.len() > MAX_POINTS { return Err(format!("More than {} million points.", MAX_POINTS / 1_000_000)) }
        }
        if hexgrid { notes.push("The map is on a hexagonal grid; it is shown on a square grid of the same step (alternate rows shifted by half a step).".into()) }
        if phases.is_empty() { phases.push(Phase { name: "Phase 1".into(), crystal: String::new() }) }
    }
    if pts.len() < 4 { return Err("No map points found: is this an EBSD text export (.ang, .ctf) or a Datum Atlas orientation map?".into()) }
    for (k, p) in phases.iter_mut().enumerate() {
        if p.name.is_empty() { p.name = format!("Phase {}", k + 1) }
        if p.crystal.is_empty() { p.crystal = "cubic".into(); notes.push(format!("{}: symmetry not given or not cubic or hexagonal; treated as cubic.", p.name)) }
    }
    // The grid: the step from the header, else the smallest spacing seen.
    let (xmin, ymin) = pts.iter().fold((f64::MAX, f64::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1)));
    let (xmax, ymax) = pts.iter().fold((f64::MIN, f64::MIN), |a, p| (a.0.max(p.0), a.1.max(p.1)));
    if step.is_nan() || step <= 0.0 { let mut xs: Vec<f64> = pts.iter().take(5000).map(|p| p.0).collect(); xs.sort_by(f64::total_cmp); step = xs.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 1e-9).fold(f64::MAX, f64::min); }
    if !(step.is_finite() && step > 0.0) { return Err("The step between points could not be found.".into()) }
    if ystep.is_nan() || ystep <= 0.0 { ystep = step }
    let (wf, hf) = (((xmax - xmin) / step).round() + 1.0, ((ymax - ymin) / ystep).round() + 1.0);
    if !(wf.is_finite() && hf.is_finite() && wf * hf <= (MAX_POINTS * 2) as f64) { return Err("The map's grid is too large.".into()) }
    let (w, h) = (wf as usize, hf as usize);
    if !(w.checked_mul(h).is_some_and(|n| n <= MAX_POINTS * 2)) || !(xmax - xmin).is_finite() || !(ymax - ymin).is_finite() { return Err("The map's grid is too large.".into()) }
    let mut m = Ebsd { name: name.into(), format: format.into(), w, h, step, euler: vec![[0.0; 3]; w * h], phase: vec![0; w * h], quality: vec![f32::NAN; w * h], conf: vec![f32::NAN; w * h], phases, quality_name: qn.into(), conf_name: cn.into(), notes };
    let np = m.phases.len() as u8;
    for p in &pts {
        let (i, j) = (((p.0 - xmin) / step).round() as usize, ((p.1 - ymin) / ystep).round() as usize);
        if i >= w || j >= h { continue }
        let k = j * w + i;
        m.euler[k] = p.2;
        m.phase[k] = if p.3 > np { 0 } else { p.3 };
        m.quality[k] = p.4;
        m.conf[k] = p.5;
    }
    Ok(m)
}

/// A made-up map (named so): the structure generator's two-phase, part-Goss map as an orientation map.
pub fn example() -> Result<Ebsd, String> {
    let q = crate::synth::Ask { w: 320, h: 240, step: 0.5, size: 12.0, texture: "goss".into(), share: 0.5, phase2: 0.2, ..Default::default() };
    let mut m = parse(&crate::synth::ebsd_text(&q)?, "Example (made-up map)")?;
    m.notes.push("A made-up map from the structure generator: two cubic phases, half the grains near Goss.".into());
    Ok(m)
}

impl Ebsd {
    pub fn summary(&self) -> Summary {
        let n = (self.w * self.h) as f64;
        let idx = self.phase.iter().filter(|&&p| p > 0).count() as f64;
        Summary { name: self.name.clone(), format: self.format.clone(), w: self.w, h: self.h, step: self.step, points: self.w * self.h, indexed: idx / n * 100.0,
            phases: self.phases.iter().enumerate().map(|(k, p)| (p.clone(), self.phase.iter().filter(|&&q| q as usize == k + 1).count() as f64 / n * 100.0)).collect(),
            quality_name: self.quality_name.clone(), conf_name: self.conf_name.clone(), notes: self.notes.clone() }
    }
    pub fn crystal(&self, ph: u8) -> &str { self.phases.get(ph as usize - 1).map_or("cubic", |p| p.crystal.as_str()) }
    pub fn g(&self, k: usize) -> orient::M3 { orient::from_euler(self.euler[k].map(|v| v as f64)) }
    /// A map as RGB bytes (base64): "ipf" (of sample direction `dir`: x, y or z), "phase", "quality" or "confidence".
    pub fn draw(&self, kind: &str, dir: &str) -> Result<String, String> {
        let s = match dir { "x" => [1.0, 0.0, 0.0], "y" => [0.0, 1.0, 0.0], _ => [0.0, 0.0, 1.0] };
        const PAL: [[u8; 3]; 6] = [[108, 74, 182], [242, 178, 51], [0, 158, 115], [213, 94, 0], [0, 114, 178], [204, 121, 167]];
        let grey = |v: &Vec<f32>| { let (lo, hi) = v.iter().filter(|x| x.is_finite()).fold((f32::MAX, f32::MIN), |a, &x| (a.0.min(x), a.1.max(x))); move |x: f32| if x.is_finite() && hi > lo { (((x - lo) / (hi - lo)) * 255.0) as u8 } else { 0 } };
        let (gq, gc) = (grey(&self.quality), grey(&self.conf));
        if (kind == "quality" && self.quality.iter().all(|x| !x.is_finite())) || (kind == "confidence" && self.conf.iter().all(|x| !x.is_finite())) { return Err("This map has no such column.".into()) }
        let mut out = Vec::with_capacity(self.w * self.h * 3);
        for k in 0..self.w * self.h {
            let ph = self.phase[k];
            let c = match kind {
                "quality" => [gq(self.quality[k]); 3],
                "confidence" => [gc(self.conf[k]); 3],
                _ if ph == 0 => [0, 0, 0],
                "phase" => PAL[(ph as usize - 1) % PAL.len()],
                _ => orient::ipf(&self.g(k), s, self.crystal(ph)),
            };
            out.extend_from_slice(&c);
        }
        Ok(b64_encode(&out))
    }
}
