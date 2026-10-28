//! Datum Atlas desktop shell: owns the open selection project, the materials database (built in
//! plus the user's own), files, options and recovery, and talks to the window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod update;

use datum_atlas_core::{chart, compare, db::{self, Db}, doc::{write_atomic, Doc, Op}, export, goals, select, sheet, templates, Database, Material, Project, Source, FORMAT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::{Path, PathBuf}, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Options {
    /// auto, light or dark.
    theme: String,
    /// Minutes between recovery saves; 0 turns them off.
    autosave: u32,
    /// Offer beta (pre-release) versions in Check for updates.
    beta: bool,
    /// Reopen the last project at start.
    open_last: bool,
    /// New charts: material labels and the slope 1, 2, 3 guide lines.
    labels: bool,
    guides: bool,
    /// Picture export resolution.
    dpi: u32,
    /// "si" or "imperial".
    units: String,
    /// Printing and branding.
    paper: String,
    landscape: bool,
    company: String,
    logo: String,
    /// Other databases (.datdb) opened beside the built-in data and your own materials.
    databases: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self { theme: "auto".into(), autosave: 5, beta: false, open_last: false, labels: false, guides: true, dpi: 300, units: "si".into(), paper: "A4".into(), landscape: true, company: String::new(), logo: String::new(), databases: vec![] }
    }
}

struct App {
    doc: Doc,
    opts: Options,
    rev: u64,
    /// Built-in + user materials, and the user's own database as saved.
    db: Db,
    user: Database,
    db_rev: u64,
    /// What happened to the user's database file at start (damaged and set aside, values left out).
    user_note: Option<String>,
    /// False when the file exists but could not be read or set aside: it is then never written over.
    user_ok: bool,
    /// Other databases opened (read only here), with their paths.
    others: Vec<(PathBuf, Database)>,
}

impl App {
    /// The materials the window sees: built in, yours, then the other databases.
    fn rebuild(&mut self) {
        let others: Vec<Database> = self.others.iter().map(|o| o.1.clone()).collect();
        self.db = Db::new(db::builtin(), Some(&self.user)).with_others(&others);
        self.db_rev += 1;
        self.rev += 1;
    }
}

struct St(Mutex<App>);
impl St {
    /// The open project. A command that panicked must not lock every later one out, so a poisoned lock is reused.
    fn lock(&self) -> std::sync::MutexGuard<'_, App> { self.0.lock().unwrap_or_else(|e| e.into_inner()) }
}
type R<T> = Result<T, String>;

/// What the window gets after every change: the project, each chart laid out, and the results.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Out<'a> {
    project: &'a Project,
    path: Option<String>,
    dirty: bool,
    undo: bool,
    redo: bool,
    rev: u64,
    charts: Vec<chart::ChartOut>,
    results: select::Results,
    compare: compare::Compare,
    db_rev: u64,
}

fn out(a: &App) -> Value {
    let d = &a.doc;
    // The project's formula properties are worked out into every material first.
    let db = a.db.with_formulas(&d.p.formulas);
    let charts = d.p.charts.iter().map(|c| chart::build(&db, c)).collect();
    serde_json::to_value(Out {
        project: &d.p, path: d.path.as_ref().map(|p| p.display().to_string()), dirty: d.dirty, undo: d.can_undo(), redo: d.can_redo(), rev: a.rev,
        charts, results: select::run(&db, &d.p), compare: compare::build(&db, &d.p), db_rev: a.db_rev,
    })
    .unwrap_or_default()
}

fn dir(app: &AppHandle, data: bool) -> Option<PathBuf> {
    let d = if data { app.path().app_data_dir() } else { app.path().app_config_dir() }.ok()?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

#[tauri::command]
fn state(st: State<St>) -> Value {
    out(&st.lock())
}

#[tauri::command]
fn apply(st: State<St>, op: Op) -> R<Value> {
    let mut a = st.lock();
    // A design goal needs the database to place its line.
    let op = match op { Op::Goal { chart, goal } => goals::apply(&a.db, &a.doc.p, chart, &goal)?, o => o };
    a.doc.apply(op)?;
    a.rev += 1;
    Ok(out(&a))
}

#[tauri::command]
fn history(st: State<St>, redo: bool) -> Value {
    let mut a = st.lock();
    if redo { a.doc.redo() } else { a.doc.undo() }
    a.rev += 1;
    out(&a)
}

#[tauri::command]
fn templates() -> Vec<templates::Tpl> {
    templates::list()
}

#[tauri::command]
fn goal_list() -> &'static [goals::Goal] {
    goals::GOALS
}

/// A new project from a template, with the options for new charts.
fn fresh(db: &Db, opts: &Options, template: Option<&str>) -> R<Project> {
    let mut p = templates::build(db, template).ok_or("Unknown template.")?;
    for c in &mut p.charts {
        c.labels = opts.labels;
        if !opts.guides { c.guides.clear(); }
    }
    Ok(p)
}

#[tauri::command]
fn new_project(st: State<St>, template: Option<String>) -> R<Value> {
    let mut a = st.lock();
    a.doc = Doc::new(fresh(&a.db, &a.opts, template.as_deref())?);
    a.rev += 1;
    Ok(out(&a))
}

// ---------- the materials database ----------

/// Everything the window shows about materials: the list, the sources, the property list and the family tree.
#[tauri::command]
fn database(st: State<St>) -> Value {
    let a = st.lock();
    json!({ "materials": a.db.materials, "sources": a.db.sources, "userCount": a.db.user_count, "props": db::props_out(), "tree": db::tree(&a.db.materials), "rev": a.db_rev, "userPath": "", "userNote": a.user_note })
}

fn user_file(app: &AppHandle) -> Option<PathBuf> {
    dir(app, true).map(|d| d.join("My materials.datdb"))
}

/// The user's database, a note for the window, and whether the file may be written. A damaged file
/// is renamed aside (never written over); a file that cannot be read at all is left alone and locked.
fn load_user(app: &AppHandle) -> (Database, Option<String>, bool) {
    let empty = Database { format: FORMAT, name: "My materials".into(), ..Default::default() };
    let Some(f) = user_file(app).filter(|f| f.exists()) else { return (empty, None, true) };
    let text = match std::fs::read(&f) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(e) => return (empty, Some(format!("Your materials file could not be read ({e}). It was left as it is, and adding or deleting your own materials is off until it can be read: {}", f.display())), false),
    };
    match db::read_user(&text).map_err(|e| e.trim_end_matches('.').to_string()) {
        Ok((d, notes)) if notes.is_empty() => (d, None, true),
        Ok((d, notes)) => (d, Some(format!("Some values in your materials could not be used and were left out:\n{}", notes.iter().take(8).cloned().collect::<Vec<_>>().join("\n"))), true),
        Err(e) => {
            let aside = f.with_file_name(format!("My materials (damaged {}).datdb", chrono::Local::now().format("%Y-%m-%d %H%M%S")));
            match std::fs::rename(&f, &aside) {
                Ok(()) => (empty, Some(format!("Your materials file was damaged ({e}). It was kept, unchanged, as {}. Your own materials start empty.", aside.display())), true),
                Err(r) => (empty, Some(format!("Your materials file was damaged ({e}) and could not be set aside ({r}). It is left as it is, and adding your own materials is off: {}", f.display())), false),
            }
        }
    }
}

fn save_user(app: &AppHandle, a: &mut App) -> R<()> {
    if !a.user_ok { return Err("Your materials file could not be read, so it is not written over. Close Datum Atlas, check the file, and start again.".into()); }
    let f = user_file(app).ok_or("The app's data folder is not available.")?;
    let text = serde_json::to_string_pretty(&a.user).map_err(|e| e.to_string())?;
    write_atomic(&f, text.as_bytes())?;
    a.rebuild();
    Ok(())
}

/// Adds a material to the user's database, or replaces the one with the same id. Its sources
/// (new ones the user typed) are added too; unused user sources are dropped.
#[tauri::command]
fn save_material(app: AppHandle, st: State<St>, mut material: Material, sources: Vec<Source>) -> R<Value> {
    let mut a = st.lock();
    if material.id.is_empty() || !material.id.starts_with("u-") {
        let n = (1..).find(|k| !a.user.materials.iter().any(|m| m.id == format!("u-{k}"))).unwrap_or(1);
        material.id = format!("u-{n}");
    }
    let mut all = a.user.sources.clone();
    for s in sources {
        all.retain(|x| x.id != s.id);
        all.push(Source { kind: if s.kind.is_empty() { "user".into() } else { s.kind.clone() }, ..s });
    }
    let builtin: Vec<Source> = db::builtin().sources;
    db::check_user(&material, &[all.clone(), builtin].concat())?;
    material.status = "user".into();
    if let Some(m) = a.user.materials.iter_mut().find(|m| m.id == material.id) { *m = material; } else { a.user.materials.push(material); }
    let used: std::collections::HashSet<String> = a.user.materials.iter().flat_map(|m| m.props.values().map(|v| v.src().to_string())).collect();
    all.retain(|s| used.contains(&s.id));
    a.user.sources = all;
    save_user(&app, &mut a)?;
    Ok(out(&a))
}

#[tauri::command]
fn delete_material(app: AppHandle, st: State<St>, id: String) -> R<Value> {
    let mut a = st.lock();
    let n = a.user.materials.len();
    a.user.materials.retain(|m| m.id != id);
    if n == a.user.materials.len() {
        return Err("Only your own materials can be deleted.".into());
    }
    save_user(&app, &mut a)?;
    Ok(out(&a))
}

// ---------- other databases, and spreadsheets in and out ----------

fn save_opts(app: &AppHandle, o: &Options) {
    if let Some(f) = options_file(app) { let _ = write_atomic(&f, serde_json::to_string(o).unwrap_or_default().as_bytes()); }
}

/// Your database and the other ones open: name, path, number of materials.
#[tauri::command]
fn databases(app: AppHandle, st: State<St>) -> Value {
    let a = st.lock();
    let mut v = vec![json!({ "name": "My materials", "path": user_file(&app).map(|p| p.display().to_string()), "count": a.user.materials.len(), "mine": true })];
    v.extend(a.others.iter().map(|(p, d)| json!({ "name": if d.name.is_empty() { "Database".to_string() } else { d.name.clone() }, "path": p.display().to_string(), "count": d.materials.len(), "mine": false })));
    json!(v)
}

/// Opens another .datdb beside the built-in data (read only); it stays open next time.
#[tauri::command]
async fn open_database(app: AppHandle, st: State<'_, St>, path: Option<String>) -> R<Option<Value>> {
    let Some(path) = path.map(PathBuf::from).or_else(|| pick(&app, "Datum Atlas database", &["datdb"])) else { return Ok(None) };
    let text = std::fs::read(&path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    let (mut d, _) = db::read_user(&String::from_utf8_lossy(&text))?;
    if d.name.trim().is_empty() { d.name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(); }
    let mut a = st.lock();
    if a.others.iter().any(|o| o.0 == path) { return Err("That database is already open.".into()); }
    a.others.push((path.clone(), d));
    a.opts.databases.push(path.display().to_string());
    save_opts(&app, &a.opts);
    a.rebuild();
    Ok(Some(out(&a)))
}

#[tauri::command]
fn close_database(app: AppHandle, st: State<St>, path: String) -> R<Value> {
    let mut a = st.lock();
    a.others.retain(|o| o.0.display().to_string() != path);
    a.opts.databases.retain(|p| *p != path);
    save_opts(&app, &a.opts);
    a.rebuild();
    Ok(out(&a))
}

/// Saves your own materials as a .datdb to share, or as a spreadsheet or CSV.
#[tauri::command]
async fn export_database(app: AppHandle, st: State<'_, St>, fmt: String) -> R<Option<String>> {
    let user = st.lock().user.clone();
    if user.materials.is_empty() { return Err("You have no materials of your own yet.".into()); }
    let ext = match fmt.as_str() { "csv" => "csv", "xlsx" => "xlsx", _ => "datdb" };
    let bytes = if ext == "datdb" { serde_json::to_string_pretty(&user).map_err(|e| e.to_string())?.into_bytes() } else { sheet::bytes(&sheet::of_database(&user), ext, "My materials")? };
    let Some(path) = app.dialog().file().add_filter(ext.to_uppercase(), &[ext]).set_file_name(format!("My materials.{ext}")).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    write_atomic(&path, &bytes)?;
    Ok(Some(path.display().to_string()))
}

/// Reads a spreadsheet or CSV and guesses what each column holds, for the import window.
#[tauri::command]
async fn import_preview(app: AppHandle, path: Option<String>) -> R<Option<Value>> {
    let Some(path) = path.map(PathBuf::from).or_else(|| pick(&app, "Spreadsheet or CSV", &["xlsx", "xls", "ods", "csv", "txt"])) else { return Ok(None) };
    let t = sheet::read(&path)?;
    Ok(Some(json!({ "path": path.display().to_string(), "head": t.head, "rows": t.rows.iter().take(30).collect::<Vec<_>>(), "count": t.rows.len(), "guess": t.guess })))
}

/// Adds the rows of a spreadsheet to your own materials, with the columns mapped as chosen.
#[tauri::command]
fn import_materials(app: AppHandle, st: State<St>, path: String, map: Vec<String>, family: String, source: String) -> R<Value> {
    let t = sheet::read(Path::new(&path))?;
    let (d, notes) = sheet::to_database(&t, &map, &family, &source)?;
    let mut a = st.lock();
    let n0 = a.user.materials.len();
    // Fresh ids and source ids in your database.
    let free = |used: &[String], stem: &str| (1..).map(|k| format!("{stem}{k}")).find(|id| !used.contains(id)).unwrap_or_default();
    let mut ren: Vec<(String, String)> = vec![];
    for s in d.sources {
        let used: Vec<String> = a.user.sources.iter().map(|x| x.id.clone()).collect();
        let id = free(&used, "u-s-imp");
        ren.push((s.id.clone(), id.clone()));
        a.user.sources.push(Source { id, ..s });
    }
    for mut m in d.materials {
        let used: Vec<String> = a.user.materials.iter().map(|x| x.id.clone()).collect();
        m.id = free(&used, "u-");
        for v in m.props.values_mut() {
            let to = ren.iter().find(|r| r.0 == v.src()).map(|r| r.1.clone()).unwrap_or_default();
            match v { datum_atlas_core::Val::Range(_, _, x) | datum_atlas_core::Val::RangeT(_, _, x, _) | datum_atlas_core::Val::Flag(_, x) | datum_atlas_core::Val::Rating(_, x) => *x = to }
        }
        a.user.materials.push(m);
    }
    let added = a.user.materials.len() - n0;
    save_user(&app, &mut a)?;
    Ok(json!({ "out": out(&a), "added": added, "notes": notes }))
}

fn pick(app: &AppHandle, name: &str, exts: &[&str]) -> Option<PathBuf> {
    app.dialog().file().add_filter(name, exts).blocking_pick_file().and_then(|f| f.into_path().ok())
}

#[tauri::command]
async fn open(app: AppHandle, st: State<'_, St>, path: Option<String>) -> R<Option<Value>> {
    let Some(path) = path.map(PathBuf::from).or_else(|| pick(&app, "Datum Atlas project", &["datlas"])) else { return Ok(None) };
    let d = Doc::open(&path)?;
    remember(&app, &path);
    let mut a = st.lock();
    a.doc = d;
    a.rev += 1;
    Ok(Some(out(&a)))
}

#[tauri::command]
async fn save(app: AppHandle, st: State<'_, St>, as_new: bool) -> R<Option<Value>> {
    let (known, name) = {
        let a = st.lock();
        (a.doc.path.clone().filter(|_| !as_new), a.doc.p.name.clone())
    };
    let path = match known {
        Some(p) => p,
        None => match app.dialog().file().add_filter("Datum Atlas project", &["datlas"]).set_file_name(format!("{name}.datlas")).blocking_save_file() {
            Some(f) => f.into_path().map_err(|e| e.to_string())?,
            None => return Ok(None),
        },
    };
    let mut a = st.lock();
    if a.doc.path.is_none() || as_new {
        if let Some(stem) = path.file_stem() {
            a.doc.p.name = stem.to_string_lossy().into_owned();
        }
    }
    a.doc.save(&path)?;
    remember(&app, &path);
    clear_recovery(&app);
    a.rev += 1;
    Ok(Some(out(&a)))
}

/// Saves the ranked results with their reasons as a spreadsheet (.xlsx) or CSV.
#[tauri::command]
async fn export_results(app: AppHandle, st: State<'_, St>, fmt: String) -> R<Option<String>> {
    let (name, bytes) = {
        let a = st.lock();
        let db = a.db.with_formulas(&a.doc.p.formulas);
        let t = export::results(&db, &a.doc.p);
        let bytes = if fmt == "csv" { export::csv(&t).into_bytes() } else { export::xlsx(&t, &a.doc.p.name)? };
        (format!("{} results.{}", a.doc.p.name, if fmt == "csv" { "csv" } else { "xlsx" }), bytes)
    };
    let ext = if fmt == "csv" { "csv" } else { "xlsx" };
    let Some(path) = app.dialog().file().add_filter(if fmt == "csv" { "CSV" } else { "Spreadsheet" }, &[ext]).set_file_name(&name).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    write_atomic(&path, &bytes)?;
    Ok(Some(path.display().to_string()))
}

/// Saves a picture or a page the window made (SVG text, or a PNG data URL).
#[tauri::command]
async fn save_file(app: AppHandle, name: String, data: String) -> R<Option<String>> {
    let ext = name.rsplit('.').next().unwrap_or("txt").to_string();
    let Some(path) = app.dialog().file().add_filter(ext.to_uppercase(), &[&ext]).set_file_name(&name).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    let bytes = match data.strip_prefix("data:").and_then(|d| d.split_once(";base64,")) {
        Some((_, b64)) => base64_decode(b64)?,
        None => data.into_bytes(),
    };
    write_atomic(&path, &bytes)?;
    Ok(Some(path.display().to_string()))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for ch in bytes.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            s.push(if k <= ch.len() { B64[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
        }
    }
    s
}

fn base64_decode(s: &str) -> R<Vec<u8>> {
    let val = |c: u8| match c { b'A'..=b'Z' => Some(c - b'A'), b'a'..=b'z' => Some(c - b'a' + 26), b'0'..=b'9' => Some(c - b'0' + 52), b'+' => Some(62), b'/' => Some(63), _ => None };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        acc = acc << 6 | val(c).ok_or("The picture data is damaged.")? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Reads a picture for page headers and returns it as a data URL.
#[tauri::command]
async fn pick_logo(app: AppHandle) -> R<Option<String>> {
    let Some(path) = pick(&app, "Pictures", &["png", "jpg", "jpeg", "svg", "gif"]) else { return Ok(None) };
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.len() > 1_500_000 {
        return Err("Choose a picture smaller than 1.5 MB.".into());
    }
    let mime = match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() { "svg" => "image/svg+xml", "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif", _ => "image/png" };
    Ok(Some(format!("data:{mime};base64,{}", base64(&bytes))))
}

/// A chart picture (PNG data URL) to the Windows clipboard, for Word and PowerPoint.
#[tauri::command]
fn copy_png(app: AppHandle, data: String) -> R<()> {
    let b64 = data.split_once(";base64,").map_or(data.as_str(), |x| x.1);
    let img = tauri::image::Image::from_bytes(&base64_decode(b64)?).map_err(|e| format!("Could not read the picture: {e}"))?;
    app.clipboard().write_image(&img).map_err(|e| format!("Could not copy the picture: {e}"))
}

/// Text to the Windows clipboard through Rust (WebView2 blocks the browser clipboard).
#[tauri::command]
fn copy_text(app: AppHandle, text: String) -> R<()> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

// ---------- options, recovery, recent files ----------

fn options_file(app: &AppHandle) -> Option<PathBuf> {
    dir(app, false).map(|d| d.join("options.json"))
}

#[tauri::command]
fn options(st: State<St>) -> Options {
    st.lock().opts.clone()
}

#[tauri::command]
fn set_options(app: AppHandle, st: State<St>, mut opts: Options) -> Value {
    let mut a = st.lock();
    // The list of open databases is kept by the shell; the window's options do not carry it.
    opts.databases = a.opts.databases.clone();
    save_opts(&app, &opts);
    a.opts = opts;
    out(&a)
}

#[derive(Serialize, Deserialize)]
struct Recovery { name: String, path: Option<String>, saved: String }

fn recovery_files(app: &AppHandle) -> Option<(PathBuf, PathBuf)> {
    dir(app, true).map(|d| (d.join("recovery.datlas"), d.join("recovery.json")))
}

fn clear_recovery(app: &AppHandle) {
    if let Some((a, b)) = recovery_files(app) {
        let _ = std::fs::remove_file(a);
        let _ = std::fs::remove_file(b);
    }
}

#[tauri::command]
fn autosave(app: AppHandle, st: State<St>) {
    let a = st.lock();
    if let (true, Some((file, meta))) = (a.doc.dirty, recovery_files(&app)) {
        let info = Recovery { name: a.doc.p.name.clone(), path: a.doc.path.as_ref().map(|p| p.display().to_string()), saved: chrono::Local::now().format("%d-%b-%y %H:%M").to_string() };
        let _ = write_atomic(&file, a.doc.json().as_bytes());
        let _ = write_atomic(&meta, serde_json::to_string(&info).unwrap_or_default().as_bytes());
    }
}

#[tauri::command]
fn recovery(app: AppHandle) -> Option<Recovery> {
    let (file, meta) = recovery_files(&app)?;
    file.exists().then_some(())?;
    serde_json::from_str(&std::fs::read_to_string(meta).ok()?).ok()
}

#[tauri::command]
fn recover(app: AppHandle, st: State<St>, keep: bool) -> R<Option<Value>> {
    let info = recovery(app.clone());
    let files = recovery_files(&app);
    if !keep {
        clear_recovery(&app);
        return Ok(None);
    }
    let (file, _) = files.ok_or("No recovered project was found.")?;
    let mut d = Doc::open(&file)?;
    d.path = info.and_then(|i| i.path).map(PathBuf::from);
    d.dirty = true;
    let mut a = st.lock();
    a.doc = d;
    a.rev += 1;
    Ok(Some(out(&a)))
}

#[derive(Serialize)]
struct Recent { path: String, name: String, modified: String }

fn recent_paths(app: &AppHandle) -> Vec<String> {
    options_file(app).map(|f| f.with_file_name("recent.json")).and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn remember(app: &AppHandle, path: &Path) {
    let p = path.display().to_string();
    let mut v = recent_paths(app);
    v.retain(|x| *x != p);
    v.insert(0, p);
    v.truncate(15);
    if let Some(f) = options_file(app).map(|f| f.with_file_name("recent.json")) {
        let _ = write_atomic(&f, serde_json::to_string(&v).unwrap_or_default().as_bytes());
    }
}

#[tauri::command]
fn recent(app: AppHandle) -> Vec<Recent> {
    recent_paths(&app)
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p).ok()?.modified().ok()?;
            let modified = chrono::DateTime::<chrono::Local>::from(m).format("%d-%b-%y %H:%M").to_string();
            let name = Path::new(&p).file_stem()?.to_string_lossy().into_owned();
            Some(Recent { path: p, name, modified })
        })
        .collect()
}

/// A .datlas path passed on the command line (double-clicked in Explorer), or the last project.
#[tauri::command]
fn startup_file(app: AppHandle, st: State<St>) -> Option<String> {
    let last = || if st.lock().opts.open_last { recent_paths(&app).into_iter().find(|p| Path::new(p).exists()) } else { None };
    std::env::args().skip(1).find(|a| a.to_lowercase().ends_with(".datlas")).or_else(last)
}

/// Opens one of the project's GitHub pages in the browser (only those).
#[tauri::command]
fn open_url(url: String) -> R<()> {
    if !url.starts_with("https://github.com/Edgar-Mendonca/Datum-Atlas") {
        return Err("Only the Datum Atlas GitHub pages can be opened.".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&url).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
fn quit(app: AppHandle) {
    clear_recovery(&app);
    app.exit(0);
}

#[tauri::command]
async fn check_update(st: State<'_, St>) -> R<update::Info> {
    let beta = st.lock().opts.beta;
    tauri::async_runtime::spawn_blocking(move || update::check(beta)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn install_update(app: AppHandle, url: String, sums: Option<String>) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || update::install(&app, &url, sums.as_deref())).await.map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let opts: Options = options_file(app.handle()).and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let (user, mut user_note, user_ok) = load_user(app.handle());
            // The other databases opened last time; one that cannot be read is closed and reported.
            let mut others = vec![];
            for p in &opts.databases {
                match std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| db::read_user(&String::from_utf8_lossy(&b))) {
                    Ok((d, _)) => others.push((PathBuf::from(p), d)),
                    Err(e) => user_note = Some(format!("{}The database {p} could not be opened ({e}); it was closed.", user_note.map(|n| format!("{n}\n")).unwrap_or_default())),
                }
            }
            let mut opts = opts;
            opts.databases = others.iter().map(|o: &(PathBuf, Database)| o.0.display().to_string()).collect();
            let all: Vec<Database> = others.iter().map(|o| o.1.clone()).collect();
            let db = Db::new(db::builtin(), Some(&user)).with_others(&all);
            let p = fresh(&db, &opts, None).unwrap_or_default();
            app.manage(St(Mutex::new(App { doc: Doc::new(p), opts, rev: 0, db, user, db_rev: 1, user_note, user_ok, others })));
            Ok(())
        })
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                if w.state::<St>().lock().doc.dirty {
                    api.prevent_close();
                    let _ = w.emit("close-requested", ());
                } else {
                    clear_recovery(w.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            state, apply, history, new_project, templates, database, save_material, delete_material, open, save, save_file, export_results, databases, open_database, close_database, export_database, import_preview, import_materials, pick_logo, copy_text, copy_png, goal_list,
            options, set_options, autosave, recovery, recover, recent, startup_file, version, open_url, quit, check_update, install_update
        ])
        .run(tauri::generate_context!())
        .expect("Datum Atlas failed to start");
}
