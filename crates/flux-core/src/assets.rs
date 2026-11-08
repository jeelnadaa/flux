//! Generated audio you can listen to before it goes anywhere ("takes"), and each project's assets.
//!
//! A take lives in the data folder, outside any project, so generating needs no open project. Saving copies it into a
//! project's assets, open or closed. An asset is a file in a project's media pool: it's in the project whether or not
//! it is on the timeline.

use crate::api::{err, ob, opos, os, ou, s_, Api};
use crate::media::{decode_file, Decoded, Media};
use crate::project::{now_secs, MediaRef, Project};
use crate::store::{atomic_write, new_id, safe_id};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Saved {
    pub project: String,
    pub media: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Take {
    pub id: String,
    pub name: String,
    /// "speech", "cloned speech" or "converted take".
    pub kind: String,
    pub voice: String,
    pub seconds: f64,
    pub created: u64,
    #[serde(default)]
    pub sounds_left_out: usize,
    #[serde(default)]
    pub saved: Vec<Saved>,
    pub synthetic: bool,
}

fn dir(api: &Api) -> Result<PathBuf> {
    let d = api.lock().store.takes_dir();
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

/// A new take's id and the file its audio should be written to.
pub fn new_take(api: &Api) -> Result<(String, PathBuf)> {
    let id = format!("m{}", new_id());
    let p = dir(api)?.join(format!("{id}.wav"));
    Ok((id, p))
}

/// Does this call want the result held to be listened to first?
pub fn wants_take(v: &Value) -> Result<bool> {
    match os(v, "to").as_deref() {
        None | Some("timeline") => Ok(false),
        Some("take") => Ok(true),
        Some(o) => bail!(err(
            "bad_params",
            format!("`to` is \"take\" or \"timeline\", not \"{o}\".")
        )),
    }
}

fn write_meta(api: &Api, t: &Take) -> Result<()> {
    atomic_write(
        &dir(api)?.join(format!("{}.json", t.id)),
        &serde_json::to_vec(t)?,
    )
}

pub struct NewTake<'a> {
    pub id: &'a str,
    pub path: &'a Path,
    pub short: &'a str,
    pub kind: &'a str,
    pub voice: &'a str,
    pub sounds_left_out: usize,
}

/// Record a finished generation as a take.
pub fn finish(api: &Api, n: NewTake) -> Result<Value> {
    let d = decode_file(n.path)?;
    let seconds = d.data.first().map_or(0, Vec::len) as f64 / d.rate.max(1) as f64;
    let t = Take {
        id: n.id.to_string(),
        name: format!("Synthetic speech: {}", n.short),
        kind: n.kind.to_string(),
        voice: n.voice.to_string(),
        seconds,
        created: now_secs(),
        sounds_left_out: n.sounds_left_out,
        saved: vec![],
        synthetic: true,
    };
    write_meta(api, &t)?;
    Ok(
        json!({"take": t, "seconds": seconds, "synthetic": true, "sounds_left_out": n.sounds_left_out}),
    )
}

fn load(api: &Api, id: &str) -> Result<Take> {
    safe_id(id)?;
    let b = std::fs::read(dir(api)?.join(format!("{id}.json")))
        .map_err(|_| err("not_found", "That generated audio is gone."))?;
    Ok(serde_json::from_slice(&b)?)
}

pub fn h_take_list(api: &Api, _: &Value) -> Result<Value> {
    let mut out: Vec<Take> = std::fs::read_dir(dir(api)?)?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let t: Take = serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok()?;
            e.path().with_extension("wav").is_file().then_some(t)
        })
        .collect();
    out.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
    Ok(json!({"takes": out}))
}

pub fn h_take_delete(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    load(api, &id)?;
    let d = dir(api)?;
    std::fs::remove_file(d.join(format!("{id}.wav"))).ok();
    std::fs::remove_file(d.join(format!("{id}.json")))?;
    Ok(json!({"ok": true}))
}

fn is_open(api: &Api, pid: &str) -> bool {
    api.lock().project.as_ref().is_some_and(|p| p.id == pid)
}

/// The project a call is about: `project`, or the open one.
fn target(api: &Api, v: &Value) -> Result<String> {
    if let Some(p) = os(v, "project") {
        safe_id(&p)?;
        return Ok(p);
    }
    api.lock()
        .project
        .as_ref()
        .map(|p| p.id.clone())
        .ok_or_else(|| err("no_project", "Choose a project."))
}

fn project_rate(api: &Api, pid: &str) -> Result<u32> {
    let s = api.lock();
    match s.project.as_ref().filter(|p| p.id == pid) {
        Some(p) => Ok(p.sample_rate),
        None => Ok(s.store.load_project(pid)?.sample_rate),
    }
}

/// Add a file already in the project's media folder to its assets. Undoable when the project is open.
fn add_ref(api: &Api, pid: &str, mut m: MediaRef, d: Decoded) -> Result<()> {
    let sr = project_rate(api, pid)?;
    let media = Media::new(&m.id, d.data, d.rate, sr);
    m.frames = media.frames as i64;
    m.channels = d.channels;
    m.source_rate = d.rate;
    m.format = d.format;
    m.bits = d.bits;
    m.tags = d.tags;
    let mut s = api.lock();
    if s.project.as_ref().is_some_and(|p| p.id == pid) {
        s.add_media(media);
        s.commit(&format!("Add {} to assets", m.name), |p| {
            p.media.push(m);
            Ok(json!({}))
        })?;
    } else {
        if s.store.lock_state(pid).0 {
            bail!(err(
                "in_use",
                "That project is open in another window. Close it there first."
            ));
        }
        let mut p = s.store.load_project(pid)?;
        p.media.push(m);
        p.modified = now_secs();
        s.store.save_project(&p)?;
    }
    Ok(())
}

/// Save a take into a project's assets: the open one, another one, or a new one that isn't opened.
pub fn h_take_save(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let mut t = load(api, &id)?;
    let pid = match os(v, "new_project").filter(|n| !n.trim().is_empty()) {
        Some(name) => {
            let s = api.lock();
            let mut p = Project::new(name.trim(), s.settings.default_rate);
            p.id = new_id();
            crate::methods::template(&mut p, "blank");
            s.store.save_project(&p)?;
            p.id
        }
        None => target(api, v)?,
    };
    if t.saved.iter().any(|s| s.project == pid) {
        return Ok(json!({"project": pid, "media": id, "already": true}));
    }
    let src = dir(api)?.join(format!("{id}.wav"));
    let d = decode_file(&src)?;
    let copy = api.lock().store.safe_copy(&pid, &id, &src)?;
    let m = MediaRef {
        id: id.clone(),
        name: t.name.clone(),
        path: copy.display().to_string(),
        synthetic: true,
        ..Default::default()
    };
    add_ref(api, &pid, m, d)?;
    t.saved.push(Saved {
        project: pid.clone(),
        media: id.clone(),
    });
    write_meta(api, &t)?;
    Ok(json!({"project": pid, "media": id, "open": is_open(api, &pid)}))
}

/// A project's assets, and how many times each is on its timeline.
pub fn h_asset_list(api: &Api, v: &Value) -> Result<Value> {
    let open = api.lock().project.as_ref().map(|p| p.id.clone());
    let Some(pid) = os(v, "project").or(open.clone()) else {
        return Ok(json!({"project": null, "assets": []}));
    };
    safe_id(&pid)?;
    let p = {
        let s = api.lock();
        match s.project.as_ref().filter(|p| p.id == pid) {
            Some(p) => p.clone(),
            None => s.store.load_project(&pid)?,
        }
    };
    let sr = p.sample_rate.max(1) as f64;
    let assets: Vec<Value> = p
        .media
        .iter()
        .map(|m| {
            let uses = p
                .tracks
                .iter()
                .flat_map(|t| &t.clips)
                .filter(|c| c.media == m.id)
                .count();
            json!({"id": m.id, "name": m.name, "seconds": m.frames as f64 / sr, "channels": m.channels, "format": m.format, "synthetic": m.synthetic, "uses": uses})
        })
        .collect();
    Ok(
        json!({"project": {"id": p.id, "name": p.name}, "open": open.as_deref() == Some(p.id.as_str()), "assets": assets}),
    )
}

/// Add a file to a project's assets, leaving the timeline alone. Works on a project that isn't open.
pub fn h_asset_add(api: &Api, v: &Value) -> Result<Value> {
    let path = PathBuf::from(s_(v, "path")?);
    if !path.is_file() {
        bail!(err("not_found", "That file doesn't exist"));
    }
    let pid = target(api, v)?;
    project_rate(api, &pid)?;
    let fname = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let a = api.clone();
    let title = format!("Adding {fname}");
    let job = api.jobs.spawn(
        "asset",
        &title,
        &["Reading audio", "Saving a safe copy in your project"],
        move |ctx| {
            ctx.stage("Reading audio", "active");
            let d = decode_file(&path)?;
            ctx.stage("Reading audio", "done");
            ctx.stage("Saving a safe copy in your project", "active");
            let id = format!("m{}", new_id());
            let copy = a.lock().store.safe_copy(&pid, &id, &path)?;
            let m = MediaRef {
                id: id.clone(),
                name: fname,
                path: copy.display().to_string(),
                original: path.display().to_string(),
                ..Default::default()
            };
            add_ref(&a, &pid, m, d)?;
            ctx.stage("Saving a safe copy in your project", "done");
            Ok(json!({"media": id, "project": pid}))
        },
    );
    if ob(v, "wait").unwrap_or(false) {
        let j = api
            .jobs
            .wait(&job, std::time::Duration::from_secs(600))
            .unwrap();
        if j.state == "failed" {
            bail!(err("failed", j.error.unwrap_or_default()));
        }
        return Ok(json!({"job": job, "result": j.result}));
    }
    Ok(json!({"job": job}))
}

/// Put an asset of the open project on the timeline as one undoable step.
pub fn h_asset_place(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "media")?;
    let track = ou(v, "track");
    let mut s = api.lock();
    let sr = s.project()?.sample_rate;
    let at = opos(v, "at", sr);
    let pos = at
        .unwrap_or_else(|| s.engine.as_ref().map(|e| e.meters(0).pos).unwrap_or(0))
        .max(0);
    s.commit("Add to timeline", |p| {
        let m = p
            .media
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or_else(|| err("not_found", "That asset isn't in this project."))?;
        let stem = Path::new(&m.name)
            .file_stem()
            .map(|x| x.to_string_lossy().to_string())
            .filter(|_| !m.synthetic)
            .unwrap_or_else(|| m.name.clone());
        let name = if m.synthetic {
            "Generated speech".to_string()
        } else {
            stem.clone()
        };
        let free = |t: &crate::project::Track| {
            t.clips
                .iter()
                .all(|c| c.end() <= pos || c.start >= pos + m.frames)
        };
        let tid = match track {
            Some(t) => t,
            None => {
                let usable = p.tracks.iter().find(|t| {
                    free(t)
                        && ((m.synthetic && t.name == name) || (!m.synthetic && t.clips.is_empty()))
                });
                match usable {
                    Some(t) => t.id,
                    None => {
                        let c = p.tracks.len();
                        let t = p.add_track(&name);
                        p.track_mut(t)?.color = (c % 8 + 1) as u8;
                        t
                    }
                }
            }
        };
        let cid = p.add_clip(tid, &id, pos, 0, None)?;
        p.clip_mut(cid)?.name = if m.synthetic {
            m.name.replacen("Synthetic speech: ", "Synthetic · ", 1)
        } else {
            stem
        };
        Ok(json!({"clip": cid, "track": tid, "at": pos}))
    })
}

/// Take an asset out of the open project. Undoable. One still on the timeline stays.
pub fn h_asset_remove(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "media")?;
    let mut s = api.lock();
    s.commit("Remove asset", |p| {
        if !p.media.iter().any(|m| m.id == id) {
            bail!(err("not_found", "That asset isn't in this project."));
        }
        let uses = p
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .filter(|c| c.media == id)
            .count();
        if uses > 0 {
            bail!(err(
                "in_use",
                "That asset is on the timeline. Remove its clips first."
            ));
        }
        p.media.retain(|m| m.id != id);
        Ok(json!({"ok": true}))
    })
}

const KEEP_PREVIEWS: usize = 6;
const MAX_READ: u64 = 4 << 20;

/// Make something playable for the listen button: decoded, as a plain 16-bit WAV, so any source format plays.
pub fn h_preview(api: &Api, v: &Value) -> Result<Value> {
    let src = if let Some(t) = os(v, "take") {
        load(api, &t)?;
        dir(api)?.join(format!("{t}.wav"))
    } else {
        let id = s_(v, "media")?;
        let pid = target(api, v)?;
        let s = api.lock();
        let p = match s.project.as_ref().filter(|p| p.id == pid) {
            Some(p) => p.clone(),
            None => s.store.load_project(&pid)?,
        };
        PathBuf::from(
            p.media
                .iter()
                .find(|m| m.id == id)
                .ok_or_else(|| err("not_found", "That asset isn't in this project."))?
                .path
                .clone(),
        )
    };
    let d = decode_file(&src)?;
    let cache = api.lock().store.preview_dir();
    std::fs::create_dir_all(&cache)?;
    trim_cache(&cache);
    let token = new_id();
    let out = cache.join(format!("{token}.wav"));
    let spec = hound::WavSpec {
        channels: d.data.len().clamp(1, 2) as u16,
        sample_rate: d.rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&out, spec)?;
    let n = d.data.first().map_or(0, Vec::len);
    for i in 0..n {
        for c in d.data.iter().take(2) {
            w.write_sample((c[i].clamp(-1.0, 1.0) * 32767.0).round() as i16)?;
        }
    }
    w.finalize()?;
    Ok(
        json!({"token": token, "bytes": std::fs::metadata(&out)?.len(), "seconds": n as f64 / d.rate.max(1) as f64}),
    )
}

fn trim_cache(cache: &Path) {
    let Ok(rd) = std::fs::read_dir(cache) else {
        return;
    };
    let mut files: Vec<_> = rd
        .flatten()
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    files.sort();
    let extra = files.len().saturating_sub(KEEP_PREVIEWS - 1);
    for (_, p) in files.into_iter().take(extra) {
        std::fs::remove_file(p).ok();
    }
}

/// A piece of a prepared preview, base64.
pub fn h_preview_read(api: &Api, v: &Value) -> Result<Value> {
    let token = s_(v, "token")?;
    safe_id(&token)?;
    let at = ou(v, "at").unwrap_or(0) as u64;
    let path = api.lock().store.preview_dir().join(format!("{token}.wav"));
    let mut f =
        std::fs::File::open(path).map_err(|_| err("not_found", "That preview has expired."))?;
    let size = f.metadata()?.len();
    f.seek(SeekFrom::Start(at.min(size)))?;
    let mut b = vec![0u8; MAX_READ.min(size.saturating_sub(at)) as usize];
    f.read_exact(&mut b)?;
    Ok(json!({"data": base64(&b), "at": at, "more": at + (b.len() as u64) < size}))
}

fn base64(b: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        s.push(A[(n >> 18) as usize & 63] as char);
        s.push(A[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    s
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_matches_the_standard_vectors() {
        for (a, b) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(a.as_bytes()), b);
        }
    }
}
