//! Phase identification: a reference pack of inorganic structures from the Crystallography Open
//! Database (public domain), each reduced to its strongest powder lines (d and |F|²·multiplicity,
//! so any wavelength works), and search–match of a measured pattern's peaks against it, with an
//! element filter; rough weight fractions of the accepted phases from their reference intensity
//! ratios, worked out from the structures (|F|², cell volume and density).
use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8; 8] = b"DATLASPK";
pub const VERSION: u32 = 1;
const LINES: usize = 30;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub cod: u32, pub name: String, pub formula: String, pub sg: String,
    pub cell: [f32; 6], pub volume: f32, pub density: f32, pub elements: Vec<u8>,
    /// Lines: d (Å) and |F|²·multiplicity (absolute, electrons²).
    pub lines: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pack { pub date: String, pub entries: Vec<Entry> }

/// Inorganic: no carbon–hydrogen compounds (carbonates and carbides are kept).
pub fn inorganic(elements: &[u8]) -> bool { !(elements.contains(&6) && elements.contains(&1)) }

/// One pack entry from a CIF: its strongest lines (Cu Kα1 ranking, d down to 0.82 Å).
pub fn entry(cod: u32, text: &str) -> Result<Entry, String> {
    let p = crate::cif::parse(text, &cod.to_string())?;
    let mut el: Vec<u8> = p.sites.iter().filter_map(|s| crate::elements::find(&s.element)).map(|e| e.z as u8).collect();
    el.sort_unstable(); el.dedup();
    if !inorganic(&el) { return Err("organic".into()) }
    let inf = crate::cif::info(&p);
    if inf.volume > 20_000.0 { return Err("cell too large".into()) }
    let mut ls = crate::cif::pattern(&p, 1.540_592_9, "", 140.0, 0.0)?;
    ls.sort_by(|a, b| b.intensity.total_cmp(&a.intensity));
    ls.truncate(LINES);
    ls.sort_by(|a, b| b.d.total_cmp(&a.d));
    let c = p.cell;
    Ok(Entry { cod, name: p.name, formula: p.formula, sg: p.space_group, cell: [c.a, c.b, c.c, c.alpha, c.beta, c.gamma].map(|v| v as f32),
        volume: inf.volume as f32, density: (inf.density / 1000.0) as f32, elements: el, lines: ls.iter().map(|q| (q.d as f32, (q.f2 * q.mult as f64) as f32)).collect() })
}

/// Build a pack from every .cif file under `dir` (named by its COD number), on all cores.
/// Returns the pack and how many files were kept, organic, too large or unreadable.
pub fn build_dir(dir: &std::path::Path, date: &str) -> Result<(Pack, [usize; 4]), String> {
    let mut files = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for f in std::fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))?.flatten() {
            let p = f.path();
            if p.is_dir() { stack.push(p) } else if p.extension().is_some_and(|x| x == "cif") { files.push(p) }
        }
    }
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = files.len().div_ceil(threads).max(1);
    let parts: Vec<(Vec<Entry>, [usize; 4])> = std::thread::scope(|sc| {
        files.chunks(chunk).map(|c| sc.spawn(move || {
            let (mut v, mut st) = (vec![], [0usize; 4]);
            for p in c {
                let Some(cod) = p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<u32>().ok()) else { st[3] += 1; continue };
                let Ok(text) = std::fs::read(p).map(|b| String::from_utf8_lossy(&b).into_owned()) else { st[3] += 1; continue };
                match entry(cod, &text) { Ok(e) if !e.lines.is_empty() => { v.push(e); st[0] += 1 } Err(e) if e == "organic" => st[1] += 1, Err(e) if e == "cell too large" => st[2] += 1, _ => st[3] += 1 }
            }
            (v, st)
        })).collect::<Vec<_>>().into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    let mut stats = [0usize; 4];
    let mut entries = vec![];
    for (v, s) in parts { entries.extend(v); for k in 0..4 { stats[k] += s[k] } }
    entries.sort_by_key(|e| e.cod);
    Ok((Pack { date: date.into(), entries }, stats))
}

// ---------- the pack file ----------

fn put_str(o: &mut Vec<u8>, s: &str) { let b = s.as_bytes(); let n = b.len().min(255); o.push(n as u8); o.extend(&b[..n]); }
fn put_f(o: &mut Vec<u8>, v: f32) { o.extend(v.to_le_bytes()) }

/// Write a pack: magic, version, then the zlib-compressed body (whose checksum guards it).
pub fn write(p: &Pack) -> Vec<u8> {
    let mut b = vec![];
    put_str(&mut b, &p.date);
    b.extend((p.entries.len() as u32).to_le_bytes());
    for e in &p.entries {
        b.extend(e.cod.to_le_bytes());
        put_str(&mut b, &e.name); put_str(&mut b, &e.formula); put_str(&mut b, &e.sg);
        for v in e.cell { put_f(&mut b, v) }
        put_f(&mut b, e.volume); put_f(&mut b, e.density);
        b.push(e.elements.len() as u8); b.extend(&e.elements);
        b.push(e.lines.len() as u8);
        for (d, f) in &e.lines { put_f(&mut b, *d); put_f(&mut b, *f) }
    }
    let mut out = MAGIC.to_vec();
    out.extend(VERSION.to_le_bytes());
    out.extend(miniz_oxide::deflate::compress_to_vec_zlib(&b, 9));
    out
}

const BAD: &str = "This is not a Datum Atlas reference pack, or it is damaged: download it again.";

/// Read a pack; a damaged file (or another file) is refused, never half read.
pub fn read(bytes: &[u8]) -> Result<Pack, String> {
    if bytes.len() < 12 || &bytes[..8] != MAGIC { return Err(BAD.into()) }
    let v = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| BAD)?);
    if v > VERSION { return Err("This reference pack is newer than this Datum Atlas: update the app.".into()) }
    let b = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&bytes[12..], 1 << 31).map_err(|_| BAD)?;
    let mut k = 0usize;
    let take = |k: &mut usize, n: usize| -> Result<&[u8], String> { let s = b.get(*k..*k + n).ok_or(BAD)?; *k += n; Ok(s) };
    let get_str = |k: &mut usize| -> Result<String, String> { let n = take(k, 1)?[0] as usize; Ok(String::from_utf8_lossy(take(k, n)?).into_owned()) };
    let get_f = |k: &mut usize| -> Result<f32, String> { Ok(f32::from_le_bytes(take(k, 4)?.try_into().map_err(|_| BAD)?)) };
    let date = get_str(&mut k)?;
    let n = u32::from_le_bytes(take(&mut k, 4)?.try_into().map_err(|_| BAD)?) as usize;
    let mut entries = Vec::with_capacity(n.min(2_000_000));
    for _ in 0..n {
        let cod = u32::from_le_bytes(take(&mut k, 4)?.try_into().map_err(|_| BAD)?);
        let (name, formula, sg) = (get_str(&mut k)?, get_str(&mut k)?, get_str(&mut k)?);
        let mut cell = [0f32; 6];
        for c in &mut cell { *c = get_f(&mut k)? }
        let (volume, density) = (get_f(&mut k)?, get_f(&mut k)?);
        let ne = take(&mut k, 1)?[0] as usize;
        let elements = take(&mut k, ne)?.to_vec();
        let nl = take(&mut k, 1)?[0] as usize;
        let mut lines = Vec::with_capacity(nl);
        for _ in 0..nl { lines.push((get_f(&mut k)?, get_f(&mut k)?)) }
        entries.push(Entry { cod, name, formula, sg, cell, volume, density, elements, lines });
    }
    if k != b.len() { return Err(BAD.into()) }
    Ok(Pack { date, entries })
}

// ---------- search–match ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Query {
    /// Elements the phase must contain, may contain (empty: any), must not contain.
    pub must: Vec<String>, pub may: Vec<String>, pub not: Vec<String>,
    /// Position tolerance (degrees 2θ).
    pub tol: f64,
    /// Phases already accepted (COD numbers): their peaks count as explained.
    pub chosen: Vec<u32>,
    pub limit: usize,
}
impl Default for Query { fn default() -> Self { Self { must: vec![], may: vec![], not: vec![], tol: 0.15, chosen: vec![], limit: 30 } } }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit { pub cod: u32, pub name: String, pub formula: String, pub sg: String, pub score: f64, pub found: usize, pub strong: usize, pub explained: f64, pub shift: f64, pub density: f32,
    /// The phase's lines in the measured range: 2θ and intensity (strongest = 100).
    pub lines: Vec<[f64; 2]> }

fn zs(list: &[String]) -> Result<Vec<u8>, String> {
    list.iter().filter(|s| !s.trim().is_empty()).map(|s| crate::elements::find(s.trim()).map(|e| e.z as u8).ok_or(format!("“{}” is not an element symbol.", s.trim()))).collect()
}

fn lp(tt: f64) -> f64 { let th = (tt / 2.0).to_radians(); (1.0 + (2.0 * th).cos().powi(2)) / (th.sin().powi(2) * th.cos()) }

/// An entry's lines in [lo, hi] degrees 2θ for wavelength λ: (2θ, |F|²m·LP, intensity scaled to 100).
pub fn lines_at(e: &Entry, lambda: f64, lo: f64, hi: f64) -> Vec<(f64, f64, f64)> {
    let mut v: Vec<(f64, f64)> = e.lines.iter().filter_map(|&(d, f)| { let s = lambda / 2.0 / d as f64; (s < 1.0).then(|| { let tt = 2.0 * s.asin().to_degrees(); (tt, f as f64 * lp(tt)) }) }).filter(|q| q.0 >= lo && q.0 <= hi).collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let top = v.iter().map(|q| q.1).fold(0.0, f64::max).max(f64::MIN_POSITIVE);
    v.into_iter().map(|(t, i)| (t, i, i / top * 100.0)).collect()
}

/// The measured peaks explained by the accepted phases' lines (above 3 % of their strongest).
fn explained_by(pack: &Pack, peaks: &[[f64; 2]], lambda: f64, chosen: &[u32], tol: f64, lo: f64, hi: f64) -> Vec<bool> {
    let mut x = vec![false; peaks.len()];
    for e in pack.entries.iter().filter(|e| chosen.contains(&e.cod)) {
        for (t, _, i) in lines_at(e, lambda, lo, hi) { if i >= 3.0 { for (k, p) in peaks.iter().enumerate() { if (p[0] - t).abs() <= tol { x[k] = true } } } }
    }
    x
}

/// Rank the pack's phases against measured peaks [2θ, intensity] for wavelength λ.
pub fn search(pack: &Pack, peaks: &[[f64; 2]], lambda: f64, q: &Query) -> Result<Vec<Hit>, String> {
    if peaks.is_empty() { return Err("No peaks to match: find the peaks first (Peaks page).".into()) }
    let (must, may, not) = (zs(&q.must)?, zs(&q.may)?, zs(&q.not)?);
    let tol = q.tol.clamp(0.02, 1.0);
    let lo = peaks.iter().map(|p| p[0]).fold(f64::MAX, f64::min) - 2.0 * tol;
    let hi = peaks.iter().map(|p| p[0]).fold(f64::MIN, f64::max) + 2.0 * tol;
    let used = explained_by(pack, peaks, lambda, &q.chosen, tol, lo, hi);
    let total: f64 = peaks.iter().zip(&used).filter(|(_, u)| !**u).map(|(p, _)| p[1].max(0.0)).sum();
    let mut hits = vec![];
    for e in &pack.entries {
        if q.chosen.contains(&e.cod) { continue }
        if !must.iter().all(|z| e.elements.contains(z)) || e.elements.iter().any(|z| not.contains(z)) { continue }
        if !may.is_empty() && !e.elements.iter().all(|z| may.contains(z) || must.contains(z)) { continue }
        let ls = lines_at(e, lambda, lo, hi);
        let strong: Vec<&(f64, f64, f64)> = ls.iter().filter(|l| l.2 >= 5.0).collect();
        if strong.is_empty() { continue }
        let (mut wf, mut wt, mut found, mut dsum) = (0.0, 0.0, 0usize, 0.0);
        let mut hit_peaks = vec![false; peaks.len()];
        for l in &strong {
            wt += l.2;
            let near = peaks.iter().enumerate().map(|(k, p)| (k, (p[0] - l.0).abs())).filter(|x| x.1 <= tol).min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((k, dd)) = near { wf += l.2; found += 1; dsum += dd; hit_peaks[k] = true }
        }
        if found < strong.len().min(2) { continue }
        let s1 = wf / wt;
        let expl: f64 = peaks.iter().zip(&hit_peaks).zip(&used).filter(|((_, h), u)| **h && !**u).map(|((p, _), _)| p[1].max(0.0)).sum();
        let s2 = if total > 0.0 { expl / total } else { 0.0 };
        let s3 = 1.0 - dsum / found as f64 / tol;
        let score = 100.0 * s1 * s2.sqrt() * (0.5 + 0.5 * s3);
        if score <= 0.5 { continue }
        hits.push(Hit { cod: e.cod, name: e.name.clone(), formula: e.formula.clone(), sg: e.sg.clone(), score, found, strong: strong.len(), explained: s2 * 100.0, shift: dsum / found as f64, density: e.density,
            lines: ls.iter().map(|l| [l.0, l.2]).collect() });
    }
    hits.sort_by(|a, b| b.score.total_cmp(&a.score));
    hits.truncate(q.limit.clamp(1, 200));
    Ok(hits)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fraction { pub cod: u32, pub name: String, pub weight: Option<f64>, pub line: Option<f64>, pub note: String }

/// Rough weight fractions of the accepted phases by the reference intensity ratio method: for each,
/// the measured intensity of the peak at its strongest line, divided by that line's calculated
/// |F|²m·LP/(V²ρ); normalised to 100 %. Overlapping lines and preferred orientation make it rough.
pub fn fractions(pack: &Pack, peaks: &[[f64; 2]], lambda: f64, chosen: &[u32], tol: f64) -> Vec<Fraction> {
    let lo = peaks.iter().map(|p| p[0]).fold(f64::MAX, f64::min) - tol;
    let hi = peaks.iter().map(|p| p[0]).fold(f64::MIN, f64::max) + tol;
    let mut out: Vec<(Fraction, f64)> = vec![];
    for &c in chosen {
        let Some(e) = pack.entries.iter().find(|e| e.cod == c) else { continue };
        let ls = lines_at(e, lambda, lo, hi);
        let mut f = Fraction { cod: c, name: e.name.clone(), weight: None, line: None, note: String::new() };
        // Its strongest line that has a measured peak, and that no other accepted phase claims.
        let others: Vec<f64> = chosen.iter().filter(|&&o| o != c).filter_map(|o| pack.entries.iter().find(|e| e.cod == *o)).flat_map(|o| lines_at(o, lambda, lo, hi).into_iter().filter(|l| l.2 >= 10.0).map(|l| l.0)).collect();
        let mut cands: Vec<&(f64, f64, f64)> = ls.iter().filter(|l| l.2 >= 20.0).collect();
        cands.sort_by(|a, b| b.2.total_cmp(&a.2));
        let mut val = 0.0;
        for l in cands {
            if others.iter().any(|t| (t - l.0).abs() <= 2.0 * tol) { continue }
            if let Some(p) = peaks.iter().filter(|p| (p[0] - l.0).abs() <= tol).max_by(|a, b| a[1].total_cmp(&b[1])) {
                val = p[1] / (l.1 / (e.volume as f64).powi(2) / e.density.max(0.01) as f64);
                f.line = Some(l.0);
                break;
            }
        }
        if f.line.is_none() { f.note = "No peak of its own (all its strong lines overlap another phase's, or are missing).".into() }
        out.push((f, val));
    }
    let sum: f64 = out.iter().map(|x| x.1).sum();
    out.into_iter().map(|(mut f, v)| { if sum > 0.0 && f.line.is_some() { f.weight = Some(v / sum * 100.0) } f }).collect()
}

/// Phase identification of a measured pattern: its peaks (with the Peaks page's settings), the
/// ranked candidates, the weight fractions of the accepted phases, and what the graph needs.
pub fn run(pack: &Pack, pattern: &crate::xrd::Pattern, ask: &crate::xrd::Ask, q: &Query) -> Result<serde_json::Value, String> {
    let a = crate::xrd::analyse(pattern, ask)?;
    let peaks: Vec<[f64; 2]> = a.peaks.iter().map(|p| [p.two_theta, p.area.max(0.0)]).collect();
    let hits = search(pack, &peaks, a.lambda, q)?;
    let chosen: Vec<serde_json::Value> = q.chosen.iter().filter_map(|c| pack.entries.iter().find(|e| e.cod == *c)).map(|e| {
        let lo = a.x.first().copied().unwrap_or(0.0); let hi = a.x.last().copied().unwrap_or(180.0);
        serde_json::json!({ "cod": e.cod, "name": e.name, "formula": e.formula, "sg": e.sg, "lines": lines_at(e, a.lambda, lo, hi).iter().map(|l| [l.0, l.2]).collect::<Vec<_>>() })
    }).collect();
    let step = a.x.len().div_ceil(3000).max(1);
    Ok(serde_json::json!({ "hits": hits, "chosen": chosen, "fractions": fractions(pack, &peaks, a.lambda, &q.chosen, q.tol.clamp(0.02, 1.0)), "peaks": peaks, "lambda": a.lambda,
        "x": a.x.iter().step_by(step).collect::<Vec<_>>(), "y": a.net.iter().step_by(step).collect::<Vec<_>>(), "notes": a.notes }))
}

#[cfg(test)]
pub mod t {
    use super::*;
    /// A pack built from the COD test files in tests_ui/cod.
    pub fn test_pack() -> Pack {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests_ui/cod/");
        let mut entries = vec![];
        for f in std::fs::read_dir(dir).unwrap().flatten() {
            let n = f.file_name().to_string_lossy().to_string();
            if let Some(id) = n.strip_suffix(".cif") { entries.push(entry(id.parse().unwrap(), &std::fs::read_to_string(f.path()).unwrap()).unwrap()) }
        }
        entries.sort_by_key(|e| e.cod);
        Pack { date: "test".into(), entries }
    }
    #[test]
    fn pack_round_trip() {
        let p = test_pack();
        assert_eq!(p.entries.len(), 12);
        let b = write(&p);
        assert_eq!(read(&b).unwrap(), p);
        let mut bad = b.clone(); let n = bad.len(); bad[n / 2] ^= 0xff;
        assert!(read(&bad).is_err(), "a damaged pack is refused");
        assert!(read(b"hello").is_err());
        assert!(inorganic(&[6, 8, 20]) && !inorganic(&[1, 6, 8]));
    }
    #[test]
    fn identifies_the_example_mixture() {
        // The whole way: the made-up pattern, its peaks found and fitted, then search–match.
        let pack = test_pack();
        let pat = crate::xrd::example_mix().unwrap();
        let a = crate::xrd::analyse(&pat, &crate::xrd::Ask { strip: true, ..Default::default() }).unwrap();
        let peaks: Vec<[f64; 2]> = a.peaks.iter().map(|p| [p.two_theta, p.area]).collect();
        let s = search(&pack, &peaks, a.lambda, &Query::default()).unwrap();
        let top2: Vec<u32> = s.iter().take(2).map(|h| h.cod).collect();
        assert!(top2.contains(&1000017) && top2.contains(&2300370), "{:?}", s.iter().map(|h| (h.cod, &h.name, h.score as i32)).collect::<Vec<_>>());
        let f = fractions(&pack, &peaks, a.lambda, &[1000017, 2300370], 0.15);
        let w: Vec<f64> = f.iter().map(|x| x.weight.unwrap_or(0.0)).collect();
        assert!((w[0] - 60.0).abs() < 10.0 && (w[1] - 40.0).abs() < 10.0, "{w:?} {f:?}");
    }
    #[test]
    fn builds_from_a_folder() {
        let (p, st) = build_dir(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests_ui/cod/")), "test").unwrap();
        assert_eq!((p.entries.len(), st), (12, [12, 0, 0, 0]));
        assert_eq!(p, test_pack());
    }
    #[test]
    fn finds_a_mixture() {
        let pack = test_pack();
        let lambda = 1.540_592_9;
        // A measured mixture: corundum and quartz lines (from their own structures), weighted 60:40 by mass.
        let mut peaks: Vec<[f64; 2]> = vec![];
        for (cod, w) in [(1000017u32, 0.6), (2300370u32, 0.4)] {
            let e = pack.entries.iter().find(|e| e.cod == cod).unwrap();
            for (t, i, _) in lines_at(e, lambda, 15.0, 80.0) { peaks.push([t + 0.02, w * i / (e.volume as f64).powi(2) / e.density as f64]) }
        }
        let s = search(&pack, &peaks, lambda, &Query::default()).unwrap();
        let top2: Vec<u32> = s.iter().take(2).map(|h| h.cod).collect();
        assert!(top2.contains(&1000017) && top2.contains(&2300370), "{:?}", s.iter().map(|h| (h.cod, &h.name, h.score)).collect::<Vec<_>>());
        // After accepting corundum, quartz explains what is left.
        let s2 = search(&pack, &peaks, lambda, &Query { chosen: vec![1000017], ..Default::default() }).unwrap();
        assert_eq!(s2[0].cod, 2300370);
        assert!(s2[0].explained > 80.0, "{}", s2[0].explained);
        // Element filter: only phases with Ti.
        let s3 = search(&pack, &peaks, lambda, &Query { must: vec!["Ti".into()], ..Default::default() }).unwrap();
        assert!(s3.iter().all(|h| h.formula.contains("Ti")));
        assert!(search(&pack, &peaks, lambda, &Query { must: vec!["Xx".into()], ..Default::default() }).is_err());
        // Weight fractions back from the intensities.
        let f = fractions(&pack, &peaks, lambda, &[1000017, 2300370], 0.15);
        let w: Vec<f64> = f.iter().map(|x| x.weight.unwrap()).collect();
        assert!((w[0] - 60.0).abs() < 3.0 && (w[1] - 40.0).abs() < 3.0, "{w:?}");
    }
}
