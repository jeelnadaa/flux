//! Spectrogram tile generator: STFT, Linear/Log/Mel frequency scales, and color maps.

use realfft::RealFftPlanner;
use std::f32::consts::PI;

/// Frequency scale mapping for spectrogram visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyScale {
    /// Linear frequency bins directly from FFT.
    Linear,
    /// Logarithmic frequency spacing (musical pitch octaves).
    Logarithmic,
    /// Perceptual Mel scale filterbank.
    Mel,
}

/// Perceptually uniform color palette for spectrogram rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMap {
    /// Magma (black to purple to yellow, high contrast).
    Magma,
    /// Viridis (deep blue to turquoise to bright yellow).
    Viridis,
    /// Inferno (black to red-orange to light yellow).
    Inferno,
}

/// Configuration options for spectrogram tile generation.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectrogramConfig {
    /// FFT window size in samples (e.g. 512, 1024, 2048).
    pub window_size: usize,
    /// Hop size in samples between consecutive FFT frames (e.g. window_size / 2).
    pub hop_size: usize,
    /// Frequency scale mapping.
    pub scale: FrequencyScale,
    /// Number of output frequency bins in the generated tile.
    pub num_freq_bins: usize,
    /// Dynamic range minimum floor in dBFS (e.g. -120.0 dBFS).
    pub min_dbfs: f32,
    /// Dynamic range ceiling in dBFS (typically 0.0 dBFS).
    pub max_dbfs: f32,
    /// Color map for RGBA pixel rendering.
    pub colormap: ColorMap,
}

impl Default for SpectrogramConfig {
    fn default() -> Self {
        Self {
            window_size: 1024,
            hop_size: 512,
            scale: FrequencyScale::Linear,
            num_freq_bins: 256,
            min_dbfs: -100.0,
            max_dbfs: 0.0,
            colormap: ColorMap::Magma,
        }
    }
}

/// 2D spectrogram tile holding time-frequency normalized energy values or rendered RGBA pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectrogramTile {
    /// Number of time columns (time frames).
    pub width: usize,
    /// Number of frequency rows (frequency bins, low to high).
    pub height: usize,
    /// Normalized magnitude values in [0.0, 1.0]: `values[row * width + col]`.
    pub normalized_magnitudes: Vec<f32>,
    /// Pre-rendered RGBA pixel bytes (width * height * 4).
    pub rgba_pixels: Vec<u8>,
}

/// STFT and Spectrogram tile generator.
pub struct SpectrogramTileGenerator;

impl SpectrogramTileGenerator {
    /// Generates a spectrogram tile from a slice of audio frames.
    pub fn generate_tile(
        signal: &[f32],
        sample_rate: u32,
        config: &SpectrogramConfig,
    ) -> SpectrogramTile {
        let win_size = config.window_size.max(64);
        let hop_size = config.hop_size.max(16);
        let num_time_frames = if signal.len() >= win_size {
            (signal.len() - win_size) / hop_size + 1
        } else {
            0
        };

        if num_time_frames == 0 {
            return SpectrogramTile {
                width: 0,
                height: config.num_freq_bins,
                normalized_magnitudes: Vec::new(),
                rgba_pixels: Vec::new(),
            };
        }

        // 1. Prepare Hann window
        let mut window = Vec::with_capacity(win_size);
        let mut win_energy = 0.0f32;
        for i in 0..win_size {
            let w = 0.5 * (1.0 - (2.0 * PI * (i as f32) / (win_size as f32)).cos());
            window.push(w);
            win_energy += w;
        }
        let norm_factor = 2.0 / win_energy.max(1e-6);

        // 2. Real FFT planner
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(win_size);
        let num_fft_bins = win_size / 2 + 1;

        let mut in_buffer = vec![0.0f32; win_size];
        let mut spectrum = r2c.make_output_vec();

        let num_out_bins = config.num_freq_bins.min(num_fft_bins);
        let mut tile_mags = vec![0.0f32; num_time_frames * num_out_bins];

        // 3. Process STFT frames
        for t in 0..num_time_frames {
            let start = t * hop_size;
            for i in 0..win_size {
                in_buffer[i] = signal[start + i] * window[i];
            }

            r2c.process(&mut in_buffer, &mut spectrum).unwrap();

            // Extract magnitudes in dBFS and map onto output frequency bins
            for (freq_row, out_val) in tile_mags.chunks_mut(num_time_frames).enumerate() {
                let fft_bin = match config.scale {
                    FrequencyScale::Linear => {
                        (freq_row * (num_fft_bins - 1)) / (num_out_bins - 1).max(1)
                    }
                    FrequencyScale::Logarithmic => {
                        // Log scale from 20 Hz to Nyquist
                        let min_freq = 20.0f32;
                        let max_freq = sample_rate as f32 / 2.0;
                        let frac = (freq_row as f32) / (num_out_bins as f32);
                        let freq = min_freq * (max_freq / min_freq).powf(frac);
                        ((freq / (sample_rate as f32)) * (win_size as f32)).round() as usize
                    }
                    FrequencyScale::Mel => {
                        // Mel formula: m = 2595 * log10(1 + f / 700)
                        let max_mel = 2595.0 * (1.0 + (sample_rate as f32 * 0.5) / 700.0).log10();
                        let frac = (freq_row as f32) / (num_out_bins as f32);
                        let mel = frac * max_mel;
                        let freq = 700.0 * (10.0f32.powf(mel / 2595.0) - 1.0);
                        ((freq / (sample_rate as f32)) * (win_size as f32)).round() as usize
                    }
                };

                let bin_idx = fft_bin.min(num_fft_bins - 1);
                let mag = (spectrum[bin_idx].norm() * norm_factor).max(1e-7);
                let db = 20.0 * mag.log10();

                // Normalize dB to [0.0, 1.0]
                let norm_val =
                    ((db - config.min_dbfs) / (config.max_dbfs - config.min_dbfs)).clamp(0.0, 1.0);
                out_val[t] = norm_val;
            }
        }

        // 4. Generate RGBA pixel array with colormap
        let mut rgba_pixels = Vec::with_capacity(tile_mags.len() * 4);
        for &val in &tile_mags {
            let (r, g, b) = Self::sample_colormap(val, config.colormap);
            rgba_pixels.push(r);
            rgba_pixels.push(g);
            rgba_pixels.push(b);
            rgba_pixels.push(255);
        }

        SpectrogramTile {
            width: num_time_frames,
            height: num_out_bins,
            normalized_magnitudes: tile_mags,
            rgba_pixels,
        }
    }

    /// Samples a perceptually uniform colormap (Magma, Viridis, Inferno).
    fn sample_colormap(t: f32, colormap: ColorMap) -> (u8, u8, u8) {
        let t = t.clamp(0.0, 1.0);
        match colormap {
            ColorMap::Magma => {
                // Approximate Magma: black (0.0) -> dark violet -> orange -> yellow (1.0)
                let r = (255.0 * (t * 1.5).min(1.0).powf(0.8)) as u8;
                let g = (255.0 * (t.powf(2.0) * 0.9)) as u8;
                let b = (255.0 * ((1.0 - (t - 0.5).abs() * 2.0).max(0.0) * 0.7 + t.powf(3.0) * 0.3))
                    as u8;
                (r, g, b)
            }
            ColorMap::Viridis => {
                // Approximate Viridis: deep purple -> blue -> teal -> yellow
                let r = (255.0 * (t.powf(3.0) * 0.9 + (1.0 - t).powf(4.0) * 0.2)) as u8;
                let g = (255.0 * t.powf(0.85)) as u8;
                let b = (255.0
                    * ((1.0 - t).powf(0.5) * 0.6 + (0.5 - (t - 0.5).abs()).max(0.0) * 0.4))
                    as u8;
                (r, g, b)
            }
            ColorMap::Inferno => {
                // Approximate Inferno: black -> deep red -> bright orange -> pale yellow
                let r = (255.0 * (t * 1.3).min(1.0).powf(0.7)) as u8;
                let g = (255.0 * t.powf(1.8)) as u8;
                let b = (255.0 * (t.powf(4.0) * 0.5 + (0.3 - t).max(0.0) * 0.5)) as u8;
                (r, g, b)
            }
        }
    }
}
