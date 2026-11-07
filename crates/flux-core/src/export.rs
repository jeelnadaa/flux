//! Export: normalization, TPDF dither (only here), encoding and a verification pass that
//! decodes the written file and measures it.

use crate::analysis;
use crate::dsp::{db_to_lin, resample, Rng};
use crate::fx::{self, Ctx};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ExportOpts {
    pub format: String,
    pub sample_rate: Option<u32>,
    /// 16, 24 or 32 (32 = float for WAV)
    pub bits: u16,
    pub bitrate: u32,
    /// 0..10 for Vorbis/VBR
    pub quality: f32,
    pub vbr: bool,
    pub mono: bool,
    /// "none" | "lufs" | "peak"
    pub normalize: String,
    pub target: f32,
    pub ceiling: f32,
    pub dither: bool,
    /// Fade the end of the export to silence over this many seconds (0 = off).
    pub fade_out_s: f32,
    pub tags: BTreeMap<String, String>,
    pub seed: u64,
}
impl Default for ExportOpts {
    fn default() -> Self {
        ExportOpts {
            format: "wav".into(),
            sample_rate: None,
            bits: 24,
            bitrate: 320,
            quality: 6.0,
            vbr: false,
            mono: false,
            normalize: "none".into(),
            target: -14.0,
            ceiling: -1.0,
            dither: true,
            fade_out_s: 0.0,
            tags: BTreeMap::new(),
            seed: 1,
        }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct ExportReport {
    pub path: String,
    pub format: String,
    pub bytes: u64,
    pub seconds: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub integrated: f64,
    pub true_peak: f64,
    pub lra: f64,
    pub gain_applied_db: f32,
    pub limited: bool,
    pub verified: bool,
    pub encoder: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct FormatInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub ext: &'static str,
    pub lossless: bool,
    pub available: bool,
    pub reason: Option<String>,
}

pub fn find_tool(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(&exe);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn aac_tool() -> Option<(&'static str, PathBuf)> {
    if let Some(p) = find_tool("ffmpeg") {
        return Some(("ffmpeg", p));
    }
    if cfg!(target_os = "macos") {
        if let Some(p) = find_tool("afconvert") {
            return Some(("afconvert", p));
        }
    }
    None
}

pub fn formats() -> Vec<FormatInfo> {
    let aac = aac_tool();
    let ff = find_tool("ffmpeg");
    let ext_reason = |ok: bool, what: &str| {
        if ok {
            None
        } else {
            Some(format!(
                "{what} needs ffmpeg on this computer. Install it and restart Flux."
            ))
        }
    };
    vec![
        FormatInfo {
            id: "wav",
            name: "WAV",
            ext: "wav",
            lossless: true,
            available: true,
            reason: None,
        },
        FormatInfo {
            id: "flac",
            name: "FLAC",
            ext: "flac",
            lossless: true,
            available: true,
            reason: None,
        },
        FormatInfo {
            id: "mp3",
            name: "MP3",
            ext: "mp3",
            lossless: false,
            available: true,
            reason: None,
        },
        FormatInfo {
            id: "ogg",
            name: "OGG Vorbis",
            ext: "ogg",
            lossless: false,
            available: true,
            reason: None,
        },
        FormatInfo {
            id: "aac",
            name: "AAC (M4A)",
            ext: "m4a",
            lossless: false,
            available: aac.is_some(),
            reason: ext_reason(aac.is_some(), "AAC"),
        },
        FormatInfo {
            id: "m4r",
            name: "iPhone ringtone (M4R)",
            ext: "m4r",
            lossless: false,
            available: aac.is_some(),
            reason: ext_reason(aac.is_some(), "M4R"),
        },
        FormatInfo {
            id: "opus",
            name: "Opus",
            ext: "opus",
            lossless: false,
            available: ff.is_some(),
            reason: ext_reason(ff.is_some(), "Opus"),
        },
    ]
}

pub fn ext_for(format: &str) -> &'static str {
    match format {
        "flac" => "flac",
        "mp3" => "mp3",
        "ogg" => "ogg",
        "aac" | "m4a" => "m4a",
        "m4r" => "m4r",
        "opus" => "opus",
        _ => "wav",
    }
}

/// Gain then a true-peak-safe limiter, so the export meets the target and the ceiling.
pub fn normalize(l: &mut Vec<f32>, r: &mut Vec<f32>, sr: u32, o: &ExportOpts) -> (f32, bool) {
    let mut gain_db = 0.0f32;
    match o.normalize.as_str() {
        "lufs" => {
            let m = analysis::loudness(l, r, sr as f32);
            if m.integrated > -69.0 {
                gain_db = (o.target as f64 - m.integrated) as f32;
            }
        }
        "peak" => {
            let tp = analysis::true_peak(l).max(analysis::true_peak(r));
            if tp > 1e-6 {
                gain_db = o.target.min(0.0) - crate::dsp::lin_to_db(tp);
            }
        }
        _ => {}
    }
    let g = db_to_lin(gain_db.clamp(-60.0, 40.0));
    for v in l.iter_mut().chain(r.iter_mut()) {
        *v *= g;
    }
    let tp = analysis::true_peak(l).max(analysis::true_peak(r));
    let ceil = db_to_lin(o.ceiling.min(0.0));
    if o.normalize == "none" || tp <= ceil {
        return (gain_db, false);
    }
    // limit: look-ahead limiter at the ceiling minus a margin for inter-sample peaks
    let spec = fx::spec("limiter").unwrap();
    let mut params = BTreeMap::new();
    params.insert("ceiling".to_string(), o.ceiling - 0.5);
    params.insert("release".to_string(), 60.0);
    let p = fx::resolve(spec, &params);
    let mut lim = fx::build("limiter", sr as f32, &fx::Extra::default()).unwrap();
    lim.set(&p);
    let lat = lim.latency();
    l.extend(std::iter::repeat_n(0.0, lat));
    r.extend(std::iter::repeat_n(0.0, lat));
    let ctx = Ctx {
        sr: sr as f32,
        pos: 0,
        len: l.len() as i64,
        tempo: 120.0,
    };
    for (a, b) in l.chunks_mut(fx::BLOCK).zip(r.chunks_mut(fx::BLOCK)) {
        lim.process(a, b, &ctx);
    }
    l.drain(..lat);
    r.drain(..lat);
    // final safety: any residual overs from the oversampled peak are clamped
    for v in l.iter_mut().chain(r.iter_mut()) {
        *v = v.clamp(-ceil, ceil);
    }
    (gain_db, true)
}

fn to_int(x: f32, bits: u16, dither: bool, rng: &mut Rng) -> i32 {
    let max = ((1i64 << (bits - 1)) - 1) as f32;
    let d = if dither { rng.f() - rng.f() } else { 0.0 };
    (x * max + d).round().clamp(-max - 1.0, max) as i32
}

/// Write planar stereo audio. Returns the encoder used.
pub fn encode(l: &[f32], r: &[f32], sr: u32, o: &ExportOpts, path: &Path) -> Result<String> {
    let ch: u16 = if o.mono { 1 } else { 2 };
    let mono: Vec<f32>;
    let chans: Vec<&[f32]> = if o.mono {
        mono = l.iter().zip(r).map(|(a, b)| (a + b) * 0.5).collect();
        vec![&mono]
    } else {
        vec![l, r]
    };
    let n = l.len();
    let mut rng = Rng::new(o.seed ^ 0xD17E);
    let tmp = path.with_extension("part");
    let res = (|| -> Result<String> {
        match o.format.as_str() {
            "wav" => {
                let float = o.bits == 32;
                let spec = hound::WavSpec {
                    channels: ch,
                    sample_rate: sr,
                    bits_per_sample: if float { 32 } else { o.bits.clamp(16, 24) },
                    sample_format: if float {
                        hound::SampleFormat::Float
                    } else {
                        hound::SampleFormat::Int
                    },
                };
                let mut w = hound::WavWriter::create(&tmp, spec)?;
                for i in 0..n {
                    for c in &chans {
                        if float {
                            w.write_sample(c[i])?;
                        } else {
                            w.write_sample(to_int(c[i], spec.bits_per_sample, o.dither, &mut rng))?;
                        }
                    }
                }
                w.finalize()?;
                Ok(format!(
                    "WAV {}-bit{}",
                    spec.bits_per_sample,
                    if float { " float" } else { "" }
                ))
            }
            "flac" => {
                use flacenc::component::BitRepr;
                use flacenc::error::Verify;
                let bits = if o.bits >= 24 { 24 } else { 16 };
                let mut ints = Vec::with_capacity(n * ch as usize);
                for i in 0..n {
                    for c in &chans {
                        ints.push(to_int(c[i], bits, o.dither, &mut rng));
                    }
                }
                let cfg = flacenc::config::Encoder::default()
                    .into_verified()
                    .map_err(|e| anyhow!("flac config: {e:?}"))?;
                let src = flacenc::source::MemSource::from_samples(
                    &ints,
                    ch as usize,
                    bits as usize,
                    sr as usize,
                );
                let stream = flacenc::encode_with_fixed_block_size(&cfg, src, cfg.block_size)
                    .map_err(|e| anyhow!("flac: {e:?}"))?;
                let mut sink = flacenc::bitsink::ByteSink::new();
                stream
                    .write(&mut sink)
                    .map_err(|e| anyhow!("flac write: {e:?}"))?;
                let mut bytes = sink.as_slice().to_vec();
                // STREAMINFO min block size excludes the last block; with a fixed block size it
                // equals the max. flacenc writes the short last block here, which some readers reject.
                if bytes.len() > 12 && &bytes[..4] == b"fLaC" {
                    bytes[8] = bytes[10];
                    bytes[9] = bytes[11];
                }
                std::fs::write(&tmp, bytes)?;
                Ok(format!("FLAC {bits}-bit"))
            }
            "mp3" => {
                use rusty_mp3::{vbr_quality_index, Mp3Encoder, Mp3EncoderConfig};
                let sr2 = [48000, 44100, 32000]
                    .iter()
                    .copied()
                    .find(|r| *r == sr)
                    .ok_or_else(|| anyhow!("MP3 needs 32, 44.1 or 48 kHz"))?;
                let mut enc = Mp3Encoder::new(Mp3EncoderConfig {
                    bitrate_kbps: o.bitrate.clamp(32, 320),
                    vbr_quality: if o.vbr {
                        Some(vbr_quality_index(
                            (9.0 - o.quality.clamp(0.0, 9.0)).max(0.0),
                        ))
                    } else {
                        None
                    },
                });
                let mut out: Vec<u8> = Vec::new();
                let mut inter = Vec::with_capacity(1152 * 8 * ch as usize);
                for start in (0..n).step_by(1152 * 8) {
                    inter.clear();
                    for i in start..(start + 1152 * 8).min(n) {
                        for c in &chans {
                            inter.push(c[i].clamp(-1.0, 1.0));
                        }
                    }
                    enc.push_pcm_f32(&inter, ch, sr2)
                        .map_err(|e| anyhow!("mp3: {e:?}"))?;
                    while let Ok(p) = enc.next_packet() {
                        out.extend_from_slice(&p);
                    }
                }
                enc.finish();
                while let Ok(p) = enc.next_packet() {
                    out.extend_from_slice(&p);
                }
                // the first frame was a placeholder; the finished Info tag makes the file gapless
                if let Some(tag) = enc.lame_tag_frame() {
                    if out.len() >= tag.len() {
                        out[..tag.len()].copy_from_slice(tag);
                    }
                }
                std::fs::write(&tmp, out)?;
                Ok(format!(
                    "MP3 {} kbps{}",
                    o.bitrate,
                    if o.vbr { " VBR" } else { "" }
                ))
            }
            "ogg" => {
                use std::num::{NonZeroU32, NonZeroU8};
                use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};
                let f = std::fs::File::create(&tmp)?;
                let mut b = VorbisEncoderBuilder::new(
                    NonZeroU32::new(sr).unwrap(),
                    NonZeroU8::new(ch as u8).unwrap(),
                    f,
                )?;
                b.bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
                    target_quality: (o.quality / 10.0).clamp(-0.1, 1.0),
                });
                let mut enc = b.build()?;
                for start in (0..n).step_by(4096) {
                    let e = (start + 4096).min(n);
                    let blk: Vec<&[f32]> = chans.iter().map(|c| &c[start..e]).collect();
                    enc.encode_audio_block(&blk)?;
                }
                enc.finish()?;
                Ok(format!("OGG Vorbis q{:.0}", o.quality))
            }
            "aac" | "m4a" | "m4r" | "opus" => {
                let wav = path.with_extension("src.wav");
                let mut wo = o.clone();
                wo.format = "wav".into();
                wo.bits = 32;
                encode(l, r, sr, &wo, &wav)?;
                let result = if o.format == "opus" {
                    let ff = find_tool("ffmpeg")
                        .ok_or_else(|| anyhow!("Opus needs ffmpeg on this computer"))?;
                    run(std::process::Command::new(ff)
                        .args(["-y", "-loglevel", "error", "-i"])
                        .arg(&wav)
                        .args([
                            "-c:a",
                            "libopus",
                            "-b:a",
                            &format!("{}k", o.bitrate.clamp(16, 510)),
                            "-f",
                            "ogg",
                        ])
                        .arg(&tmp))
                    .map(|_| format!("Opus {} kbps (ffmpeg)", o.bitrate))
                } else {
                    let (tool, p) =
                        aac_tool().ok_or_else(|| anyhow!("AAC needs ffmpeg on this computer"))?;
                    if tool == "ffmpeg" {
                        run(std::process::Command::new(p)
                            .args(["-y", "-loglevel", "error", "-i"])
                            .arg(&wav)
                            .args([
                                "-c:a",
                                "aac",
                                "-b:a",
                                &format!("{}k", o.bitrate.clamp(32, 512)),
                                "-f",
                                "ipod",
                            ])
                            .arg(&tmp))
                        .map(|_| format!("AAC {} kbps (ffmpeg)", o.bitrate))
                    } else {
                        run(std::process::Command::new(p)
                            .args([
                                "-f",
                                "m4af",
                                "-d",
                                "aac",
                                "-b",
                                &format!("{}", o.bitrate.clamp(32, 320) * 1000),
                            ])
                            .arg(&wav)
                            .arg(&tmp))
                        .map(|_| format!("AAC {} kbps (afconvert)", o.bitrate))
                    }
                };
                let _ = std::fs::remove_file(&wav);
                result
            }
            f => bail!("unknown format {f}"),
        }
    })();
    match res {
        Ok(enc) => {
            std::fs::rename(&tmp, path)
                .with_context(|| format!("could not write {}", path.display()))?;
            Ok(enc)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

fn run(cmd: &mut std::process::Command) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let out = cmd.output().context("could not start the encoder")?;
    if !out.status.success() {
        bail!(
            "the encoder failed: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("")
        );
    }
    Ok(())
}

/// Full export of already-rendered audio: resample, normalize, encode, verify.
/// Raised-cosine fade over the last `n` samples, ending at exactly zero.
pub fn fade_out(l: &mut [f32], r: &mut [f32], n: usize) {
    let len = l.len().min(r.len());
    let n = n.min(len);
    if n == 0 {
        return;
    }
    for k in 0..n {
        let i = len - n + k;
        let g = 0.5 + 0.5 * (std::f32::consts::PI * (k + 1) as f32 / n as f32).cos();
        l[i] *= g;
        r[i] *= g;
    }
}

pub fn export_audio(
    mut l: Vec<f32>,
    mut r: Vec<f32>,
    sr: u32,
    o: &ExportOpts,
    path: &Path,
) -> Result<ExportReport> {
    if l.is_empty() {
        bail!("there is nothing to export: the selection is empty");
    }
    let mut out_sr = o.sample_rate.unwrap_or(sr);
    if o.format == "mp3" && ![32000, 44100, 48000].contains(&out_sr) {
        out_sr = if out_sr > 44100 { 48000 } else { 44100 };
    }
    if o.format == "opus" {
        out_sr = 48000;
    }
    if out_sr != sr {
        l = resample(&l, sr, out_sr);
        r = resample(&r, sr, out_sr);
    }
    fade_out(
        &mut l,
        &mut r,
        (o.fade_out_s.clamp(0.0, 30.0) * out_sr as f32) as usize,
    );
    let (gain, limited) = normalize(&mut l, &mut r, out_sr, o);
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).ok();
    }
    let encoder = encode(&l, &r, out_sr, o, path)?;
    let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    // verify: decode what we wrote and measure it
    let (verified, m, chn, secs) = match crate::media::decode_file(path) {
        Ok(d) => {
            let dl = &d.data[0];
            let dr = &d.data[d.data.len() - 1];
            let m = analysis::loudness(dl, dr, d.rate as f32);
            let secs = dl.len() as f64 / d.rate as f64;
            let ok = (secs - l.len() as f64 / out_sr as f64).abs() < 0.2;
            (ok, m, d.channels, secs)
        }
        Err(_) => (
            false,
            analysis::loudness(&l, &r, out_sr as f32),
            if o.mono { 1 } else { 2 },
            l.len() as f64 / out_sr as f64,
        ),
    };
    Ok(ExportReport {
        path: path.display().to_string(),
        format: o.format.clone(),
        bytes,
        seconds: secs,
        sample_rate: out_sr,
        channels: chn,
        integrated: m.integrated,
        true_peak: m.true_peak,
        lra: m.lra,
        gain_applied_db: gain,
        limited,
        verified,
        encoder,
    })
}

/// SRT/VTT subtitles from (start_s, end_s, text) cues.
pub fn subtitles(cues: &[(f64, f64, String)], vtt: bool) -> String {
    let ts = |t: f64| {
        let ms = (t * 1000.0).round() as u64;
        let sep = if vtt { '.' } else { ',' };
        format!(
            "{:02}:{:02}:{:02}{}{:03}",
            ms / 3_600_000,
            ms / 60_000 % 60,
            ms / 1000 % 60,
            sep,
            ms % 1000
        )
    };
    let mut s = String::new();
    if vtt {
        s.push_str("WEBVTT\n\n");
    }
    for (i, (a, b, t)) in cues.iter().enumerate() {
        if !vtt {
            s.push_str(&format!("{}\n", i + 1));
        }
        s.push_str(&format!("{} --> {}\n{}\n\n", ts(*a), ts(*b), t));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone(sr: u32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(sr as f32 * secs) as usize)
            .map(|i| (crate::dsp::TAU * 440.0 * i as f32 / sr as f32).sin() * amp)
            .collect()
    }
    fn dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("flux-export-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    #[test]
    fn wav_round_trip_bit_exact_float() {
        let x = tone(48000, 1.0, 0.5);
        let o = ExportOpts {
            bits: 32,
            ..Default::default()
        };
        let p = dir().join("a.wav");
        let rep = export_audio(x.clone(), x.clone(), 48000, &o, &p).unwrap();
        assert!(rep.verified);
        let d = crate::media::decode_file(&p).unwrap();
        for (i, v) in x.iter().enumerate() {
            assert_eq!(d.data[0][i].to_bits(), v.to_bits());
        }
    }
    #[test]
    fn every_builtin_format_encodes_and_decodes() {
        let x = tone(44100, 2.0, 0.3);
        for f in ["wav", "flac", "mp3", "ogg"] {
            let o = ExportOpts {
                format: f.into(),
                bits: 16,
                ..Default::default()
            };
            let p = dir().join(format!("t.{}", ext_for(f)));
            let rep = export_audio(x.clone(), x.clone(), 44100, &o, &p).unwrap();
            assert!(rep.verified, "{f} not verified: {rep:?}");
            assert!((rep.seconds - 2.0).abs() < 0.15, "{f} {}", rep.seconds);
            // level survives within 0.5 dB (lossy codecs included)
            let want = analysis::loudness(&x, &x, 44100.0).integrated;
            assert!(
                (rep.integrated - want).abs() < 0.5,
                "{f}: {} vs {}",
                rep.integrated,
                want
            );
        }
    }
    #[test]
    fn lufs_normalization_hits_target_and_ceiling() {
        let x = tone(48000, 6.0, 0.05);
        let o = ExportOpts {
            format: "wav".into(),
            bits: 24,
            normalize: "lufs".into(),
            target: -9.0,
            ceiling: -1.0,
            ..Default::default()
        };
        let p = dir().join("n.wav");
        let rep = export_audio(x.clone(), x, 48000, &o, &p).unwrap();
        assert!(rep.true_peak <= -0.9, "true peak {}", rep.true_peak);
        let o2 = ExportOpts { target: -20.0, ..o };
        let rep2 = export_audio(
            tone(48000, 6.0, 0.05),
            tone(48000, 6.0, 0.05),
            48000,
            &o2,
            &dir().join("n2.wav"),
        )
        .unwrap();
        assert!((rep2.integrated + 20.0).abs() < 0.2, "{}", rep2.integrated);
    }
    #[test]
    fn fade_out_reaches_silence_and_leaves_the_rest() {
        let mut l = vec![1.0f32; 1000];
        let mut r = vec![0.5f32; 1000];
        fade_out(&mut l, &mut r, 100);
        assert_eq!(l[899], 1.0);
        assert_eq!(r[899], 0.5);
        assert!(l[900] < 1.0 && l[900] > 0.99);
        assert!(l[950] > 0.4 && l[950] < 0.6);
        assert!(l[999].abs() < 1e-6 && r[999].abs() < 1e-6);
        // monotone: no bumps on the way down
        assert!(l[900..].windows(2).all(|w| w[1] <= w[0]));
        // zero length does nothing; a fade longer than the audio covers all of it
        let before = l.clone();
        fade_out(&mut l, &mut r, 0);
        assert_eq!(l, before);
        let mut a = vec![1.0f32; 10];
        let mut b = vec![1.0f32; 10];
        fade_out(&mut a, &mut b, 5000);
        assert!(a[0] < 1.0 && a[9].abs() < 1e-6);
    }

    #[test]
    fn dither_is_seeded_and_only_on_integer_paths() {
        let x = tone(48000, 0.2, 0.001);
        let o = ExportOpts {
            bits: 16,
            ..Default::default()
        };
        let a = dir().join("d1.wav");
        let b = dir().join("d2.wav");
        export_audio(x.clone(), x.clone(), 48000, &o, &a).unwrap();
        export_audio(x.clone(), x, 48000, &o, &b).unwrap();
        assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
    }
    #[test]
    fn subtitles_format() {
        let s = subtitles(&[(1.5, 3.25, "Hello".into())], false);
        assert!(s.starts_with("1\n00:00:01,500 --> 00:00:03,250\nHello"));
        assert!(subtitles(&[(0.0, 1.0, "x".into())], true).starts_with("WEBVTT"));
    }
}
