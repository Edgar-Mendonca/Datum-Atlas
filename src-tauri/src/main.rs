//! Datum Atlas desktop shell: owns the open selection project, the materials database (built in
//! plus the user's own), files, options and recovery, and talks to the window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod update;

use datum_atlas_core::{chart, db::{self, Db}, doc::{Doc, Op}, goals, select, templates, Database, Material, Project, Source, FORMAT};
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
}

impl Default for Options {
    fn default() -> Self {
        Self { theme: "auto".into(), autosave: 5, beta: false, open_last: false, labels: false, guides: true, dpi: 300, units: "si".into(), paper: "A4".into(), landscape: true, company: String::new(), logo: String::new() }
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
}

struct St(Mutex<App>);
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
    db_rev: u64,
}

fn out(a: &App) -> Value {
    let d = &a.doc;
    let charts = d.p.charts.iter().map(|c| chart::build(&a.db, c)).collect();
    serde_json::to_value(Out {
        project: &d.p, path: d.path.as_ref().map(|p| p.display().to_string()), dirty: d.dirty, undo: d.can_undo(), redo: d.can_redo(), rev: a.rev,
        charts, results: select::run(&a.db, &d.p), db_rev: a.db_rev,
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
    out(&st.0.lock().unwrap())
}

#[tauri::command]
fn apply(st: State<St>, op: Op) -> R<Value> {
    let mut a = st.0.lock().unwrap();
    // A design goal needs the database to place its line.
    let op = match op { Op::Goal { chart, goal } => goals::apply(&a.db, &a.doc.p, chart, &goal)?, o => o };
    a.doc.apply(op)?;
    a.rev += 1;
    Ok(out(&a))
}

#[tauri::command]
fn history(st: State<St>, redo: bool) -> Value {
    let mut a = st.0.lock().unwrap();
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
    let mut a = st.0.lock().unwrap();
    a.doc = Doc::new(fresh(&a.db, &a.opts, template.as_deref())?);
    a.rev += 1;
    Ok(out(&a))
}

// ---------- the materials database ----------

/// Everything the window shows about materials: the list, the sources, the property list and the family tree.
#[tauri::command]
fn database(st: State<St>) -> Value {
    let a = st.0.lock().unwrap();
    json!({ "materials": a.db.materials, "sources": a.db.sources, "userCount": a.db.user_count, "props": db::props_out(), "tree": db::tree(&a.db.materials), "rev": a.db_rev, "userPath": "" })
}

fn user_file(app: &AppHandle) -> Option<PathBuf> {
    dir(app, true).map(|d| d.join("My materials.datdb"))
}

fn load_user(app: &AppHandle) -> Database {
    user_file(app).and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Database { format: FORMAT, name: "My materials".into(), ..Default::default() })
}

fn save_user(app: &AppHandle, a: &mut App) -> R<()> {
    let f = user_file(app).ok_or("The app's data folder is not available.")?;
    let text = serde_json::to_string_pretty(&a.user).map_err(|e| e.to_string())?;
    std::fs::write(&f, text).map_err(|e| format!("Could not save {}: {e}", f.display()))?;
    a.db = Db::new(db::builtin(), Some(&a.user));
    a.db_rev += 1;
    a.rev += 1;
    Ok(())
}

/// Adds a material to the user's database, or replaces the one with the same id. Its sources
/// (new ones the user typed) are added too; unused user sources are dropped.
#[tauri::command]
fn save_material(app: AppHandle, st: State<St>, mut material: Material, sources: Vec<Source>) -> R<Value> {
    let mut a = st.0.lock().unwrap();
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
    let mut a = st.0.lock().unwrap();
    let n = a.user.materials.len();
    a.user.materials.retain(|m| m.id != id);
    if n == a.user.materials.len() {
        return Err("Only your own materials can be deleted.".into());
    }
    save_user(&app, &mut a)?;
    Ok(out(&a))
}

fn pick(app: &AppHandle, name: &str, exts: &[&str]) -> Option<PathBuf> {
    app.dialog().file().add_filter(name, exts).blocking_pick_file().and_then(|f| f.into_path().ok())
}

#[tauri::command]
async fn open(app: AppHandle, st: State<'_, St>, path: Option<String>) -> R<Option<Value>> {
    let Some(path) = path.map(PathBuf::from).or_else(|| pick(&app, "Datum Atlas project", &["datlas"])) else { return Ok(None) };
    let d = Doc::open(&path)?;
    remember(&app, &path);
    let mut a = st.0.lock().unwrap();
    a.doc = d;
    a.rev += 1;
    Ok(Some(out(&a)))
}

#[tauri::command]
async fn save(app: AppHandle, st: State<'_, St>, as_new: bool) -> R<Option<Value>> {
    let (known, name) = {
        let a = st.0.lock().unwrap();
        (a.doc.path.clone().filter(|_| !as_new), a.doc.p.name.clone())
    };
    let path = match known {
        Some(p) => p,
        None => match app.dialog().file().add_filter("Datum Atlas project", &["datlas"]).set_file_name(format!("{name}.datlas")).blocking_save_file() {
            Some(f) => f.into_path().map_err(|e| e.to_string())?,
            None => return Ok(None),
        },
    };
    let mut a = st.0.lock().unwrap();
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

/// Saves a picture or a page the window made (SVG text, or a PNG data URL).
#[tauri::command]
async fn save_file(app: AppHandle, name: String, data: String) -> R<Option<String>> {
    let ext = name.rsplit('.').next().unwrap_or("txt").to_string();
    let Some(path) = app.dialog().file().add_filter(ext.to_uppercase(), &[&ext]).set_file_name(&name).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    let bytes = match data.strip_prefix("data:").and_then(|d| d.split_once(";base64,")) {
        Some((_, b64)) => base64_decode(b64)?,
        None => data.into_bytes(),
    };
    std::fs::write(&path, bytes).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
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
    st.0.lock().unwrap().opts.clone()
}

#[tauri::command]
fn set_options(app: AppHandle, st: State<St>, opts: Options) -> Value {
    if let Some(f) = options_file(&app) {
        let _ = std::fs::write(f, serde_json::to_string(&opts).unwrap_or_default());
    }
    let mut a = st.0.lock().unwrap();
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
    let a = st.0.lock().unwrap();
    if let (true, Some((file, meta))) = (a.doc.dirty, recovery_files(&app)) {
        let info = Recovery { name: a.doc.p.name.clone(), path: a.doc.path.as_ref().map(|p| p.display().to_string()), saved: chrono::Local::now().format("%d-%b-%y %H:%M").to_string() };
        let _ = std::fs::write(file, a.doc.json());
        let _ = std::fs::write(meta, serde_json::to_string(&info).unwrap_or_default());
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
    let mut a = st.0.lock().unwrap();
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
        let _ = std::fs::write(f, serde_json::to_string(&v).unwrap());
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
    let last = || if st.0.lock().unwrap().opts.open_last { recent_paths(&app).into_iter().find(|p| Path::new(p).exists()) } else { None };
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
    let beta = st.0.lock().unwrap().opts.beta;
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
            let user = load_user(app.handle());
            let db = Db::new(db::builtin(), Some(&user));
            let p = fresh(&db, &opts, None).unwrap_or_default();
            app.manage(St(Mutex::new(App { doc: Doc::new(p), opts, rev: 0, db, user, db_rev: 1 })));
            Ok(())
        })
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                if w.state::<St>().0.lock().unwrap().doc.dirty {
                    api.prevent_close();
                    let _ = w.emit("close-requested", ());
                } else {
                    clear_recovery(w.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            state, apply, history, new_project, templates, database, save_material, delete_material, open, save, save_file, pick_logo, copy_text, copy_png, goal_list,
            options, set_options, autosave, recovery, recover, recent, startup_file, version, open_url, quit, check_update, install_update
        ])
        .run(tauri::generate_context!())
        .expect("Datum Atlas failed to start");
}
