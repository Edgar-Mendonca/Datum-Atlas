//! Schematic microstructures: a picture for every material, drawn from a recipe for its class (grain
//! size, phases, fibres, cells…), so a reader sees what the material is made of. They are schematics,
//! never micrographs: the caption says so, and nothing here claims to show a real sample. The same
//! material always gives the same picture (the random numbers are seeded by its id).
use crate::model::Material;
use serde::Serialize;
use std::fmt::Write;

const W: f64 = 300.0;
const H: f64 = 200.0;
type P = [f64; 2];

struct Rng(u64);
impl Rng {
    fn new(s: &str) -> Self { Rng(s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3)) | 1) }
    fn f(&mut self) -> f64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; (self.0 >> 11) as f64 / (1u64 << 53) as f64 }
    fn r(&mut self, a: f64, b: f64) -> f64 { a + (b - a) * self.f() }
    fn pt(&mut self) -> P { [self.r(0.0, W), self.r(0.0, H)] }
}

/// What a schematic shows. Sizes are counts in the 300 × 200 frame; fractions are 0–1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Kind {
    Grains { n: usize, twins: f64 },
    Pearlite { n: usize, frac: f64 },
    Martensite { n: usize },
    Duplex { n: usize, frac: f64 },
    Lamellar { n: usize },
    Precipitates { n: usize },
    Graphite { shape: u8 },
    Dendrites { frac: f64 },
    Ceramic { n: usize, pores: f64 },
    Glass,
    Fibres { frac: f64 },
    Plies { wood: bool },
    Layers,
    Cermet { n: usize },
    ShortFibres { frac: f64 },
    Particles { frac: f64, big: bool, light: bool },
    Foam { cells: usize, open: bool },
    Wood { hard: bool },
    Cork,
    Bamboo,
    Network,
    Bone,
    Spherulites { n: usize },
    Amorphous { particles: f64 },
    Stone { n: usize, kinds: u8 },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Micro { pub kind: Kind, pub caption: String, pub scale: String, pub svg: String }

/// The recipe for a material: its kind, what the picture shows, and the scale bar.
pub fn recipe(m: &Material) -> (Kind, &'static str, &'static str) {
    use Kind::*;
    let id = m.id.as_str();
    let path = m.family.join(" › ");
    let has = |s: &str| path.contains(s);
    let any = |l: &[&str]| l.iter().any(|x| id == *x || id.starts_with(&format!("{x}-")));
    match () {
        _ if any(&["steel-1020", "steel-1010", "steel-a36", "steel-s355", "steel-8620", "steel-dp600"]) => (Pearlite { n: 60, frac: 0.25 }, "ferrite (light) with some pearlite (dark, lamellar)", "50 µm"),
        _ if any(&["steel-1045", "steel-4130"]) => (Pearlite { n: 55, frac: 0.55 }, "ferrite and about half pearlite", "50 µm"),
        _ if any(&["steel-1080", "steel-1095"]) => (Pearlite { n: 40, frac: 0.97 }, "nearly all pearlite: lamellae of ferrite and cementite", "20 µm"),
        _ if any(&["ss-2205"]) => (Duplex { n: 70, frac: 0.5 }, "duplex: austenite islands in ferrite, about half each", "50 µm"),
        _ if any(&["ss-430", "ss-409"]) => (Grains { n: 45, twins: 0.0 }, "ferrite grains, no twins", "50 µm"),
        _ if any(&["ss-17-4ph", "ss-15-5ph", "steel-maraging", "ss-410", "ss-420", "ss-440c", "steel-4340", "steel-4140", "steel-52100", "steel-22mnb5", "steel-d2", "steel-h13", "steel-m2"]) => (Martensite { n: 18 }, "tempered martensite: fine needles and laths inside the former austenite grains", "20 µm"),
        _ if has("Stainless") || any(&["ni-200", "ni-400", "ni-600", "ni-625", "ni-c276", "co-r30188", "fe-ni36", "fe-ni-co", "cu-c11000", "cu-c10100", "cu-c26000", "cu-c51000", "cu-c70600", "cu-c71500", "ag", "au", "pt", "al-1100", "al-3003", "al-5083", "al-5052", "al-5754", "ti-gr2", "ti-gr4", "niti", "zr", "pb", "ta", "mo"]) =>
            (Grains { n: 50, twins: if has("Stainless") || has("Copper") || has("Nickel") || has("Precious") { 0.45 } else { 0.0 } }, "equiaxed grains of one phase; straight bands are annealing twins where the metal has them", "50 µm"),
        _ if any(&["ci-grey"]) => (Graphite { shape: 0 }, "graphite flakes (black) in a pearlite matrix", "100 µm"),
        _ if any(&["ci-cgi"]) => (Graphite { shape: 1 }, "compacted (worm-like) graphite in the matrix", "100 µm"),
        _ if any(&["ci-ductile", "ci-ductile-80", "ci-adi", "ci-malleable"]) => (Graphite { shape: 2 }, "graphite nodules (black) in a ferrite or pearlite matrix", "100 µm"),
        _ if any(&["ci-white"]) => (Duplex { n: 40, frac: 0.45 }, "white iron: a network of hard carbides (light) in pearlite", "50 µm"),
        _ if has("Cast") && (has("Aluminium") || has("Magnesium")) || any(&["zn-ag40a", "zn-za27", "cu-c93200", "sac305"]) => (Dendrites { frac: 0.3 }, "cast structure: dendrites (light) with eutectic between their arms (dark)", "100 µm"),
        _ if any(&["ti-6al-4v", "ti-6242", "ti-10-2-3", "ti-3al-2-5v", "cu-c36000", "cu-c46400", "cu-c61400"]) => (Lamellar { n: 20 }, "two phases in lamellar colonies (α and β)", "50 µm"),
        _ if has("Aluminium") || has("Nickel") || has("Magnesium") || any(&["cu-c17200", "cu-c18150", "w-heavy", "w", "be", "zn"]) => (Precipitates { n: 45 }, "grains strengthened by fine particles (precipitates) inside them", "50 µm"),
        _ if has("Metal matrix") => (Particles { frac: 0.2, big: false, light: false }, "hard particles (dark) spread in a metal matrix", "50 µm"),
        _ if any(&["wc-co"]) => (Cermet { n: 150 }, "hard carbide grains bound by a little metal (cobalt, dark)", "5 µm"),
        _ if has("Glasses") => (Glass, "glass: no grains, no crystals (amorphous); a few bubbles", "1 mm"),
        _ if has("Technical ceramics") => (Ceramic { n: 130, pores: if id.ends_with("-rb") { 0.05 } else { 0.015 } }, "fine sintered grains with pores at the grain corners", "5 µm"),
        _ if any(&["porcelain", "brick"]) => (Particles { frac: 0.35, big: false, light: false }, "crystals and pores in a glassy bond", "100 µm"),
        _ if any(&["concrete", "grc"]) => (Particles { frac: 0.6, big: true, light: false }, "aggregate (stones and sand) in cement paste", "10 mm"),
        _ if any(&["gypsum"]) => (Martensite { n: 6 }, "interlocking needle crystals of gypsum", "20 µm"),
        _ if any(&["granite"]) => (Stone { n: 70, kinds: 3 }, "interlocking crystals of quartz, feldspar and mica", "2 mm"),
        _ if any(&["marble", "limestone"]) => (Stone { n: 60, kinds: 1 }, "calcite crystals", "1 mm"),
        _ if any(&["sandstone"]) => (Particles { frac: 0.75, big: false, light: true }, "rounded sand grains, cement and pores", "1 mm"),
        _ if any(&["slate"]) => (Layers, "fine layers of clay minerals", "100 µm"),
        _ if any(&["graphite"]) => (Ceramic { n: 90, pores: 0.04 }, "fine graphite grains and pores", "20 µm"),
        _ if any(&["cfrp-qi", "gfrp-qi", "cfrp-fabric", "cc"]) => (Plies { wood: false }, "plies at different angles: fibres seen end-on and along their length", "200 µm"),
        _ if any(&["smc", "gfrp-csm", "wpc"]) => (ShortFibres { frac: 0.3 }, "short fibres (or wood particles) at random in the matrix", "1 mm"),
        _ if has("Polymer matrix") => (Fibres { frac: 0.6 }, "fibres seen end-on (about 60 %) in a polymer matrix", "20 µm"),
        _ if has("Foams") => (Foam { cells: if id == "al-foam" { 18 } else { 32 }, open: id == "pu-flex" }, if id == "pu-flex" { "open cells: struts (edges) only" } else { "closed cells: thin walls round gas" }, "1 mm"),
        _ if any(&["cork"]) => (Cork, "small closed, prism-shaped cells", "50 µm"),
        _ if any(&["bamboo"]) => (Bamboo, "fibre bundles (vascular bundles) in softer tissue", "1 mm"),
        _ if any(&["paper", "mdf", "osb"]) => (Network, "a felt of wood fibres (or strands) bonded together", "1 mm"),
        _ if any(&["plywood"]) => (Plies { wood: true }, "veneers glued with the grain crossing", "2 mm"),
        _ if any(&["bone"]) => (Bone, "osteons: rings of bone round blood channels", "200 µm"),
        _ if any(&["leather"]) => (Network, "a weave of collagen fibre bundles", "200 µm"),
        _ if has("Softwoods") => (Wood { hard: false }, "cross-section: rows of tracheids; dense latewood bands mark the growth rings", "500 µm"),
        _ if has("Hardwoods") => (Wood { hard: true }, "cross-section: fibres with large vessels (pores)", "500 µm"),
        _ if has("Filled") => (ShortFibres { frac: 0.3 }, "short glass or carbon fibres in the polymer", "200 µm"),
        _ if any(&["abs", "ps-hi", "asa", "pc-abs", "pp-copo", "tpv", "sebs"]) => (Amorphous { particles: 0.18 }, "rubber particles (dark) toughening a glassy polymer", "5 µm"),
        _ if any(&["pe-ld", "pe-hd", "pe-lld", "pe-uhmw", "pp", "pa6", "pa66", "pa12", "pa11", "pom", "pom-h", "pet", "pbt", "peek", "pps", "ptfe", "pvdf", "etfe", "fep", "pla", "pha", "lcp", "tpc"]) => (Spherulites { n: 16 }, "spherulites: crystals grown radially from centres, between amorphous regions", "50 µm"),
        _ if has("Elastomers") => (Amorphous { particles: 0.12 }, "cross-linked rubber with filler particles (carbon black or silica)", "1 µm"),
        _ => (Amorphous { particles: 0.0 }, "amorphous: tangled chains, no crystals or grains to see", "10 µm"),
    }
}

// ---------- geometry ----------

/// The cell of seed `i`: the frame clipped by the half-planes nearer `i` than every other seed.
fn cell(seeds: &[P], i: usize) -> Vec<P> {
    let mut poly = vec![[0.0, 0.0], [W, 0.0], [W, H], [0.0, H]];
    let s = seeds[i];
    for (j, o) in seeds.iter().enumerate() {
        if j == i || poly.is_empty() { continue }
        let n = [o[0] - s[0], o[1] - s[1]];
        let c = (n[0] * (o[0] + s[0]) + n[1] * (o[1] + s[1])) / 2.0;
        let side = |p: &P| n[0] * p[0] + n[1] * p[1] - c;
        let mut out = vec![];
        for k in 0..poly.len() {
            let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
            let (da, db) = (side(&a), side(&b));
            if da <= 0.0 { out.push(a) }
            if (da <= 0.0) != (db <= 0.0) { let t = da / (da - db); out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]) }
        }
        poly = out;
    }
    poly
}
fn centroid(p: &[P]) -> P {
    let (mut a, mut x, mut y) = (0.0, 0.0, 0.0);
    for k in 0..p.len() {
        let (u, v) = (p[k], p[(k + 1) % p.len()]);
        let c = u[0] * v[1] - v[0] * u[1];
        a += c; x += (u[0] + v[0]) * c; y += (u[1] + v[1]) * c;
    }
    if a.abs() < 1e-9 { return p.first().copied().unwrap_or([0.0, 0.0]) }
    [x / (3.0 * a), y / (3.0 * a)]
}
/// Grains: random seeds evened out by two Lloyd steps (so grain sizes look natural).
fn grains(r: &mut Rng, n: usize) -> Vec<Vec<P>> {
    let mut s: Vec<P> = (0..n).map(|_| r.pt()).collect();
    for _ in 0..2 { s = (0..n).map(|i| centroid(&cell(&s, i))).collect(); }
    (0..n).map(|i| cell(&s, i)).collect()
}
fn pts(p: &[P]) -> String { p.iter().map(|q| format!("{:.1},{:.1}", q[0], q[1])).collect::<Vec<_>>().join(" ") }
fn grey(v: f64) -> String { let g = v.clamp(0.0, 255.0) as u8; format!("#{g:02x}{g:02x}{:02x}", g.saturating_add(4)) }

/// Parallel lines at angle `a`, `d` apart, over the polygon `p` (or the frame), clipped later.
fn hatch(o: &mut String, p: Option<&[P]>, a: f64, d: f64, w: f64, col: &str) {
    let (c, s) = (a.cos(), a.sin());
    let (ce, r) = match p {
        Some(p) => { let ce = centroid(p); (ce, p.iter().map(|q| ((q[0] - ce[0]).powi(2) + (q[1] - ce[1]).powi(2)).sqrt()).fold(0.0, f64::max) + 1.0) }
        None => ([W / 2.0, H / 2.0], (W * W + H * H).sqrt() / 2.0),
    };
    let mut t = -r;
    while t < r {
        let (mx, my) = (ce[0] + t * -s, ce[1] + t * c);
        let _ = write!(o, r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{col}" stroke-width="{w}"/>"#, mx - r * c, my - r * s, mx + r * c, my + r * s);
        t += d;
    }
}

/// The SVG of a material's schematic microstructure, with its scale bar and a "Schematic" mark.
pub fn draw(m: &Material) -> Micro {
    let (kind, what, scale) = recipe(m);
    let mut r = Rng::new(&m.id);
    let uid: String = m.id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let mut o = String::new();
    let mut defs = String::new();
    let edge = |o: &mut String, cells: &[Vec<P>], col: &str, w: f64| for c in cells { let _ = write!(o, r##"<polygon points="{}" fill="none" stroke="{col}" stroke-width="{w}" stroke-linejoin="round"/>"##, pts(c)); };
    let clip = |defs: &mut String, k: usize, c: &[P]| { let _ = write!(defs, r#"<clipPath id="m{uid}{k}"><polygon points="{}"/></clipPath>"#, pts(c)); format!("url(#m{uid}{k})") };
    let bg = |o: &mut String, col: &str| { let _ = write!(o, r#"<rect width="{W}" height="{H}" fill="{col}"/>"#); };
    match kind {
        Kind::Grains { n, .. } | Kind::Precipitates { n } | Kind::Pearlite { n, .. } | Kind::Martensite { n } | Kind::Duplex { n, .. } | Kind::Lamellar { n } | Kind::Ceramic { n, .. } | Kind::Stone { n, .. } | Kind::Cermet { n } => {
            let cells = grains(&mut r, n);
            for (k, c) in cells.iter().enumerate() {
                let tone = r.r(185.0, 240.0);
                let fill = match kind {
                    Kind::Stone { kinds: 3, .. } => ["#d9d4cc", "#e9c9b5", "#3d3a37"][(r.f() * 2.6) as usize].to_string(),
                    Kind::Duplex { frac, .. } => if r.f() < frac { grey(232.0) } else { grey(165.0) },
                    _ => grey(tone),
                };
                let _ = write!(o, r#"<polygon points="{}" fill="{fill}"/>"#, pts(c));
                let cp = clip(&mut defs, k, c);
                let a = r.r(0.0, std::f64::consts::PI);
                match kind {
                    Kind::Grains { twins, .. } if r.f() < twins => { let _ = write!(o, r#"<g clip-path="{cp}">"#); let x = r.r(-20.0, 20.0); let (c0, s0) = (a.cos(), a.sin()); let ce = centroid(c); for t in [x, x + r.r(5.0, 9.0)] { let _ = write!(o, r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#, ce[0] - 80.0 * c0 - t * s0, ce[1] - 80.0 * s0 + t * c0, ce[0] + 80.0 * c0 - t * s0, ce[1] + 80.0 * s0 + t * c0, grey(tone - 55.0)); } o.push_str("</g>"); }
                    Kind::Pearlite { frac, .. } if r.f() < frac => { let _ = write!(o, r##"<g clip-path="{cp}"><polygon points="{}" fill="#9a958e"/>"##, pts(c)); hatch(&mut o, Some(c), a, 2.2, 1.1, "#4a4642"); o.push_str("</g>"); }
                    Kind::Lamellar { .. } => { let _ = write!(o, r#"<g clip-path="{cp}">"#); hatch(&mut o, Some(c), a, 4.5, 1.6, "#6d6878"); o.push_str("</g>"); }
                    Kind::Martensite { .. } => { let _ = write!(o, r#"<g clip-path="{cp}">"#); let ce = centroid(c); for _ in 0..30 { let b = if r.f() < 0.5 { a } else { a + 1.05 }; let (x, y) = (ce[0] + r.r(-30.0, 30.0), ce[1] + r.r(-30.0, 30.0)); let l = r.r(4.0, 14.0); let _ = write!(o, r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}" stroke-linecap="round"/>"#, x - l * b.cos(), y - l * b.sin(), x + l * b.cos(), y + l * b.sin(), grey(tone - 80.0), r.r(0.8, 2.0)); } o.push_str("</g>"); }
                    Kind::Precipitates { .. } => { let ce = centroid(c); for _ in 0..14 { let _ = write!(o, r#"<circle cx="{:.1}" cy="{:.1}" r="0.7" fill="{}"/>"#, ce[0] + r.r(-10.0, 10.0), ce[1] + r.r(-8.0, 8.0), grey(tone - 90.0)); } }
                    _ => {}
                }
            }
            edge(&mut o, &cells, match kind { Kind::Ceramic { .. } => "#6a4a4a", Kind::Stone { .. } => "#4a4a4a", Kind::Cermet { .. } => "#3a3533", _ => "#2e2e2e" }, if matches!(kind, Kind::Cermet { .. }) { 2.2 } else { 0.9 });
            if let Kind::Ceramic { pores, .. } = kind { for c in &cells { if r.f() < pores * 25.0 { if let Some(p) = c.get(r.r(0.0, c.len() as f64) as usize) { let _ = write!(o, r##"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="#1a1a1a"/>"##, p[0], p[1], r.r(0.8, 2.2)); } } } }
        }
        Kind::Graphite { shape } => {
            bg(&mut o, "#b8b3aa");
            hatch(&mut o, None, 0.6, 3.0, 0.6, "#9d978d");
            for _ in 0..if shape == 2 { 45 } else { 70 } {
                let p = r.pt();
                match shape {
                    2 => { let _ = write!(o, r##"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="#e6e2da"/><circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="#1c1c1c"/>"##, p[0], p[1], 10.0, p[0], p[1], r.r(3.0, 5.5)); }
                    _ => { let (a, l) = (r.r(0.0, 6.3), if shape == 0 { r.r(8.0, 22.0) } else { r.r(4.0, 9.0) }); let m = [p[0] + l * a.cos(), p[1] + l * a.sin()]; let q = [m[0] + r.r(-6.0, 6.0), m[1] + r.r(-6.0, 6.0)]; let _ = write!(o, r##"<path d="M{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1}" stroke="#1c1c1c" stroke-width="{:.1}" fill="none" stroke-linecap="round"/>"##, p[0], p[1], m[0], m[1], q[0], q[1], if shape == 0 { 1.6 } else { 3.2 }); }
                }
            }
        }
        Kind::Dendrites { frac } => {
            bg(&mut o, "#5d5a57");
            for _ in 0..9 {
                let (p, a) = (r.pt(), r.r(0.0, 6.3));
                for arm in 0..4 {
                    let b = a + arm as f64 * std::f64::consts::FRAC_PI_2;
                    let l = r.r(30.0, 55.0);
                    let e = [p[0] + l * b.cos(), p[1] + l * b.sin()];
                    let _ = write!(o, r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#dcd8d0" stroke-width="{:.1}" stroke-linecap="round"/>"##, p[0], p[1], e[0], e[1], 7.0 * (1.0 - frac));
                    let mut t = 6.0;
                    while t < l { let q = [p[0] + t * b.cos(), p[1] + t * b.sin()]; let s = (l - t) * 0.35; for side in [1.0, -1.0] { let c = b + side * std::f64::consts::FRAC_PI_2; let _ = write!(o, r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#d2cec6" stroke-width="3.6" stroke-linecap="round"/>"##, q[0], q[1], q[0] + s * c.cos(), q[1] + s * c.sin()); } t += 7.0; }
                }
            }
        }
        Kind::Glass => {
            bg(&mut o, "#e6eef0");
            for _ in 0..7 { let p = r.pt(); let _ = write!(o, r##"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="#f8fbfb" stroke="#9fb3b8" stroke-width="0.8"/>"##, p[0], p[1], r.r(1.5, 5.0)); }
        }
        Kind::Fibres { frac } => {
            bg(&mut o, "#4a4f57");
            let mut f: Vec<P> = vec![];
            let target = (frac * W * H / (std::f64::consts::PI * 3.3 * 3.3)) as usize;
            for _ in 0..12000 { if f.len() >= target { break } let p = r.pt(); if f.iter().all(|q| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) > 49.0) { f.push(p) } }
            for p in f { let _ = write!(o, r##"<circle cx="{:.1}" cy="{:.1}" r="3.3" fill="#dfe1e4" stroke="#a9acb1" stroke-width="0.6"/>"##, p[0], p[1]); }
        }
        Kind::Layers => {
            let mut y = 0.0;
            while y < H { let h = r.r(2.0, 7.0); let _ = write!(o, r##"<path d="M0,{y:.1} Q{:.1},{:.1} {W},{:.1} L{W},{:.1} Q{:.1},{:.1} 0,{:.1} Z" fill="{}"/>"##, W / 2.0, y + r.r(-3.0, 3.0), y + r.r(-1.0, 1.0), y + h, W / 2.0, y + h + r.r(-3.0, 3.0), y + h, grey(r.r(70.0, 125.0))); y += h; }
        }
        Kind::Plies { wood } => {
            let n = 6;
            for k in 0..n {
                let (y0, h) = (k as f64 * H / n as f64, H / n as f64);
                let end_on = k % 2 == 0;
                let _ = write!(o, r##"<rect y="{y0:.1}" width="{W}" height="{h:.1}" fill="{}"/>"##, match (wood, end_on) { (true, true) => "#f2e7d2", (true, false) => "#d9b98c", (false, true) => "#4a4f57", _ => "#8a8f97" });
                let _ = write!(o, r#"<g clip-path="url(#m{uid}p{k})">"#);
                let _ = write!(defs, r#"<clipPath id="m{uid}p{k}"><rect y="{y0:.1}" width="{W}" height="{h:.1}"/></clipPath>"#);
                if wood && end_on { let mut x = 0.0; while x < W { let mut y = y0; while y < y0 + h { let _ = write!(o, r##"<rect x="{x:.1}" y="{y:.1}" width="5" height="5" fill="none" stroke="#8a6a43" stroke-width="0.8"/>"##); y += 5.0; } x += 5.0; } }
                else if wood { let mut y = y0 + 1.0; while y < y0 + h { let _ = write!(o, r##"<path d="M0,{y:.1} Q{:.1},{:.1} {W},{:.1}" stroke="#a37a4c" stroke-width="0.9" fill="none"/>"##, W / 2.0, y + r.r(-1.5, 1.5), y + r.r(-0.8, 0.8)); y += 2.6; } }
                else if end_on { let mut x = 2.0; while x < W { let mut y = y0 + 2.0; while y < y0 + h { let _ = write!(o, r##"<circle cx="{:.1}" cy="{:.1}" r="1.9" fill="#dfe1e4"/>"##, x + r.r(-0.6, 0.6), y + r.r(-0.6, 0.6)); y += 4.6; } x += 4.6; } }
                else { let mut y = y0 + 1.5; while y < y0 + h { let _ = write!(o, r##"<line x1="0" y1="{y:.1}" x2="{W}" y2="{y:.1}" stroke="#d6d8dc" stroke-width="2"/>"##); y += 4.0; } }
                o.push_str("</g>");
            }
        }
        Kind::ShortFibres { frac } | Kind::Particles { frac, .. } => {
            let big = matches!(kind, Kind::Particles { big: true, .. });
            let light = matches!(kind, Kind::Particles { light: true, .. });
            bg(&mut o, if matches!(kind, Kind::ShortFibres { .. }) { "#6d7380" } else if big { "#a7a39c" } else if light { "#5a5550" } else { "#d4cfc6" });
            let count = if big { 55 } else if matches!(kind, Kind::ShortFibres { .. }) { (frac * 260.0) as usize } else { (frac * 420.0) as usize };
            for _ in 0..count {
                let p = r.pt();
                if let Kind::ShortFibres { .. } = kind { let (a, l) = (r.r(0.0, 6.3), r.r(10.0, 28.0)); let _ = write!(o, r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#e1e3e6" stroke-width="2.2" stroke-linecap="round"/>"##, p[0], p[1], p[0] + l * a.cos(), p[1] + l * a.sin()); }
                else { let s = if big { r.r(8.0, 22.0) } else { r.r(3.5, 7.5) }; let pts_: Vec<P> = (0..6).map(|k| { let a = k as f64 * 1.047 + r.r(-0.3, 0.3); let d = s * r.r(0.7, 1.1); [p[0] + d * a.cos(), p[1] + d * a.sin()] }).collect(); let _ = write!(o, r##"<polygon points="{}" fill="{}" stroke="#2e2e2e" stroke-width="0.6"/>"##, pts(&pts_), if big { grey(r.r(110.0, 175.0)) } else if light { grey(r.r(200.0, 235.0)) } else { grey(r.r(70.0, 120.0)) }); }
            }
        }
        Kind::Foam { cells, open } => {
            let c = grains(&mut r, cells);
            bg(&mut o, if open { "#2b2b2b" } else { "#f6f5f0" });
            if open { for p in &c { let _ = write!(o, r##"<polygon points="{}" fill="#2b2b2b" stroke="#e7d9b8" stroke-width="2.4" stroke-linejoin="round"/>"##, pts(p)); } }
            else { edge(&mut o, &c, "#6b5f52", 2.4); }
        }
        Kind::Wood { hard } => {
            bg(&mut o, "#f2e7d2");
            let mut x = 0.0;
            while x < W {
                let late = (x % 100.0) > 74.0;
                let w = if late { 4.5 } else { 9.0 };
                let mut y = r.r(-4.0, 0.0);
                while y < H { let _ = write!(o, r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{w:.1}" fill="none" stroke="#8a6a43" stroke-width="{}"/>"##, x, y, if late { 4.0 } else { 8.5 }, if late { 2.0 } else { 1.0 }); y += w; }
                x += if late { 4.5 } else { 9.0 };
            }
            if hard { for _ in 0..16 { let p = r.pt(); let _ = write!(o, r##"<ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="#fbf6ea" stroke="#6e5434" stroke-width="1.6"/>"##, p[0], p[1], r.r(6.0, 10.0), r.r(7.0, 12.0)); } }
        }
        Kind::Cork => {
            bg(&mut o, "#e9d3b0");
            let (s, h) = (8.0, 8.0 * 0.866);
            let mut row = 0;
            let mut y = 0.0;
            while y < H + h { let mut x = if row % 2 == 0 { 0.0 } else { s * 0.75 }; while x < W + s { let hex: Vec<P> = (0..6).map(|k| { let a = k as f64 * std::f64::consts::FRAC_PI_3; [x + s / 2.0 * a.cos(), y + s / 2.0 * a.sin()] }).collect(); let _ = write!(o, r##"<polygon points="{}" fill="none" stroke="#8b5e34" stroke-width="0.9"/>"##, pts(&hex)); x += s * 1.5; } y += h / 2.0; row += 1; }
        }
        Kind::Bamboo => {
            bg(&mut o, "#efe3c4");
            for _ in 0..34 { let p = r.pt(); let _ = write!(o, r##"<ellipse cx="{:.1}" cy="{:.1}" rx="7" ry="9" fill="#8d6c3f"/><circle cx="{:.1}" cy="{:.1}" r="2.2" fill="#efe3c4"/><circle cx="{:.1}" cy="{:.1}" r="2.2" fill="#efe3c4"/>"##, p[0], p[1], p[0] - 2.5, p[1] - 2.0, p[0] + 2.5, p[1] - 2.0); }
        }
        Kind::Network => {
            bg(&mut o, "#d8cdb8");
            for _ in 0..140 { let (p, a, l) = (r.pt(), r.r(0.0, 6.3), r.r(30.0, 90.0)); let q = [p[0] + l * a.cos(), p[1] + l * a.sin()]; let m = [(p[0] + q[0]) / 2.0 + r.r(-10.0, 10.0), (p[1] + q[1]) / 2.0 + r.r(-10.0, 10.0)]; let _ = write!(o, r##"<path d="M{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1}" stroke="{}" stroke-width="{:.1}" fill="none"/>"##, p[0], p[1], m[0], m[1], q[0], q[1], ["#8a6a43", "#a98a5f", "#6e5434"][(r.f() * 2.9) as usize], r.r(1.2, 3.0)); }
        }
        Kind::Bone => {
            bg(&mut o, "#e8e2d4");
            for _ in 0..9 { let p = r.pt(); let rr = r.r(18.0, 26.0); let mut k = rr; while k > 3.0 { let _ = write!(o, r##"<circle cx="{:.1}" cy="{:.1}" r="{k:.1}" fill="none" stroke="#b8ab92" stroke-width="1.1"/>"##, p[0], p[1]); k -= 3.0; } let _ = write!(o, r##"<circle cx="{:.1}" cy="{:.1}" r="3" fill="#5b4a3a"/>"##, p[0], p[1]); }
        }
        Kind::Spherulites { n } => {
            let mut s: Vec<P> = (0..n).map(|_| r.pt()).collect();
            s = (0..n).map(|i| centroid(&cell(&s, i))).collect();
            let cells: Vec<Vec<P>> = (0..n).map(|i| cell(&s, i)).collect();
            bg(&mut o, "#d7d9e3");
            for (k, c) in cells.iter().enumerate() {
                let cp = clip(&mut defs, k, c);
                let _ = write!(o, r#"<g clip-path="{cp}">"#);
                let mut a = 0.0;
                while a < std::f64::consts::TAU { let _ = write!(o, r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#aeb2c3" stroke-width="1.1"/>"##, s[k][0], s[k][1], s[k][0] + 200.0 * a.cos(), s[k][1] + 200.0 * a.sin()); a += 0.12; }
                o.push_str("</g>");
            }
            edge(&mut o, &cells, "#7a5560", 0.9);
        }
        Kind::Amorphous { particles } => {
            bg(&mut o, "#e3e6ee");
            for _ in 0..90 { let (p, a) = (r.pt(), r.r(0.0, 6.3)); let q = [p[0] + 30.0 * a.cos(), p[1] + 30.0 * a.sin()]; let _ = write!(o, r##"<path d="M{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1}" stroke="#c6cbd8" stroke-width="0.8" fill="none"/>"##, p[0], p[1], p[0] + r.r(-25.0, 25.0), p[1] + r.r(-25.0, 25.0), q[0], q[1]); }
            for _ in 0..(particles * 400.0) as usize { let p = r.pt(); let _ = write!(o, r##"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="#3b3f4a"/>"##, p[0], p[1], r.r(1.2, 3.2)); }
        }
    }
    // Scale bar and the schematic mark.
    let _ = write!(o, r##"<rect x="{}" y="{}" width="76" height="16" fill="#ffffff" opacity="0.9"/><rect x="{}" y="{}" width="36" height="3" fill="#0C1A2B"/><text x="{}" y="{}" font-size="9" font-family="IBM Plex Sans, Arial, sans-serif" fill="#0C1A2B">{scale}</text>"##, W - 82.0, H - 21.0, W - 78.0, H - 14.0, W - 38.0, H - 9.5);
    let _ = write!(o, r##"<rect x="6" y="6" width="58" height="14" fill="#ffffff" opacity="0.9"/><text x="10" y="16" font-size="9" font-family="IBM Plex Sans, Arial, sans-serif" fill="#5B6878">SCHEMATIC</text>"##);
    let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}"><defs>{defs}</defs><g>{o}</g></svg>"#);
    Micro { kind, caption: format!("Schematic, not a micrograph: {what}."), scale: scale.into(), svg }
}
