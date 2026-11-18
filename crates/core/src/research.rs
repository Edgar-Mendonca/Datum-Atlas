//! Research work kept in the project: the micrograph on Microstructure › Images, the EBSD map and
//! the DIC displacement field, each with its page settings. The data is packed (a small JSON head,
//! then the arrays as little-endian bytes), deflated and stored as base64 text in the .datlas file.
//! Packed data is shared (`Arc`), so undo copies of the project cost nothing, and the window gets
//! the project without it (`Project::lite`).
use crate::{dic::Dic, ebsd::Ebsd, image::{b64_decode, b64_encode}};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

pub const KINDS: [&str; 4] = ["image", "ebsd", "dic", "xrd"];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Research { pub image: Option<Item>, pub ebsd: Option<Item>, pub dic: Option<Item>, /// X-ray patterns (all of them in one item).
    pub xrd: Option<Item> }

/// One piece of kept work: what the pages show about it (`info`), a fingerprint of the data (`id`,
/// so a page knows when it changed), the packed data and the page settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Item { pub info: Value, pub id: String, pub data: Arc<str>, pub settings: Value }

impl Research {
    pub fn slot(&mut self, which: &str) -> Result<&mut Option<Item>, String> {
        match which { "image" => Ok(&mut self.image), "ebsd" => Ok(&mut self.ebsd), "dic" => Ok(&mut self.dic), "xrd" => Ok(&mut self.xrd), _ => Err(format!("No research slot “{which}”.")) }
    }
    /// The same, without the packed data (for the window).
    pub fn lite(&self) -> Self {
        let f = |i: &Option<Item>| i.as_ref().map(|i| Item { data: Arc::from(""), ..i.clone() });
        Self { image: f(&self.image), ebsd: f(&self.ebsd), dic: f(&self.dic), xrd: f(&self.xrd) }
    }
}

fn fnv(s: &str) -> String { format!("{:016x}", s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))) }

impl Item {
    fn new(info: Value, head: Value, parts: &[Vec<u8>]) -> Self {
        let data = pack(&head, parts);
        Self { info, id: fnv(&data), data: data.into(), settings: json!({}) }
    }
    /// A grey picture (`w` × `h` bytes) for image analysis.
    pub fn image(name: &str, w: usize, h: usize, px: &[u8], made: bool) -> Result<Self, String> {
        if w == 0 || h == 0 || px.len() != w * h { return Err("The picture's size does not match its pixels.".into()); }
        Ok(Self::new(json!({ "name": name, "w": w, "h": h, "made": made }), json!({ "w": w, "h": h }), &[px.to_vec()]))
    }
    pub fn image_px(&self) -> Result<(usize, usize, Vec<u8>), String> {
        let (h, mut p) = unpack(&self.data)?;
        let (w, ht) = (us(&h["w"]), us(&h["h"]));
        let px = p.pop().filter(|p| p.len() == w * ht && w > 0).ok_or(DAMAGED)?;
        Ok((w, ht, px))
    }
    pub fn ebsd(m: &Ebsd) -> Self {
        let head = json!({ "name": m.name, "format": m.format, "w": m.w, "h": m.h, "step": m.step, "phases": m.phases, "qn": m.quality_name, "cn": m.conf_name, "notes": m.notes });
        let e: Vec<f32> = m.euler.iter().flatten().copied().collect();
        Self::new(json!(m.summary()), head, &[f32s(&e), m.phase.clone(), f32s(&m.quality), f32s(&m.conf)])
    }
    pub fn to_ebsd(&self) -> Result<Ebsd, String> {
        let (h, p) = unpack(&self.data)?;
        let (w, ht) = (us(&h["w"]), us(&h["h"]));
        let n = w * ht;
        let [e, ph, q, c] = <[Vec<u8>; 4]>::try_from(p).map_err(|_| DAMAGED)?;
        let (e, q, c) = (to_f32(&e), to_f32(&q), to_f32(&c));
        if n == 0 || e.len() != 3 * n || ph.len() != n || q.len() != n || c.len() != n { return Err(DAMAGED.into()); }
        Ok(Ebsd { name: st(&h["name"]), format: st(&h["format"]), w, h: ht, step: h["step"].as_f64().unwrap_or(1.0), euler: e.chunks(3).map(|k| [k[0], k[1], k[2]]).collect(), phase: ph, quality: q, conf: c,
            phases: serde_json::from_value(h["phases"].clone()).map_err(|_| DAMAGED)?, quality_name: st(&h["qn"]), conf_name: st(&h["cn"]), notes: serde_json::from_value(h["notes"].clone()).unwrap_or_default() })
    }
    pub fn dic(d: &Dic) -> Self {
        let head = json!({ "name": d.name, "w": d.w, "h": d.h, "x0": d.x0, "y0": d.y0, "dx": d.dx, "dy": d.dy, "notes": d.notes });
        Self::new(json!(d.summary()), head, &[f64s(&d.u), f64s(&d.v)])
    }
    pub fn to_dic(&self) -> Result<Dic, String> {
        let (h, p) = unpack(&self.data)?;
        let (w, ht) = (us(&h["w"]), us(&h["h"]));
        let [u, v] = <[Vec<u8>; 2]>::try_from(p).map_err(|_| DAMAGED)?;
        let (u, v) = (to_f64(&u), to_f64(&v));
        if w * ht == 0 || u.len() != w * ht || v.len() != w * ht { return Err(DAMAGED.into()); }
        let g = |k: &str| h[k].as_f64().unwrap_or(f64::NAN);
        Ok(Dic { name: st(&h["name"]), w, h: ht, x0: g("x0"), y0: g("y0"), dx: g("dx"), dy: g("dy"), u, v, notes: serde_json::from_value(h["notes"].clone()).unwrap_or_default() })
    }
}

impl Item {
    /// X-ray patterns, all in one item; `info` lists each pattern's name, points, range and anode.
    pub fn xrd(ps: &[crate::xrd::Pattern], settings: serde_json::Value) -> Self {
        let head = json!(ps.iter().map(|p| json!({ "name": p.name, "anode": p.anode, "notes": p.notes })).collect::<Vec<_>>());
        let parts: Vec<Vec<u8>> = ps.iter().flat_map(|p| [f64s(&p.x), f64s(&p.y)]).collect();
        let info = json!(ps.iter().map(|p| json!({ "name": p.name, "points": p.x.len(), "from": p.x.first(), "to": p.x.last(), "anode": p.anode, "notes": p.notes })).collect::<Vec<_>>());
        Self { settings, ..Self::new(info, head, &parts) }
    }
    pub fn to_xrd(&self) -> Result<Vec<crate::xrd::Pattern>, String> {
        let (h, p) = unpack(&self.data)?;
        let heads = h.as_array().ok_or(DAMAGED)?;
        if p.len() != 2 * heads.len() { return Err(DAMAGED.into()) }
        heads.iter().enumerate().map(|(k, m)| {
            let (x, y) = (to_f64(&p[2 * k]), to_f64(&p[2 * k + 1]));
            if x.len() != y.len() || x.is_empty() { return Err(DAMAGED.to_string()) }
            Ok(crate::xrd::Pattern { name: st(&m["name"]), anode: st(&m["anode"]), notes: serde_json::from_value(m["notes"].clone()).unwrap_or_default(), x, y })
        }).collect()
    }
}

const DAMAGED: &str = "The research data kept in this project is damaged and could not be read.";
fn us(v: &Value) -> usize { v.as_u64().unwrap_or(0) as usize }
fn st(v: &Value) -> String { v.as_str().unwrap_or("").to_string() }
fn f32s(v: &[f32]) -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() }
fn f64s(v: &[f64]) -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() }
fn to_f32(b: &[u8]) -> Vec<f32> { b.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect() }
fn to_f64(b: &[u8]) -> Vec<f64> { b.as_chunks::<8>().0.iter().map(|c| f64::from_le_bytes(*c)).collect() }

/// The head as JSON, then each part, each with its length (u64, little-endian); deflated, base64.
pub fn pack(head: &Value, parts: &[Vec<u8>]) -> String {
    let mut raw = vec![];
    for p in std::iter::once(head.to_string().into_bytes()).chain(parts.iter().cloned()) {
        raw.extend((p.len() as u64).to_le_bytes());
        raw.extend(p);
    }
    b64_encode(&miniz_oxide::deflate::compress_to_vec(&raw, 6))
}

pub fn unpack(s: &str) -> Result<(Value, Vec<Vec<u8>>), String> {
    let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(&b64_decode(s).map_err(|_| DAMAGED)?, 1 << 31).map_err(|_| DAMAGED)?;
    let (mut parts, mut k) = (vec![], 0usize);
    while k < raw.len() {
        let n = raw.get(k..k + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8])) as usize).ok_or(DAMAGED)?;
        let p = raw.get(k + 8..(k + 8).checked_add(n).ok_or(DAMAGED)?).ok_or(DAMAGED)?;
        parts.push(p.to_vec());
        k += 8 + n;
    }
    if parts.is_empty() { return Err(DAMAGED.into()); }
    let head = serde_json::from_slice(&parts.remove(0)).map_err(|_| DAMAGED)?;
    Ok((head, parts))
}

/// The EBSD map and DIC field of the open project, unpacked once and reused until the project's
/// data changes (opening another file, undo, a new project).
#[derive(Default)]
pub struct Open { ebsd: Option<(Arc<str>, Result<Ebsd, String>)>, dic: Option<(Arc<str>, Result<Dic, String>)> }

impl Open {
    fn sync(&mut self, r: &Research) {
        match &r.ebsd {
            Some(i) if !matches!(&self.ebsd, Some((d, _)) if Arc::ptr_eq(d, &i.data)) => self.ebsd = Some((i.data.clone(), i.to_ebsd())),
            None => self.ebsd = None,
            _ => {}
        }
        match &r.dic {
            Some(i) if !matches!(&self.dic, Some((d, _)) if Arc::ptr_eq(d, &i.data)) => self.dic = Some((i.data.clone(), i.to_dic())),
            None => self.dic = None,
            _ => {}
        }
    }
    pub fn ebsd(&mut self, r: &Research) -> Result<&Ebsd, String> {
        self.sync(r);
        match &self.ebsd { Some((_, Ok(m))) => Ok(m), Some((_, Err(e))) => Err(e.clone()), None => Err("Open an EBSD map first.".into()) }
    }
    pub fn dic(&mut self, r: &Research) -> Result<&Dic, String> {
        self.sync(r);
        match &self.dic { Some((_, Ok(d))) => Ok(d), Some((_, Err(e))) => Err(e.clone()), None => Err("Open a displacement field first.".into()) }
    }
    /// Both, for strain by grain.
    pub fn both(&mut self, r: &Research) -> Result<(&Ebsd, &Dic), String> {
        self.sync(r);
        let m = match &self.ebsd { Some((_, Ok(m))) => m, Some((_, Err(e))) => return Err(e.clone()), None => return Err("Open an EBSD map on EBSD › Maps first.".into()) };
        let d = match &self.dic { Some((_, Ok(d))) => d, Some((_, Err(e))) => return Err(e.clone()), None => return Err("Open a displacement field on DIC › Strain maps first.".into()) };
        Ok((m, d))
    }
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn round_trips() {
        let m = crate::ebsd::example().unwrap();
        let i = Item::ebsd(&m);
        let b = i.to_ebsd().unwrap();
        assert_eq!((b.w, b.h, b.step, b.phase.len()), (m.w, m.h, m.step, m.phase.len()));
        assert_eq!(b.euler, m.euler);
        assert_eq!(b.phases.len(), m.phases.len());
        assert_eq!(i.info["points"], json!(m.w * m.h));
        let d = crate::dic::example_on(&m);
        let e = Item::dic(&d).to_dic().unwrap();
        assert_eq!((e.w, e.h, e.dx), (d.w, d.h, d.dx));
        assert!(e.u.iter().zip(&d.u).all(|(a, b)| a == b || (a.is_nan() && b.is_nan())));
        let px: Vec<u8> = (0..600u32).map(|k| (k % 251) as u8).collect();
        let im = Item::image("a.png", 30, 20, &px, false).unwrap();
        assert_eq!(im.image_px().unwrap(), (30, 20, px.clone()));
        assert!(Item::image("a", 31, 20, &px, false).is_err());
        // Same data, same fingerprint; damage is an error, never a panic.
        assert_eq!(Item::image("b", 30, 20, &px, true).unwrap().id, im.id);
        for bad in ["", "AAAA", &im.data[..im.data.len() / 2]] { assert!(Item { data: bad.into(), ..im.clone() }.image_px().is_err()); }
        assert!(Item { data: im.data.clone(), ..Item::default() }.to_ebsd().is_err());
    }
    #[test]
    fn xrd_patterns_round_trip() {
        let a = crate::xrd::example().unwrap();
        let mut b = a.clone(); b.name = "second".into(); b.anode = "".into();
        let it = Item::xrd(&[a.clone(), b.clone()], json!({ "k": 1 }));
        assert_eq!(it.to_xrd().unwrap(), vec![a, b]);
        assert_eq!(it.info[1]["name"], json!("second"));
        assert_eq!(it.settings["k"], json!(1));
        assert_eq!(Item::xrd(&[], json!({})).to_xrd().unwrap(), vec![]);
    }
    #[test]
    fn project_keeps_it() {
        let mut p = crate::model::Project::default();
        let m = crate::ebsd::example().unwrap();
        *p.research.slot("ebsd").unwrap() = Some(Item::ebsd(&m));
        let q = crate::model::Project::from_json(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(q.research, p.research);
        assert_eq!(p.lite().research.ebsd.unwrap().data.len(), 0);
        let mut o = Open::default();
        assert_eq!(o.ebsd(&q.research).unwrap().w, m.w);
        assert!(o.dic(&q.research).is_err());
    }
}
