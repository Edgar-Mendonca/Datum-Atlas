//! Crystal structures from CIF files (the IUCr's open format, as in the Crystallography Open
//! Database): cell, symmetry operations, atom sites with occupancy and displacement, and the
//! source to credit. From a structure: its full unit cell and an accurate powder pattern, with
//! atomic form factors and anomalous scattering from the Livermore photon data library
//! (data/xray.json), Debye–Waller factors, multiplicity and Lorentz–polarisation.
use crate::crystal::Cell;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

// ---------- X-ray scattering data ----------

#[derive(Deserialize)]
struct XEl { f0: Vec<f64>, #[serde(default)] f1: std::collections::HashMap<String, f64>, #[serde(default)] f2: std::collections::HashMap<String, f64> }
#[derive(Deserialize)]
struct XData { step: f64, elements: std::collections::HashMap<String, XEl> }
fn xdata() -> &'static XData {
    static D: OnceLock<XData> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../../../data/xray.json")).unwrap_or_else(|e| panic!("data/xray.json: {e}")))
}

/// The atomic scattering factor of element `z` at s = sin θ / λ (Å⁻¹): the form factor (cubic
/// interpolation in the table) plus, for an anode, the anomalous f′ and f″. Returns (real, imaginary).
pub fn scattering(z: u32, s: f64, anode: &str) -> Option<(f64, f64)> {
    let e = xdata().elements.get(&z.to_string())?;
    let t = s.max(0.0) / xdata().step;
    let n = e.f0.len();
    let f0 = if t >= (n - 1) as f64 { e.f0[n - 1] } else {
        let i = t.floor() as usize;
        let u = t - i as f64;
        let p = |k: isize| e.f0[(i as isize + k).clamp(0, n as isize - 1) as usize];
        // Catmull–Rom through the neighbouring points.
        let (a, b, c, d) = (p(-1), p(0), p(1), p(2));
        b + 0.5 * u * (c - a + u * (2.0 * a - 5.0 * b + 4.0 * c - d + u * (3.0 * (b - c) + d - a)))
    };
    Some((f0 + e.f1.get(anode).copied().unwrap_or(0.0), e.f2.get(anode).copied().unwrap_or(0.0)))
}

// ---------- CIF reading ----------

/// One atom site as written in the file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Site { pub label: String, pub element: String, pub xyz: [f64; 3], pub occ: f64, /// Isotropic displacement B (Å²).
    pub b: f64 }

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Phase {
    pub name: String, pub formula: String, pub mineral: String, pub space_group: String, pub cell: CellOut,
    /// Symmetry operations (rotation rows and translation), as read.
    pub ops: Vec<([[f64; 3]; 3], [f64; 3])>, pub sites: Vec<Site>,
    /// Where it comes from, to credit: COD id, authors, journal, year, title.
    pub cod: String, pub authors: String, pub journal: String, pub year: String, pub title: String, pub notes: Vec<String>,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct CellOut { pub a: f64, pub b: f64, pub c: f64, pub alpha: f64, pub beta: f64, pub gamma: f64 }
impl CellOut { pub fn cell(&self) -> Cell { Cell { a: self.a, b: self.b, c: self.c, alpha: self.alpha, beta: self.beta, gamma: self.gamma } } }

/// Split CIF text into tokens: tags, values (quoted or bare), semicolon text fields, loop_ and data_.
fn tokens(text: &str) -> Vec<String> {
    let mut out = vec![];
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix(';') {
            let mut s = rest.to_string();
            for l in lines.by_ref() { if l.starts_with(';') { break } s.push('\n'); s.push_str(l); }
            out.push(s.trim().to_string());
            continue;
        }
        let ch: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < ch.len() {
            let c = ch[i];
            if c.is_whitespace() { i += 1; continue }
            if c == '#' { break }
            if c == '\'' || c == '"' {
                // A quote closes only when followed by white space or the end of the line.
                let mut j = i + 1;
                while j < ch.len() && !(ch[j] == c && (j + 1 == ch.len() || ch[j + 1].is_whitespace())) { j += 1 }
                out.push(ch[i + 1..j.min(ch.len())].iter().collect());
                i = j + 1;
                continue;
            }
            let mut j = i;
            while j < ch.len() && !ch[j].is_whitespace() { j += 1 }
            out.push(ch[i..j].iter().collect());
            i = j;
        }
    }
    out
}

/// A number from a CIF value: "5.431(2)" → 5.431; "?" and "." are missing.
pub fn cnum(s: &str) -> Option<f64> { let t = s.split('(').next()?.trim(); if t == "?" || t == "." { return None } t.parse().ok() }

/// One symmetry operation "x-y+1/2, -x, z+1/3" as rows of a rotation and a translation.
pub fn symop(s: &str) -> Result<([[f64; 3]; 3], [f64; 3]), String> {
    let parts: Vec<&str> = s.split(',').map(str::trim).collect();
    if parts.len() != 3 { return Err(format!("Not a symmetry operation: “{s}”.")) }
    let mut r = [[0.0; 3]; 3];
    let mut t = [0.0; 3];
    for (k, p) in parts.iter().enumerate() {
        let p = p.to_lowercase().replace(' ', "");
        let mut sign = 1.0;
        let mut num = String::new();
        let flush = |num: &mut String, sign: f64, t: &mut f64| -> Result<(), String> {
            if num.is_empty() { return Ok(()) }
            let v = if let Some((a, b)) = num.split_once('/') { a.parse::<f64>().map_err(|_| "bad fraction")? / b.parse::<f64>().map_err(|_| "bad fraction")? } else { num.parse::<f64>().map_err(|_| "bad number")? };
            *t += sign * v; num.clear(); Ok(())
        };
        for c in p.chars() {
            match c {
                '+' | '-' => { flush(&mut num, sign, &mut t[k]).map_err(|e| format!("{e} in “{s}”"))?; sign = if c == '-' { -1.0 } else { 1.0 } }
                'x' | 'y' | 'z' => {
                    let f = if num.is_empty() { 1.0 } else { num.parse::<f64>().map_err(|_| format!("Not a symmetry operation: “{s}”."))? };
                    num.clear();
                    r[k][(c as u8 - b'x') as usize] += sign * f; sign = 1.0;
                }
                '*' => {}
                d if d.is_ascii_digit() || d == '.' || d == '/' => num.push(d),
                _ => return Err(format!("Not a symmetry operation: “{s}”.")),
            }
        }
        flush(&mut num, sign, &mut t[k]).map_err(|e| format!("{e} in “{s}”"))?;
    }
    Ok((r, t))
}

/// The element of an atom: from the type symbol ("Fe2+", "O2-") or the label ("Fe1", "O2a").
fn element_of(s: &str) -> String {
    let letters: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let mut cs = letters.chars();
    let (a, b) = (cs.next(), cs.next());
    let two = match (a, b) { (Some(a), Some(b)) => format!("{}{}", a.to_ascii_uppercase(), b.to_ascii_lowercase()), _ => String::new() };
    if !two.is_empty() && crate::elements::find(&two).is_some() { return two }
    a.map(|a| a.to_ascii_uppercase().to_string()).unwrap_or_default()
}

/// A CIF loop: its column names and its values, row after row.
type Loop = (Vec<String>, Vec<String>);

/// Read the first data block of a CIF that has a cell.
pub fn parse(text: &str, name: &str) -> Result<Phase, String> {
    let tk = tokens(text);
    let mut tags: Vec<(String, String)> = vec![];
    let mut loops: Vec<Loop> = vec![];
    let mut i = 0;
    let mut blocks = 0;
    while i < tk.len() {
        let t = &tk[i];
        let low = t.to_lowercase();
        if low.starts_with("data_") {
            blocks += 1;
            if blocks > 1 && tags.iter().any(|(k, _)| k == "_cell_length_a") { break }
            i += 1; continue;
        }
        if low == "loop_" {
            let mut heads = vec![];
            i += 1;
            while i < tk.len() && tk[i].starts_with('_') { heads.push(tk[i].to_lowercase()); i += 1 }
            let mut vals = vec![];
            while i < tk.len() && !tk[i].starts_with('_') && !tk[i].to_lowercase().starts_with("loop_") && !tk[i].to_lowercase().starts_with("data_") { vals.push(tk[i].clone()); i += 1 }
            loops.push((heads, vals));
            continue;
        }
        if t.starts_with('_') {
            let v = tk.get(i + 1).cloned().unwrap_or_default();
            tags.push((low, v));
            i += 2; continue;
        }
        i += 1;
    }
    let tag = |k: &str| tags.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
    let num = |k: &str| tag(k).and_then(|v| cnum(&v));
    let column = |names: &[&str]| -> Option<(usize, &Loop)> {
        loops.iter().find_map(|l| names.iter().find_map(|n| l.0.iter().position(|h| h == n)).map(|p| (p, l)))
    };
    let (Some(a), Some(b), Some(c)) = (num("_cell_length_a"), num("_cell_length_b"), num("_cell_length_c")) else { return Err("No unit cell in this file: is it a CIF of a crystal structure?".into()) };
    let cell = CellOut { a, b, c, alpha: num("_cell_angle_alpha").unwrap_or(90.0), beta: num("_cell_angle_beta").unwrap_or(90.0), gamma: num("_cell_angle_gamma").unwrap_or(90.0) };
    if [a, b, c].iter().any(|v| !(*v > 0.5 && *v < 200.0)) { return Err("The cell lengths should be between 0.5 and 200 Å.".into()) }
    let mut notes = vec![];
    // Symmetry operations.
    let mut ops = vec![];
    if let Some((p, (h, v))) = column(&["_space_group_symop_operation_xyz", "_symmetry_equiv_pos_as_xyz"]) {
        for row in v.chunks(h.len()) { if let Some(s) = row.get(p) { ops.push(symop(s)?) } }
    } else if let Some(s) = tag("_space_group_symop_operation_xyz").or_else(|| tag("_symmetry_equiv_pos_as_xyz")) { ops.push(symop(&s)?) }
    if ops.is_empty() {
        ops.push(([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], [0.0; 3]));
        notes.push("The file lists no symmetry operations: the atoms are taken as they are (space group P1). If it names a space group, export the CIF with its symmetry operations.".into());
    }
    // Atom sites.
    let (_, (h, v)) = column(&["_atom_site_fract_x"]).ok_or("No atom positions (_atom_site_fract_x) in this file.")?;
    let col = |n: &str| h.iter().position(|x| x == n);
    let (cx, cy, cz) = (col("_atom_site_fract_x").ok_or("No x")?, col("_atom_site_fract_y").ok_or("No y in the atom list.")?, col("_atom_site_fract_z").ok_or("No z in the atom list.")?);
    let (cl, ct, co, cu, cb) = (col("_atom_site_label"), col("_atom_site_type_symbol"), col("_atom_site_occupancy"), col("_atom_site_u_iso_or_equiv"), col("_atom_site_b_iso_or_equiv"));
    // Anisotropic displacements, for atoms without an isotropic one: the mean of U11, U22, U33.
    let aniso: Vec<(String, f64)> = column(&["_atom_site_aniso_u_11"]).map(|(_, (ah, av))| {
        let p = |n: &str| ah.iter().position(|x| x == n);
        match (p("_atom_site_aniso_label"), p("_atom_site_aniso_u_11"), p("_atom_site_aniso_u_22"), p("_atom_site_aniso_u_33")) {
            (Some(l), Some(a1), Some(a2), Some(a3)) => av.chunks(ah.len()).filter(|r| r.len() == ah.len()).filter_map(|r| Some((r[l].clone(), (cnum(&r[a1])? + cnum(&r[a2])? + cnum(&r[a3])?) / 3.0))).collect(),
            _ => vec![],
        }
    }).unwrap_or_default();
    let mut sites = vec![];
    let mut no_b = 0;
    for r in v.chunks(h.len()).filter(|r| r.len() == h.len()) {
        let label = cl.map(|k| r[k].clone()).unwrap_or_default();
        let element = element_of(ct.map(|k| r[k].as_str()).unwrap_or(&label));
        if crate::elements::find(&element).is_none() { notes.push(format!("{label}: element “{element}” not known; left out.")); continue }
        let (Some(x), Some(y), Some(z)) = (cnum(&r[cx]), cnum(&r[cy]), cnum(&r[cz])) else { continue };
        let occ = co.and_then(|k| cnum(&r[k])).unwrap_or(1.0);
        let pi8 = 8.0 * std::f64::consts::PI * std::f64::consts::PI;
        let b = cb.and_then(|k| cnum(&r[k])).or_else(|| cu.and_then(|k| cnum(&r[k])).map(|u| pi8 * u)).or_else(|| aniso.iter().find(|(l, _)| *l == label).map(|(_, u)| pi8 * u));
        if b.is_none() { no_b += 1 }
        sites.push(Site { label, element, xyz: [x, y, z], occ, b: b.unwrap_or(0.5) });
    }
    if sites.is_empty() { return Err("No atoms with known elements and positions.".into()) }
    if no_b > 0 { notes.push(format!("{no_b} atom site(s) without a displacement parameter: B = 0.5 Å² taken.")) }
    let authors = column(&["_publ_author_name"]).map(|(p, (h, v))| v.chunks(h.len()).filter_map(|r| r.get(p).cloned()).collect::<Vec<_>>().join("; ")).or_else(|| tag("_publ_author_name")).unwrap_or_default();
    let clean = |s: Option<String>| s.filter(|v| v != "?" && v != ".").map(|v| v.split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default();
    let formula = clean(tag("_chemical_formula_sum"));
    let mineral = clean(tag("_chemical_name_mineral"));
    let sg = clean(tag("_space_group_name_h-m_alt").or_else(|| tag("_symmetry_space_group_name_h-m")));
    let common = clean(tag("_chemical_name_common"));
    let title = [&mineral, &common, &formula].into_iter().find(|s| !s.is_empty()).cloned().unwrap_or_else(|| name.to_string());
    Ok(Phase { name: title, formula, mineral, space_group: sg, cell, ops, sites, cod: clean(tag("_cod_database_code")), authors,
        journal: clean(tag("_journal_name_full")), year: clean(tag("_journal_year")), title: clean(tag("_publ_section_title")), notes })
}

/// Every atom of the unit cell: each site under each symmetry operation, brought into the cell,
/// with duplicates (special positions) removed.
pub fn unit_cell(p: &Phase) -> Vec<(String, [f64; 3], f64, f64)> {
    let mut out: Vec<(String, [f64; 3], f64, f64)> = vec![];
    for s in &p.sites {
        for (r, t) in &p.ops {
            let q: [f64; 3] = std::array::from_fn(|k| (r[k][0] * s.xyz[0] + r[k][1] * s.xyz[1] + r[k][2] * s.xyz[2] + t[k]).rem_euclid(1.0));
            let same = |u: &[f64; 3]| (0..3).all(|k| { let d = (u[k] - q[k]).abs(); d.min(1.0 - d) < 1e-3 });
            if !out.iter().any(|(e, u, _, _)| *e == s.element && same(u)) { out.push((s.element.clone(), q, s.occ, s.b)) }
        }
    }
    out
}

// ---------- structures kept in the project ----------

use crate::research::{Item, Research};
pub const MAX_PHASES: usize = 24;

/// What the window shows about a structure (or why it cannot be read).
pub fn summary(text: &str, file: &str) -> serde_json::Value {
    match parse(text, file) {
        Ok(p) => { let i = info(&p); serde_json::json!({ "file": file, "name": p.name, "formula": p.formula, "spaceGroup": p.space_group, "cell": p.cell, "atoms": i.atoms, "density": i.density, "z": i.z_formula,
            "cod": p.cod, "authors": p.authors, "journal": p.journal, "year": p.year, "title": p.title, "notes": p.notes, "ops": p.ops.len() }) }
        Err(e) => serde_json::json!({ "file": file, "name": file, "error": e }),
    }
}
pub fn kept(r: &Research) -> Result<Vec<(String, String)>, String> { r.cif.as_ref().map_or(Ok(vec![]), Item::to_cifs) }
fn settings(r: &Research) -> serde_json::Value { r.cif.as_ref().map_or(serde_json::json!({}), |i| i.settings.clone()) }
/// The kept structures with one more (read first, so a file that is not a structure is refused).
pub fn add(r: &Research, file: &str, text: &str) -> Result<Item, String> {
    parse(text, file)?;
    let mut v = kept(r)?;
    if v.len() >= MAX_PHASES { return Err(format!("Up to {MAX_PHASES} structures in one project; remove one first.")) }
    v.push((file.to_string(), text.to_string()));
    Ok(Item::cifs(&v, settings(r)))
}
pub fn remove(r: &Research, k: usize) -> Result<Option<Item>, String> {
    let mut v = kept(r)?;
    if k >= v.len() { return Err("That structure is no longer open.".into()) }
    v.remove(k);
    Ok((!v.is_empty()).then(|| Item::cifs(&v, settings(r))))
}
/// Structure `k` of the project.
pub fn phase(r: &Research, k: usize) -> Result<Phase, String> {
    let v = kept(r)?;
    let (f, t) = v.get(k).ok_or("That structure is no longer open.")?;
    parse(t, f)
}

/// The example: rock salt (NaCl, a = 5.64 Å, the crystal module's textbook cell), written out atom
/// by atom as a CIF without symmetry operations.
pub fn example() -> String {
    let mut s = "data_rock_salt\n_chemical_name_common 'Rock salt (example)'\n_chemical_name_systematic 'Sodium chloride: the crystal module textbook cell, atoms written out'\n_chemical_formula_sum 'Cl Na'\n_cell_length_a 5.64\n_cell_length_b 5.64\n_cell_length_c 5.64\n_cell_angle_alpha 90\n_cell_angle_beta 90\n_cell_angle_gamma 90\nloop_\n_symmetry_equiv_pos_as_xyz\nx,y,z\nloop_\n_atom_site_label\n_atom_site_type_symbol\n_atom_site_fract_x\n_atom_site_fract_y\n_atom_site_fract_z\n_atom_site_B_iso_or_equiv\n".to_string();
    for (k, f) in [[0.0, 0.0, 0.0], [0.0, 0.5, 0.5], [0.5, 0.0, 0.5], [0.5, 0.5, 0.0]].iter().enumerate() {
        s += &format!("Na{} Na {} {} {} 1.6\n", k + 1, f[0], f[1], f[2]);
        s += &format!("Cl{} Cl {} {} {} 1.2\n", k + 1, (f[0] + 0.5) % 1.0, f[1], f[2]);
    }
    s
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PatAsk { pub anode: String, pub lambda: f64, pub max: f64, pub mono: f64, pub fwhm: f64, pub kalpha2: bool }
impl Default for PatAsk { fn default() -> Self { Self { anode: "Cu".into(), lambda: 0.0, max: 120.0, mono: 0.0, fwhm: 0.1, kalpha2: true } } }

/// Lines and a profile of structure `k`, and, when a measured pattern is given, that pattern
/// (background taken off) with each chosen structure's profile scaled to its strongest point.
pub fn calculate(r: &Research, k: usize, over: Option<usize>, with: &[usize], q: &PatAsk) -> Result<serde_json::Value, String> {
    let (l1, l2) = crate::xrd::TUBES.iter().find(|t| t.0 == q.anode).map(|t| (t.1, t.2)).unwrap_or((q.lambda, q.lambda));
    if l1.is_nan() || l1 <= 0.05 { return Err("Choose the anode, or type the wavelength in Å.".into()) }
    let anode = if crate::xrd::TUBES.iter().any(|t| t.0 == q.anode) { q.anode.as_str() } else { "" };
    let p = phase(r, k)?;
    let lines = pattern(&p, l1, anode, q.max, q.mono)?;
    let l2 = if q.kalpha2 { l2 } else { l1 };
    let mut out = serde_json::json!({ "lines": lines, "lambda": l1 });
    if let Some(m) = over {
        let pats = crate::xrd::kept(r)?;
        let pt = pats.get(m).ok_or("That pattern is no longer open.")?;
        let a = crate::xrd::analyse(pt, &crate::xrd::Ask { anode: anode.into(), lambda: l1, smooth: 0, bg: 3.0, strip: false, ..Default::default() })?;
        let step = a.x.len().div_ceil(4000).max(1);
        let x: Vec<f64> = a.x.iter().step_by(step).copied().collect();
        let y: Vec<f64> = a.net.iter().step_by(step).copied().collect();
        let top = y.iter().cloned().fold(0.0, f64::max).max(1.0);
        let mut series = vec![];
        for &j in with {
            let ph = phase(r, j)?;
            let ls = pattern(&ph, l1, anode, x.last().copied().unwrap_or(q.max).min(175.0), q.mono)?;
            let c = profile(&ls, &x, q.fwhm, l1, l2);
            let cm = c.iter().cloned().fold(0.0, f64::max).max(1e-9);
            series.push(serde_json::json!({ "name": ph.name, "y": c.iter().map(|v| v / cm * top).collect::<Vec<_>>(), "lines": ls.iter().map(|q| [q.two_theta, q.intensity / 100.0 * top]).collect::<Vec<_>>() }));
        }
        out["over"] = serde_json::json!({ "name": pt.name, "x": x, "y": y, "series": series });
    } else {
        let x: Vec<f64> = (0..=((q.max.clamp(10.0, 175.0) - 5.0) / 0.02) as usize).map(|i| 5.0 + i as f64 * 0.02).collect();
        out["profile"] = serde_json::json!({ "x": x.iter().step_by(2).collect::<Vec<_>>(), "y": profile(&lines, &x, q.fwhm, l1, l2).into_iter().step_by(2).collect::<Vec<_>>() });
    }
    Ok(out)
}

// ---------- the calculated pattern ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line { pub hkl: [i32; 3], pub mult: usize, pub d: f64, pub two_theta: f64, pub f2: f64, pub intensity: f64 }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info { pub atoms: usize, pub density: f64, pub volume: f64, pub z_formula: Option<f64> }

/// Cell contents and density (occupancy-weighted masses).
pub fn info(p: &Phase) -> Info {
    let atoms = unit_cell(p);
    let mass: f64 = atoms.iter().filter_map(|(e, _, o, _)| crate::elements::find(e).map(|x| x.mass * o)).sum();
    let v = p.cell.cell().volume();
    // Formula units in the cell, when the formula sum can be read.
    let fz = (|| {
        let mut fm = 0.0;
        for part in p.formula.split_whitespace() {
            let el: String = part.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            let n: f64 = part[el.len()..].parse().unwrap_or(1.0);
            fm += crate::elements::find(&el)?.mass * n;
        }
        (fm > 0.0).then(|| (mass / fm * 100.0).round() / 100.0)
    })();
    Info { atoms: atoms.len(), density: mass / 6.022_140_76e23 / (v * 1e-30) / 1000.0, volume: v, z_formula: fz }
}

/// The powder pattern of a phase: positions by Bragg's law, intensity ∝ multiplicity × |F|² ×
/// Lorentz–polarisation (with a monochromator angle 2θm, 0 for none), F with form factors,
/// anomalous scattering for the anode, occupancy and Debye–Waller factors. Strongest = 100.
pub fn pattern(p: &Phase, lambda: f64, anode: &str, max: f64, mono: f64) -> Result<Vec<Line>, String> {
    if !(lambda > 0.05 && lambda < 10.0) { return Err("The wavelength must be between 0.05 and 10 Å.".into()) }
    let max = max.clamp(5.0, 175.0);
    let cell = p.cell.cell();
    let dmin = lambda / 2.0 / (max.to_radians() / 2.0).sin();
    let n = |x: f64| (x / dmin).ceil().min(200.0) as i32 + 1;
    let (nh, nk, nl) = (n(cell.a), n(cell.b), n(cell.c));
    if (2 * nh as i64 + 1) * (2 * nk as i64 + 1) * (2 * nl as i64 + 1) > 3_000_000 { return Err("Too many reflections: shorten the range of 2θ.".into()) }
    // Each element's scattering factor is worked out once per reflection, not once per atom.
    let mut zs: Vec<u32> = vec![];
    let atoms: Vec<(usize, [f64; 3], f64, f64)> = unit_cell(p).into_iter().filter_map(|(e, x, o, b)| crate::elements::find(&e).map(|el| {
        let j = zs.iter().position(|&z| z == el.z).unwrap_or_else(|| { zs.push(el.z); zs.len() - 1 });
        (j, x, o, b) })).collect();
    let mut fz: Vec<Option<(f64, f64)>> = vec![None; zs.len()];
    // Without anomalous scattering, hkl and its opposite have the same |F|² (Friedel's law): work out half.
    let friedel = zs.iter().all(|z| scattering(*z, 0.1, anode).is_none_or(|f| f.1 == 0.0));
    let cm = (mono.to_radians()).cos().powi(2);
    let mut all: Vec<([i32; 3], f64, f64)> = vec![];
    for h in -nh..=nh { for k in -nk..=nk { for l in -nl..=nl {
        if h == 0 && k == 0 && l == 0 || friedel && [h, k, l] < [0, 0, 0] { continue }
        let d = cell.d([h, k, l]);
        if d < dmin { continue }
        let s = 0.5 / d;
        let (mut re, mut im) = (0.0, 0.0);
        for (f, z) in fz.iter_mut().zip(&zs) { *f = scattering(*z, s, anode) }
        for (j, x, o, b) in &atoms {
            let Some((fr, fi)) = fz[*j] else { continue };
            let w = o * (-b * s * s).exp();
            let ph = std::f64::consts::TAU * (h as f64 * x[0] + k as f64 * x[1] + l as f64 * x[2]);
            let (c, sn) = (ph.cos(), ph.sin());
            re += w * (fr * c - fi * sn);
            im += w * (fr * sn + fi * c);
        }
        let f2 = re * re + im * im;
        if f2 > 1e-6 { all.push(([h, k, l], d, f2)); if friedel { all.push(([-h, -k, -l], d, f2)) } }
    }}}
    // Every reflection at the same spacing falls in one powder line: symmetry equivalents, and
    // forms that are not equivalent but share d (trigonal {101} and {011}, cubic {333} and {511}).
    // The line is labelled by its strongest family; `f2` is the mean |F|² of its reflections.
    all.sort_by(|a, b| b.1.total_cmp(&a.1).then(b.2.total_cmp(&a.2)));
    let mut out: Vec<Line> = vec![];
    let mut best: Vec<f64> = vec![];
    for (h, d, f2) in all {
        match out.last_mut().filter(|q| (q.d - d).abs() < 1e-6 * d) {
            Some(q) => {
                let n = best.len() - 1;
                q.f2 = (q.f2 * q.mult as f64 + f2) / (q.mult + 1) as f64;
                q.mult += 1;
                if f2 > best[n] * (1.0 + 1e-5) || ((f2 - best[n]).abs() <= 1e-5 * best[n] && better(h, q.hkl)) { q.hkl = h; best[n] = f2.max(best[n]) }
            }
            None => { let th = (lambda / 2.0 / d).asin(); out.push(Line { hkl: h, mult: 1, d, two_theta: 2.0 * th.to_degrees(), f2, intensity: 0.0 }); best.push(f2) }
        }
    }
    for q in &mut out {
        let th = (q.two_theta / 2.0).to_radians();
        let lp = (1.0 + cm * (2.0 * th).cos().powi(2)) / (th.sin().powi(2) * th.cos()) / (1.0 + cm);
        q.intensity = q.mult as f64 * q.f2 * lp;
    }
    let top = out.iter().map(|q| q.intensity).fold(0.0, f64::max).max(f64::MIN_POSITIVE);
    for q in &mut out { q.intensity = q.intensity / top * 100.0 }
    out.retain(|q| q.intensity >= 0.01);
    Ok(out)
}
/// The label for a family: all indices positive and in descending order where possible.
fn better(a: [i32; 3], b: [i32; 3]) -> bool { let key = |h: [i32; 3]| (h.iter().filter(|&&x| x >= 0).count(), h[0], h[1], h[2]); key(a) > key(b) }

/// A calculated profile over 2θ points: pseudo-Voigt lines of width `fwhm` (degrees), with the Kα2
/// doublet when `l2` > 0 (half the Kα1 height).
pub fn profile(lines: &[Line], x: &[f64], fwhm: f64, l1: f64, l2: f64) -> Vec<f64> {
    let mut y = vec![0.0; x.len()];
    for q in lines {
        let mut add = |c: f64, h: f64| {
            for (i, &t) in x.iter().enumerate() { if (t - c).abs() < 12.0 * fwhm { y[i] += crate::xrd::pv(t, c, h, fwhm, 0.5) } }
        };
        add(q.two_theta, q.intensity);
        if l2 > l1 { let s = l2 / l1 * (q.two_theta / 2.0).to_radians().sin(); if s < 1.0 { add(2.0 * s.asin().to_degrees(), q.intensity / 2.0) } }
    }
    y
}

#[cfg(test)]
mod t {
    use super::*;
    /// Rutile, TiO₂ (P4₂/mnm): the textbook positions, written as a CIF with its 16 operations.
    pub const RUTILE: &str = "data_rutile\n_chemical_name_mineral Rutile\n_chemical_formula_sum 'O2 Ti'\n_cell_length_a 4.5937(3)\n_cell_length_b 4.5937(3)\n_cell_length_c 2.9587(2)\n_cell_angle_alpha 90\n_cell_angle_beta 90\n_cell_angle_gamma 90\n_symmetry_space_group_name_H-M 'P 42/m n m'\nloop_\n_symmetry_equiv_pos_as_xyz\nx,y,z\n-x,-y,z\n-y+1/2,x+1/2,z+1/2\ny+1/2,-x+1/2,z+1/2\n-x+1/2,y+1/2,-z+1/2\nx+1/2,-y+1/2,-z+1/2\ny,x,-z\n-y,-x,-z\n-x,-y,-z\nx,y,-z\ny+1/2,-x+1/2,-z+1/2\n-y+1/2,x+1/2,-z+1/2\nx+1/2,-y+1/2,z+1/2\n-x+1/2,y+1/2,z+1/2\n-y,-x,z\ny,x,z\nloop_\n_atom_site_label\n_atom_site_type_symbol\n_atom_site_fract_x\n_atom_site_fract_y\n_atom_site_fract_z\n_atom_site_U_iso_or_equiv\nTi1 Ti4+ 0 0 0 0.006\nO1 O2- 0.30478(6) 0.30478(6) 0 0.007\n";
    #[test]
    fn scattering_factors() {
        // Iron at s = 0.3 Å⁻¹: 16.729 in the photon library (Hubbell et al. 1975 give 16.729); at 0, Z.
        assert!((scattering(26, 0.3, "").unwrap().0 - 16.729).abs() < 1e-3);
        assert!((scattering(26, 0.0, "").unwrap().0 - 26.0).abs() < 1e-9);
        // Between grid points, smooth; with Cu Kα the anomalous terms are added.
        let a = scattering(26, 0.31, "").unwrap().0;
        assert!(a < 16.729 && a > 13.809);
        let (r, i) = scattering(26, 0.3, "Cu").unwrap();
        assert!(r < 16.729 && i > 3.0);
        assert!(scattering(120, 0.1, "").is_none());
    }
    #[test]
    fn reads_cif() {
        assert_eq!(symop("-y+1/2, x+1/2, z+1/2").unwrap(), ([[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], [0.5, 0.5, 0.5]));
        assert_eq!(symop("x-y,x,z+1/6").unwrap().0[0], [1.0, -1.0, 0.0]);
        assert!(symop("x,y").is_err());
        let p = parse(RUTILE, "rutile.cif").unwrap();
        assert_eq!((p.name.as_str(), p.ops.len(), p.sites.len(), p.space_group.as_str()), ("Rutile", 16, 2, "P 42/m n m"));
        assert_eq!(p.sites[1].element, "O");
        assert!((p.sites[1].b - 8.0 * std::f64::consts::PI.powi(2) * 0.007).abs() < 1e-9);
        let atoms = unit_cell(&p);
        assert_eq!(atoms.iter().filter(|a| a.0 == "Ti").count(), 2);
        assert_eq!(atoms.iter().filter(|a| a.0 == "O").count(), 4);
        let inf = info(&p);
        assert!((inf.density - 4250.0).abs() < 30.0 && inf.z_formula == Some(2.0), "{inf:?}");
        // Quotes, comments and a text field.
        let q = "data_x\n# comment\n_publ_section_title\n;\nA long\ntitle\n;\n_cell_length_a 3\n_cell_length_b 3\n_cell_length_c 3\nloop_\n_atom_site_label\n_atom_site_fract_x\n_atom_site_fract_y\n_atom_site_fract_z\n'Fe 1' 0 0 0\n";
        let p = parse(q, "x.cif").unwrap();
        assert_eq!((p.title.as_str(), p.sites[0].element.as_str(), p.notes.len()), ("A long title", "Fe", 2));
        assert!(parse("data_y\n_cell_length_a 3\n", "y.cif").is_err());
        assert!(parse("hello", "z.cif").is_err());
    }
    #[test]
    fn kept_and_overlaid() {
        let mut r = Research::default();
        r.cif = Some(add(&r, "rutile.cif", RUTILE).unwrap());
        r.cif = Some(add(&r, "rock salt.cif", &example()).unwrap());
        assert!(add(&r, "x.cif", "nothing").is_err());
        let ex = phase(&r, 1).unwrap();
        assert_eq!(unit_cell(&ex).len(), 8);
        let c = calculate(&r, 1, None, &[], &PatAsk::default()).unwrap();
        // Rock salt: (200) strongest; (111) weak (Na and Cl scatter against each other); (100) absent.
        let lines: Vec<Line> = pattern(&ex, 1.5405929, "Cu", 90.0, 0.0).unwrap();
        let top = lines.iter().max_by(|a, b| a.intensity.total_cmp(&b.intensity)).unwrap();
        assert_eq!((top.hkl, top.mult), ([2, 0, 0], 6));
        assert!(lines.iter().find(|q| q.hkl == [1, 1, 1]).unwrap().intensity < 30.0 && !lines.iter().any(|q| q.hkl == [1, 0, 0]));
        assert!(c["profile"]["x"].as_array().unwrap().len() > 100);
        // Over a measured pattern.
        r.xrd = Some(crate::xrd::add(&r, crate::xrd::example().unwrap()).unwrap());
        let o = calculate(&r, 0, Some(0), &[0, 1], &PatAsk::default()).unwrap();
        assert_eq!(o["over"]["series"].as_array().unwrap().len(), 2);
        r.cif = remove(&r, 0).unwrap();
        assert_eq!(kept(&r).unwrap().len(), 1);
        assert_eq!(r.cif.as_ref().unwrap().info[0]["name"], serde_json::json!("Rock salt (example)"));
    }
    #[test]
    fn rutile_pattern() {
        let p = parse(RUTILE, "rutile.cif").unwrap();
        let lines = pattern(&p, 1.5405929, "Cu", 90.0, 0.0).unwrap();
        // The strongest line of rutile is (110) at 2θ ≈ 27.4° with Cu Kα1; (101) near 36.1°, (211) near 54.3°.
        let top = lines.iter().max_by(|a, b| a.intensity.total_cmp(&b.intensity)).unwrap();
        assert_eq!(top.hkl, [1, 1, 0]);
        assert!((top.two_theta - 27.44).abs() < 0.05 && top.mult == 4, "{top:?}");
        let at = |t: f64| lines.iter().find(|q| (q.two_theta - t).abs() < 0.15).map(|q| q.intensity).unwrap_or(0.0);
        assert!(at(36.08) > 30.0 && at(54.32) > 30.0, "{} {}", at(36.08), at(54.32));
        // (100) is forbidden by the 4₂ screw and n glide (h0l: h + l odd, 00l: l odd).
        assert!(!lines.iter().any(|q| q.hkl == [1, 0, 0] || q.hkl == [0, 0, 1]));
        // Profile with Kα2.
        let x: Vec<f64> = (0..400).map(|i| 25.0 + i as f64 * 0.02).collect();
        let y = profile(&lines, &x, 0.1, 1.5405929, 1.5444274);
        let i0 = ((27.44 - 25.0) / 0.02) as usize;
        assert!(y[i0] > 90.0 && y[i0 + 4] > 30.0);
    }
}

#[cfg(test)]
mod cod_files {
    use super::*;
    /// Real structures from the Crystallography Open Database (public domain), in tests_ui/cod:
    /// each strongest line with Cu Kα1 where the textbooks put it.
    #[test]
    fn strongest_lines_of_known_phases() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests_ui/cod/");
        let cases = [("1000017", "Al2 O3", 35.15, [1, 0, 4]), ("2300370", "O2 Si", 26.64, [1, 0, 1]), ("1530150", "O2 Ti", 27.45, [1, 1, 0]), ("1530151", "O2 Ti", 25.3, [1, 0, 1]),
            ("1011240", "Fe2 O3", 33.15, [1, 0, 4]), ("1010369", "Fe3 O4", 35.45, [3, 1, 1]), ("1547350", "C Ca O3", 29.4, [1, 0, 4]), ("7132177", "Cl Na", 31.7, [2, 0, 0]),
            ("1000053", "Mg O", 42.9, [2, 0, 0]), ("1000043", "Ca F2", 28.27, [1, 1, 1]), ("2300112", "O Zn", 36.25, [1, 0, 1]), ("9011998", "Si", 28.44, [1, 1, 1])];
        for (id, formula, tt, _) in cases {
            let p = parse(&std::fs::read_to_string(format!("{dir}{id}.cif")).unwrap(), id).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(p.formula, formula, "{id}");
            let ls = pattern(&p, 1.5405929, "Cu", 90.0, 0.0).unwrap();
            // The expected line is the strongest, or within 15 % of it (fluorite's (111) and (220) are nearly equal in theory).
            let near = ls.iter().find(|q| (q.two_theta - tt).abs() < 0.15).unwrap_or_else(|| panic!("{id} {formula}: no line at {tt}°"));
            assert!(near.intensity > 85.0, "{id} {formula}: the line at {tt}° has {:.1} % of the strongest", near.intensity);
        }
    }
}
