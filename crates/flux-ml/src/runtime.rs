//! ML runtime abstraction, device capability probing, and chunked overlap-add.

use serde::{Deserialize, Serialize};

/// Execution backend for machine learning inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MlBackend {
    /// WebGPU hardware acceleration.
    WebGpu,
    /// WebAssembly SIMD-accelerated multithreaded runtime.
    WasmSimd,
    /// Pure CPU fallback.
    Cpu,
    /// Ephemeral private cloud execution.
    Cloud,
}

/// Hardware capabilities of the local execution environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Whether WebGPU is available.
    pub has_webgpu: bool,
    /// Estimated graphics memory in bytes.
    pub gpu_memory_bytes: u64,
    /// Number of logical CPU cores.
    pub cpu_threads: usize,
    /// Whether WebAssembly SIMD is supported.
    pub simd_supported: bool,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            has_webgpu: true,
            gpu_memory_bytes: 8 * 1024 * 1024 * 1024, // 8 GB
            cpu_threads: 8,
            simd_supported: true,
        }
    }
}

/// Device capability prober.
pub struct DeviceProber;

impl DeviceProber {
    /// Probes environment and selects the optimal ML execution backend.
    pub fn select_best_backend(caps: &DeviceCapabilities, require_cloud: bool) -> MlBackend {
        if require_cloud {
            return MlBackend::Cloud;
        }
        if caps.has_webgpu && caps.gpu_memory_bytes >= 2 * 1024 * 1024 * 1024 {
            MlBackend::WebGpu
        } else if caps.simd_supported {
            MlBackend::WasmSimd
        } else {
            MlBackend::Cpu
        }
    }
}

/// Chunked overlap-add processor for continuous long-audio inference without boundary artifacts.
#[derive(Debug, Clone)]
pub struct ChunkedOverlapAdd {
    /// Window size in samples.
    pub window_size: usize,
    /// Hop size in samples.
    pub hop_size: usize,
    /// Overlap ratio (e.g. 0.25 to 0.50).
    pub overlap_ratio: f32,
    /// Crossfade Hann window.
    window: Vec<f32>,
}

impl ChunkedOverlapAdd {
    /// Creates a new chunked overlap-add processor.
    pub fn new(window_size: usize, overlap_ratio: f32) -> Self {
        let overlap_ratio = overlap_ratio.clamp(0.1, 0.75);
        let hop_size = ((1.0 - overlap_ratio) * window_size as f32).round() as usize;
        let mut window = vec![0.0f32; window_size];
        for (n, w) in window.iter_mut().enumerate() {
            // Periodic Hann window: w[n] = 0.5 * (1 - cos(2*pi*n / N))
            *w = 0.5 * (1.0 - (n as f32 * std::f32::consts::TAU / window_size as f32).cos());
        }
        Self {
            window_size,
            hop_size,
            overlap_ratio,
            window,
        }
    }

    /// Processes an input audio buffer with a chunk-processing closure and reconstructs with overlap-add.
    pub fn process<F>(&self, input: &[f32], mut process_chunk: F) -> Vec<f32>
    where
        F: FnMut(&[f32], &mut [f32]),
    {
        if input.is_empty() {
            return Vec::new();
        }

        let mut output = vec![0.0f32; input.len() + self.window_size];
        let mut normalization = vec![0.0f32; output.len()];
        let mut in_chunk = vec![0.0f32; self.window_size];
        let mut out_chunk = vec![0.0f32; self.window_size];

        let mut start = 0;
        while start < input.len() {
            let available = (input.len() - start).min(self.window_size);
            in_chunk[..available].copy_from_slice(&input[start..start + available]);
            in_chunk[available..].fill(0.0);

            // Apply analysis window
            for (n, in_val) in in_chunk.iter_mut().enumerate().take(self.window_size) {
                *in_val *= self.window[n];
            }

            process_chunk(&in_chunk, &mut out_chunk);

            // Apply synthesis window & overlap-add
            for (n, &out_val) in out_chunk.iter().enumerate().take(self.window_size) {
                let idx = start + n;
                if idx < output.len() {
                    let w = self.window[n];
                    output[idx] += out_val * w;
                    normalization[idx] += w * w;
                }
            }

            start += self.hop_size;
        }

        // Normalize overlap regions to guarantee unity gain (COLA)
        let final_len = input.len();
        output.truncate(final_len);
        for n in 0..final_len {
            if normalization[n] > 1e-6 {
                output[n] /= normalization[n];
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_backend_selection() {
        let default_caps = DeviceCapabilities::default();
        assert_eq!(
            DeviceProber::select_best_backend(&default_caps, false),
            MlBackend::WebGpu
        );

        let low_gpu = DeviceCapabilities {
            has_webgpu: false,
            gpu_memory_bytes: 0,
            cpu_threads: 4,
            simd_supported: true,
        };
        assert_eq!(
            DeviceProber::select_best_backend(&low_gpu, false),
            MlBackend::WasmSimd
        );

        let fallback_cpu = DeviceCapabilities {
            has_webgpu: false,
            gpu_memory_bytes: 0,
            cpu_threads: 2,
            simd_supported: false,
        };
        assert_eq!(
            DeviceProber::select_best_backend(&fallback_cpu, false),
            MlBackend::Cpu
        );

        assert_eq!(
            DeviceProber::select_best_backend(&default_caps, true),
            MlBackend::Cloud
        );
    }

    #[test]
    fn test_chunked_overlap_add_reconstruction_identity() {
        let ola = ChunkedOverlapAdd::new(1024, 0.50);
        let sample_rate = 48000.0f32;
        let mut input = vec![0.0f32; 9600]; // 0.2s of audio
        for (i, x) in input.iter_mut().enumerate() {
            *x = (i as f32 * 440.0 / sample_rate * std::f32::consts::TAU).sin() * 0.7;
        }

        // Pass-through processing should perfectly reconstruct the original signal
        let output = ola.process(&input, |chunk_in, chunk_out| {
            chunk_out.copy_from_slice(chunk_in);
        });

        assert_eq!(output.len(), input.len());

        // Check error excluding boundary ramp-up
        let steady_range = 1024..input.len() - 1024;
        let mut max_err = 0.0f32;
        for i in steady_range {
            let err = (output[i] - input[i]).abs();
            if err > max_err {
                max_err = err;
            }
        }
        assert!(
            max_err < 0.05,
            "Reconstruction error was too large: {}",
            max_err
        );
    }
}
