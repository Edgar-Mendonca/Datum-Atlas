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
    /// Moves the chart's selection line, sets a box, or removes it (None).
    Select { chart: u32, select: Option<ChartSel> },
    SetStages { stages: Stages },
    Name { name: String },
    Notes { text: String },
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
        std::fs::write(path, self.json()).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
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
        Op::Select { chart, select } => {
            if let Some(ChartSel::Line { slope, x, y, .. }) = &select {
                if !(slope.is_finite() && x.is_finite() && y.is_finite()) {
                    return Err("Type a number for the slope.".into());
                }
            }
            p.charts.iter_mut().find(|c| c.id == chart).ok_or("That chart no longer exists.")?.select = select;
        }
        Op::SetStages { stages } => {
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
