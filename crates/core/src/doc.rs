//! An open selection project: edit operations, undo/redo and .datlas files.
use crate::model::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const UNDO_LIMIT: usize = 200;

#[derive(Debug, Clone, Deserialize)]
pub enum Op {
    /// Adds a chart (its id is assigned here).
    AddChart { chart: Box<Chart> },
    /// Replaces a chart's settings (axes, scales, hulls, selection…), keeping its id.
    SetChart { chart: Box<Chart> },
    DelChart { id: u32 },
    /// Replaces the chart's selections (lines, boxes, polygons); an empty list removes them.
    Select { chart: u32, sels: Vec<ChartSel> },
    /// Moves line `k` of a chart (keeping its slope) so its index has the value typed.
    PassThrough { chart: u32, k: usize, value: f64 },
    SetStages { stages: Stages },
    Name { name: String },
    Notes { text: String },
    /// Applies a design goal to a chart ("" to explore). The shell turns it into the edits.
    Goal { chart: u32, goal: String },
    /// Stores window settings under a key.
    Ui { key: String, value: serde_json::Value },
    Batch(Vec<Op>),
}

pub struct Doc {
    pub p: Project,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    undo: Vec<Project>,
    redo: Vec<Project>,
}

impl Doc {
    pub fn new(p: Project) -> Self {
        Self { p, path: None, dirty: false, undo: vec![], redo: vec![] }
    }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
    pub fn undo_len(&self) -> usize { self.undo.len() }

    /// Applies `op` to a copy and keeps it only if it worked and changed something.
    pub fn apply(&mut self, op: Op) -> Result<(), String> {
        let mut q = self.p.clone();
        exec(&mut q, op)?;
        q.normalize();
        if q == self.p {
            return Ok(());
        }
        self.undo.push(std::mem::replace(&mut self.p, q));
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
        Ok(())
    }
    pub fn undo(&mut self) {
        if let Some(p) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.p, p));
            self.dirty = true;
        }
    }
    pub fn redo(&mut self) {
        if let Some(p) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.p, p));
            self.dirty = true;
        }
    }
    pub fn open(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
        let p = Project::from_json(&text).map_err(|e| format!("This is not a Datum Atlas project ({e})."))?;
        let mut d = Self::new(p);
        d.path = Some(path.into());
        Ok(d)
    }
    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        write_atomic(path, self.json().as_bytes())?;
        self.path = Some(path.into());
        self.dirty = false;
        Ok(())
    }
    pub fn json(&self) -> String {
        serde_json::to_string_pretty(&self.p).unwrap_or_default()
    }
}

fn exec(p: &mut Project, op: Op) -> Result<(), String> {
    match op {
        Op::AddChart { chart } => {
            p.add_chart(*chart);
        }
        Op::SetChart { chart } => {
            let c = p.charts.iter_mut().find(|c| c.id == chart.id).ok_or("That chart no longer exists.")?;
            let auto = c.name == chart_name(c);
            *c = *chart;
            if auto || c.name.trim().is_empty() {
                c.name = chart_name(c);
            }
        }
        Op::DelChart { id } => {
            let n = p.charts.len();
            p.charts.retain(|c| c.id != id);
            if n == p.charts.len() {
                return Err("Nothing to delete.".into());
            }
        }
        Op::Select { chart, sels } => {
            if !sels.iter().all(ChartSel::valid) {
                return Err("Type a number for the slope; a polygon needs at least three corners.".into());
            }
            p.charts.iter_mut().find(|c| c.id == chart).ok_or("That chart no longer exists.")?.sels = sels;
        }
        Op::PassThrough { chart, k, value } => {
            let c = p.charts.iter_mut().find(|c| c.id == chart).ok_or("That chart no longer exists.")?;
            let probe = c.clone();
            let Some(ChartSel::Line { slope, x, y, .. }) = c.sels.get_mut(k) else { return Err("That selection line no longer exists.".into()) };
            *y = crate::select::y_for(&probe, *slope, value, *x).ok_or("That value cannot be on this chart; type a number above zero.")?;
        }
        Op::SetStages { stages } => {
            if stages.weights.iter().any(|w| !(w.w.is_finite() && w.w >= 0.0)) {
                return Err("A weight must be a number of zero or more.".into());
            }
            for l in &stages.limits {
                if let (Some(a), Some(b)) = (l.min, l.max) {
                    if a > b {
                        return Err(format!("The minimum for {} is above its maximum.", prop(&l.prop).map_or("this property", |p| p.name)));
                    }
                }
            }
            p.stages = stages;
        }
        Op::Name { name } => {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err("Type a name.".into());
            }
            p.name = name;
        }
        Op::Notes { text } => p.notes = text,
        Op::Goal { .. } => return Err("A goal needs the database; the shell applies it.".into()),
        Op::Ui { key, value } => {
            if !p.ui.is_object() {
                p.ui = serde_json::json!({});
            }
            p.ui[key] = value;
        }
        Op::Batch(ops) => {
            for o in ops {
                exec(p, o)?;
            }
        }
    }
    Ok(())
}

/// Writes a file through a temporary file beside it and a rename, so a crash or a full disk
/// mid-save leaves the old file whole.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension(format!("{}.tmp", path.extension().and_then(|e| e.to_str()).unwrap_or("")));
    let err = |e: std::io::Error| format!("Could not save {}: {e}", path.display());
    std::fs::write(&tmp, bytes).map_err(err)?;
    std::fs::rename(&tmp, path).map_err(|e| { let _ = std::fs::remove_file(&tmp); err(e) })
}
