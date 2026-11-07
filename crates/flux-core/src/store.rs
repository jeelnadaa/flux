//! On-disk storage. Projects are folders of ordinary files:
//! `<data>/Projects/<id>/project.json` (atomic writes), `media/` (safe copies of imported audio,
//! never modified), `journal.jsonl` (every committed command), `snapshots/`, and a `.lock` file
//! that marks a project as open so a crash can be detected on the next start.

use crate::project::Project;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("FLUX_DATA_DIR") {
        return PathBuf::from(d);
    }
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Flux")
}

pub fn default_export_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("FLUX_EXPORT_DIR") {
        return PathBuf::from(d);
    }
    dirs::audio_dir()
        .or_else(dirs::document_dir)
        .unwrap_or_else(data_dir)
        .join("Flux")
}

/// Write via a temporary file and rename, so a crash mid-write never leaves a torn file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().ok_or_else(|| anyhow!("bad path"))?;
    std::fs::create_dir_all(dir)?;
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    {
        let mut f = std::fs::File::create(&tmp)
            .with_context(|| format!("could not write {}", tmp.display()))?;
        f.write_all(bytes)?;
        f.sync_all().ok();
    }
    // keep the previous version as .bak for recovery
    if path.exists() {
        let _ = std::fs::copy(path, path.with_extension("bak"));
    }
    std::fs::rename(&tmp, path).with_context(|| format!("could not replace {}", path.display()))?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub font: String,
    pub mode: String,
    pub language: String,
    pub reduce_motion: bool,
    pub high_contrast: bool,
    pub large_text: bool,
    pub screen_reader_hints: bool,
    pub output: Option<String>,
    pub input: Option<String>,
    pub buffer: Option<u32>,
    pub safety_limiter: bool,
    pub default_rate: u32,
    pub export_dir: Option<String>,
    pub autosave: bool,
    pub crash_reports: bool,
    pub cloud: bool,
    pub hrtf_profile: String,
    pub head_width_cm: f32,
    pub headphones_checked: bool,
    pub onboarded: bool,
    pub keys: std::collections::BTreeMap<String, String>,
    pub folders: Vec<String>,
    pub favorites: Vec<String>,
    pub recent_fx: Vec<String>,
    pub mcp_enabled: bool,
    /// Download and install new releases in the background (desktop app only).
    pub auto_update: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "cassette".into(),
            font: "bricolage".into(),
            mode: "pro".into(),
            language: "en".into(),
            reduce_motion: false,
            high_contrast: false,
            large_text: false,
            screen_reader_hints: true,
            output: None,
            input: None,
            buffer: None,
            safety_limiter: true,
            default_rate: 48000,
            export_dir: None,
            autosave: true,
            crash_reports: false,
            cloud: false,
            hrtf_profile: "C".into(),
            head_width_cm: 17.5,
            headphones_checked: false,
            onboarded: false,
            keys: Default::default(),
            folders: vec!["My Audio".into()],
            favorites: vec![],
            recent_fx: vec![],
            mcp_enabled: true,
            auto_update: true,
        }
    }
}

pub struct Store {
    pub root: PathBuf,
}

#[derive(Serialize, Clone, Debug)]
pub struct LibraryEntry {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub seconds: f64,
    pub bytes: u64,
    pub modified: u64,
    pub created: u64,
    pub tracks: usize,
    pub folder: String,
    pub missing_media: usize,
    pub open_elsewhere: bool,
    pub crashed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Preset {
    pub id: String,
    pub name: String,
    /// effect kind, or "chain" for a whole chain
    pub kind: String,
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, f32>,
    #[serde(default)]
    pub chain: Vec<crate::project::Effect>,
    #[serde(default)]
    pub created: u64,
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Action {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub steps: Vec<ActionStep>,
    #[serde(default)]
    pub created: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionStep {
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

fn dir_size(p: &Path) -> u64 {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let md = match e.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            n += if md.is_dir() {
                dir_size(&e.path())
            } else {
                md.len()
            };
        }
    }
    n
}

fn pid_alive(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .creation_flags(0x08000000)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")))
            .unwrap_or(false)
    }
    #[cfg(unix)]
    {
        Path::new(&format!("/proc/{pid}")).exists()
            || std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
    }
}

pub fn safe_id(s: &str) -> Result<&str> {
    if s.is_empty()
        || s.len() > 64
        || !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("invalid id");
    }
    Ok(s)
}

impl Store {
    pub fn open() -> Result<Store> {
        Self::at(data_dir())
    }
    pub fn at(root: PathBuf) -> Result<Store> {
        for d in ["Projects", "Presets", "Actions", "Trash"] {
            std::fs::create_dir_all(root.join(d))
                .with_context(|| format!("could not create {}", root.display()))?;
        }
        Ok(Store { root })
    }
    pub fn project_dir(&self, id: &str) -> PathBuf {
        self.root.join("Projects").join(id)
    }
    pub fn media_dir(&self, id: &str) -> PathBuf {
        self.project_dir(id).join("media")
    }

    pub fn settings(&self) -> Settings {
        std::fs::read(self.root.join("settings.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        atomic_write(
            &self.root.join("settings.json"),
            &serde_json::to_vec_pretty(s)?,
        )
    }

    pub fn save_project(&self, p: &Project) -> Result<()> {
        safe_id(&p.id)?;
        atomic_write(
            &self.project_dir(&p.id).join("project.json"),
            &serde_json::to_vec(p)?,
        )
    }

    pub fn load_project(&self, id: &str) -> Result<Project> {
        safe_id(id)?;
        let d = self.project_dir(id);
        let main = d.join("project.json");
        let parse = |p: &Path| -> Result<Project> {
            let b = std::fs::read(p)?;
            if b.len() > 256 << 20 {
                bail!("project file is too large");
            }
            let mut pr: Project = serde_json::from_slice(&b).context("project file is damaged")?;
            pr.validate()?;
            Ok(pr)
        };
        match parse(&main) {
            Ok(p) => Ok(p),
            Err(e) => parse(&main.with_extension("bak")).map_err(|_| e),
        }
    }

    pub fn journal(&self, id: &str, method: &str, params: &Value, hash: u64) {
        let line = serde_json::json!({ "t": crate::project::now_secs(), "m": method, "p": params, "h": format!("{hash:016x}") });
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.project_dir(id).join("journal.jsonl"))
        {
            let _ = writeln!(f, "{line}");
        }
    }
    pub fn journal_tail(&self, id: &str, n: usize) -> Vec<Value> {
        let s =
            std::fs::read_to_string(self.project_dir(id).join("journal.jsonl")).unwrap_or_default();
        let v: Vec<Value> = s
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        v[v.len().saturating_sub(n)..].to_vec()
    }

    pub fn lock(&self, id: &str) {
        let _ = std::fs::write(
            self.project_dir(id).join(".lock"),
            std::process::id().to_string(),
        );
    }
    pub fn unlock(&self, id: &str) {
        let _ = std::fs::remove_file(self.project_dir(id).join(".lock"));
    }
    /// (locked by a live process, locked by a dead process = crashed)
    fn lock_state(&self, id: &str) -> (bool, bool) {
        match std::fs::read_to_string(self.project_dir(id).join(".lock"))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            Some(pid) if pid == std::process::id() => (false, false),
            Some(pid) => {
                let alive = pid_alive(pid);
                (alive, !alive)
            }
            None => (false, false),
        }
    }

    pub fn library(&self) -> Vec<LibraryEntry> {
        let mut out = vec![];
        let Ok(rd) = std::fs::read_dir(self.root.join("Projects")) else {
            return out;
        };
        for e in rd.flatten() {
            let id = e.file_name().to_string_lossy().to_string();
            if safe_id(&id).is_err() {
                continue;
            }
            let Ok(p) = self.load_project(&id) else {
                continue;
            };
            let (open_elsewhere, crashed) = self.lock_state(&id);
            let missing = p
                .media
                .iter()
                .filter(|m| !Path::new(&m.path).exists())
                .count();
            out.push(LibraryEntry {
                id: id.clone(),
                name: p.name.clone(),
                kind: p.kind.clone(),
                seconds: p.length() as f64 / p.sample_rate as f64,
                bytes: dir_size(&e.path()),
                modified: p.modified,
                created: p.created,
                tracks: p.tracks.len(),
                folder: p.folder.clone(),
                missing_media: missing,
                open_elsewhere,
                crashed,
            });
        }
        out.sort_by_key(|e| std::cmp::Reverse(e.modified));
        out
    }

    pub fn crashed(&self) -> Vec<LibraryEntry> {
        self.library().into_iter().filter(|e| e.crashed).collect()
    }

    /// Move to Trash (recoverable), never a hard delete.
    pub fn trash_project(&self, id: &str) -> Result<()> {
        safe_id(id)?;
        let to = self
            .root
            .join("Trash")
            .join(format!("{id}-{}", crate::project::now_secs()));
        std::fs::rename(self.project_dir(id), to).context("could not move the project to the trash")
    }

    /// Copy an imported file into the project, unchanged. Returns the new path.
    pub fn safe_copy(&self, project: &str, media_id: &str, src: &Path) -> Result<PathBuf> {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_lowercase();
        let ext: String = ext
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(8)
            .collect();
        let d = self.media_dir(project);
        std::fs::create_dir_all(&d)?;
        let dst = d.join(format!("{media_id}.{ext}"));
        if src != dst {
            std::fs::copy(src, &dst)
                .with_context(|| format!("could not copy {}", src.display()))?;
        }
        Ok(dst)
    }

    pub fn save_snapshot(&self, p: &Project, name: &str) -> Result<String> {
        let id = format!("{}", crate::project::now_millis());
        let mut v = serde_json::to_value(p)?;
        v["snapshot_name"] = Value::String(name.to_string());
        atomic_write(
            &self
                .project_dir(&p.id)
                .join("snapshots")
                .join(format!("{id}.json")),
            &serde_json::to_vec(&v)?,
        )?;
        Ok(id)
    }
    pub fn snapshots(&self, project: &str) -> Vec<Value> {
        let mut out = vec![];
        if let Ok(rd) = std::fs::read_dir(self.project_dir(project).join("snapshots")) {
            for e in rd.flatten() {
                if let Ok(v) = std::fs::read(e.path())
                    .map_err(anyhow::Error::from)
                    .and_then(|b| Ok(serde_json::from_slice::<Value>(&b)?))
                {
                    let id = e
                        .path()
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    out.push(serde_json::json!({ "id": id, "name": v["snapshot_name"], "modified": v["modified"] }));
                }
            }
        }
        out.sort_by(|a, b| b["id"].as_str().cmp(&a["id"].as_str()));
        out
    }
    pub fn load_snapshot(&self, project: &str, id: &str) -> Result<Project> {
        safe_id(id)?;
        let b = std::fs::read(
            self.project_dir(project)
                .join("snapshots")
                .join(format!("{id}.json")),
        )?;
        let mut p: Project = serde_json::from_slice(&b)?;
        p.validate()?;
        Ok(p)
    }

    /// `.flux` package: a zip with project.json and media/.
    pub fn export_package(&self, p: &Project, to: &Path) -> Result<u64> {
        let f = std::fs::File::create(to)
            .with_context(|| format!("could not create {}", to.display()))?;
        let mut z = zip::ZipWriter::new(f);
        let opt = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut pc = p.clone();
        for m in &mut pc.media {
            let name = Path::new(&m.path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            m.path = format!("media/{name}");
        }
        z.start_file("project.json", opt)?;
        z.write_all(&serde_json::to_vec_pretty(&pc)?)?;
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for m in &p.media {
            let src = Path::new(&m.path);
            if !src.exists() {
                continue;
            }
            let name = src
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            z.start_file(format!("media/{name}"), stored)?;
            let mut r = std::fs::File::open(src)?;
            std::io::copy(&mut r, &mut z)?;
        }
        z.finish()?;
        Ok(std::fs::metadata(to)?.len())
    }

    pub fn import_package(&self, from: &Path) -> Result<Project> {
        let f = std::fs::File::open(from)
            .with_context(|| format!("could not open {}", from.display()))?;
        let mut z = zip::ZipArchive::new(f).context("this is not a .flux file")?;
        let mut pj = String::new();
        z.by_name("project.json")
            .context("the .flux file has no project")?
            .take(64 << 20)
            .read_to_string(&mut pj)?;
        let mut p: Project =
            serde_json::from_str(&pj).context("the project in the .flux file is damaged")?;
        p.validate()?;
        p.id = new_id();
        let md = self.media_dir(&p.id);
        std::fs::create_dir_all(&md)?;
        for i in 0..z.len() {
            let mut e = z.by_index(i)?;
            let Some(name) = e.enclosed_name() else {
                continue;
            };
            let Ok(rest) = name.strip_prefix("media") else {
                continue;
            };
            let Some(fname) = rest.file_name() else {
                continue;
            };
            let dst = md.join(fname);
            let mut out = std::fs::File::create(&dst)?;
            // bounded copy: 4 GB per file
            std::io::copy(&mut (&mut e).take(4 << 30), &mut out)?;
        }
        for m in &mut p.media {
            let fname = Path::new(&m.path)
                .file_name()
                .map(|s| s.to_os_string())
                .unwrap_or_default();
            m.path = md.join(fname).display().to_string();
        }
        self.save_project(&p)?;
        Ok(p)
    }

    // presets and actions
    pub fn presets(&self) -> Vec<Preset> {
        let mut v: Vec<Preset> = read_dir_json(&self.root.join("Presets"));
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
    pub fn save_preset(&self, p: &Preset) -> Result<()> {
        safe_id(&p.id)?;
        atomic_write(
            &self.root.join("Presets").join(format!("{}.json", p.id)),
            &serde_json::to_vec_pretty(p)?,
        )
    }
    pub fn delete_preset(&self, id: &str) -> Result<()> {
        safe_id(id)?;
        std::fs::remove_file(self.root.join("Presets").join(format!("{id}.json")))
            .context("no such preset")
    }
    pub fn actions(&self) -> Vec<Action> {
        let mut v: Vec<Action> = read_dir_json(&self.root.join("Actions"));
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
    pub fn save_action(&self, a: &Action) -> Result<()> {
        safe_id(&a.id)?;
        atomic_write(
            &self.root.join("Actions").join(format!("{}.json", a.id)),
            &serde_json::to_vec_pretty(a)?,
        )
    }
    pub fn delete_action(&self, id: &str) -> Result<()> {
        safe_id(id)?;
        std::fs::remove_file(self.root.join("Actions").join(format!("{id}.json")))
            .context("no such action")
    }

    pub fn usage(&self) -> Value {
        serde_json::json!({
            "root": self.root.display().to_string(),
            "projects": dir_size(&self.root.join("Projects")),
            "trash": dir_size(&self.root.join("Trash")),
            "presets": dir_size(&self.root.join("Presets")),
            "free": free_space(&self.root),
        })
    }
    pub fn empty_trash(&self) -> Result<u64> {
        let t = self.root.join("Trash");
        let n = dir_size(&t);
        std::fs::remove_dir_all(&t)?;
        std::fs::create_dir_all(&t)?;
        Ok(n)
    }
}

fn free_space(_p: &Path) -> Option<u64> {
    None
}

fn read_dir_json<T: serde::de::DeserializeOwned>(d: &Path) -> Vec<T> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(d) {
        for e in rd.flatten() {
            if e.path().extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            if let Ok(b) = std::fs::read(e.path()) {
                if b.len() < 4 << 20 {
                    if let Ok(v) = serde_json::from_slice(&b) {
                        out.push(v);
                    }
                }
            }
        }
    }
    out
}

pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let t = crate::project::now_millis();
    let n = N.fetch_add(1, Ordering::Relaxed);
    let mut r = crate::dsp::Rng::new(t ^ (std::process::id() as u64) << 20 ^ n);
    format!("{:x}{:04x}", t, r.next_u64() & 0xffff)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("flux-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }
    #[test]
    fn save_load_round_trip_and_library() {
        let s = Store::at(tmp("a")).unwrap();
        let mut p = Project::new("Song", 48000);
        p.id = new_id();
        p.add_track("A");
        s.save_project(&p).unwrap();
        let q = s.load_project(&p.id).unwrap();
        assert_eq!(p.hash(), q.hash());
        assert_eq!(s.library().len(), 1);
    }
    #[test]
    fn kill_mid_write_keeps_last_good_version() {
        let s = Store::at(tmp("b")).unwrap();
        let mut p = Project::new("Song", 48000);
        p.id = new_id();
        s.save_project(&p).unwrap();
        p.name = "Second".into();
        s.save_project(&p).unwrap();
        // simulate a crash that left a torn main file: the .bak is used
        let main = s.project_dir(&p.id).join("project.json");
        std::fs::write(&main, b"{\"version\":1,\"na").unwrap();
        let q = s.load_project(&p.id).unwrap();
        assert_eq!(q.name, "Song");
        // and a crash before rename leaves only a stray tmp file, which is ignored
        std::fs::write(main.with_extension("tmp999"), b"garbage").unwrap();
        assert!(s.load_project(&p.id).is_ok());
    }
    #[test]
    fn hostile_project_files_are_rejected() {
        let s = Store::at(tmp("c")).unwrap();
        let id = "abc";
        std::fs::create_dir_all(s.project_dir(id)).unwrap();
        for bad in [
            &b"null"[..],
            b"[]",
            b"{\"tracks\":5}",
            b"\xff\xfe",
            b"{\"version\":1,\"id\":\"../../x\",\"name\":\"x\",\"sample_rate\":0}",
        ] {
            std::fs::write(s.project_dir(id).join("project.json"), bad).unwrap();
            let _ = std::fs::remove_file(s.project_dir(id).join("project.bak"));
            assert!(s.load_project(id).is_err());
        }
        assert!(s.load_project("../etc").is_err());
    }
    #[test]
    fn package_round_trip() {
        let s = Store::at(tmp("d")).unwrap();
        let mut p = Project::new("Pkg", 48000);
        p.id = new_id();
        let src = s.root.join("src.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&src, spec).unwrap();
        for i in 0..4800 {
            w.write_sample((i % 100) as i16).unwrap();
        }
        w.finalize().unwrap();
        let copy = s.safe_copy(&p.id, "m1", &src).unwrap();
        p.media.push(crate::project::MediaRef {
            id: "m1".into(),
            path: copy.display().to_string(),
            frames: 4800,
            ..Default::default()
        });
        s.save_project(&p).unwrap();
        let pkg = s.root.join("x.flux");
        s.export_package(&p, &pkg).unwrap();
        let q = s.import_package(&pkg).unwrap();
        assert_ne!(q.id, p.id);
        assert_eq!(
            std::fs::read(&q.media[0].path).unwrap(),
            std::fs::read(&src).unwrap()
        );
        // a zip with a path-traversal entry cannot write outside the project
        let evil = s.root.join("evil.flux");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("project.json", o).unwrap();
        z.write_all(&serde_json::to_vec(&p).unwrap()).unwrap();
        z.start_file("../../evil.txt", o).unwrap();
        z.write_all(b"x").unwrap();
        z.finish().unwrap();
        let _ = s.import_package(&evil);
        assert!(
            !s.root.join("evil.txt").exists()
                && !s.root.parent().unwrap().join("evil.txt").exists()
        );
    }
}
