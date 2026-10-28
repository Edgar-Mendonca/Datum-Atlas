//! Formulas of material properties, in spreadsheet style: `[E]^0.5/[rho]`, `[Yield strength]^2/[E]`.
//! A property is named in brackets by its key, name or symbol. Values are ranges, so a formula is
//! worked out on ranges (interval arithmetic): the result holds every combination of the inputs.
//! A missing input, a division by a range that holds zero, or a root of a negative number gives no
//! value (a gap), never zero. The parser follows Datum Plan's formula engine.
use crate::model::{prop, PROPS};

#[derive(Clone, Debug, PartialEq)]
pub enum Ex { Num(f64), Prop(String), Neg(Box<Ex>), Bin(u8, Box<Ex>, Box<Ex>), Call(String, Vec<Ex>) }

pub const FUNCS: [&str; 7] = ["sqrt", "ln", "log10", "exp", "abs", "min", "max"];

struct P<'a> { s: &'a [u8], i: usize, depth: usize }

impl P<'_> {
    fn ws(&mut self) { while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() { self.i += 1 } }
    fn eat(&mut self, c: u8) -> bool { self.ws(); if self.s.get(self.i) == Some(&c) { self.i += 1; true } else { false } }
    fn bin(&mut self, next: fn(&mut Self) -> Result<Ex, String>, ops: &[u8]) -> Result<Ex, String> {
        let mut l = next(self)?;
        'outer: loop {
            for &op in ops {
                if self.eat(op) { l = Ex::Bin(op, Box::new(l), Box::new(next(self)?)); continue 'outer; }
            }
            return Ok(l);
        }
    }
    fn add(&mut self) -> Result<Ex, String> {
        self.depth += 1;
        if self.depth > 200 { return Err("the formula is nested too deeply".into()); }
        let r = self.bin(Self::mul, b"+-");
        self.depth -= 1;
        r
    }
    fn mul(&mut self) -> Result<Ex, String> { self.bin(Self::neg, b"*/") }
    fn neg(&mut self) -> Result<Ex, String> { if self.eat(b'-') { Ok(Ex::Neg(Box::new(self.neg()?))) } else { self.eat(b'+'); self.pow() } }
    fn pow(&mut self) -> Result<Ex, String> {
        let b = self.atom()?;
        if self.eat(b'^') { Ok(Ex::Bin(b'^', Box::new(b), Box::new(self.neg()?))) } else { Ok(b) }
    }
    fn atom(&mut self) -> Result<Ex, String> {
        self.ws();
        let Some(&c) = self.s.get(self.i) else { return Err("the formula ends too early".into()) };
        match c {
            b'(' => { self.i += 1; let e = self.add()?; if !self.eat(b')') { return Err("a ) is missing".into()) } Ok(e) }
            b'[' => {
                let e = self.s[self.i + 1..].iter().position(|&x| x == b']').map(|k| self.i + 1 + k).ok_or("a ] is missing")?;
                let name = String::from_utf8_lossy(&self.s[self.i + 1..e]).trim().to_string();
                self.i = e + 1;
                Ok(Ex::Prop(name))
            }
            b'0'..=b'9' | b'.' => {
                let st = self.i;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') { self.i += 1 }
                if self.i < self.s.len() && matches!(self.s[self.i], b'e' | b'E') {
                    let save = self.i;
                    self.i += 1;
                    if self.i < self.s.len() && matches!(self.s[self.i], b'+' | b'-') { self.i += 1 }
                    if self.i < self.s.len() && self.s[self.i].is_ascii_digit() { while self.i < self.s.len() && self.s[self.i].is_ascii_digit() { self.i += 1 } } else { self.i = save }
                }
                let t = String::from_utf8_lossy(&self.s[st..self.i]);
                t.parse().map(Ex::Num).map_err(|_| format!("{t} is not a number"))
            }
            c if c.is_ascii_alphabetic() => {
                let st = self.i;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'_') { self.i += 1 }
                let name = String::from_utf8_lossy(&self.s[st..self.i]).to_lowercase();
                if name == "pi" { return Ok(Ex::Num(std::f64::consts::PI)); }
                if !FUNCS.contains(&name.as_str()) { return Err(format!("{name} is not a known function (use {})", FUNCS.join(", "))); }
                if !self.eat(b'(') { return Err(format!("{name} needs ( )")); }
                let mut args = vec![self.add()?];
                while self.eat(b',') { args.push(self.add()?) }
                if !self.eat(b')') { return Err(format!("{name}( needs commas between values and a closing )")); }
                let want = if matches!(name.as_str(), "min" | "max") { 2 } else { 1 };
                if args.len() != want { return Err(format!("{name} takes {want} value{}", if want > 1 { "s" } else { "" })); }
                Ok(Ex::Call(name, args))
            }
            c => Err(format!("unexpected “{}”", c as char)),
        }
    }
}

/// Parses a formula and finds the property each bracket names. `extra` are other formulas' keys
/// and names (`f:…`) that may be used too.
pub fn parse(text: &str, extra: &[(String, String)]) -> Result<Ex, String> {
    let mut p = P { s: text.as_bytes(), i: 0, depth: 0 };
    let e = p.add()?;
    p.ws();
    if p.i < p.s.len() { return Err(format!("unexpected “{}”", String::from_utf8_lossy(&p.s[p.i..]).chars().next().unwrap_or(' '))); }
    resolve(e, extra)
}

/// The key a bracket names: a key, a name or a symbol (any case), or another formula's name.
pub fn key_of(name: &str, extra: &[(String, String)]) -> Option<String> {
    let n = name.trim().to_lowercase();
    PROPS.iter().filter(|p| p.kind == "range").find(|p| p.key.to_lowercase() == n || p.name.to_lowercase() == n || (!p.sym.is_empty() && p.sym.to_lowercase() == n)).map(|p| p.key.to_string())
        .or_else(|| extra.iter().find(|(k, nm)| k.to_lowercase() == n || nm.to_lowercase() == n).map(|(k, _)| k.clone()))
}

fn resolve(e: Ex, extra: &[(String, String)]) -> Result<Ex, String> {
    Ok(match e {
        Ex::Prop(n) => Ex::Prop(key_of(&n, extra).ok_or_else(|| format!("[{n}] is not a property with values (try [E], [rho], [Yield strength])"))?),
        Ex::Neg(a) => Ex::Neg(Box::new(resolve(*a, extra)?)),
        Ex::Bin(o, a, b) => Ex::Bin(o, Box::new(resolve(*a, extra)?), Box::new(resolve(*b, extra)?)),
        Ex::Call(f, args) => Ex::Call(f, args.into_iter().map(|a| resolve(a, extra)).collect::<Result<_, _>>()?),
        x => x,
    })
}

/// The properties a formula reads.
pub fn refs(e: &Ex, out: &mut Vec<String>) {
    match e {
        Ex::Prop(k) => if !out.contains(k) { out.push(k.clone()) },
        Ex::Neg(a) => refs(a, out),
        Ex::Bin(_, a, b) => { refs(a, out); refs(b, out) }
        Ex::Call(_, args) => args.iter().for_each(|a| refs(a, out)),
        Ex::Num(_) => {}
    }
}

type R = (f64, f64);
fn ok(r: R) -> Option<R> { (r.0.is_finite() && r.1.is_finite() && r.0 <= r.1).then_some(r) }
fn mul(a: R, b: R) -> Option<R> { let v = [a.0 * b.0, a.0 * b.1, a.1 * b.0, a.1 * b.1]; ok((v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max))) }

/// a^b on ranges: a positive base takes any exponent; a negative base only a whole constant one.
fn pow(a: R, b: R) -> Option<R> {
    if a.0 > 0.0 {
        let v = [a.0.powf(b.0), a.0.powf(b.1), a.1.powf(b.0), a.1.powf(b.1)];
        return ok((v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max)));
    }
    if b.0 != b.1 || b.0.fract() != 0.0 { return None; }
    let k = b.0 as i32;
    if k < 0 && a.0 <= 0.0 && a.1 >= 0.0 { return None; }
    let (x, y) = (a.0.powi(k), a.1.powi(k));
    let (lo, hi) = (x.min(y), x.max(y));
    ok(if k % 2 == 0 && k > 0 && a.0 < 0.0 && a.1 > 0.0 { (0.0, hi) } else { (lo, hi) })
}

/// The range of a formula for one material; `get` gives a property's range. None is a gap.
pub fn eval(e: &Ex, get: &dyn Fn(&str) -> Option<R>) -> Option<R> {
    match e {
        Ex::Num(n) => ok((*n, *n)),
        Ex::Prop(k) => get(k).and_then(ok),
        Ex::Neg(a) => eval(a, get).map(|(lo, hi)| (-hi, -lo)),
        Ex::Bin(op, a, b) => {
            let (a, b) = (eval(a, get)?, eval(b, get)?);
            match op {
                b'+' => ok((a.0 + b.0, a.1 + b.1)),
                b'-' => ok((a.0 - b.1, a.1 - b.0)),
                b'*' => mul(a, b),
                b'/' => if b.0 > 0.0 || b.1 < 0.0 { mul(a, (1.0 / b.1, 1.0 / b.0)) } else { None },
                _ => pow(a, b),
            }
        }
        Ex::Call(f, args) => {
            let a = eval(&args[0], get)?;
            match f.as_str() {
                "sqrt" => (a.0 >= 0.0).then(|| (a.0.sqrt(), a.1.sqrt())),
                "ln" => (a.0 > 0.0).then(|| (a.0.ln(), a.1.ln())),
                "log10" => (a.0 > 0.0).then(|| (a.0.log10(), a.1.log10())),
                "exp" => ok((a.0.exp(), a.1.exp())),
                "abs" => Some(if a.0 >= 0.0 { a } else if a.1 <= 0.0 { (-a.1, -a.0) } else { (0.0, a.1.max(-a.0)) }),
                "min" => { let b = eval(&args[1], get)?; Some((a.0.min(b.0), a.1.min(b.1))) }
                _ => { let b = eval(&args[1], get)?; Some((a.0.max(b.0), a.1.max(b.1))) }
            }.and_then(ok)
        }
    }
}

/// A formula written back with property symbols, for labels: `E^0.5/ρ`.
pub fn show(e: &Ex) -> String {
    let sym = |k: &str| prop(k).map_or_else(|| k.trim_start_matches("f:").to_string(), |p| if p.sym.is_empty() { p.name.to_string() } else { p.sym.to_string() });
    let wrap = |e: &Ex| if matches!(e, Ex::Bin(b'+' | b'-', ..)) { format!("({})", show(e)) } else { show(e) };
    match e {
        Ex::Num(n) => crate::units::fmt(*n),
        Ex::Prop(k) => sym(k),
        Ex::Neg(a) => format!("−{}", wrap(a)),
        Ex::Bin(op, a, b) => match op {
            b'+' | b'-' => format!("{} {} {}", show(a), if *op == b'+' { "+" } else { "−" }, show(b)),
            b'*' => format!("{}·{}", wrap(a), wrap(b)),
            b'/' => format!("{}/{}", wrap(a), if matches!(**b, Ex::Bin(b'*' | b'/', ..)) { format!("({})", show(b)) } else { wrap(b) }),
            _ => format!("{}^{}", if matches!(**a, Ex::Bin(..)) { format!("({})", show(a)) } else { show(a) }, wrap(b)),
        },
        Ex::Call(f, args) => format!("{f}({})", args.iter().map(show).collect::<Vec<_>>().join(", ")),
    }
}
