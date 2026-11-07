//! Decoding and the in-memory media pool. Every opened file is treated as hostile: decode
//! errors are reported, sizes are bounded, and nothing is written back to the source.

use anyhow::{anyhow, bail, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub const PEAK_BIN: usize = 256;
/// Hard limit on decoded length: 6 hours at 48 kHz.
pub const MAX_FRAMES: usize = 48000 * 3600 * 6;

pub struct Decoded {
    pub rate: u32,
    pub channels: u16,
    /// Planar, at most two channels (more are folded down).
    pub data: Vec<Vec<f32>>,
    pub format: String,
    pub bits: u16,
    pub tags: BTreeMap<String, String>,
}

/// The file's sample rate from its header, without decoding.
pub fn probe_rate(path: &Path) -> Option<u32> {
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(&e.to_lowercase());
    }
    let format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .ok()?;
    let t = format.default_track(TrackType::Audio)?;
    t.codec_params
        .as_ref()
        .and_then(|c| c.audio())
        .and_then(|a| a.sample_rate)
}

pub fn decode_file(path: &Path) -> Result<Decoded> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::{MetadataOptions, StandardTag};

    let file =
        std::fs::File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !ext.is_empty() {
        hint.with_extension(&ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| anyhow!("this file is not audio Flux can read ({e})"))?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| anyhow!("the file has no audio track"))?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|c| c.audio())
        .ok_or_else(|| anyhow!("the audio codec is not supported"))?
        .clone();
    let bits = params.bits_per_sample.unwrap_or(0) as u16;
    let track_id = track.id;
    let mut dec = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| anyhow!("the audio codec is not supported ({e})"))?;

    let mut tags = BTreeMap::new();
    if let Some(rev) = format.metadata().current() {
        for t in &rev.media.tags {
            let (k, v) = match &t.std {
                Some(StandardTag::TrackTitle(v)) => ("title", v.to_string()),
                Some(StandardTag::Artist(v)) => ("artist", v.to_string()),
                Some(StandardTag::Album(v)) => ("album", v.to_string()),
                _ => continue,
            };
            tags.insert(k.to_string(), v);
        }
    }

    let mut rate = 0u32;
    let mut chans = 0usize;
    let mut inter: Vec<f32> = Vec::new();
    let mut data: Vec<Vec<f32>> = Vec::new();
    let mut errors = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(Error::IoError(_)) => break,
            Err(e) => {
                errors += 1;
                if errors > 50 {
                    bail!("the file is damaged ({e})");
                }
                continue;
            }
        };
        if packet.track_id != track_id {
            continue;
        }
        match dec.decode(&packet) {
            Ok(buf) => {
                if rate == 0 {
                    rate = buf.spec().rate();
                    chans = buf.spec().channels().count().max(1);
                    data = vec![Vec::new(); chans.min(2)];
                    if !(1000..=768_000).contains(&rate) || chans > 64 {
                        bail!("unsupported audio format ({rate} Hz, {chans} channels)");
                    }
                }
                inter.resize(buf.samples_interleaved(), 0.0);
                buf.copy_to_slice_interleaved(&mut inter);
                let frames = inter.len() / chans;
                for f in 0..frames {
                    let fr = &inter[f * chans..(f + 1) * chans];
                    if chans <= 2 {
                        for (c, d) in data.iter_mut().enumerate() {
                            d.push(fr[c]);
                        }
                    } else {
                        // fold surround to stereo: odd channels left, even right
                        let (mut l, mut r) = (0.0, 0.0);
                        for (c, v) in fr.iter().enumerate() {
                            if c % 2 == 0 {
                                l += v;
                            } else {
                                r += v;
                            }
                        }
                        let k = 2.0 / chans as f32;
                        data[0].push(l * k);
                        data[1].push(r * k);
                    }
                }
                if data[0].len() > MAX_FRAMES {
                    bail!("the file is longer than 6 hours");
                }
            }
            Err(Error::DecodeError(_)) => {
                errors += 1;
                if errors > 50 {
                    bail!("the file is damaged");
                }
            }
            Err(e) => bail!("the file is damaged ({e})"),
        }
    }
    if rate == 0 || data.is_empty() || data[0].is_empty() {
        bail!("the file has no audio");
    }
    for d in &mut data {
        for v in d.iter_mut() {
            if !v.is_finite() {
                *v = 0.0;
            }
        }
    }
    Ok(Decoded {
        rate,
        channels: chans as u16,
        data,
        format: ext,
        bits,
        tags,
    })
}

/// Decoded audio at the project rate plus its peak cache.
pub struct Media {
    pub id: String,
    pub data: Vec<Arc<Vec<f32>>>,
    /// Per channel: (min, max) per PEAK_BIN samples.
    pub peaks: Vec<Vec<(f32, f32)>>,
    pub frames: usize,
}

impl Media {
    pub fn new(id: &str, mut data: Vec<Vec<f32>>, from_rate: u32, to_rate: u32) -> Self {
        if from_rate != to_rate {
            data = data
                .iter()
                .map(|c| crate::dsp::resample(c, from_rate, to_rate))
                .collect();
        }
        let peaks = data
            .iter()
            .map(|c| {
                c.chunks(PEAK_BIN)
                    .map(|w| {
                        w.iter()
                            .fold((0f32, 0f32), |(a, b), v| (a.min(*v), b.max(*v)))
                    })
                    .collect()
            })
            .collect();
        let frames = data[0].len();
        Media {
            id: id.into(),
            data: data.into_iter().map(Arc::new).collect(),
            peaks,
            frames,
        }
    }
    #[inline]
    pub fn get(&self, ch: usize, i: usize) -> f32 {
        let c = &self.data[ch.min(self.data.len() - 1)];
        if i < c.len() {
            c[i]
        } else {
            0.0
        }
    }
    /// Min/max peaks over `[start, end)` in `bins` columns, per channel.
    pub fn peaks_range(
        &self,
        start: i64,
        end: i64,
        bins: usize,
        reverse: bool,
    ) -> Vec<Vec<[f32; 2]>> {
        let bins = bins.clamp(1, 4000);
        let (start, end) = (
            start.max(0) as usize,
            (end.max(0) as usize).min(self.frames),
        );
        let span = end.saturating_sub(start).max(1) as f64;
        self.data
            .iter()
            .enumerate()
            .map(|(ch, d)| {
                (0..bins)
                    .map(|b| {
                        let mut a = start + (span * b as f64 / bins as f64) as usize;
                        let mut z = (start + (span * (b + 1) as f64 / bins as f64) as usize)
                            .max(a + 1)
                            .min(end.max(a + 1));
                        if reverse {
                            let (ra, rz) = (start + end - z, start + end - a);
                            a = ra;
                            z = rz;
                        }
                        let (mut lo, mut hi) = (0f32, 0f32);
                        if z - a > PEAK_BIN * 2 {
                            let pk = &self.peaks[ch];
                            for p in &pk[a / PEAK_BIN..(z / PEAK_BIN).min(pk.len())] {
                                lo = lo.min(p.0);
                                hi = hi.max(p.1);
                            }
                        } else {
                            for v in &d[a.min(d.len())..z.min(d.len())] {
                                lo = lo.min(*v);
                                hi = hi.max(*v);
                            }
                        }
                        [lo, hi]
                    })
                    .collect()
            })
            .collect()
    }
}

pub type Pool = BTreeMap<String, Arc<Media>>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_wav_written_by_hound() {
        let dir = std::env::temp_dir().join(format!("flux-media-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&p, spec).unwrap();
        for i in 0..44100 {
            let v = ((i as f32 * 0.05).sin() * 16000.0) as i16;
            w.write_sample(v).unwrap();
            w.write_sample(-v).unwrap();
        }
        w.finalize().unwrap();
        let d = decode_file(&p).unwrap();
        assert_eq!(d.rate, 44100);
        assert_eq!(d.channels, 2);
        assert_eq!(d.data[0].len(), 44100);
        assert!((d.data[0][100] - (100f32 * 0.05).sin() * 16000.0 / 32768.0).abs() < 1e-3);
        assert!((d.data[1][100] + d.data[0][100]).abs() < 1e-4);
    }
    #[test]
    fn hostile_input_is_rejected_not_panicking() {
        let dir = std::env::temp_dir().join(format!("flux-media-h-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut rng = crate::dsp::Rng::new(9);
        for (i, head) in [
            &b"RIFF\x24\x00\x00\x00WAVEfmt "[..],
            b"fLaC",
            b"ID3\x03",
            b"OggS",
            b"",
        ]
        .iter()
        .enumerate()
        {
            let p = dir.join(format!("bad{i}.bin"));
            let mut bytes = head.to_vec();
            bytes.extend((0..4000).map(|_| (rng.next_u64() & 0xff) as u8));
            std::fs::write(&p, bytes).unwrap();
            assert!(decode_file(&p).is_err());
        }
    }
}
