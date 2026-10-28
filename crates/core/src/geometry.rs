//! Chart geometry in axis space (log10 of the value on a log axis, the value itself on a linear
//! one): a bubble is an axis-aligned ellipse spanning a material's two ranges; a family hull is
//! the convex hull of its bubbles, offset by a radius so its corners are round and every bubble
//! lies inside.

/// Axis-space coordinate of a value; None when it cannot be shown (≤ 0 on a log axis).
pub fn t(v: f64, log: bool) -> Option<f64> {
    if !v.is_finite() { return None; }
    if log { (v > 0.0).then(|| v.log10()) } else { Some(v) }
}

/// Centre and half-width in axis space of a range.
pub fn span(lo: f64, hi: f64, log: bool) -> Option<(f64, f64)> {
    let (a, b) = (t(lo, log)?, t(hi, log)?);
    Some(((a + b) / 2.0, (b - a).abs() / 2.0))
}

/// The value at the centre of a range: geometric mean on a log axis, arithmetic otherwise.
pub fn mean(lo: f64, hi: f64, log: bool) -> f64 {
    if log && lo > 0.0 && hi > 0.0 { (lo * hi).sqrt() } else { (lo + hi) / 2.0 }
}

/// Points on an ellipse (centre, radii), `n` of them.
pub fn ellipse(c: [f64; 2], r: [f64; 2], n: usize) -> Vec<[f64; 2]> {
    (0..n).map(|k| { let a = k as f64 / n as f64 * std::f64::consts::TAU; [c[0] + r[0] * a.cos(), c[1] + r[1] * a.sin()] }).collect()
}

fn cross(o: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// Convex hull, counter-clockwise (monotone chain).
pub fn convex(mut pts: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    pts.retain(|p| p[0].is_finite() && p[1].is_finite());
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    pts.dedup();
    if pts.len() < 3 { return pts; }
    let mut h: Vec<[f64; 2]> = vec![];
    for pass in 0..2 {
        let start = h.len();
        let iter: Box<dyn Iterator<Item = &[f64; 2]>> = if pass == 0 { Box::new(pts.iter()) } else { Box::new(pts.iter().rev()) };
        for &p in iter {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], p) <= 0.0 { h.pop(); }
            h.push(p);
        }
        h.pop();
    }
    h
}

/// A rounded hull: the convex hull of the ellipses grown by `pad` (in units scaled by `scale`
/// per axis, so the rounding looks circular on a chart whose axes span `scale`).
pub fn hull(bubbles: &[([f64; 2], [f64; 2])], pad: f64, scale: [f64; 2]) -> Vec<[f64; 2]> {
    let n = |p: [f64; 2]| [p[0] / scale[0], p[1] / scale[1]];
    let pts: Vec<[f64; 2]> = bubbles.iter().flat_map(|(c, r)| ellipse(n(*c), [r[0] / scale[0], r[1] / scale[1]], 32)).collect();
    let h = convex(pts);
    let mut out = vec![];
    match h.len() {
        0 => {}
        1 | 2 => {
            // One small bubble or a line of them: a capsule around the points.
            let (a, b) = (h[0], *h.last().unwrap());
            let ang = (b[1] - a[1]).atan2(b[0] - a[0]);
            for (p, a0) in [(b, ang - std::f64::consts::FRAC_PI_2), (a, ang + std::f64::consts::FRAC_PI_2)] {
                for k in 0..=12 { let a = a0 + k as f64 / 12.0 * std::f64::consts::PI; out.push([p[0] + pad * a.cos(), p[1] + pad * a.sin()]); }
            }
        }
        m => for i in 0..m {
            // Arc around each vertex between the normals of its two edges (outward, CCW hull).
            let (p, a, b) = (h[i], h[(i + m - 1) % m], h[(i + 1) % m]);
            let n0 = (p[1] - a[1]).atan2(p[0] - a[0]) - std::f64::consts::FRAC_PI_2;
            let mut n1 = (b[1] - p[1]).atan2(b[0] - p[0]) - std::f64::consts::FRAC_PI_2;
            while n1 < n0 { n1 += std::f64::consts::TAU; }
            let steps = ((n1 - n0) / 0.2).ceil().max(1.0) as usize;
            for k in 0..=steps { let a = n0 + (n1 - n0) * k as f64 / steps as f64; out.push([p[0] + pad * a.cos(), p[1] + pad * a.sin()]); }
        },
    }
    out.into_iter().map(|p| [p[0] * scale[0], p[1] * scale[1]]).collect()
}

/// Is `p` inside the convex polygon `poly` (CCW), allowing `tol`?
pub fn inside(poly: &[[f64; 2]], p: [f64; 2], tol: f64) -> bool {
    let m = poly.len();
    m >= 3 && (0..m).all(|i| {
        let (a, b) = (poly[i], poly[(i + 1) % m]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt().max(1e-12);
        cross(a, b, p) / len >= -tol
    })
}

/// Is `p` inside the polygon `poly` (any shape, either winding)? Even-odd rule.
pub fn in_polygon(poly: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut inside = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + poly.len() - 1) % poly.len()]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] { inside = !inside }
    }
    inside
}

/// Radius of an axis-aligned ellipse in direction (dx, dy).
fn reach(r: [f64; 2], dx: f64, dy: f64) -> f64 {
    let d = (dx * dx + dy * dy).sqrt();
    if d == 0.0 { return r[0].max(r[1]); }
    let (c, s) = (dx / d, dy / d);
    r[0] * r[1] / ((r[1] * c).powi(2) + (r[0] * s).powi(2)).sqrt().max(1e-12)
}

/// Groups of bubbles that lie within `gap` of each other (in axis units scaled by `scale`), so a
/// family far apart on a chart gets an envelope per group instead of one around empty space.
pub fn clusters(bubbles: &[([f64; 2], [f64; 2])], scale: [f64; 2], gap: f64) -> Vec<Vec<usize>> {
    let n = bubbles.len();
    let mut root: Vec<usize> = (0..n).collect();
    fn find(r: &mut [usize], i: usize) -> usize { let mut i = i; while r[i] != i { r[i] = r[r[i]]; i = r[i]; } i }
    let norm = |(c, r): ([f64; 2], [f64; 2])| ([c[0] / scale[0], c[1] / scale[1]], [r[0] / scale[0], r[1] / scale[1]]);
    for i in 0..n {
        for j in i + 1..n {
            let ((a, ra), (b, rb)) = (norm(bubbles[i]), norm(bubbles[j]));
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let between = (dx * dx + dy * dy).sqrt() - reach(ra, dx, dy) - reach(rb, dx, dy);
            if between < gap {
                let (x, y) = (find(&mut root, i), find(&mut root, j));
                root[x] = y;
            }
        }
    }
    let mut groups: Vec<(usize, Vec<usize>)> = vec![];
    for i in 0..n {
        let r = find(&mut root, i);
        match groups.iter_mut().find(|g| g.0 == r) { Some(g) => g.1.push(i), None => groups.push((r, vec![i])) }
    }
    groups.into_iter().map(|g| g.1).collect()
}
