//! Flux desktop shell.
//!
//! The UI in `apps/desktop/ui` runs in a native webview and calls the engine in-process through
//! one `call` command (the same `Api::call` that `flux-mcp` and the HTTP server use). The shell
//! adds what a webview can't do alone: native file dialogs, reveal-in-folder, opening links, the
//! localhost control server that lets `flux-mcp` drive this session, and background updates.
//!
//! Updates: when "Update automatically" is on, the shell checks GitHub releases shortly after
//! launch and every six hours, downloads a newer signed build in the background, tells the UI
//! (`flux://update-ready`), and installs it on "Restart now" or when Flux quits.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use flux_core::api::{error_code, Api};
use flux_core::server::{self, Server, ServerOpts};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

const FIRST_CHECK: Duration = Duration::from_secs(5);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

struct Shell {
    api: Api,
    control: Mutex<Option<Server>>,
    update: Mutex<UpdateState>,
}

#[derive(Default)]
struct UpdateState {
    /// A downloaded, signature-checked update waiting to be installed.
    ready: Option<(Update, Vec<u8>)>,
    busy: bool,
    last_checked: Option<u64>,
    error: Option<String>,
}

fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn fail(code: &str, message: impl std::fmt::Display) -> Value {
    json!({"code": code, "message": message.to_string()})
}

/// The flux-mcp binary bundled next to this executable.
fn mcp_path() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let p = exe
        .parent()?
        .join(format!("flux-mcp{}", std::env::consts::EXE_SUFFIX));
    Some(p.display().to_string())
}

/// Run the control server only while agents are allowed (Settings > Agents).
fn sync_control(shell: &Shell) {
    let want = shell.api.lock().store.settings().mcp_enabled;
    let mut c = shell.control.lock().unwrap_or_else(|e| e.into_inner());
    match (want, c.is_some()) {
        (true, false) => {
            *c = server::start(
                shell.api.clone(),
                ServerOpts {
                    port: 0,
                    token: Some(server::new_token()),
                    static_dir: None,
                    publish: true,
                },
            )
            .ok()
        }
        (false, true) => {
            if let Some(s) = c.take() {
                s.stop();
            }
        }
        _ => {}
    }
}

#[tauri::command]
async fn call(app: AppHandle, method: String, params: Option<Value>) -> Result<Value, Value> {
    let api = app.state::<Shell>().api.clone();
    let m = method.clone();
    let r =
        tauri::async_runtime::spawn_blocking(move || api.call(&m, params.unwrap_or(Value::Null)))
            .await
            .map_err(|e| fail("internal", e))?;
    match r {
        Ok(mut v) => {
            match method.as_str() {
                "app.info" => {
                    if let Some(o) = v.as_object_mut() {
                        o.insert("mcp_path".into(), json!(mcp_path()));
                        o.insert("desktop".into(), json!(true));
                    }
                }
                "settings.set" | "settings.reset" => sync_control(&app.state::<Shell>()),
                _ => {}
            }
            Ok(v)
        }
        Err(e) => Err(fail(error_code(&e), format!("{e:#}"))),
    }
}

fn filters(kind: &str) -> (&'static str, &'static [&'static str]) {
    match kind {
        "flux" => ("Flux project package", &["flux"]),
        "preset" => ("Flux preset", &["fluxpreset", "json"]),
        "json" => ("JSON", &["json"]),
        "live" => ("Flux live preset", &["fluxlive", "json"]),
        _ => (
            "Audio",
            &[
                "wav", "wave", "flac", "mp3", "ogg", "oga", "opus", "m4a", "aac", "mp4", "aif",
                "aiff", "caf", "mkv", "webm",
            ],
        ),
    }
}

#[tauri::command]
async fn pick_files(app: AppHandle, multiple: bool, kind: String) -> Vec<String> {
    let (name, ext) = filters(&kind);
    let d = app.dialog().file().add_filter(name, ext);
    let picked = if multiple {
        d.blocking_pick_files().unwrap_or_default()
    } else {
        d.blocking_pick_file().into_iter().collect()
    };
    picked
        .into_iter()
        .filter_map(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
        .collect()
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
}

#[tauri::command]
async fn pick_save(app: AppHandle, name: String, kind: String) -> Option<String> {
    let mut d = app.dialog().file().set_file_name(&name);
    if kind != "audio" {
        let (n, ext) = filters(&kind);
        d = d.add_filter(n, ext);
    }
    d.blocking_save_file()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
}

/// Write a text file the user just chose in a save dialog.
#[tauri::command]
async fn write_text(path: String, text: String) -> Result<(), Value> {
    std::fs::write(&path, text).map_err(|e| fail("io", e))
}

#[tauri::command]
async fn reveal(app: AppHandle, path: String) -> Result<(), Value> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| fail("io", e))
}

/// Only web links leave the app. Everything else is refused.
#[tauri::command]
async fn open_url(app: AppHandle, url: String) -> Result<(), Value> {
    if !url.starts_with("https://") {
        return Err(fail("bad_params", "Only https links can be opened."));
    }
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| fail("io", e))
}

fn status(app: &AppHandle) -> Value {
    let shell = app.state::<Shell>();
    let u = shell.update.lock().unwrap_or_else(|e| e.into_inner());
    json!({
        "current": app.package_info().version.to_string(),
        "auto": shell.api.lock().store.settings().auto_update,
        "busy": u.busy,
        "ready": u.ready.as_ref().map(|(up, _)| json!({"version": up.version, "notes": up.body, "date": up.date.map(|d| d.to_string())})),
        "last_checked": u.last_checked,
        "error": u.error,
    })
}

/// Check GitHub releases; if a newer signed build exists, download it and announce it.
async fn check(app: &AppHandle) -> Result<Value, Value> {
    {
        let shell = app.state::<Shell>();
        let mut u = shell.update.lock().unwrap_or_else(|e| e.into_inner());
        if u.busy || u.ready.is_some() {
            drop(u);
            return Ok(status(app));
        }
        u.busy = true;
        u.error = None;
    }
    let _ = app.emit("flux://update-status", status(app));
    let result: Result<Option<(Update, Vec<u8>)>, String> = async {
        let up = app
            .updater()
            .map_err(|e| e.to_string())?
            .check()
            .await
            .map_err(|e| e.to_string())?;
        let Some(up) = up else { return Ok(None) };
        let total = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let (a, t) = (app.clone(), total.clone());
        let bytes = up
            .download(
                move |chunk, len| {
                    let got = t.fetch_add(chunk as u64, std::sync::atomic::Ordering::Relaxed)
                        + chunk as u64;
                    let _ = a.emit(
                        "flux://update-progress",
                        json!({"bytes": got, "total": len}),
                    );
                },
                || {},
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(Some((up, bytes)))
    }
    .await;
    let shell = app.state::<Shell>();
    {
        let mut u = shell.update.lock().unwrap_or_else(|e| e.into_inner());
        u.busy = false;
        u.last_checked = Some(now_s());
        match result {
            Ok(r) => u.ready = r,
            Err(e) => u.error = Some(e),
        }
    }
    let s = status(app);
    if !s["ready"].is_null() {
        let _ = app.emit("flux://update-ready", s["ready"].clone());
    }
    let _ = app.emit("flux://update-status", s.clone());
    Ok(s)
}

#[tauri::command]
async fn update_status(app: AppHandle) -> Value {
    status(&app)
}

#[tauri::command]
async fn update_check(app: AppHandle) -> Result<Value, Value> {
    check(&app).await
}

/// Install the downloaded update and restart into it.
#[tauri::command]
async fn update_install(app: AppHandle) -> Result<(), Value> {
    let ready = app
        .state::<Shell>()
        .update
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .ready
        .take();
    let Some((up, bytes)) = ready else {
        return Err(fail("not_found", "No update is ready to install."));
    };
    up.install(bytes).map_err(|e| fail("io", e))?;
    app.restart();
}

#[tauri::command]
fn relaunch(app: AppHandle) {
    app.restart();
}

fn main() {
    let api = match Api::open_default(false) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Flux could not open its data folder: {e:#}");
            std::process::exit(1);
        }
    };
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Shell {
            api,
            control: Mutex::new(None),
            update: Mutex::new(UpdateState::default()),
        })
        .invoke_handler(tauri::generate_handler![
            call,
            pick_files,
            pick_folder,
            pick_save,
            write_text,
            reveal,
            open_url,
            update_status,
            update_check,
            update_install,
            relaunch
        ])
        .setup(|app| {
            sync_control(&app.state::<Shell>());
            // Debug builds never update themselves: a dev build would replace itself with a release.
            if !cfg!(debug_assertions) {
                let h = app.handle().clone();
                std::thread::Builder::new()
                    .name("flux-updater".into())
                    .spawn(move || {
                        std::thread::sleep(FIRST_CHECK);
                        loop {
                            if h.state::<Shell>().api.lock().store.settings().auto_update {
                                let _ = tauri::async_runtime::block_on(check(&h));
                            }
                            std::thread::sleep(CHECK_EVERY);
                        }
                    })?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Flux could not start: {e:#}");
            std::process::exit(1);
        });
    app.run(|h, ev| {
        if let RunEvent::Exit = ev {
            let shell = h.state::<Shell>();
            if let Some(s) = shell
                .control
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
            {
                s.stop();
            }
            // "Later" means "when I quit": install a downloaded update on the way out.
            let ready = shell
                .update
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .ready
                .take();
            if let Some((up, bytes)) = ready {
                let _ = up.install(bytes);
            }
        }
    });
}
