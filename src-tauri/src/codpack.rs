// The phase-identification reference pack (inorganic structures of the Crystallography Open
// Database): downloaded from Atlas's own release page the first time Phase ID needs it, checked
// against its SHA-256, kept in the local app data; or imported from a file (offline computers).
use datum_atlas_core::phaseid::{self, Pack};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::Read, path::PathBuf, sync::{Arc, Mutex, OnceLock}};
use tauri::{AppHandle, Emitter, Manager};

/// The pack this version of Atlas asks for: the COD archive date it was built from.
pub const DATE: &str = "2025.02";
fn name() -> String { format!("cod-inorganic-{DATE}.pack") }
fn url(file: &str) -> String { format!("https://github.com/{}/releases/download/cod-pack-{DATE}/{file}", crate::update::REPO) }

fn slot() -> &'static Mutex<Option<Arc<Pack>>> { static S: OnceLock<Mutex<Option<Arc<Pack>>>> = OnceLock::new(); S.get_or_init(|| Mutex::new(None)) }
fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let d = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("cod");
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d.join("cod-inorganic.pack"))
}

/// The pack, loaded once from the local file.
pub fn pack(app: &AppHandle) -> Result<Arc<Pack>, String> {
    let mut s = slot().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(p) = s.as_ref() { return Ok(p.clone()) }
    let f = path(app)?;
    if !f.exists() { return Err("The reference pack is not on this computer yet: download it (or import the file) on the Phase ID page.".into()) }
    let p = Arc::new(phaseid::read(&std::fs::read(&f).map_err(|e| e.to_string())?)?);
    *s = Some(p.clone());
    Ok(p)
}

pub fn status(app: &AppHandle) -> Value {
    match pack(app) {
        Ok(p) => json!({ "present": true, "entries": p.entries.len(), "date": p.date, "want": DATE, "size": path(app).ok().and_then(|f| std::fs::metadata(f).ok()).map_or(0, |m| m.len()) }),
        Err(e) => json!({ "present": false, "want": DATE, "error": if path(app).map(|f| f.exists()).unwrap_or(false) { Some(e) } else { None } }),
    }
}

/// Keep a checked pack: read it fully first (a damaged file is refused), then write it in place.
fn keep(app: &AppHandle, bytes: &[u8]) -> Result<Value, String> {
    let p = phaseid::read(bytes)?;
    datum_atlas_core::doc::write_atomic(&path(app)?, bytes)?;
    *slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(p));
    Ok(status(app))
}

/// Download the pack and its SHA-256 from the release page; progress goes to the window as "codprogress".
pub fn download(app: &AppHandle) -> Result<Value, String> {
    let agent = crate::update::agent();
    let get = |u: &str| agent.get(u).call().map_err(|e| match e { ureq::Error::Status(404, _) => "The reference pack is not published yet; try again later, or import the file.".to_string(), e => format!("Could not download the reference pack: {e}") });
    let sums = get(&url(&format!("{}.sha256", name())))?.into_string().map_err(|e| e.to_string())?;
    let want = sums.split_whitespace().next().ok_or("The checksum file is empty.")?.to_lowercase();
    let r = get(&url(&name()))?;
    let total: u64 = r.header("Content-Length").and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut rd = r.into_reader();
    let (mut bytes, mut buf) = (Vec::with_capacity(total as usize), vec![0u8; 1 << 16]);
    let mut last = 0u64;
    loop {
        let n = rd.read(&mut buf).map_err(|e| format!("The download stopped: {e}"))?;
        if n == 0 { break }
        bytes.extend_from_slice(&buf[..n]);
        if bytes.len() as u64 - last > 1 << 20 { last = bytes.len() as u64; let _ = app.emit("codprogress", json!({ "done": last, "total": total })); }
    }
    let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    if got != want { return Err("The downloaded pack does not match its checksum; nothing was kept. Try again.".into()) }
    keep(app, &bytes)
}

/// Import a pack file chosen by the user (copied from another computer).
pub fn import(app: &AppHandle, file: &std::path::Path) -> Result<Value, String> {
    keep(app, &std::fs::read(file).map_err(|e| format!("Could not read {}: {e}", file.display()))?)
}
