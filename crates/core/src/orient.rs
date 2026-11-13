//! Crystal orientations: Bunge Euler angles and orientation matrices (g takes sample axes into
//! crystal axes), cubic and hexagonal symmetry, misorientation angles, random and textured
//! orientations, and inverse pole figure (IPF) colours. Written from the textbook definitions.
use serde::Serialize;

pub type M3 = [[f64; 3]; 3];
type V3 = [f64; 3];

pub fn mul(a: &M3, b: &M3) -> M3 { let mut c = [[0.0; 3]; 3]; for i in 0..3 { for j in 0..3 { c[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum() } } c }
pub fn tr(a: &M3) -> M3 { let mut c = [[0.0; 3]; 3]; for i in 0..3 { for j in 0..3 { c[i][j] = a[j][i] } } c }
pub fn apply(a: &M3, v: V3) -> V3 { [0, 1, 2].map(|i| a[i][0] * v[0] + a[i][1] * v[1] + a[i][2] * v[2]) }

/// The orientation matrix of Bunge Euler angles (φ1, Φ, φ2), in degrees.
pub fn from_euler(e: [f64; 3]) -> M3 {
    let [p1, p, p2] = e.map(f64::to_radians);
    let (c1, s1, c, s, c2, s2) = (p1.cos(), p1.sin(), p.cos(), p.sin(), p2.cos(), p2.sin());
    [[c1 * c2 - s1 * s2 * c, s1 * c2 + c1 * s2 * c, s2 * s], [-c1 * s2 - s1 * c2 * c, -s1 * s2 + c1 * c2 * c, c2 * s], [s1 * s, -c1 * s, c]]
}
/// Bunge Euler angles (degrees, φ1 and φ2 in 0–360) of an orientation matrix.
pub fn to_euler(g: &M3) -> [f64; 3] {
    let p = g[2][2].clamp(-1.0, 1.0).acos();
    let (p1, p2) = if p.sin().abs() > 1e-6 { (g[2][0].atan2(-g[2][1]), g[0][2].atan2(g[1][2])) } else { (g[0][1].atan2(g[0][0]), 0.0) };
    let w = |a: f64| a.to_degrees().rem_euclid(360.0);
    [w(p1), p.to_degrees(), w(p2)]
}
/// A rotation of `ang` degrees about the unit axis `a` (Rodrigues).
pub fn axis_angle(a: V3, ang: f64) -> M3 {
    let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-12);
    let [x, y, z] = a.map(|v| v / n);
    let (c, s) = (ang.to_radians().cos(), ang.to_radians().sin());
    let t = 1.0 - c;
    [[t * x * x + c, t * x * y - s * z, t * x * z + s * y], [t * x * y + s * z, t * y * y + c, t * y * z - s * x], [t * x * z - s * y, t * y * z + s * x, t * z * z + c]]
}

/// The rotations of the crystal's symmetry: "cubic" (24) or "hex" (12).
pub fn symmetry(c: &str) -> Vec<M3> {
    if c == "hex" {
        let mut v: Vec<M3> = (0..6).map(|k| axis_angle([0.0, 0.0, 1.0], 60.0 * k as f64)).collect();
        v.extend((0..6).map(|k| { let a = (30.0 * k as f64).to_radians(); axis_angle([a.cos(), a.sin(), 0.0], 180.0) }));
        return v;
    }
    let mut v = vec![];
    for p in [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] { for s in 0..8 {
        let sg = [1.0 - 2.0 * (s & 1) as f64, 1.0 - 2.0 * (s >> 1 & 1) as f64, 1.0 - 2.0 * (s >> 2 & 1) as f64];
        let mut m = [[0.0; 3]; 3];
        for i in 0..3 { m[i][p[i]] = sg[i] }
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        if det > 0.0 { v.push(m) }
    }}
    v
}

fn angle(m: &M3) -> f64 { (((m[0][0] + m[1][1] + m[2][2]) - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees() }
/// The misorientation angle (degrees) between two orientations of the same symmetry.
pub fn misorientation(a: &M3, b: &M3, sym: &[M3]) -> f64 {
    let d = mul(b, &tr(a));
    sym.iter().map(|s| angle(&mul(s, &d))).fold(f64::MAX, f64::min)
}

/// A uniformly random orientation (Shoemake's method), from three numbers in 0–1.
pub fn random(u: [f64; 3]) -> M3 {
    let (a, b) = ((1.0 - u[0]).sqrt(), u[0].sqrt());
    let (t1, t2) = (std::f64::consts::TAU * u[1], std::f64::consts::TAU * u[2]);
    let (w, x, y, z) = (b * t2.cos(), a * t1.sin(), a * t1.cos(), b * t2.sin());
    [[1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w)], [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w)], [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y)]]
}

/// Texture components of cubic metals: [id, name, Bunge Euler angles].
pub const COMPONENTS: &[(&str, &str, [f64; 3])] = &[
    ("cube", "Cube {001}<100>", [0.0, 0.0, 0.0]), ("goss", "Goss {110}<001>", [0.0, 45.0, 0.0]), ("brass", "Brass {110}<112>", [35.26, 45.0, 0.0]),
    ("copper", "Copper {112}<111>", [90.0, 35.26, 45.0]), ("s", "S {123}<634>", [58.98, 36.7, 63.43]), ("rcube", "Rotated cube {001}<110>", [45.0, 0.0, 0.0]),
];
/// An orientation with crystal direction `d` along the sample's Z (normal) direction, then turned `rot` degrees about Z.
pub fn fibre(d: V3, rot: f64) -> M3 {
    let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let z = d.map(|v| v / n);
    let t = if z[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let x = { let c = [z[1] * t[2] - z[2] * t[1], z[2] * t[0] - z[0] * t[2], z[0] * t[1] - z[1] * t[0]]; let m = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt(); c.map(|v| v / m) };
    let y = [z[1] * x[2] - z[2] * x[1], z[2] * x[0] - z[0] * x[2], z[0] * x[1] - z[1] * x[0]];
    // g's columns are the sample axes written in crystal axes.
    let g0 = [[x[0], y[0], z[0]], [x[1], y[1], z[1]], [x[2], y[2], z[2]]];
    mul(&g0, &axis_angle([0.0, 0.0, 1.0], rot))
}

/// The IPF colour of sample direction `s` for an orientation: red, green and blue at the corners of
/// the standard triangle ([001], [101], [111] for cubic; [0001], [2-1-10], [10-10] for hexagonal).
pub fn ipf(g: &M3, s: V3, crystal: &str) -> [u8; 3] {
    let d = apply(g, s);
    let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-12);
    let [a, b, c] = d.map(|v| (v / n).abs());
    let rgb = if crystal == "hex" {
        let t = c.clamp(0.0, 1.0).acos() / std::f64::consts::FRAC_PI_2;
        let phi = b.atan2(a).to_degrees().rem_euclid(60.0);
        let u = (if phi > 30.0 { 60.0 - phi } else { phi }) / 30.0;
        [1.0 - t, t * (1.0 - u), t * u]
    } else {
        let mut v = [a, b, c];
        v.sort_by(f64::total_cmp); // y ≤ x ≤ z
        let (y, x, z) = (v[0], v[1], v[2]);
        [z - x, x - y, y]
    };
    let m = rgb.iter().copied().fold(1e-12, f64::max);
    rgb.map(|v| ((v / m).max(0.0).powf(0.5) * 255.0).round() as u8)
}

/// Points of the IPF colour key: the standard triangle on a stereographic projection, scaled so it fits 0–1, with their colours.
#[derive(Debug, Clone, Serialize)]
pub struct KeyPt { pub x: f64, pub y: f64, pub c: [u8; 3] }
pub fn ipf_key(crystal: &str, n: usize) -> Vec<KeyPt> {
    let mut out = vec![];
    // The triangle reaches r = tan(22.5°) for cubic and r = 1 for hexagonal; x and y are scaled by it.
    let rm = if crystal == "hex" { 1.0 } else { 0.4142 };
    for i in 0..=n { for j in 0..=n {
        let (px, py) = (i as f64 / n as f64 * rm, j as f64 / n as f64 * rm);
        let r2 = px * px + py * py;
        let d = [2.0 * px / (1.0 + r2), 2.0 * py / (1.0 + r2), (1.0 - r2) / (1.0 + r2)]; // inverse stereographic
        let inside = if crystal == "hex" { py <= px * (30f64).to_radians().tan() && r2 <= 1.0 } else { py <= px && d[0] <= d[2] };
        if inside { out.push(KeyPt { x: px / rm, y: py / rm, c: ipf(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], d, crystal) }) }
    }}
    out
}
