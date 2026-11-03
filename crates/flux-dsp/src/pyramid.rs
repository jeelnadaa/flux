//! Multi-resolution hierarchical peak pyramids for waveform rendering and instant zoom.

/// A single peak summary bin containing minimum, maximum, and RMS amplitudes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakBin {
    /// Minimum sample value in the bin.
    pub min: f32,
    /// Maximum sample value in the bin.
    pub max: f32,
    /// Root-mean-square amplitude in the bin.
    pub rms: f32,
}

impl Default for PeakBin {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 0.0,
            rms: 0.0,
        }
    }
}

/// Multi-resolution pyramid level holding bins for each channel.
#[derive(Debug, Clone, PartialEq)]
pub struct PyramidLevel {
    /// Number of audio sample frames represented by each bin in this level.
    pub frames_per_bin: usize,
    /// Per-channel bins: `channels[channel_idx][bin_idx]`.
    pub channels: Vec<Vec<PeakBin>>,
}

/// Multi-resolution peak pyramid supporting multi-channel waveform zoom from sample to whole file.
#[derive(Debug, Clone, PartialEq)]
pub struct PeakPyramid {
    /// Base bin size in samples at Level 0 (typically 64 samples).
    pub base_bin_size: usize,
    /// Total audio frames in the source signal.
    pub total_frames: usize,
    /// Number of audio channels.
    pub num_channels: usize,
    /// Hierarchical pyramid levels, from Level 0 (highest detail) to Level N (coarsest overview).
    pub levels: Vec<PyramidLevel>,
}

impl PeakPyramid {
    /// Default decimation factor between successive levels (4x per level).
    pub const DECIMATION_FACTOR: usize = 4;

    /// Builds a multi-resolution peak pyramid from multi-channel planar audio frames.
    pub fn build(channels: &[&[f32]], base_bin_size: usize) -> Self {
        let num_channels = channels.len();
        if num_channels == 0 {
            return Self {
                base_bin_size,
                total_frames: 0,
                num_channels: 0,
                levels: Vec::new(),
            };
        }

        let total_frames = channels[0].len();
        let base_size = base_bin_size.max(8);

        // 1. Level 0: aggregate raw audio frames into base_size chunks
        let mut l0_channels = Vec::with_capacity(num_channels);
        for ch in channels {
            let num_bins = ch.len().div_ceil(base_size);
            let mut bins = Vec::with_capacity(num_bins);

            for chunk in ch.chunks(base_size) {
                let mut min_val = f32::INFINITY;
                let mut max_val = f32::NEG_INFINITY;
                let mut sum_sq = 0.0f32;

                for &s in chunk {
                    if s < min_val {
                        min_val = s;
                    }
                    if s > max_val {
                        max_val = s;
                    }
                    sum_sq += s * s;
                }

                let rms = (sum_sq / (chunk.len() as f32)).sqrt();
                bins.push(PeakBin {
                    min: if min_val.is_finite() { min_val } else { 0.0 },
                    max: if max_val.is_finite() { max_val } else { 0.0 },
                    rms,
                });
            }
            l0_channels.push(bins);
        }

        let mut levels = vec![PyramidLevel {
            frames_per_bin: base_size,
            channels: l0_channels,
        }];

        // 2. Build successive pyramid levels until top level has <= 128 bins
        let mut current_frames_per_bin = base_size;
        loop {
            let prev_level = levels.last().unwrap();
            let prev_len = prev_level.channels[0].len();
            if prev_len <= 128 {
                break;
            }

            let next_frames_per_bin = current_frames_per_bin * Self::DECIMATION_FACTOR;
            let mut next_channels = Vec::with_capacity(num_channels);

            for prev_ch in &prev_level.channels {
                let next_len = prev_ch.len().div_ceil(Self::DECIMATION_FACTOR);
                let mut next_bins = Vec::with_capacity(next_len);

                for group in prev_ch.chunks(Self::DECIMATION_FACTOR) {
                    let mut min_val = f32::INFINITY;
                    let mut max_val = f32::NEG_INFINITY;
                    let mut sum_rms_sq = 0.0f32;

                    for bin in group {
                        if bin.min < min_val {
                            min_val = bin.min;
                        }
                        if bin.max > max_val {
                            max_val = bin.max;
                        }
                        sum_rms_sq += bin.rms * bin.rms;
                    }

                    let rms = (sum_rms_sq / (group.len() as f32)).sqrt();
                    next_bins.push(PeakBin {
                        min: min_val,
                        max: max_val,
                        rms,
                    });
                }
                next_channels.push(next_bins);
            }

            levels.push(PyramidLevel {
                frames_per_bin: next_frames_per_bin,
                channels: next_channels,
            });
            current_frames_per_bin = next_frames_per_bin;
        }

        Self {
            base_bin_size: base_size,
            total_frames,
            num_channels,
            levels,
        }
    }

    /// Queries the optimal pyramid level for rendering a timeline range on a canvas of `target_bins` width.
    pub fn query_range(
        &self,
        channel: usize,
        start_frame: usize,
        end_frame: usize,
        target_bins: usize,
    ) -> &[PeakBin] {
        if self.levels.is_empty() || channel >= self.num_channels || start_frame >= end_frame {
            return &[];
        }

        let range_frames = (end_frame - start_frame).min(self.total_frames);
        let desired_frames_per_bin = (range_frames as f64 / target_bins.max(1) as f64).max(1.0);

        // Find the coarsest level whose frames_per_bin <= desired_frames_per_bin
        let mut chosen_level_idx = 0;
        for (idx, level) in self.levels.iter().enumerate() {
            if (level.frames_per_bin as f64) <= desired_frames_per_bin {
                chosen_level_idx = idx;
            } else {
                break;
            }
        }

        let level = &self.levels[chosen_level_idx];
        let ch_bins = &level.channels[channel];
        let fpb = level.frames_per_bin;

        let start_bin = (start_frame / fpb).min(ch_bins.len());
        let end_bin = (end_frame.div_ceil(fpb)).min(ch_bins.len());

        if start_bin < end_bin {
            &ch_bins[start_bin..end_bin]
        } else {
            &[]
        }
    }

    /// Serializes the pyramid into binary bytes with `FPYR` magic header.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FPYR"); // Magic
        out.extend_from_slice(&1u16.to_le_bytes()); // Version 1
        out.extend_from_slice(&(self.num_channels as u16).to_le_bytes());
        out.extend_from_slice(&(self.base_bin_size as u32).to_le_bytes());
        out.extend_from_slice(&(self.total_frames as u64).to_le_bytes());
        out.extend_from_slice(&(self.levels.len() as u32).to_le_bytes());

        for level in &self.levels {
            out.extend_from_slice(&(level.frames_per_bin as u32).to_le_bytes());
            let bins_count = if level.channels.is_empty() {
                0
            } else {
                level.channels[0].len()
            };
            out.extend_from_slice(&(bins_count as u32).to_le_bytes());

            for ch in &level.channels {
                for b in ch {
                    out.extend_from_slice(&b.min.to_le_bytes());
                    out.extend_from_slice(&b.max.to_le_bytes());
                    out.extend_from_slice(&b.rms.to_le_bytes());
                }
            }
        }

        out
    }

    /// Deserializes binary pyramid bytes into a `PeakPyramid`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 24 || &bytes[0..4] != b"FPYR" {
            return Err("Invalid peak pyramid binary header");
        }

        let num_channels = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
        let base_bin_size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let total_frames = u64::from_le_bytes([
            bytes[12], bytes[13], bytes[14], bytes[15], bytes[16], bytes[17], bytes[18], bytes[19],
        ]) as usize;
        let num_levels = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as usize;

        let mut offset = 24;
        let mut levels = Vec::with_capacity(num_levels);

        for _ in 0..num_levels {
            if offset + 8 > bytes.len() {
                return Err("Unexpected EOF reading pyramid level header");
            }
            let frames_per_bin = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            let bins_count = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8;

            let mut channels = Vec::with_capacity(num_channels);
            for _ in 0..num_channels {
                let mut bins = Vec::with_capacity(bins_count);
                for _ in 0..bins_count {
                    if offset + 12 > bytes.len() {
                        return Err("Unexpected EOF reading peak bins");
                    }
                    let min = f32::from_le_bytes([
                        bytes[offset],
                        bytes[offset + 1],
                        bytes[offset + 2],
                        bytes[offset + 3],
                    ]);
                    let max = f32::from_le_bytes([
                        bytes[offset + 4],
                        bytes[offset + 5],
                        bytes[offset + 6],
                        bytes[offset + 7],
                    ]);
                    let rms = f32::from_le_bytes([
                        bytes[offset + 8],
                        bytes[offset + 9],
                        bytes[offset + 10],
                        bytes[offset + 11],
                    ]);
                    offset += 12;
                    bins.push(PeakBin { min, max, rms });
                }
                channels.push(bins);
            }

            levels.push(PyramidLevel {
                frames_per_bin,
                channels,
            });
        }

        Ok(Self {
            base_bin_size,
            total_frames,
            num_channels,
            levels,
        })
    }
}
