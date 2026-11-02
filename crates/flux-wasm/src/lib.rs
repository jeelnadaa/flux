//! Flux WebAssembly bindings for browser execution and AudioWorklet rendering.
#![deny(missing_docs)]

use flux_dsp::binaural::BinauralNode;
use flux_dsp::biquad::{BiquadChain, BiquadType};
use flux_dsp::node::DspNode;
use wasm_bindgen::prelude::*;

/// Returns the Flux DSP engine version string.
#[wasm_bindgen]
pub fn flux_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// WebAssembly wrapper around `BiquadChain` for high-level JS APIs.
#[wasm_bindgen]
pub struct WasmBiquadChain {
    inner: BiquadChain,
}

#[wasm_bindgen]
impl WasmBiquadChain {
    /// Creates a new cascaded biquad filter chain.
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f32, stages: usize) -> Self {
        let mut chain = BiquadChain::new(stages);
        chain.prepare(sample_rate, 1);
        Self { inner: chain }
    }

    /// Configures a parametric peaking EQ stage.
    pub fn configure_peaking(&mut self, stage: usize, freq: f32, q: f32, gain_db: f32) {
        self.inner
            .configure_stage(stage, BiquadType::Peaking, freq, q, gain_db);
    }

    /// Configures a second-order lowpass filter stage.
    pub fn configure_lowpass(&mut self, stage: usize, freq: f32, q: f32) {
        self.inner
            .configure_stage(stage, BiquadType::Lowpass, freq, q, 0.0);
    }

    /// Configures a second-order highpass filter stage.
    pub fn configure_highpass(&mut self, stage: usize, freq: f32, q: f32) {
        self.inner
            .configure_stage(stage, BiquadType::Highpass, freq, q, 0.0);
    }

    /// Processes an input block into an output slice (mono).
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        self.inner.process(&[input], &mut [output]);
    }

    /// Generates a test tone of specified frequency and filters it through the biquad chain.
    pub fn process_tone(&mut self, freq: f32, frames: usize, sample_rate: f32) -> Vec<f32> {
        let mut in_buf = Vec::with_capacity(frames);
        let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
        for i in 0..frames {
            in_buf.push((i as f32 * omega).sin());
        }

        let mut out_buf = vec![0.0f32; frames];
        self.inner.process(&[&in_buf], &mut [&mut out_buf]);
        out_buf
    }

    /// Resets all internal delay buffers and filter states.
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

// ----------------------------------------------------------------------------
// AudioWorklet Real-Time C ABI
// Direct C exports for quantum processing (128 samples) with 0 glue overhead.
// ----------------------------------------------------------------------------

/// Allocates an f32 buffer in WebAssembly linear memory.
#[no_mangle]
pub extern "C" fn biquad_alloc_buffer(len: usize) -> *mut f32 {
    let mut vec = Vec::with_capacity(len);
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec);
    ptr
}

/// Frees an f32 buffer in WebAssembly linear memory.
///
/// # Safety
/// `ptr` must have been returned by `biquad_alloc_buffer` with capacity `len`.
#[no_mangle]
pub unsafe extern "C" fn biquad_free_buffer(ptr: *mut f32, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

/// Creates a new `BiquadChain` instance.
#[no_mangle]
pub extern "C" fn biquad_create(sample_rate: f32, stages: usize) -> *mut BiquadChain {
    let mut chain = Box::new(BiquadChain::new(stages));
    chain.prepare(sample_rate, 1);
    Box::into_raw(chain)
}

/// Destroys a `BiquadChain` instance.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `biquad_create`.
#[no_mangle]
pub unsafe extern "C" fn biquad_destroy(ptr: *mut BiquadChain) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Configures a peaking EQ stage.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BiquadChain`.
#[no_mangle]
pub unsafe extern "C" fn biquad_configure_peaking(
    ptr: *mut BiquadChain,
    stage: usize,
    freq: f32,
    q: f32,
    gain_db: f32,
) {
    if let Some(chain) = ptr.as_mut() {
        chain.configure_stage(stage, BiquadType::Peaking, freq, q, gain_db);
    }
}

/// Configures a lowpass filter stage.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BiquadChain`.
#[no_mangle]
pub unsafe extern "C" fn biquad_configure_lowpass(
    ptr: *mut BiquadChain,
    stage: usize,
    freq: f32,
    q: f32,
) {
    if let Some(chain) = ptr.as_mut() {
        chain.configure_stage(stage, BiquadType::Lowpass, freq, q, 0.0);
    }
}

/// Configures a highpass filter stage.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BiquadChain`.
#[no_mangle]
pub unsafe extern "C" fn biquad_configure_highpass(
    ptr: *mut BiquadChain,
    stage: usize,
    freq: f32,
    q: f32,
) {
    if let Some(chain) = ptr.as_mut() {
        chain.configure_stage(stage, BiquadType::Highpass, freq, q, 0.0);
    }
}

/// Processes an AudioWorklet render quantum (128 frames) in real time.
///
/// # Safety
/// `ptr` must be valid, and `in_ptr` / `out_ptr` must point to valid slices of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn biquad_process_quantum(
    ptr: *mut BiquadChain,
    in_ptr: *const f32,
    out_ptr: *mut f32,
    frames: usize,
) {
    if let Some(chain) = ptr.as_mut() {
        let in_slice = std::slice::from_raw_parts(in_ptr, frames);
        let out_slice = std::slice::from_raw_parts_mut(out_ptr, frames);
        chain.process(&[in_slice], &mut [out_slice]);
    }
}

/// Creates a new `BinauralNode` instance.
#[no_mangle]
pub extern "C" fn binaural_create(sample_rate: f32) -> *mut BinauralNode {
    let mut node = Box::new(BinauralNode::new());
    node.prepare(sample_rate, 2);
    Box::into_raw(node)
}

/// Destroys a `BinauralNode` instance.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `binaural_create`.
#[no_mangle]
pub unsafe extern "C" fn binaural_destroy(ptr: *mut BinauralNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets the azimuth angle in degrees.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BinauralNode`.
#[no_mangle]
pub unsafe extern "C" fn binaural_set_azimuth(ptr: *mut BinauralNode, azimuth_deg: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_azimuth(azimuth_deg);
    }
}

/// Sets automated 8D orbit speed in Hz.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BinauralNode`.
#[no_mangle]
pub unsafe extern "C" fn binaural_set_orbit_speed(ptr: *mut BinauralNode, speed_hz: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_orbit_speed(speed_hz);
    }
}

/// Processes an AudioWorklet render quantum (128 frames) through the binaural panner.
///
/// # Safety
/// `ptr` must be valid, and `in_ptr`, `out_l_ptr`, `out_r_ptr` must point to valid slices of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn binaural_process_quantum(
    ptr: *mut BinauralNode,
    in_ptr: *const f32,
    out_l_ptr: *mut f32,
    out_r_ptr: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let in_slice = std::slice::from_raw_parts(in_ptr, frames);
        let out_l_slice = std::slice::from_raw_parts_mut(out_l_ptr, frames);
        let out_r_slice = std::slice::from_raw_parts_mut(out_r_ptr, frames);
        node.process(&[in_slice], &mut [out_l_slice, out_r_slice]);
    }
}

/// Renders a synthesized test tone through the biquad chain.
///
/// # Safety
/// `ptr` must be valid, and `out_ptr` must point to a buffer of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn biquad_render_tone(
    ptr: *mut BiquadChain,
    freq: f32,
    sample_rate: f32,
    out_ptr: *mut f32,
    frames: usize,
) {
    if let Some(chain) = ptr.as_mut() {
        let out_slice = std::slice::from_raw_parts_mut(out_ptr, frames);
        let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;

        // Populate in-place with sine, then filter in-place
        for (i, sample) in out_slice.iter_mut().enumerate().take(frames) {
            *sample = (i as f32 * omega).sin();
        }

        // To avoid allocation in worklet thread, clone input to temp stack buffer or process in-place
        // BiquadChain supports in-place processing when in_slice == out_slice
        let in_slice = std::slice::from_raw_parts(out_ptr, frames);
        let out_slice_mut = std::slice::from_raw_parts_mut(out_ptr, frames);
        chain.process(&[in_slice], &mut [out_slice_mut]);
    }
}

/// Resets the biquad chain filter state.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BiquadChain`.
#[no_mangle]
pub unsafe extern "C" fn biquad_reset(ptr: *mut BiquadChain) {
    if let Some(chain) = ptr.as_mut() {
        chain.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(flux_version(), "0.1.0");
    }

    #[test]
    fn test_wasm_biquad_wrapper() {
        let mut chain = WasmBiquadChain::new(48000.0, 4);
        chain.configure_peaking(0, 1000.0, 1.0, 6.0);
        let in_buf = vec![0.5f32; 128];
        let mut out_buf = vec![0.0f32; 128];
        chain.process_block(&in_buf, &mut out_buf);
        assert!(!out_buf[127].is_nan());
    }

    #[test]
    fn test_c_abi_biquad_quantum() {
        unsafe {
            let chain = biquad_create(48000.0, 4);
            assert!(!chain.is_null());
            biquad_configure_peaking(chain, 0, 1000.0, 1.0, 3.0);

            let in_buf = biquad_alloc_buffer(128);
            let out_buf = biquad_alloc_buffer(128);
            assert!(!in_buf.is_null());
            assert!(!out_buf.is_null());

            for i in 0..128 {
                *in_buf.add(i) = 0.5;
            }

            biquad_process_quantum(chain, in_buf, out_buf, 128);
            assert!(!(*out_buf.add(127)).is_nan());

            biquad_free_buffer(in_buf, 128);
            biquad_free_buffer(out_buf, 128);
            biquad_destroy(chain);
        }
    }

    #[test]
    fn test_c_abi_binaural_quantum() {
        unsafe {
            let node = binaural_create(48000.0);
            assert!(!node.is_null());
            binaural_set_azimuth(node, 45.0);
            binaural_set_orbit_speed(node, 0.2);

            let in_buf = biquad_alloc_buffer(128);
            let out_l = biquad_alloc_buffer(128);
            let out_r = biquad_alloc_buffer(128);

            for i in 0..128 {
                *in_buf.add(i) = 0.5;
            }

            binaural_process_quantum(node, in_buf, out_l, out_r, 128);
            assert!(!(*out_l.add(127)).is_nan());
            assert!(!(*out_r.add(127)).is_nan());

            biquad_free_buffer(in_buf, 128);
            biquad_free_buffer(out_l, 128);
            biquad_free_buffer(out_r, 128);
            binaural_destroy(node);
        }
    }
}
