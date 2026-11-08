//! Voice cloning models: what can be downloaded, downloading it safely, the runtime that runs it, the person's
//! choices, and running a model.
//!
//! - **Catalog.** `voice_models.json`, built in: every file pinned to a commit with its size and SHA-256, the
//!   portable Python archive per platform with its SHA-256, and the pip steps. A `Models/catalog.override.json` in
//!   the data folder replaces it (mirrors and tests); checksums are still enforced against whatever catalog is used.
//! - **Downloads** only start from `clone.model.download` with `confirm: true`, which the app sends when the person
//!   presses Download after seeing the size, licence and folder. Files resume after a cancel or a dropped
//!   connection, are checked against their SHA-256 before use, and models from one repository share files.
//! - **Runtime.** Python 3.11 (python-build-standalone) unpacked into Flux's folder, then PyTorch and the Chatterbox
//!   package installed with pip at pinned versions. `flux_voice.py` runs the models with the Hub switched off.
//! - **Choice.** Any downloaded model can be used for speech or conversion, on the device the person picks, with
//!   every generation setting the model has, and folders of compatible models can be added by hand.

use crate::api::{err, Api};
use crate::jobs::JobCtx;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

pub const RUNNER: &str = include_str!("flux_voice.py");
const BUILTIN: &str = include_str!("voice_models.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub hub: String,
    pub repos: BTreeMap<String, Repo>,
    pub models: Vec<ModelSpec>,
    pub runtime: RuntimeSpec,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Repo {
    pub repo: String,
    pub rev: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelSpec {
    pub id: String,
    pub name: String,
    /// `speech` or `convert`.
    pub role: String,
    /// `turbo`, `english`, `multilingual` or `vc`: which Chatterbox class loads it.
    pub loader: String,
    pub repo: String,
    pub licence: String,
    pub languages: Vec<String>,
    #[serde(default)]
    pub language_names: BTreeMap<String, String>,
    /// Performs [laugh], [sigh] and other sounds.
    pub sounds: bool,
    pub min_vram_gb: f32,
    pub note: String,
    pub files: Vec<FileSpec>,
    #[serde(default)]
    pub params: Vec<Param>,
    /// `light`, `balanced` or `best`: how a model compares with the others for its job.
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub min_ram_gb: f32,
    /// Ready-made synthetic voices (text to speech models).
    #[serde(default)]
    pub voices: Vec<VoiceOpt>,
    /// Extra packages this model needs in the engine, installed with its first download.
    #[serde(default)]
    pub install: Vec<Step>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceOpt {
    pub id: String,
    pub name: String,
    pub language: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSpec {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    /// Where the file is when it isn't in the model's Hugging Face repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Param {
    pub key: String,
    pub label: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
    pub tip: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeSpec {
    pub id: String,
    pub name: String,
    pub version: String,
    pub note: String,
    pub python: BTreeMap<String, PyArchive>,
    pub steps: Vec<Step>,
    pub approx_mb: BTreeMap<String, u64>,
    /// Tests only: run this interpreter instead of the unpacked one, with this PYTHONPATH.
    #[serde(default)]
    pub python_override: Option<String>,
    #[serde(default)]
    pub pythonpath: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PyArchive {
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub exe: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub index: BTreeMap<String, String>,
}

fn builtin() -> &'static Catalog {
    static C: OnceLock<Catalog> = OnceLock::new();
    C.get_or_init(|| serde_json::from_str(BUILTIN).expect("voice_models.json"))
}

pub fn models_root(api: &Api) -> PathBuf {
    api.lock().store.root.join("Models")
}

/// The catalog in use: the built-in one, or the data folder's override.
pub fn catalog(api: &Api) -> Result<Catalog> {
    let p = models_root(api).join("catalog.override.json");
    match std::fs::read(&p) {
        Ok(b) => serde_json::from_slice(&b).map_err(|e| {
            err(
                "bad_file",
                format!("catalog.override.json isn't valid: {e}"),
            )
        }),
        Err(_) => Ok(builtin().clone()),
    }
}

pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn repo_dir(root: &Path, c: &Catalog, key: &str) -> Result<PathBuf> {
    let r = c
        .repos
        .get(key)
        .ok_or_else(|| err("bad_file", format!("the catalog has no repository `{key}`")))?;
    let safe: String = r
        .repo
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    Ok(root
        .join("files")
        .join(format!("{safe}@{}", &r.rev[..r.rev.len().min(12)])))
}

/// A path inside the catalog must stay inside its folder.
fn safe_rel(p: &str) -> Result<&str> {
    let bad = p.is_empty()
        || p.starts_with('/')
        || p.starts_with('\\')
        || p.contains(':')
        || p.split(['/', '\\']).any(|s| s == ".." || s.is_empty());
    if bad {
        bail!(err(
            "bad_file",
            format!("unsafe file name in the catalog: {p}")
        ));
    }
    Ok(p)
}

fn marker(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".verified");
    PathBuf::from(s)
}
fn part(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// A file is ready when its marker holds the expected hash and its size matches.
fn file_ready(p: &Path, f: &FileSpec) -> bool {
    std::fs::read_to_string(marker(p))
        .map(|s| s.trim() == f.sha256)
        .unwrap_or(false)
        && std::fs::metadata(p)
            .map(|m| m.len() == f.size)
            .unwrap_or(false)
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct State {
    pub installed: bool,
    pub bytes_total: u64,
    pub bytes_have: u64,
    pub files_missing: usize,
    /// A stopped download left part of a file: Download resumes it.
    pub partial: bool,
    /// The engine has the packages this model needs (always true for models that need none).
    pub packages: bool,
}

pub fn model_state(root: &Path, c: &Catalog, m: &ModelSpec) -> State {
    let dir = match repo_dir(root, c, &m.repo) {
        Ok(d) => d,
        Err(_) => return State::default(),
    };
    let mut st = State::default();
    for f in &m.files {
        st.bytes_total += f.size;
        let p = dir.join(&f.path);
        if file_ready(&p, f) {
            st.bytes_have += f.size;
        } else {
            st.files_missing += 1;
            st.partial |= part(&p).exists();
            st.bytes_have += std::fs::metadata(part(&p))
                .map(|m| m.len().min(f.size))
                .unwrap_or(0);
        }
    }
    st.packages = packages_ready(root, c, m);
    st.installed = st.files_missing == 0 && st.packages;
    st
}

/// One key per install step, so models that share packages install them once.
fn step_key(s: &Step) -> String {
    s.args.join(" ")
}
fn packages_file(root: &Path, c: &Catalog) -> PathBuf {
    runtime_dir(root, c).join("packages.json")
}
fn packages_done(root: &Path, c: &Catalog) -> std::collections::BTreeSet<String> {
    std::fs::read(packages_file(root, c))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
pub fn packages_ready(root: &Path, c: &Catalog, m: &ModelSpec) -> bool {
    if m.install.is_empty() {
        return true;
    }
    let done = packages_done(root, c);
    m.install.iter().all(|s| done.contains(&step_key(s)))
}
fn packages_todo(root: &Path, c: &Catalog, m: &ModelSpec) -> Vec<Step> {
    let done = packages_done(root, c);
    m.install
        .iter()
        .filter(|s| !done.contains(&step_key(s)))
        .cloned()
        .collect()
}
fn mark_package(root: &Path, c: &Catalog, s: &Step) -> Result<()> {
    let mut done = packages_done(root, c);
    done.insert(step_key(s));
    crate::store::atomic_write(
        &packages_file(root, c),
        serde_json::to_string_pretty(&done)?.as_bytes(),
    )
}

fn runtime_dir(root: &Path, c: &Catalog) -> PathBuf {
    let v: String = c
        .runtime
        .version
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    root.join("runtime").join(v)
}

pub fn runtime_ready(root: &Path, c: &Catalog) -> Option<Value> {
    let b = std::fs::read(runtime_dir(root, c).join("ready.json")).ok()?;
    let v: Value = serde_json::from_slice(&b).ok()?;
    (v["version"] == c.runtime.version).then_some(v)
}

fn python_exe(root: &Path, c: &Catalog) -> Result<PathBuf> {
    if let Some(p) = &c.runtime.python_override {
        return Ok(PathBuf::from(p));
    }
    let a = c.runtime.python.get(&platform()).ok_or_else(|| {
        err(
            "unavailable",
            "The voice engine has no build for this kind of computer.",
        )
    })?;
    Ok(runtime_dir(root, c).join(safe_rel(&a.exe)?))
}

/// What kind of PyTorch to install: `cuda` with an NVIDIA chip on Windows or Linux, `mps` on Apple silicon,
/// otherwise `cpu`.
pub fn torch_kind(h: &crate::voiceclone::Hardware) -> &'static str {
    if h.os == "macos" {
        "mps"
    } else if h.gpu_vendor == "nvidia" {
        "cuda"
    } else {
        "cpu"
    }
}

// ---------------------------------------------------------------- downloading

fn client() -> Result<reqwest::blocking::Client> {
    build_client(None)
}

/// An HTTPS client for cloud engines, with an overall time limit per request.
pub(crate) fn http_client(timeout: std::time::Duration) -> Result<reqwest::blocking::Client> {
    build_client(Some(timeout))
}

fn build_client(timeout: Option<std::time::Duration>) -> Result<reqwest::blocking::Client> {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    Ok(reqwest::blocking::Client::builder()
        .user_agent(concat!("Flux/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(timeout)
        .build()?)
}

/// Only HTTPS, except to this computer (tests and local mirrors).
fn check_url(u: &str) -> Result<()> {
    let ok = u.starts_with("https://")
        || u.starts_with("http://127.0.0.1:")
        || u.starts_with("http://localhost:");
    if !ok {
        bail!(err("bad_file", "Flux only downloads models over HTTPS."));
    }
    Ok(())
}

/// Progress across a whole job: bytes done of bytes to do.
struct Meter<'a> {
    ctx: &'a JobCtx,
    done: u64,
    total: u64,
    from: f32,
    span: f32,
}
impl Meter<'_> {
    fn add(&mut self, n: u64) -> bool {
        self.done += n;
        let p = if self.total == 0 {
            1.0
        } else {
            self.done as f32 / self.total as f32
        };
        self.ctx.progress(self.from + self.span * p.min(1.0))
    }
}

fn cancelled() -> anyhow::Error {
    err(
        "cancelled",
        "Cancelled. Download again to pick up where it stopped.",
    )
}

/// Download one file to `dest`, resuming a `.part`, and keep it only if the size and SHA-256 match.
fn fetch(url: &str, dest: &Path, size: u64, sha: &str, meter: &mut Meter) -> Result<()> {
    check_url(url)?;
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = part(dest);
    let mut tries = 0;
    loop {
        let have = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
        if have > size {
            std::fs::remove_file(&tmp)?;
            continue;
        }
        if have < size {
            match pull(url, &tmp, have, size, meter) {
                Ok(()) => {}
                Err(e) if e.downcast_ref::<crate::api::ApiError>().is_some() => return Err(e),
                Err(e) => {
                    tries += 1;
                    if tries >= 4 {
                        return Err(err(
                            "network",
                            format!("The download kept failing ({e:#}). Check the connection and press Download again; it resumes."),
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500 * tries));
                    continue;
                }
            }
        }
        // Check the whole file before it is used.
        let mut h = Sha256::new();
        let mut f = std::fs::File::open(&tmp)?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            if meter.ctx.cancelled() {
                return Err(cancelled());
            }
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        let got = format!("{:x}", h.finalize());
        let len = std::fs::metadata(&tmp)?.len();
        if got != sha || len != size {
            let _ = std::fs::remove_file(&tmp);
            bail!(err(
                "bad_download",
                format!("{} didn't match its checksum, so Flux threw it away. Press Download to try again.", dest.file_name().and_then(|s| s.to_str()).unwrap_or("A file"))
            ));
        }
        drop(f);
        std::fs::rename(&tmp, dest)?;
        std::fs::write(marker(dest), sha)?;
        return Ok(());
    }
}

fn pull(url: &str, tmp: &Path, have: u64, size: u64, meter: &mut Meter) -> Result<()> {
    let mut req = client()?.get(url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let mut res = req.send()?;
    let status = res.status();
    let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
    if !(status.is_success()) {
        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            std::fs::remove_file(tmp).ok();
        }
        if status.is_client_error() && status != reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            bail!(err(
                "network",
                format!("The server refused the download ({status}).")
            ));
        }
        bail!("server said {status}");
    }
    let mut f = if resumed {
        std::fs::OpenOptions::new().append(true).open(tmp)?
    } else {
        // The server sent the whole file: start over, and take back the progress counted for the old part.
        meter.done = meter.done.saturating_sub(have);
        std::fs::File::create(tmp)?
    };
    let mut buf = vec![0u8; 1 << 18];
    let mut written = if resumed { have } else { 0 };
    loop {
        let n = res.read(&mut buf)?;
        if n == 0 {
            break;
        }
        written += n as u64;
        if written > size {
            bail!(err(
                "bad_download",
                "The server sent more than the file's size."
            ));
        }
        f.write_all(&buf[..n])?;
        if !meter.add(n as u64) {
            f.flush()?;
            return Err(cancelled());
        }
    }
    f.flush()?;
    if written < size {
        bail!("connection closed early");
    }
    Ok(())
}

fn free_space_check(root: &Path, need: u64) -> Result<()> {
    std::fs::create_dir_all(root)?;
    if let Ok(free) = fs4::available_space(root) {
        let margin = 256 * 1024 * 1024;
        if free < need + margin {
            bail!(err(
                "disk_full",
                format!(
                    "This needs {} free, and the disk has {}. Free some space and press Download again.",
                    gb(need + margin),
                    gb(free)
                )
            ));
        }
    }
    Ok(())
}

pub fn gb(b: u64) -> String {
    if b >= 1_000_000_000 {
        format!("{:.2} GB", b as f64 / 1e9)
    } else {
        format!("{:.0} MB", (b as f64 / 1e6).max(1.0))
    }
}

/// Bytes still to download for a model's files.
fn model_todo(root: &Path, c: &Catalog, m: &ModelSpec) -> Result<Vec<(PathBuf, FileSpec)>> {
    let dir = repo_dir(root, c, &m.repo)?;
    let mut v = vec![];
    for f in &m.files {
        let p = dir.join(safe_rel(&f.path)?);
        if !file_ready(&p, f) {
            v.push((p, f.clone()));
        }
    }
    Ok(v)
}

fn file_url(c: &Catalog, m: &ModelSpec, f: &FileSpec) -> Result<String> {
    if let Some(u) = &f.url {
        check_url(u)?;
        return Ok(u.clone());
    }
    let r = &c.repos[&m.repo];
    Ok(format!(
        "{}/{}/resolve/{}/{}",
        c.hub.trim_end_matches('/'),
        r.repo,
        r.rev,
        f.path
    ))
}

// ---------------------------------------------------------------- the runtime

fn run_logged(mut cmd: Command, ctx: &JobCtx, stage: &str) -> Result<String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd
        .spawn()
        .with_context(|| format!("couldn't start {stage}"))?;
    let out = child.stdout.take().unwrap();
    let errs = child.stderr.take().unwrap();
    let tail = std::sync::Arc::new(Mutex::new(Vec::<String>::new()));
    let t2 = tail.clone();
    let eh = std::thread::spawn(move || {
        for l in BufReader::new(errs).lines().map_while(Result::ok) {
            let mut t = t2.lock().unwrap();
            t.push(l);
            if t.len() > 12 {
                t.remove(0);
            }
        }
    });
    let mut all = String::new();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let oh = std::thread::spawn(move || {
        for l in BufReader::new(out).lines().map_while(Result::ok) {
            if tx.send(l).is_err() {
                break;
            }
        }
    });
    loop {
        if ctx.cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(cancelled());
        }
        match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(l) => {
                if let Some(p) = l.strip_prefix("PROGRESS ") {
                    if let Ok(x) = p.trim().parse::<f32>() {
                        ctx.progress(x);
                    }
                } else if l.starts_with("Downloading ") || l.starts_with("Collecting ") {
                    let name: String = l
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .chars()
                        .take(60)
                        .collect();
                    ctx.stage(stage, &format!("active: {name}"));
                }
                all.push_str(&l);
                all.push('\n');
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = oh.join();
    let st = child.wait()?;
    let _ = eh.join();
    if !st.success() {
        let msg = all
            .lines()
            .rev()
            .find_map(|l| l.strip_prefix("ERROR "))
            .map(String::from)
            .unwrap_or_else(|| {
                tail.lock()
                    .unwrap()
                    .join(" ")
                    .chars()
                    .rev()
                    .take(400)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect()
            });
        bail!(err("failed", format!("{stage} failed: {msg}")));
    }
    Ok(all)
}

fn python_cmd(root: &Path, c: &Catalog) -> Result<Command> {
    let exe = python_exe(root, c)?;
    let mut cmd = Command::new(exe);
    cmd.env("PYTHONNOUSERSITE", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .env("HF_HUB_DISABLE_TELEMETRY", "1");
    if let Some(pp) = &c.runtime.pythonpath {
        cmd.env("PYTHONPATH", pp);
    }
    Ok(cmd)
}

/// Stand-ins for modules that packages import but Flux never calls (they would bring in LGPL code). The runner puts
/// them last on its path, so a real package always wins.
const SHIMS: &[(&str, &str)] = &[
    ("av.py", include_str!("py_shims/av.py")),
    ("num2words.py", include_str!("py_shims/num2words.py")),
    (
        "espeakng_loader.py",
        include_str!("py_shims/espeakng_loader.py"),
    ),
    (
        "phonemizer/__init__.py",
        include_str!("py_shims/phonemizer/__init__.py"),
    ),
    (
        "phonemizer/backend/__init__.py",
        include_str!("py_shims/phonemizer/backend/__init__.py"),
    ),
    (
        "phonemizer/backend/espeak/__init__.py",
        include_str!("py_shims/phonemizer/backend/espeak/__init__.py"),
    ),
    (
        "phonemizer/backend/espeak/wrapper.py",
        include_str!("py_shims/phonemizer/backend/espeak/wrapper.py"),
    ),
];

fn write_runner(dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let p = dir.join("flux_voice.py");
    if std::fs::read_to_string(&p).ok().as_deref() != Some(RUNNER) {
        std::fs::write(&p, RUNNER)?;
    }
    for (rel, text) in SHIMS {
        let f = dir.join("flux_shims").join(rel);
        if std::fs::read_to_string(&f).ok().as_deref() != Some(*text) {
            std::fs::create_dir_all(f.parent().unwrap())?;
            std::fs::write(&f, text)?;
        }
    }
    Ok(p)
}

/// One pip install, shown as a stage of the job.
fn pip_step(root: &Path, c: &Catalog, kind: &str, ctx: &JobCtx, s: &Step) -> Result<()> {
    ctx.stage(&s.name, "active");
    let mut cmd = python_cmd(root, c)?;
    cmd.args([
        "-m",
        "pip",
        "install",
        "--disable-pip-version-check",
        "--no-input",
        "--no-cache-dir",
        "--no-warn-script-location",
    ]);
    cmd.args(&s.args);
    if let Some(ix) = s.index.get(kind) {
        check_url(ix)?;
        cmd.args(["--index-url", ix]);
    }
    run_logged(cmd, ctx, &s.name)?;
    ctx.stage(&s.name, "done");
    Ok(())
}

/// Install the runtime: Python, then the pip steps, then check it imports every loader. `share` is the job progress
/// at which the runtime is done; the Python download fills the meter's span and each pip step moves it further.
fn install_runtime(
    root: &Path,
    c: &Catalog,
    kind: &str,
    ctx: &JobCtx,
    meter: &mut Meter,
    share: f32,
) -> Result<Value> {
    let dir = runtime_dir(root, c);
    std::fs::create_dir_all(&dir)?;
    ctx.stage("Downloading Python", "active");
    if c.runtime.python_override.is_none() || c.runtime.python.contains_key(&platform()) {
        let a = c.runtime.python.get(&platform()).ok_or_else(|| {
            err(
                "unavailable",
                "The voice engine has no build for this kind of computer.",
            )
        })?;
        let exe = dir.join(safe_rel(&a.exe)?);
        if !exe.exists() {
            let arc = dir.join("python.tar.gz");
            if !file_ready(
                &arc,
                &FileSpec {
                    path: String::new(),
                    size: a.size,
                    sha256: a.sha256.clone(),
                    url: None,
                },
            ) {
                fetch(&a.url, &arc, a.size, &a.sha256, meter)?;
            }
            ctx.stage("Downloading Python", "done");
            ctx.stage("Unpacking", "active");
            let gz = flate2::read::GzDecoder::new(std::fs::File::open(&arc)?);
            let mut t = tar::Archive::new(gz);
            for e in t.entries()? {
                let mut e = e?;
                // unpack_in refuses anything that would land outside the folder.
                if !e.unpack_in(&dir)? {
                    bail!(err(
                        "bad_download",
                        "The Python archive has a file outside its folder."
                    ));
                }
            }
            std::fs::remove_file(&arc).ok();
            std::fs::remove_file(marker(&arc)).ok();
        }
    }
    ctx.stage("Downloading Python", "done");
    ctx.stage("Unpacking", "done");
    let py_end = meter.from + meter.span;
    let n = c.runtime.steps.len().max(1) as f32;
    for (i, s) in c.runtime.steps.iter().enumerate() {
        ctx.progress(py_end + (share * 0.95 - py_end) * i as f32 / n);
        pip_step(root, c, kind, ctx, s)?;
        ctx.progress(py_end + (share * 0.95 - py_end) * (i + 1) as f32 / n);
    }
    ctx.stage("Checking the voice engine", "active");
    let runner = write_runner(&dir)?;
    let mut cmd = python_cmd(root, c)?;
    cmd.arg(&runner).arg("check");
    let out = run_logged(cmd, ctx, "Checking the voice engine")?;
    let mut v: Value = out
        .lines()
        .rev()
        .find_map(|l| serde_json::from_str(l).ok())
        .ok_or_else(|| err("failed", "The voice engine didn't report back."))?;
    v["version"] = json!(c.runtime.version);
    v["kind"] = json!(kind);
    v["checked"] = json!(crate::project::now_secs());
    crate::store::atomic_write(
        &dir.join("ready.json"),
        serde_json::to_string_pretty(&v)?.as_bytes(),
    )?;
    ctx.stage("Checking the voice engine", "done");
    Ok(v)
}

// ---------------------------------------------------------------- settings and local models

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalModel {
    pub id: String,
    pub name: String,
    pub loader: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub speech: String,
    pub convert: String,
    /// `auto`, `cuda`, `mps` or `cpu`.
    pub device: String,
    pub language: String,
    pub seed: u64,
    #[serde(default)]
    pub params: BTreeMap<String, BTreeMap<String, f64>>,
    #[serde(default)]
    pub local: Vec<LocalModel>,
    /// The engine chosen for each smart tool (`stems`, `transcript`, `tts`): a model id, a built-in engine or a
    /// cloud service. Missing means the suggestion for this computer.
    #[serde(default)]
    pub engines: BTreeMap<String, String>,
    /// Per job: the chosen voice (text to speech) or language (transcripts).
    #[serde(default)]
    pub options: BTreeMap<String, BTreeMap<String, String>>,
    /// Cloud engines the person said yes to (what is sent, and where, was shown first).
    #[serde(default)]
    pub cloud_ok: std::collections::BTreeSet<String>,
    /// Services whose key Flux keeps in the system keychain. The key itself is never in this file.
    #[serde(default)]
    pub cloud_keys: std::collections::BTreeSet<String>,
    /// A service's address when it isn't the default (an OpenAI-compatible server, a mirror).
    #[serde(default)]
    pub cloud_base: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            speech: "chatterbox-turbo".into(),
            convert: "chatterbox-vc".into(),
            device: "auto".into(),
            language: "en".into(),
            seed: 1,
            params: BTreeMap::new(),
            local: vec![],
            engines: BTreeMap::new(),
            options: BTreeMap::new(),
            cloud_ok: Default::default(),
            cloud_keys: Default::default(),
            cloud_base: BTreeMap::new(),
        }
    }
}

/// The saved choices. A chosen model the catalog no longer has (or never had, with a mirror's catalog) falls back
/// to the catalog's first model for that role, so there is always something sensible to download and use.
pub fn settings(api: &Api) -> Settings {
    let mut s: Settings = std::fs::read(models_root(api).join("settings.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if let Ok(c) = catalog(api) {
        for role in ["speech", "convert"] {
            let cur = if role == "speech" {
                &s.speech
            } else {
                &s.convert
            };
            if resolve(&c, &s, cur).is_none() {
                if let Some(m) = c.models.iter().find(|m| m.role == role) {
                    let id = m.id.clone();
                    if role == "speech" {
                        s.speech = id;
                    } else {
                        s.convert = id;
                    }
                }
            }
        }
    }
    s
}
pub(crate) fn save_settings(api: &Api, s: &Settings) -> Result<()> {
    crate::store::atomic_write(
        &models_root(api).join("settings.json"),
        serde_json::to_string_pretty(s)?.as_bytes(),
    )
}

/// A model by id: a catalog one, or a folder the person added (it borrows the catalog entry of its loader for
/// roles, files to expect and settings).
fn resolve(c: &Catalog, s: &Settings, id: &str) -> Option<(ModelSpec, Option<LocalModel>)> {
    if let Some(m) = c.models.iter().find(|m| m.id == id) {
        return Some((m.clone(), None));
    }
    let l = s.local.iter().find(|l| l.id == id)?;
    let base = c.models.iter().find(|m| m.loader == l.loader)?;
    let mut m = base.clone();
    m.id = l.id.clone();
    m.name = l.name.clone();
    m.licence = "Your own".into();
    m.note = format!("A folder you added: {}", l.path);
    Some((m, Some(l.clone())))
}

pub(crate) fn params_for(s: &Settings, m: &ModelSpec) -> BTreeMap<String, f64> {
    let mine = s.params.get(&m.id);
    m.params
        .iter()
        .map(|p| {
            (
                p.key.clone(),
                mine.and_then(|x| x.get(&p.key))
                    .copied()
                    .unwrap_or(p.default),
            )
        })
        .collect()
}

/// Does a computer suit a model on its graphics chip? Anything runs on the processor, more slowly.
pub(crate) fn fits(m: &ModelSpec, h: &crate::voiceclone::Hardware) -> (&'static str, String) {
    if h.os == "macos" && h.arch == "x86_64" {
        return (
            "no",
            "PyTorch 2.6, which these models need, isn't made for Intel Macs.".into(),
        );
    }
    if h.gpu_vendor == "nvidia" && h.vram_gb >= m.min_vram_gb {
        return (
            "gpu",
            format!("Runs on your {} ({:.0} GB).", h.gpu_name, h.vram_gb),
        );
    }
    if h.os == "macos" {
        return ("gpu", "Runs on the Apple graphics cores.".into());
    }
    (
        "cpu",
        format!(
            "Runs on the processor, more slowly{}.",
            if h.gpu_vendor == "nvidia" {
                format!(" (it wants {:.0} GB of video memory)", m.min_vram_gb)
            } else {
                String::new()
            }
        ),
    )
}

// ---------------------------------------------------------------- handlers

static ACTIVE: OnceLock<Mutex<BTreeMap<String, String>>> = OnceLock::new();
fn active() -> &'static Mutex<BTreeMap<String, String>> {
    ACTIVE.get_or_init(|| Mutex::new(BTreeMap::new()))
}
fn active_key(root: &Path, id: &str) -> String {
    format!("{}|{id}", root.display())
}
pub(crate) fn active_job(api: &Api, root: &Path, id: &str) -> Option<Value> {
    let jid = active()
        .lock()
        .unwrap()
        .get(&active_key(root, id))
        .cloned()?;
    let j = api.jobs.get(&jid)?;
    (j.state == "running").then(|| json!({"job": jid, "progress": j.progress, "stage": j.stage}))
}

pub fn h_models(api: &Api, _: &Value) -> Result<Value> {
    let c = catalog(api)?;
    let root = models_root(api);
    let s = settings(api);
    let h = crate::voiceclone::hardware();
    let pick = crate::voiceclone::recommend(h);
    let kind = torch_kind(h);
    let rt = runtime_ready(&root, &c);
    // Stems, transcript and text to speech models are listed by ai.engines.
    let mut list: Vec<Value> = c
        .models
        .iter()
        .filter(|m| m.role == "speech" || m.role == "convert")
        .map(|m| {
            let st = model_state(&root, &c, m);
            let (fit, why) = fits(m, h);
            json!({
                "id": m.id, "name": m.name, "role": m.role, "loader": m.loader, "licence": m.licence,
                "source": format!("{}/{}", c.hub.trim_end_matches('/'), c.repos.get(&m.repo).map(|r| r.repo.as_str()).unwrap_or("")),
                "languages": m.languages, "language_names": m.language_names, "sounds": m.sounds, "note": m.note,
                "size": st.bytes_total, "size_text": gb(st.bytes_total), "have": st.bytes_have, "partial": st.partial, "installed": st.installed,
                "fits": fit, "fits_why": why,
                "recommended": pick.model.map(|p| p == m.id).unwrap_or(false) || pick.convert.map(|p| p == m.id).unwrap_or(false),
                "params": m.params, "downloading": active_job(api, &root, &m.id), "local": false,
                "in_use": s.speech == m.id || s.convert == m.id,
            })
        })
        .collect();
    for l in &s.local {
        if let Some((m, _)) = resolve(&c, &s, &l.id) {
            list.push(json!({"id": m.id, "name": m.name, "role": m.role, "loader": m.loader, "licence": m.licence, "source": l.path, "languages": m.languages, "language_names": m.language_names, "sounds": m.sounds, "note": m.note, "installed": Path::new(&l.path).is_dir(), "size": 0, "size_text": "", "have": 0, "fits": "local", "fits_why": "", "recommended": false, "params": m.params, "downloading": null, "local": true, "in_use": s.speech == m.id || s.convert == m.id}));
        }
    }
    let approx = c.runtime.approx_mb.get(kind).copied().unwrap_or(1000) * 1_000_000;
    Ok(json!({
        "models": list,
        "runtime": {
            "id": c.runtime.id, "name": c.runtime.name, "note": c.runtime.note, "version": c.runtime.version,
            "installed": rt.is_some(), "info": rt, "kind": kind, "approx": approx, "approx_text": format!("about {}", gb(approx)),
            "supported": c.runtime.python.contains_key(&platform()) && !(h.os == "macos" && h.arch == "x86_64"),
            "downloading": active_job(api, &root, &c.runtime.id),
        },
        "pick": pick, "hardware": h, "settings": s, "folder": root.display().to_string(),
    }))
}

/// Download a model (and the voice engine, when it isn't installed yet), or just the voice engine.
pub fn h_download(api: &Api, v: &Value) -> Result<Value> {
    let id = v["model"].as_str().unwrap_or("").to_string();
    let c = catalog(api)?;
    let root = models_root(api);
    let h = crate::voiceclone::hardware().clone();
    let kind = v["device_kind"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| torch_kind(&h).to_string());
    if !["cuda", "cpu", "mps"].contains(&kind.as_str()) {
        bail!(err("bad_params", "`device_kind` is cuda, cpu or mps."));
    }
    let model = if id == c.runtime.id {
        None
    } else {
        Some(
            c.models
                .iter()
                .find(|m| m.id == id)
                .cloned()
                .ok_or_else(|| {
                    err(
                        "not_found",
                        format!("There's no model `{id}`. clone.models lists them."),
                    )
                })?,
        )
    };
    if h.os == "macos" && h.arch == "x86_64" {
        bail!(err(
            "unavailable",
            "The models need PyTorch 2.6, which isn't made for Intel Macs."
        ));
    }
    let need_rt = runtime_ready(&root, &c).is_none();
    let todo = match &model {
        Some(m) => model_todo(&root, &c, m)?,
        None => vec![],
    };
    let pkgs = match &model {
        Some(m) if need_rt => m.install.clone(),
        Some(m) => packages_todo(&root, &c, m),
        None => vec![],
    };
    let bytes: u64 = todo
        .iter()
        .map(|(p, f)| {
            f.size
                - std::fs::metadata(part(p))
                    .map(|m| m.len().min(f.size))
                    .unwrap_or(0)
        })
        .sum();
    let py = c
        .runtime
        .python
        .get(&platform())
        .map(|a| a.size)
        .unwrap_or(0);
    let rt_bytes = if need_rt {
        c.runtime.approx_mb.get(&kind).copied().unwrap_or(1000) * 1_000_000
    } else {
        0
    };
    if v["confirm"].as_bool() != Some(true) {
        let what = model
            .as_ref()
            .map(|m| format!("{} ({} licence)", m.name, m.licence))
            .unwrap_or_else(|| c.runtime.name.clone());
        bail!(err(
            "confirm_required",
            format!("This downloads {what}: {} of model files{}, into {}. Ask the person first, then call again with `confirm: true`.",
                gb(bytes), if need_rt { format!(", plus the voice engine (about {})", gb(rt_bytes)) } else { String::new() }, root.display())
        ));
    }
    let key = active_key(&root, &id);
    if let Some(j) = active_job(api, &root, &id) {
        return Ok(json!({"job": j["job"], "already": true}));
    }
    if todo.is_empty() && !need_rt && pkgs.is_empty() {
        return Ok(json!({"installed": true, "bytes": 0}));
    }
    free_space_check(&root, bytes + rt_bytes)?;
    let title = model
        .as_ref()
        .map(|m| format!("Downloading {}", m.name))
        .unwrap_or_else(|| format!("Installing the {}", c.runtime.name.to_lowercase()));
    let mut stages: Vec<&str> = vec![];
    if need_rt {
        stages.extend(["Downloading Python", "Unpacking"]);
        stages.extend(c.runtime.steps.iter().map(|s| s.name.as_str()));
        stages.push("Checking the voice engine");
    }
    stages.extend(pkgs.iter().map(|s| s.name.as_str()));
    if model.is_some() {
        stages.push("Downloading model files");
    }
    let (c2, root2, kind2) = (c.clone(), root.clone(), kind.clone());
    let pkgs = pkgs.clone();
    let job = api
        .jobs
        .spawn("model-download", &title, &stages, move |ctx| {
            let mut out = json!({});
            let rt_share = if need_rt {
                if todo.is_empty() {
                    1.0
                } else {
                    0.35
                }
            } else {
                0.0
            };
            if need_rt {
                let mut m = Meter {
                    ctx,
                    done: 0,
                    total: py.max(1),
                    from: 0.0,
                    span: rt_share * 0.1,
                };
                out["runtime"] = install_runtime(&root2, &c2, &kind2, ctx, &mut m, rt_share)?;
                ctx.progress(rt_share);
            }
            for s in &pkgs {
                pip_step(&root2, &c2, &kind2, ctx, s)?;
                mark_package(&root2, &c2, s)?;
            }
            if !todo.is_empty() {
                ctx.stage("Downloading model files", "active");
                let total: u64 = todo.iter().map(|(_, f)| f.size).sum();
                let have: u64 = todo
                    .iter()
                    .map(|(p, f)| {
                        std::fs::metadata(part(p))
                            .map(|m| m.len().min(f.size))
                            .unwrap_or(0)
                    })
                    .sum();
                let mut m = Meter {
                    ctx,
                    done: have,
                    total,
                    from: rt_share,
                    span: 1.0 - rt_share,
                };
                let spec = model.clone().unwrap();
                for (p, f) in &todo {
                    fetch(&file_url(&c2, &spec, f)?, p, f.size, &f.sha256, &mut m)?;
                }
                ctx.stage("Downloading model files", "done");
                out["model"] = json!(spec.id);
                out["bytes"] = json!(total);
            }
            Ok(out)
        });
    active().lock().unwrap().insert(key, job.clone());
    if v["wait"].as_bool().unwrap_or(false) {
        let j = api
            .jobs
            .wait(&job, std::time::Duration::from_secs(4 * 3600))
            .unwrap();
        if j.state != "done" {
            bail!(err(
                if j.state == "cancelled" {
                    "cancelled"
                } else {
                    "failed"
                },
                j.error.unwrap_or_else(|| "Cancelled.".into())
            ));
        }
        return Ok(json!({"job": job, "result": j.result}));
    }
    Ok(json!({"job": job, "bytes": bytes, "runtime": need_rt}))
}

/// Delete a model's files (keeping any another downloaded model still uses), or the voice engine.
pub fn h_delete(api: &Api, v: &Value) -> Result<Value> {
    let id = v["model"].as_str().unwrap_or("");
    let c = catalog(api)?;
    let root = models_root(api);
    if active_job(api, &root, id).is_some() {
        bail!(err("busy", "It's downloading. Cancel the download first."));
    }
    if id == c.runtime.id {
        let d = runtime_dir(&root, &c);
        if d.exists() {
            std::fs::remove_dir_all(&d)?;
        }
        return Ok(json!({"deleted": id}));
    }
    let m = c
        .models
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| err("not_found", format!("There's no model `{id}`.")))?;
    let dir = repo_dir(&root, &c, &m.repo)?;
    let keep: std::collections::BTreeSet<String> = c
        .models
        .iter()
        .filter(|o| o.id != m.id && o.repo == m.repo && model_state(&root, &c, o).installed)
        .flat_map(|o| o.files.iter().map(|f| f.path.clone()))
        .collect();
    let mut freed = 0u64;
    for f in &m.files {
        if keep.contains(&f.path) {
            continue;
        }
        let p = dir.join(safe_rel(&f.path)?);
        for q in [p.clone(), marker(&p), part(&p)] {
            if let Ok(md) = std::fs::metadata(&q) {
                freed += md.len();
                std::fs::remove_file(&q)?;
            }
        }
    }
    Ok(
        json!({"deleted": id, "freed": freed, "freed_text": gb(freed), "kept_shared": m.files.iter().filter(|f| keep.contains(&f.path)).count()}),
    )
}

pub fn h_settings_get(api: &Api, _: &Value) -> Result<Value> {
    let c = catalog(api)?;
    let s = settings(api);
    let effective: BTreeMap<String, BTreeMap<String, f64>> = c
        .models
        .iter()
        .map(|m| (m.id.clone(), params_for(&s, m)))
        .collect();
    Ok(json!({"settings": s, "params": effective}))
}

pub fn h_settings_set(api: &Api, v: &Value) -> Result<Value> {
    let c = catalog(api)?;
    let mut s = settings(api);
    let role_ok = |id: &str, role: &str, s: &Settings| {
        resolve(&c, s, id)
            .map(|(m, _)| m.role == role)
            .unwrap_or(false)
    };
    if let Some(x) = v["speech"].as_str() {
        if !role_ok(x, "speech", &s) {
            bail!(err(
                "bad_params",
                format!("`{x}` isn't a speech model. clone.models lists them.")
            ));
        }
        s.speech = x.into();
    }
    if let Some(x) = v["convert"].as_str() {
        if !role_ok(x, "convert", &s) {
            bail!(err("bad_params", format!("`{x}` isn't a voice converter.")));
        }
        s.convert = x.into();
    }
    if let Some(x) = v["device"].as_str() {
        if !["auto", "cuda", "mps", "cpu"].contains(&x) {
            bail!(err("bad_params", "`device` is auto, cuda, mps or cpu."));
        }
        s.device = x.into();
    }
    if let Some(x) = v["language"].as_str() {
        let all: Vec<&String> = c.models.iter().flat_map(|m| m.languages.iter()).collect();
        if !all.iter().any(|l| *l == x) {
            bail!(err("bad_params", format!("No model speaks `{x}`.")));
        }
        s.language = x.into();
    }
    if let Some(x) = v.get("seed").filter(|x| !x.is_null()) {
        s.seed = x
            .as_u64()
            .ok_or_else(|| err("bad_params", "`seed` is a whole number."))?;
    }
    if let Some(obj) = v["params"].as_object() {
        for (mid, kv) in obj {
            let m = resolve(&c, &s, mid)
                .map(|x| x.0)
                .ok_or_else(|| err("bad_params", format!("There's no model `{mid}`.")))?;
            let kv = kv
                .as_object()
                .ok_or_else(|| err("bad_params", "`params` is {model: {setting: number}}."))?;
            for (k, val) in kv {
                let p = m.params.iter().find(|p| &p.key == k).ok_or_else(|| {
                    err(
                        "bad_params",
                        format!(
                            "{} has no setting `{k}`. It has: {}.",
                            m.name,
                            m.params
                                .iter()
                                .map(|p| p.key.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                })?;
                if val.is_null() {
                    s.params.entry(mid.clone()).or_default().remove(k);
                    continue;
                }
                let x = val
                    .as_f64()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| err("bad_params", format!("`{k}` is a number.")))?;
                s.params
                    .entry(mid.clone())
                    .or_default()
                    .insert(k.clone(), x.clamp(p.min, p.max));
            }
        }
    }
    save_settings(api, &s)?;
    h_settings_get(api, &Value::Null)
}

/// Add a folder holding a model in one of the supported formats (for example a fine-tuned Chatterbox).
pub fn h_add_local(api: &Api, v: &Value) -> Result<Value> {
    let c = catalog(api)?;
    let path = v["path"].as_str().unwrap_or("").trim().to_string();
    let loader = v["loader"].as_str().unwrap_or("");
    let base = c
        .models
        .iter()
        .find(|m| m.loader == loader)
        .ok_or_else(|| {
            err(
                "bad_params",
                format!(
                    "`loader` is one of: {}.",
                    c.models
                        .iter()
                        .map(|m| m.loader.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })?;
    let dir = Path::new(&path);
    if path.is_empty() || !dir.is_dir() {
        bail!(err("not_found", "That folder isn't there."));
    }
    let missing: Vec<&str> = base
        .files
        .iter()
        .filter(|f| !dir.join(&f.path).is_file())
        .map(|f| f.path.as_str())
        .collect();
    if !missing.is_empty() {
        bail!(err(
            "bad_file",
            format!(
                "That folder is missing files a {} model needs: {}.",
                base.name,
                missing.join(", ")
            )
        ));
    }
    let mut s = settings(api);
    let name = v["name"]
        .as_str()
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(String::from)
        .unwrap_or_else(|| {
            dir.file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("My model")
                .to_string()
        });
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let mut id = format!("local-{}", slug.trim_matches('-'));
    let mut n = 2;
    while s.local.iter().any(|l| l.id == id) || c.models.iter().any(|m| m.id == id) {
        id = format!("local-{}-{n}", slug.trim_matches('-'));
        n += 1;
    }
    s.local.push(LocalModel {
        id: id.clone(),
        name,
        loader: loader.into(),
        path,
    });
    save_settings(api, &s)?;
    Ok(json!({"id": id, "role": base.role}))
}

pub fn h_remove_local(api: &Api, v: &Value) -> Result<Value> {
    let id = v["id"].as_str().unwrap_or("");
    let mut s = settings(api);
    let before = s.local.len();
    s.local.retain(|l| l.id != id);
    if s.local.len() == before {
        bail!(err("not_found", "There's no added folder with that id."));
    }
    let d = Settings::default();
    if s.speech == id {
        s.speech = d.speech;
    }
    if s.convert == id {
        s.convert = d.convert;
    }
    save_settings(api, &s)?;
    Ok(json!({"removed": id, "files_kept": true}))
}

// ---------------------------------------------------------------- running a model

pub struct Ready {
    pub model: ModelSpec,
    pub ckpt: PathBuf,
    pub root: PathBuf,
    pub catalog: Catalog,
    pub settings: Settings,
}

/// The model for `role`, ready to run, or a clear `not_installed` error saying what to download.
pub fn ready_for(api: &Api, role: &str, want: Option<&str>) -> Result<Ready> {
    let c = catalog(api)?;
    let s = settings(api);
    let root = models_root(api);
    let id = want.map(String::from).unwrap_or_else(|| {
        if role == "convert" {
            s.convert.clone()
        } else {
            s.speech.clone()
        }
    });
    let (m, local) = resolve(&c, &s, &id).ok_or_else(|| {
        err(
            "not_installed",
            format!("The model `{id}` isn't known. clone.models lists them."),
        )
    })?;
    if m.role != role {
        bail!(err(
            "bad_params",
            format!(
                "{} is a {} model, not a {} one.",
                m.name,
                if m.role == "convert" {
                    "conversion"
                } else {
                    "speech"
                },
                if role == "convert" {
                    "conversion"
                } else {
                    "speech"
                }
            )
        ));
    }
    let ckpt = match &local {
        Some(l) => PathBuf::from(&l.path),
        None => {
            if !model_state(&root, &c, &m).installed {
                bail!(err("not_installed", format!("{} isn't downloaded yet ({}). Download it on the Voice cloning page, or with clone.model.download.", m.name, gb(m.files.iter().map(|f| f.size).sum()))));
            }
            repo_dir(&root, &c, &m.repo)?
        }
    };
    if runtime_ready(&root, &c).is_none() {
        bail!(err(
            "not_installed",
            format!(
                "The {} isn't installed yet. Downloading any model installs it.",
                c.runtime.name.to_lowercase()
            )
        ));
    }
    Ok(Ready {
        model: m,
        ckpt,
        root,
        catalog: c,
        settings: s,
    })
}

/// Run one request through the runner; returns its RESULT json. The output WAV is at `out`.
pub fn run_model(r: &Ready, req: Value, ctx: &JobCtx) -> Result<Value> {
    let work = runtime_dir(&r.root, &r.catalog).join("work");
    std::fs::create_dir_all(&work)?;
    let runner = write_runner(&runtime_dir(&r.root, &r.catalog))?;
    let rq = work.join(format!("{}.json", ctx.id));
    std::fs::write(&rq, serde_json::to_vec(&req)?)?;
    let mut cmd = python_cmd(&r.root, &r.catalog)?;
    cmd.arg(&runner).arg("run").arg(&rq);
    // Named after the job's own stage, so a failure says what was being done.
    let stage = match req["op"].as_str() {
        Some("stems") => "Separating",
        Some("transcribe") => "Listening",
        Some("tts") => "Synthesizing",
        _ => "Speaking",
    };
    let res = run_logged(cmd, ctx, stage);
    std::fs::remove_file(&rq).ok();
    let out = res?;
    out.lines()
        .rev()
        .find_map(|l| {
            l.strip_prefix("RESULT ")
                .and_then(|j| serde_json::from_str(j).ok())
        })
        .ok_or_else(|| err("failed", "The voice engine finished without a result."))
}

pub fn request(r: &Ready, op: &str, reference: &Path, out: &Path) -> Value {
    json!({
        "op": op, "loader": r.model.loader, "ckpt_dir": r.ckpt, "device": r.settings.device,
        "reference": reference, "out": out, "seed": r.settings.seed, "language": r.settings.language,
        "params": params_for(&r.settings, &r.model),
    })
}

/// Add a LIST/INFO chunk to a WAV saying it is synthetic speech and what made it.
pub fn label_wav(path: &Path, comment: &str) -> Result<()> {
    let mut b = std::fs::read(path)?;
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        bail!("not a WAV file");
    }
    let sub = |id: &[u8; 4], text: &str| {
        let mut t = text.as_bytes().to_vec();
        t.push(0);
        let mut c = id.to_vec();
        c.extend((t.len() as u32).to_le_bytes());
        if t.len() % 2 == 1 {
            t.push(0);
        }
        c.extend(t);
        c
    };
    let mut info = b"INFO".to_vec();
    info.extend(sub(b"ICMT", comment));
    info.extend(sub(b"ISFT", "Flux (synthetic speech)"));
    let mut chunk = b"LIST".to_vec();
    chunk.extend((info.len() as u32).to_le_bytes());
    chunk.extend(info);
    b.extend(chunk);
    let riff = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff.to_le_bytes());
    crate::store::atomic_write(path, &b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_catalog_is_pinned_and_safe() {
        let c = builtin();
        assert!(c.hub.starts_with("https://"));
        for r in c.repos.values() {
            assert_eq!(r.rev.len(), 40, "{} must be pinned to a commit", r.repo);
        }
        for m in &c.models {
            assert!(
                ["MIT", "Apache-2.0"].contains(&m.licence.as_str()),
                "{}",
                m.id
            );
            assert!(c.repos.contains_key(&m.repo));
            assert!(!m.files.is_empty());
            for f in &m.files {
                assert!(safe_rel(&f.path).is_ok());
                assert_eq!(f.sha256.len(), 64, "{}", f.path);
                assert!(f.sha256.chars().all(|ch| ch.is_ascii_hexdigit()));
                assert!(f.size > 0);
            }
            for p in &m.params {
                assert!(
                    p.min <= p.default && p.default <= p.max,
                    "{} {}",
                    m.id,
                    p.key
                );
                assert!(!p.tip.is_empty());
            }
        }
        assert!(c.models.iter().any(|m| m.role == "speech" && m.sounds));
        assert!(c.models.iter().any(|m| m.role == "convert"));
        for a in c.runtime.python.values() {
            assert!(a.url.starts_with("https://"));
            assert_eq!(a.sha256.len(), 64);
            assert!(safe_rel(&a.exe).is_ok());
        }
        for s in &c.runtime.steps {
            assert!(
                s.args.iter().all(|a| a.contains("==")),
                "pip packages are pinned: {:?}",
                s.args
            );
            for ix in s.index.values() {
                assert!(ix.starts_with("https://"));
            }
        }
    }

    #[test]
    fn unsafe_names_and_urls_are_refused() {
        for p in ["../x", "a/../b", "/etc/passwd", "C:\\x", "a//b", ""] {
            assert!(safe_rel(p).is_err(), "{p}");
        }
        assert!(safe_rel("a/b.json").is_ok());
        assert!(check_url("http://example.com/x").is_err());
        assert!(check_url("ftp://x").is_err());
        assert!(check_url("https://huggingface.co/x").is_ok());
        assert!(check_url("http://127.0.0.1:9/x").is_ok());
    }

    #[test]
    fn wavs_are_labelled_synthetic() {
        let d = std::env::temp_dir().join(format!("flux-label-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("a.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 24000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&p, spec).unwrap();
        for i in 0..2400 {
            w.write_sample(((i as f32 * 0.05).sin() * 8000.0) as i16)
                .unwrap();
        }
        w.finalize().unwrap();
        label_wav(&p, "Synthetic speech in the voice of Robin, with consent.").unwrap();
        let b = std::fs::read(&p).unwrap();
        assert_eq!(
            u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize,
            b.len() - 8
        );
        assert!(b.windows(4).any(|x| x == b"ICMT"));
        // Still reads as the same audio.
        let r = hound::WavReader::open(&p).unwrap();
        assert_eq!(r.len(), 2400);
        std::fs::remove_dir_all(&d).ok();
    }
}
