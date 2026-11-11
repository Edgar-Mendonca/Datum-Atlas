//! Crystal calculators: a unit cell (any of the seven systems, or a common structure such as BCC,
//! FCC, HCP, diamond or rock salt), interplanar spacing, angles between planes and directions,
//! Miller–Bravais indices, theoretical density, X-ray powder peak positions (Bragg's law) with
//! approximate intensities, and the stereographic projection of plane normals.
//! Written from the textbook formulas (metric tensor, structure factor, Lorentz–polarisation factor).
use crate::elements::{self, Element};
use serde::{Deserialize, Serialize};

type V3 = [f64; 3];
const NA: f64 = 6.022_140_76e23;

/// Lattice parameters: lengths in Å, angles in degrees.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cell { pub a: f64, pub b: f64, pub c: f64, pub alpha: f64, pub beta: f64, pub gamma: f64 }

/// A crystal structure: its cell and the atoms of the cell (species symbol, fractional position).
#[derive(Debug, Clone, Serialize)]
pub struct Structure { pub kind: String, pub name: String, pub system: String, pub cell: Cell, pub atoms: Vec<(String, V3)> }

/// The structures it knows: [id, name, number of species, what it is].
pub const KINDS: &[(&str, &str, usize, &str)] = &[
    ("sc", "Simple cubic", 1, "One atom at each corner (polonium)"),
    ("bcc", "Body-centred cubic (BCC)", 1, "Iron, chromium, tungsten, molybdenum, β-titanium"),
    ("fcc", "Face-centred cubic (FCC)", 1, "Aluminium, copper, nickel, austenite, gold, silver"),
    ("hcp", "Hexagonal close-packed (HCP)", 1, "Magnesium, titanium, zinc, cobalt, zirconium"),
    ("diamond", "Diamond cubic", 1, "Silicon, germanium, diamond"),
    ("rocksalt", "Rock salt (NaCl)", 2, "NaCl, MgO, NiO, TiC, TiN"),
    ("cscl", "Caesium chloride (B2)", 2, "CsCl, NiAl, FeAl, β-brass"),
    ("zincblende", "Zinc blende", 2, "ZnS, GaAs, cubic SiC, cubic BN"),
    ("fluorite", "Fluorite (CaF₂)", 2, "CaF₂, UO₂, cubic zirconia (ZrO₂)"),
    ("cell", "Any cell (one atom)", 1, "Your own a, b, c, α, β, γ: tetragonal, orthorhombic, monoclinic, triclinic…"),
];

fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn add(a: V3, b: V3) -> V3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn mul(a: V3, k: f64) -> V3 { [a[0] * k, a[1] * k, a[2] * k] }
fn dot(a: V3, b: V3) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn norm(a: V3) -> f64 { dot(a, a).sqrt() }
fn ang(a: V3, b: V3) -> f64 { (dot(a, b) / (norm(a) * norm(b))).clamp(-1.0, 1.0).acos().to_degrees() }
fn gcd(a: i32, b: i32) -> i32 { if b == 0 { a.abs() } else { gcd(b, a % b) } }

impl Cell {
    pub fn cubic(a: f64) -> Cell { Cell { a, b: a, c: a, alpha: 90.0, beta: 90.0, gamma: 90.0 } }
    pub fn hex(a: f64, c: f64) -> Cell { Cell { a, b: a, c, alpha: 90.0, beta: 90.0, gamma: 120.0 } }
    /// The cell edges as Cartesian vectors (a along x, b in the xy plane), Å.
    pub fn basis(&self) -> [V3; 3] {
        let (al, be, ga) = (self.alpha.to_radians(), self.beta.to_radians(), self.gamma.to_radians());
        let cy = (al.cos() - be.cos() * ga.cos()) / ga.sin();
        let cz = (1.0 - be.cos().powi(2) - cy * cy).max(0.0).sqrt();
        [[self.a, 0.0, 0.0], [self.b * ga.cos(), self.b * ga.sin(), 0.0], [self.c * be.cos(), self.c * cy, self.c * cz]]
    }
    pub fn volume(&self) -> f64 { let [a, b, c] = self.basis(); dot(a, cross(b, c)) }
    fn recip(&self) -> [V3; 3] { let [a, b, c] = self.basis(); let v = self.volume(); [mul(cross(b, c), 1.0 / v), mul(cross(c, a), 1.0 / v), mul(cross(a, b), 1.0 / v)] }
    /// The normal of plane (hkl) in reciprocal space, 1/Å (its length is 1/d).
    pub fn g(&self, h: [i32; 3]) -> V3 { let [x, y, z] = self.recip(); add(add(mul(x, h[0] as f64), mul(y, h[1] as f64)), mul(z, h[2] as f64)) }
    /// Direction [uvw] as a Cartesian vector, Å.
    pub fn r(&self, u: [f64; 3]) -> V3 { let [x, y, z] = self.basis(); add(add(mul(x, u[0]), mul(y, u[1])), mul(z, u[2])) }
    /// Interplanar spacing of (hkl), Å.
    pub fn d(&self, h: [i32; 3]) -> f64 { 1.0 / norm(self.g(h)) }
    pub fn system(&self) -> &'static str {
        let e = |x: f64, y: f64| (x - y).abs() < 1e-6;
        let r = |x: f64| e(x, 90.0);
        match (e(self.a, self.b), e(self.b, self.c), r(self.alpha), r(self.beta), r(self.gamma)) {
            (true, true, true, true, true) => "cubic",
            (true, false, true, true, true) => "tetragonal",
            (true, _, true, true, false) if e(self.gamma, 120.0) => "hexagonal",
            (true, true, false, false, false) if e(self.alpha, self.beta) && e(self.beta, self.gamma) => "rhombohedral",
            (_, _, true, true, true) => "orthorhombic",
            (_, _, true, false, true) => "monoclinic",
            _ => "triclinic",
        }
    }
    fn check(&self) -> Result<(), String> {
        for (n, v) in [("a", self.a), ("b", self.b), ("c", self.c)] { if !(v.is_finite() && v > 0.0 && v < 1000.0) { return Err(format!("{n} must be a length in Å, more than 0.")) } }
        for (n, v) in [("α", self.alpha), ("β", self.beta), ("γ", self.gamma)] { if !(v.is_finite() && v > 0.0 && v < 180.0) { return Err(format!("{n} must be an angle between 0° and 180°.")) } }
        let v = self.volume();
        if !(v.is_finite() && v > 1e-6 * self.a * self.b * self.c) { return Err("These angles do not make a cell (α, β and γ do not fit together).".into()) }
        Ok(())
    }
}

/// Plane (hkl) in four-index Miller–Bravais form (hkil), i = −(h + k).
pub fn plane4(h: [i32; 3]) -> [i32; 4] { [h[0], h[1], -(h[0] + h[1]), h[2]] }
/// Direction [uvw] in four-index form [UVTW], reduced to the smallest whole numbers.
pub fn dir4(u: [i32; 3]) -> [i32; 4] {
    let v = [2 * u[0] - u[1], 2 * u[1] - u[0], -(u[0] + u[1]), 3 * u[2]];
    let g = v.iter().fold(0, |a, &b| gcd(a, b)).max(1);
    v.map(|x| x / g)
}

fn fcc() -> Vec<V3> { vec![[0.0, 0.0, 0.0], [0.5, 0.5, 0.0], [0.5, 0.0, 0.5], [0.0, 0.5, 0.5]] }
fn shift(v: &[V3], s: V3) -> Vec<V3> { v.iter().map(|p| add(*p, s).map(|x| x.rem_euclid(1.0))).collect() }

/// Build a structure: `kind` from KINDS, lattice `cell` (only a, or a and c for HCP, are used for
/// the named structures), and the species (one or two element symbols).
pub fn build(kind: &str, cell: Cell, species: &[String]) -> Result<Structure, String> {
    let (_, name, n, _) = KINDS.iter().find(|k| k.0 == kind).ok_or(format!("No structure {kind}."))?;
    let sp: Vec<&Element> = species.iter().take(*n).map(|s| elements::find(s).ok_or(format!("{s} is not an element."))).collect::<Result<_, _>>()?;
    if sp.len() < *n { return Err(format!("{name} needs {n} elements.")) }
    let (a, b) = (sp[0].symbol.clone(), sp.get(1).map(|e| e.symbol.clone()).unwrap_or_default());
    let one = |v: Vec<V3>, s: &str| v.into_iter().map(|p| (s.to_string(), p)).collect::<Vec<_>>();
    let cub = Cell::cubic(cell.a);
    let (cell, atoms) = match kind {
        "sc" => (cub, one(vec![[0.0; 3]], &a)),
        "bcc" => (cub, one(vec![[0.0; 3], [0.5; 3]], &a)),
        "fcc" => (cub, one(fcc(), &a)),
        "hcp" => (Cell::hex(cell.a, cell.c), one(vec![[1.0 / 3.0, 2.0 / 3.0, 0.25], [2.0 / 3.0, 1.0 / 3.0, 0.75]], &a)),
        "diamond" => (cub, one([fcc(), shift(&fcc(), [0.25; 3])].concat(), &a)),
        "rocksalt" => (cub, [one(fcc(), &a), one(shift(&fcc(), [0.5, 0.0, 0.0]), &b)].concat()),
        "cscl" => (cub, [one(vec![[0.0; 3]], &a), one(vec![[0.5; 3]], &b)].concat()),
        "zincblende" => (cub, [one(fcc(), &a), one(shift(&fcc(), [0.25; 3]), &b)].concat()),
        "fluorite" => (cub, [one(fcc(), &a), one(shift(&fcc(), [0.25; 3]), &b), one(shift(&fcc(), [0.75; 3]), &b)].concat()),
        _ => (cell, one(vec![[0.0; 3]], &a)),
    };
    cell.check()?;
    Ok(Structure { kind: kind.into(), name: name.to_string(), system: cell.system().into(), cell, atoms })
}

/// An element's own structure from its data (BCC, FCC, HCP, diamond cubic, simple cubic), when
/// its lattice parameters are known.
pub fn of_element(q: &str) -> Option<Structure> {
    let e = elements::find(q)?;
    let a = e.a?;
    let kind = match e.structure.as_deref()? { "BCC" => "bcc", "FCC" => "fcc", "HCP" => "hcp", "diamond cubic" => "diamond", "simple cubic" => "sc", _ => return None };
    if kind == "hcp" && e.c.is_none() { return None }
    build(kind, Cell::hex(a, e.c.unwrap_or(a)), std::slice::from_ref(&e.symbol)).ok()
}

impl Structure {
    /// Theoretical density from the cell and its atoms, kg/m³.
    pub fn density(&self) -> f64 {
        let m: f64 = self.atoms.iter().filter_map(|(s, _)| elements::find(s)).map(|e| e.mass).sum();
        m / NA / (self.cell.volume() * 1e-30) / 1000.0
    }
    /// Structure factor squared |F|² of (hkl), with each atom's scattering factor taken as its
    /// atomic number (no fall-off with angle): right for which reflections vanish, rough for strength.
    pub fn f2(&self, h: [i32; 3]) -> f64 {
        let (mut re, mut im) = (0.0, 0.0);
        for (s, p) in &self.atoms {
            let f = elements::find(s).map_or(1.0, |e| e.z as f64);
            let ph = std::f64::consts::TAU * (h[0] as f64 * p[0] + h[1] as f64 * p[1] + h[2] as f64 * p[2]);
            re += f * ph.cos();
            im += f * ph.sin();
        }
        re * re + im * im
    }
    /// The atoms to draw: every atom of the cell with its copies on the faces, edges and corners.
    pub fn drawn(&self) -> Vec<(String, V3)> {
        let mut out: Vec<(String, V3)> = vec![];
        for (s, p) in &self.atoms {
            for k in 0..8 {
                let q = [p[0] + (k & 1) as f64, p[1] + (k >> 1 & 1) as f64, p[2] + (k >> 2 & 1) as f64];
                if q.iter().all(|&x| x <= 1.0 + 1e-9) && !out.iter().any(|(_, r)| norm(sub(*r, q)) < 1e-6) { out.push((s.clone(), q)) }
            }
        }
        out
    }
}

/// One line of a powder pattern.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peak { pub hkl: [i32; 3], pub hkil: Option<[i32; 4]>, pub mult: usize, pub d: f64, pub two_theta: f64, pub intensity: f64 }

/// Powder X-ray peaks up to `max` degrees 2θ for wavelength `lambda` (Å): Bragg's law for the
/// positions; intensity ∝ multiplicity × |F|² × Lorentz–polarisation, scaled to 100 for the strongest.
pub fn peaks(s: &Structure, lambda: f64, max: f64) -> Result<Vec<Peak>, String> {
    if !(lambda.is_finite() && lambda > 0.05 && lambda < 10.0) { return Err("The wavelength must be between 0.05 and 10 Å.".into()) }
    if !max.is_finite() { return Err("Give the largest angle 2θ in degrees.".into()) }
    let max = max.clamp(5.0, 175.0);
    let dmin = lambda / 2.0 / (max.to_radians() / 2.0).sin();
    let n = |x: f64| (x / dmin).ceil().min(1e4) as i64 + 1;
    let (nh, nk, nl) = (n(s.cell.a), n(s.cell.b), n(s.cell.c));
    if (2 * nh + 1) * (2 * nk + 1) * (2 * nl + 1) > 400_000 { return Err("Too many reflections: shorten the range of 2θ or the cell.".into()) }
    let (nh, nk, nl) = (nh as i32, nk as i32, nl as i32);
    let mut all: Vec<([i32; 3], f64, f64)> = vec![];
    let fmax = s.atoms.iter().filter_map(|(x, _)| elements::find(x)).map(|e| e.z as f64).sum::<f64>().powi(2);
    for h in -nh..=nh { for k in -nk..=nk { for l in -nl..=nl {
        if h == 0 && k == 0 && l == 0 { continue }
        let d = s.cell.d([h, k, l]);
        if d < dmin { continue }
        let f2 = s.f2([h, k, l]);
        if f2 > 1e-6 * fmax { all.push(([h, k, l], d, f2)) }
    }}}
    // Families: the same spacing and the same |F|² (symmetry-equivalent, or lines that overlap anyway).
    all.sort_by(|a, b| b.1.total_cmp(&a.1).then(b.2.total_cmp(&a.2)));
    let mut out: Vec<Peak> = vec![];
    let mut fams: Vec<(f64, f64)> = vec![];
    for (h, d, f2) in all {
        match fams.iter().position(|(dd, ff)| (dd - d).abs() < 1e-6 * d && (ff - f2).abs() < 1e-6 * f2.max(1.0)) {
            Some(i) => { out[i].mult += 1; if better(h, out[i].hkl) { out[i].hkl = h } }
            None => {
                let th = (lambda / 2.0 / d).asin();
                fams.push((d, f2));
                out.push(Peak { hkl: h, hkil: None, mult: 1, d, two_theta: 2.0 * th.to_degrees(), intensity: f2 * (1.0 + (2.0 * th).cos().powi(2)) / (th.sin().powi(2) * th.cos()) });
            }
        }
    }
    let top = out.iter().map(|p| p.intensity * p.mult as f64).fold(0.0, f64::max).max(f64::MIN_POSITIVE);
    for p in &mut out {
        p.intensity = p.intensity * p.mult as f64 / top * 100.0;
        if s.system == "hexagonal" { p.hkil = Some(plane4(p.hkl)) }
    }
    Ok(out)
}
/// The label for a family: all indices ≥ 0 and in falling order if one is, else the largest.
fn better(a: [i32; 3], b: [i32; 3]) -> bool {
    let key = |h: [i32; 3]| (h.iter().all(|&x| x >= 0), h[0] >= h[1] && h[1] >= h[2], h);
    key(a) > key(b)
}

/// A pole on a stereographic projection: the normal of plane (hkl), at (x, y) in the unit circle.
#[derive(Debug, Clone, Serialize)]
pub struct Pole { pub hkl: [i32; 3], pub x: f64, pub y: f64 }

/// The upper-hemisphere poles of the planes with indices up to `max`, seen down the normal of
/// plane `centre`, with the normal of `up` (or [100]/[010]) towards the top.
pub fn stereo(cell: &Cell, centre: [i32; 3], max: i32) -> Result<Vec<Pole>, String> {
    if centre == [0; 3] { return Err("The centre plane cannot be (000).".into()) }
    let z = { let g = cell.g(centre); mul(g, 1.0 / norm(g)) };
    let t = [cell.g([0, 1, 0]), cell.g([1, 0, 0]), cell.g([0, 0, 1])].into_iter().map(|v| sub(v, mul(z, dot(v, z)))).find(|v| norm(*v) > 1e-6 * norm(cell.g([0, 1, 0]))).unwrap_or([0.0, 1.0, 0.0]);
    let y = mul(t, 1.0 / norm(t));
    let x = cross(y, z);
    let max = max.clamp(1, 4);
    let mut out: Vec<Pole> = vec![];
    for h in -max..=max { for k in -max..=max { for l in -max..=max {
        if gcd(gcd(h, k), l) != 1 { continue } // (200) is the same normal as (100)
        let g = cell.g([h, k, l]);
        let u = mul(g, 1.0 / norm(g));
        let w = dot(u, z);
        if w < -1e-9 { continue }
        out.push(Pole { hkl: [h, k, l], x: dot(u, x) / (1.0 + w), y: dot(u, y) / (1.0 + w) });
    }}}
    Ok(out)
}

/// Everything the Crystal › Calculators page asks for at once. `element`, when given, sets the
/// structure, cell and species from that element's data.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Ask { pub element: Option<String>, pub kind: String, pub cell: Cell, pub species: Vec<String>, pub lambda: f64, pub max: f64, pub hkl: [i32; 3], pub hkl2: [i32; 3], pub uvw: [i32; 3], pub uvw2: [i32; 3], pub centre: [i32; 3], pub poles: i32 }

impl Default for Ask {
    fn default() -> Self { Ask { element: None, kind: "fcc".into(), cell: Cell::cubic(3.615), species: vec!["Cu".into()], lambda: 1.5418, max: 120.0, hkl: [1, 1, 1], hkl2: [1, 0, 0], uvw: [1, 1, 0], uvw2: [0, 0, 1], centre: [0, 0, 1], poles: 2 } }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub structure: Structure, pub basis: [V3; 3], pub drawn: Vec<(String, V3)>, pub volume: f64, pub density: f64, pub atoms_per_cell: usize,
    pub d: Option<f64>, pub bragg: Option<f64>, pub angle_planes: Option<f64>, pub angle_dirs: Option<f64>, pub plane4: Option<[i32; 4]>, pub dir4: Option<[i32; 4]>,
    pub peaks: Vec<Peak>, pub poles: Vec<Pole>,
}

pub fn answer(q: &Ask) -> Result<Answer, String> {
    let s = match q.element.as_deref().filter(|e| !e.trim().is_empty()) {
        Some(e) => of_element(e).ok_or(format!("{e}: no element with a known simple structure and lattice parameters (BCC, FCC, HCP, diamond or simple cubic)."))?,
        None => build(&q.kind, q.cell, &q.species)?,
    };
    let c = s.cell;
    let ok = |h: [i32; 3]| h != [0; 3] && h.iter().all(|x| x.abs() <= 99);
    let d = ok(q.hkl).then(|| c.d(q.hkl));
    let hex = s.system == "hexagonal";
    Ok(Answer {
        basis: c.basis(), drawn: s.drawn(), volume: c.volume(), density: s.density(), atoms_per_cell: s.atoms.len(),
        d, bragg: d.and_then(|d| { let x = q.lambda / 2.0 / d; (x <= 1.0).then(|| 2.0 * x.asin().to_degrees()) }),
        angle_planes: (ok(q.hkl) && ok(q.hkl2)).then(|| ang(c.g(q.hkl), c.g(q.hkl2))),
        angle_dirs: (ok(q.uvw) && ok(q.uvw2)).then(|| ang(c.r(q.uvw.map(f64::from)), c.r(q.uvw2.map(f64::from)))),
        plane4: (hex && ok(q.hkl)).then(|| plane4(q.hkl)), dir4: (hex && ok(q.uvw)).then(|| dir4(q.uvw)),
        peaks: peaks(&s, q.lambda, q.max)?, poles: stereo(&c, q.centre, q.poles)?, structure: s,
    })
}

/// X-ray tube lines: [name, wavelength Å] (Kα, weighted mean of Kα1 and Kα2).
pub const TUBES: &[(&str, f64)] = &[("Cu Kα", 1.5418), ("Co Kα", 1.7903), ("Mo Kα", 0.7107), ("Cr Kα", 2.2909), ("Fe Kα", 1.9373), ("Ag Kα", 0.5609)];
