//! The one command API. The desktop UI, the MCP server and the tests all call `Api::call`
//! with a method name and JSON params; the registry in `methods.rs` lists every method with a
//! description and parameter schema (that list *is* the MCP tool list).

use crate::device::{AudioEngine, DevicePrefs};
use crate::jobs::Jobs;
use crate::media::{Media, Pool};
use crate::project::Project;
use crate::render::{self, Msg, Plan};
use crate::store::{Settings, Store};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, MutexGuard};

pub const MAX_UNDO: usize = 500;

pub struct Hist {
    pub label: String,
    pub time: u64,
    pub project: Project,
}

pub struct RecCtx {
    pub media_id: String,
    pub path: std::path::PathBuf,
    pub track: Option<u32>,
    pub at: i64,
}

/// A rendered stereo buffer (left, right).
pub type Stereo = (Vec<f32>, Vec<f32>);

pub struct Session {
    pub store: Store,
    pub settings: Settings,
    pub project: Option<Project>,
    pub pool: Arc<Pool>,
    pub undo: Vec<Hist>,
    pub redo: Vec<Hist>,
    pub engine: Option<AudioEngine>,
    pub rev: u64,
    pub clipboard: Vec<(usize, crate::project::Clip)>,
    pub clip_origin: i64,
    pub magic: Vec<crate::tools::Fix>,
    pub rec: Option<RecCtx>,
    pub macro_rec: Option<Vec<crate::store::ActionStep>>,
    pub render_cache: Option<(u64, Arc<Stereo>)>,
    pub hold: bool,
    pub headless: bool,
    pub missing: Vec<String>,
    pub last_error: Option<String>,
    pub started: std::time::Instant,
    pub calls: u64,
}

#[derive(Clone)]
pub struct Api {
    pub s: Arc<Mutex<Session>>,
    pub jobs: Jobs,
}

pub fn lock(m: &Mutex<Session>) -> MutexGuard<'_, Session> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// An API error with a stable code, for the UI and MCP clients.
#[derive(Debug)]
pub struct ApiError {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ApiError {}
pub fn err(code: &'static str, message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(ApiError {
        code,
        message: message.into(),
    })
}
pub fn error_code(e: &anyhow::Error) -> &'static str {
    e.downcast_ref::<ApiError>()
        .map(|a| a.code)
        .unwrap_or("failed")
}

impl Session {
    pub fn new(store: Store, headless: bool) -> Session {
        let settings = store.settings();
        Session {
            store,
            settings,
            project: None,
            pool: Arc::new(Pool::new()),
            undo: vec![],
            redo: vec![],
            engine: None,
            rev: 1,
            clipboard: vec![],
            clip_origin: 0,
            magic: vec![],
            rec: None,
            macro_rec: None,
            render_cache: None,
            hold: false,
            headless,
            missing: vec![],
            last_error: None,
            started: std::time::Instant::now(),
            calls: 0,
        }
    }

    pub fn project(&self) -> Result<&Project> {
        self.project.as_ref().ok_or_else(|| {
            err(
                "no_project",
                "No project is open. Open or create one first.",
            )
        })
    }
    pub fn project_mut(&mut self) -> Result<&mut Project> {
        self.project.as_mut().ok_or_else(|| {
            err(
                "no_project",
                "No project is open. Open or create one first.",
            )
        })
    }

    pub fn prefs(&self) -> DevicePrefs {
        DevicePrefs {
            output: self.settings.output.clone(),
            input: self.settings.input.clone(),
            buffer: self.settings.buffer,
            null: false,
        }
    }

    /// Run an undoable edit. On error the project is left exactly as it was.
    pub fn commit<F>(&mut self, label: &str, f: F) -> Result<Value>
    where
        F: FnOnce(&mut Project) -> Result<Value>,
    {
        let p = self.project_mut()?;
        let before = p.clone();
        let h0 = before.hash();
        match f(p) {
            Err(e) => {
                *p = before;
                Err(e)
            }
            Ok(v) => {
                if p.hash() == h0 {
                    return Ok(v);
                }
                p.modified = crate::project::now_secs();
                self.undo.push(Hist {
                    label: label.to_string(),
                    time: crate::project::now_secs(),
                    project: before.clone(),
                });
                if self.undo.len() > MAX_UNDO {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.changed(Some(&before), label);
                Ok(v)
            }
        }
    }

    /// After any project change: persist, journal, update the audio engine, bump the revision.
    pub fn changed(&mut self, old: Option<&Project>, label: &str) {
        self.rev += 1;
        self.render_cache = None;
        let Some(p) = &self.project else { return };
        if self.settings.autosave {
            if let Err(e) = self.store.save_project(p) {
                self.last_error = Some(format!("Could not save: {e}"));
            }
        }
        self.store.journal(&p.id, label, &Value::Null, p.hash());
        self.sync_engine(old);
    }

    pub fn sync_engine(&mut self, old: Option<&Project>) {
        let Some(p) = self.project.as_ref() else {
            return;
        };
        let Some(e) = self.engine.as_mut() else {
            return;
        };
        if let Some(o) = old {
            if render::same_structure(o, p) {
                let (a, b) = (render::param_map(o), render::param_map(p));
                for (id, (pv, en, mix)) in &b {
                    match a.get(id) {
                        Some((pa, ea, ma)) => {
                            if pa != pv {
                                e.send(Msg::Params { fx: *id, p: *pv });
                            }
                            if ea != en || ma != mix {
                                e.send(Msg::Fx {
                                    fx: *id,
                                    enabled: *en,
                                    mix: *mix,
                                });
                            }
                        }
                        None => {
                            e.set_plan(Plan::build(p, self.pool.clone()));
                            return;
                        }
                    }
                }
                for t in &p.tracks {
                    if let Some(ot) = o.tracks.iter().find(|x| x.id == t.id) {
                        if ot.gain_db != t.gain_db
                            || ot.pan != t.pan
                            || ot.mute != t.mute
                            || ot.solo != t.solo
                        {
                            e.send(Msg::Track {
                                id: t.id,
                                gain: crate::dsp::db_to_lin(t.gain_db),
                                pan: t.pan,
                                mute: t.mute,
                                solo: t.solo,
                            });
                        }
                    }
                }
                if o.master.gain_db != p.master.gain_db {
                    e.send(Msg::MasterGain(crate::dsp::db_to_lin(p.master.gain_db)));
                }
                if o.loop_on != p.loop_on || o.loop_range != p.loop_range {
                    e.send(Msg::Loop(if p.loop_on { p.loop_range } else { None }));
                }
                return;
            }
        }
        e.set_plan(Plan::build(p, self.pool.clone()));
        e.send(Msg::Loop(if p.loop_on { p.loop_range } else { None }));
    }

    pub fn ensure_engine(&mut self) -> Result<&mut AudioEngine> {
        if self.engine.is_none() {
            let p = self.project()?.clone();
            let pool = self.pool.clone();
            let mut prefs = self.prefs();
            if self.headless && std::env::var_os("FLUX_HEADLESS_AUDIO").is_none() {
                prefs.null = true;
            }
            let mut e = AudioEngine::start(&|| Plan::build(&p, pool.clone()), &prefs);
            e.send(Msg::SafetyLimiter(self.settings.safety_limiter));
            e.send(Msg::Loop(if p.loop_on { p.loop_range } else { None }));
            self.engine = Some(e);
        }
        Ok(self.engine.as_mut().unwrap())
    }

    pub fn stop_engine(&mut self) {
        if let Some(mut e) = self.engine.take() {
            let _ = e.stop_recording();
        }
    }

    pub fn install(&mut self, p: Project, pool: Pool, missing: Vec<String>) {
        if let Some(old) = &self.project {
            self.store.unlock(&old.id);
        }
        self.stop_engine();
        self.store.lock(&p.id);
        self.project = Some(p);
        self.pool = Arc::new(pool);
        self.undo.clear();
        self.redo.clear();
        self.magic.clear();
        self.missing = missing;
        self.render_cache = None;
        self.rev += 1;
        if !self.headless {
            let _ = self.ensure_engine();
        }
    }

    pub fn close(&mut self) {
        self.stop_engine();
        if let Some(p) = self.project.take() {
            let _ = self.store.save_project(&p);
            self.store.unlock(&p.id);
        }
        self.pool = Arc::new(Pool::new());
        self.undo.clear();
        self.redo.clear();
        self.rev += 1;
    }

    pub fn add_media(&mut self, m: Media) {
        let mut pool: Pool = (*self.pool).clone();
        pool.insert(m.id.clone(), Arc::new(m));
        self.pool = Arc::new(pool);
    }

    pub fn undo(&mut self) -> Result<Value> {
        let h = self
            .undo
            .pop()
            .ok_or_else(|| err("nothing", "Nothing to undo"))?;
        let cur = self.project.replace(h.project).unwrap();
        let label = h.label.clone();
        self.redo.push(Hist {
            label: h.label,
            time: h.time,
            project: cur.clone(),
        });
        self.changed(Some(&cur), &format!("Undo {label}"));
        Ok(json!({ "undone": label }))
    }
    pub fn redo(&mut self) -> Result<Value> {
        let h = self
            .redo
            .pop()
            .ok_or_else(|| err("nothing", "Nothing to redo"))?;
        let cur = self.project.replace(h.project).unwrap();
        let label = h.label.clone();
        self.undo.push(Hist {
            label: h.label,
            time: h.time,
            project: cur.clone(),
        });
        self.changed(Some(&cur), &format!("Redo {label}"));
        Ok(json!({ "redone": label }))
    }
}

/// Decode every media file of a project at the project rate. Missing files are reported.
pub fn load_media(p: &Project) -> (Pool, Vec<String>) {
    let mut pool = Pool::new();
    let mut missing = vec![];
    let results: Vec<(String, Result<Media>)> = std::thread::scope(|sc| {
        let hs: Vec<_> = p
            .media
            .iter()
            .map(|m| {
                let sr = p.sample_rate;
                sc.spawn(move || {
                    let r = crate::media::decode_file(std::path::Path::new(&m.path))
                        .map(|d| Media::new(&m.id, d.data, d.rate, sr));
                    (m.id.clone(), r)
                })
            })
            .collect();
        hs.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    for (id, r) in results {
        match r {
            Ok(m) => {
                pool.insert(id, Arc::new(m));
            }
            Err(_) => missing.push(id),
        }
    }
    (pool, missing)
}

impl Api {
    pub fn new(store: Store, headless: bool) -> Api {
        Api {
            s: Arc::new(Mutex::new(Session::new(store, headless))),
            jobs: Jobs::default(),
        }
    }
    pub fn open_default(headless: bool) -> Result<Api> {
        Ok(Api::new(Store::open()?, headless))
    }
    pub fn lock(&self) -> MutexGuard<'_, Session> {
        lock(&self.s)
    }

    /// Call a method by name. Panics inside handlers become errors; the session stays usable.
    pub fn call(&self, method: &str, params: Value) -> Result<Value> {
        let spec = crate::methods::find(method).ok_or_else(|| {
            err(
                "unknown_method",
                format!("Unknown method {method}. Call `methods` for the list."),
            )
        })?;
        let params = if params.is_null() { json!({}) } else { params };
        if !params.is_object() {
            bail!(err("bad_params", "params must be an object"));
        }
        crate::methods::check_params(spec, &params)?;
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (spec.handler)(self, &params)
        }));
        let r = match r {
            Ok(r) => r,
            Err(p) => {
                let msg = p
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| p.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "internal error".into());
                Err(err(
                    "internal",
                    format!("Flux hit an internal error in {method}: {msg}"),
                ))
            }
        };
        let mut s = self.lock();
        s.calls += 1;
        if r.is_ok() && spec.records {
            if let Some(m) = s.macro_rec.as_mut() {
                m.push(crate::store::ActionStep {
                    method: method.to_string(),
                    params: params.clone(),
                });
            }
        }
        r
    }

    /// Clone what a long computation needs, without holding the lock while computing.
    pub fn snapshot(&self) -> Result<(Project, Arc<Pool>)> {
        let s = self.lock();
        Ok((s.project()?.clone(), s.pool.clone()))
    }

    /// Render the mix (or a range of it), cached by project hash.
    pub fn render_mix(&self, range: Option<(i64, i64)>) -> Result<Arc<(Vec<f32>, Vec<f32>)>> {
        let (p, pool) = self.snapshot()?;
        let h = p.hash()
            ^ range
                .map(|(a, b)| (a as u64).wrapping_mul(31) ^ b as u64)
                .unwrap_or(0);
        if let Some((ch, c)) = &self.lock().render_cache {
            if *ch == h {
                return Ok(c.clone());
            }
        }
        if p.length() == 0 {
            return Err(err(
                "empty",
                "The project has no audio yet. Add a file first.",
            ));
        }
        let out = Arc::new(render::render_project(&p, pool, range, range.is_none()));
        self.lock().render_cache = Some((h, out.clone()));
        Ok(out)
    }
}

pub fn need<'a>(v: &'a Value, k: &str) -> Result<&'a Value> {
    v.get(k)
        .filter(|x| !x.is_null())
        .ok_or_else(|| err("bad_params", format!("missing parameter `{k}`")))
}
pub fn s_(v: &Value, k: &str) -> Result<String> {
    need(v, k)?
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| err("bad_params", format!("`{k}` must be a string")))
}
pub fn os(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string())
}
pub fn f_(v: &Value, k: &str) -> Result<f32> {
    need(v, k)?
        .as_f64()
        .map(|x| x as f32)
        .filter(|x| x.is_finite())
        .ok_or_else(|| err("bad_params", format!("`{k}` must be a number")))
}
pub fn of(v: &Value, k: &str) -> Option<f32> {
    v.get(k)
        .and_then(|x| x.as_f64())
        .map(|x| x as f32)
        .filter(|x| x.is_finite())
}
pub fn u_(v: &Value, k: &str) -> Result<u32> {
    need(v, k)?
        .as_u64()
        .map(|x| x as u32)
        .ok_or_else(|| err("bad_params", format!("`{k}` must be a whole number")))
}
pub fn ou(v: &Value, k: &str) -> Option<u32> {
    v.get(k).and_then(|x| x.as_u64()).map(|x| x as u32)
}
pub fn ob(v: &Value, k: &str) -> Option<bool> {
    v.get(k).and_then(|x| x.as_bool())
}
/// A position in samples: `k` (samples) or `k_s` (seconds).
pub fn opos(v: &Value, k: &str, sr: u32) -> Option<i64> {
    if let Some(x) = v.get(k).and_then(|x| x.as_f64()) {
        return Some(x.round() as i64);
    }
    v.get(format!("{k}_s"))
        .and_then(|x| x.as_f64())
        .map(|s| (s * sr as f64).round() as i64)
}
pub fn pos(v: &Value, k: &str, sr: u32) -> Result<i64> {
    opos(v, k, sr).ok_or_else(|| {
        err(
            "bad_params",
            format!("missing `{k}` (samples) or `{k}_s` (seconds)"),
        )
    })
}
pub fn fparams(v: &Value, k: &str) -> Result<std::collections::BTreeMap<String, f32>> {
    let mut m = std::collections::BTreeMap::new();
    if let Some(o) = v.get(k).and_then(|x| x.as_object()) {
        for (key, val) in o {
            let x = val
                .as_f64()
                .or_else(|| val.as_bool().map(|b| if b { 1.0 } else { 0.0 }))
                .ok_or_else(|| err("bad_params", format!("parameter `{key}` must be a number")))?;
            m.insert(key.clone(), x as f32);
        }
    }
    Ok(m)
}
pub fn anyerr(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{e}")
}
