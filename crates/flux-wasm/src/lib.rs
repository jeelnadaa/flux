//! Flux WebAssembly bindings for browser execution and AudioWorklet rendering.
#![deny(missing_docs)]

use flux_dsp::binaural::BinauralNode;
use flux_dsp::biquad::{BiquadChain, BiquadType};
use flux_dsp::node::DspNode;
use flux_dsp::stretch::StretchNode;
use flux_engine::capture::{ChunkedRecordWriter, InputLevelMeter, PreRollBuffer};
use flux_engine::codec::{AudioMetadata, Mp3Encoder, OpusEncoder, VorbisEncoder};
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
    let mut vec = vec![0.0f32; len];
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
    if !ptr.is_null() && len > 0 {
        drop(Vec::from_raw_parts(ptr, len, len));
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

/// Creates a new `StretchNode` instance.
#[no_mangle]
pub extern "C" fn stretch_create(sample_rate: f32) -> *mut StretchNode {
    let mut node = Box::new(StretchNode::new());
    node.prepare(sample_rate, 2);
    Box::into_raw(node)
}

/// Destroys a `StretchNode` instance.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `stretch_create`.
#[no_mangle]
pub unsafe extern "C" fn stretch_destroy(ptr: *mut StretchNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets the time-stretch playback ratio.
///
/// # Safety
/// `ptr` must be a valid pointer to a `StretchNode`.
#[no_mangle]
pub unsafe extern "C" fn stretch_set_ratio(ptr: *mut StretchNode, ratio: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_stretch_ratio(ratio);
    }
}

/// Sets the pitch shift in semitones.
///
/// # Safety
/// `ptr` must be a valid pointer to a `StretchNode`.
#[no_mangle]
pub unsafe extern "C" fn stretch_set_pitch(ptr: *mut StretchNode, semitones: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_pitch_shift(semitones);
    }
}

/// Processes an audio quantum through the time/pitch stretcher.
///
/// # Safety
/// `ptr` must be valid, and `in_ptr` and `out_ptr` must point to valid slices of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn stretch_process_quantum(
    ptr: *mut StretchNode,
    in_ptr: *const f32,
    out_ptr: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let in_slice = std::slice::from_raw_parts(in_ptr, frames);
        let out_slice = std::slice::from_raw_parts_mut(out_ptr, frames);
        node.process(&[in_slice], &mut [out_slice]);
    }
}

/// Encodes audio samples to MP3 in WebAssembly.
///
/// # Safety
/// `samples_ptr` must point to `samples_len` floats, and `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn codec_encode_mp3(
    samples_ptr: *const f32,
    samples_len: usize,
    sample_rate: u32,
    out_len: *mut usize,
) -> *mut u8 {
    let samples = std::slice::from_raw_parts(samples_ptr, samples_len);
    let encoder = Mp3Encoder::new(sample_rate, 2, 128);
    let meta = AudioMetadata::default();
    let mut encoded = encoder.encode(samples, &meta);
    encoded.shrink_to_fit();
    *out_len = encoded.len();
    let ptr = encoded.as_mut_ptr();
    std::mem::forget(encoded);
    ptr
}

/// Encodes audio samples to Ogg Opus in WebAssembly.
///
/// # Safety
/// `samples_ptr` must point to `samples_len` floats, and `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn codec_encode_opus(
    samples_ptr: *const f32,
    samples_len: usize,
    sample_rate: u32,
    out_len: *mut usize,
) -> *mut u8 {
    let samples = std::slice::from_raw_parts(samples_ptr, samples_len);
    let encoder = OpusEncoder::new(sample_rate, 2);
    let meta = AudioMetadata::default();
    let mut encoded = encoder.encode(samples, &meta);
    encoded.shrink_to_fit();
    *out_len = encoded.len();
    let ptr = encoded.as_mut_ptr();
    std::mem::forget(encoded);
    ptr
}

/// Encodes audio samples to Ogg Vorbis in WebAssembly.
///
/// # Safety
/// `samples_ptr` must point to `samples_len` floats, and `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn codec_encode_vorbis(
    samples_ptr: *const f32,
    samples_len: usize,
    sample_rate: u32,
    out_len: *mut usize,
) -> *mut u8 {
    let samples = std::slice::from_raw_parts(samples_ptr, samples_len);
    let encoder = VorbisEncoder::new(sample_rate, 2);
    let meta = AudioMetadata::default();
    let mut encoded = encoder.encode(samples, &meta);
    encoded.shrink_to_fit();
    *out_len = encoded.len();
    let ptr = encoded.as_mut_ptr();
    std::mem::forget(encoded);
    ptr
}

/// Frees an encoded byte buffer allocated by `codec_encode_*`.
///
/// # Safety
/// `ptr` must be a valid pointer returned by a codec encode function.
#[no_mangle]
pub unsafe extern "C" fn codec_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// Decodes in-memory WAV or AIFF audio bytes into interleaved 32-bit floats.
///
/// # Safety
/// `bytes_ptr` must point to `bytes_len` bytes. Out pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn codec_decode_audio(
    bytes_ptr: *const u8,
    bytes_len: usize,
    out_channels: *mut usize,
    out_frames: *mut usize,
    out_sample_rate: *mut u32,
    out_samples_len: *mut usize,
) -> *mut f32 {
    let bytes = std::slice::from_raw_parts(bytes_ptr, bytes_len);
    match flux_engine::decoder::decode_buffer(bytes) {
        Ok(buf) => {
            *out_channels = buf.num_channels();
            *out_frames = buf.num_frames();
            *out_sample_rate = buf.sample_rate;
            let mut interleaved = buf.to_interleaved();
            interleaved.shrink_to_fit();
            *out_samples_len = interleaved.len();
            let ptr = interleaved.as_mut_ptr();
            std::mem::forget(interleaved);
            ptr
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees an interleaved float buffer allocated by `codec_decode_audio`.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `codec_decode_audio` with length `len`.
#[no_mangle]
pub unsafe extern "C" fn codec_free_audio_buffer(ptr: *mut f32, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Vec::from_raw_parts(ptr, len, len));
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

/// WebAssembly wrapper for input level meter.
#[wasm_bindgen]
pub struct WasmLevelMeter {
    inner: InputLevelMeter,
    last_peaks: Vec<f32>,
    last_rms: Vec<f32>,
    last_clipped: Vec<bool>,
    last_clip_held: Vec<bool>,
}

#[wasm_bindgen]
impl WasmLevelMeter {
    /// Creates a new `WasmLevelMeter`.
    #[wasm_bindgen(constructor)]
    pub fn new(num_channels: usize, sample_rate: u32) -> Self {
        Self {
            inner: InputLevelMeter::new(num_channels, sample_rate),
            last_peaks: vec![-120.0; num_channels],
            last_rms: vec![-120.0; num_channels],
            last_clipped: vec![false; num_channels],
            last_clip_held: vec![false; num_channels],
        }
    }

    /// Processes an interleaved audio buffer of length `frames * channels`.
    pub fn process_interleaved(&mut self, input: &[f32], channels: usize) {
        if channels == 0 || input.is_empty() {
            return;
        }
        let num_frames = input.len() / channels;
        let mut deinterleaved = vec![Vec::with_capacity(num_frames); channels];
        for frame in 0..num_frames {
            let base = frame * channels;
            for ch in 0..channels {
                deinterleaved[ch].push(input[base + ch]);
            }
        }
        let channel_refs: Vec<&[f32]> = deinterleaved.iter().map(|c| c.as_slice()).collect();
        let readings = self.inner.process(&channel_refs);
        for (i, r) in readings.into_iter().enumerate() {
            if i < self.last_peaks.len() {
                self.last_peaks[i] = r.peak_db;
                self.last_rms[i] = r.rms_db;
                self.last_clipped[i] = r.clipped;
                self.last_clip_held[i] = r.clip_held;
            }
        }
    }

    /// Gets peak level in dBFS for channel.
    pub fn get_peak_db(&self, channel: usize) -> f32 {
        self.last_peaks.get(channel).copied().unwrap_or(-120.0)
    }

    /// Gets RMS level in dBFS for channel.
    pub fn get_rms_db(&self, channel: usize) -> f32 {
        self.last_rms.get(channel).copied().unwrap_or(-120.0)
    }

    /// Returns whether channel is currently clipping.
    pub fn is_clipped(&self, channel: usize) -> bool {
        self.last_clipped.get(channel).copied().unwrap_or(false)
    }

    /// Returns whether channel has a held clip indication.
    pub fn is_clip_held(&self, channel: usize) -> bool {
        self.last_clip_held.get(channel).copied().unwrap_or(false)
    }

    /// Clears held clip indicators.
    pub fn clear_clips(&mut self) {
        self.inner.clear_clips();
        self.last_clip_held.fill(false);
    }
}

/// WebAssembly wrapper for pre-roll circular audio buffer.
#[wasm_bindgen]
pub struct WasmPreRollBuffer {
    inner: PreRollBuffer,
    channels: usize,
}

#[wasm_bindgen]
impl WasmPreRollBuffer {
    /// Creates a new `WasmPreRollBuffer`.
    #[wasm_bindgen(constructor)]
    pub fn new(num_channels: usize, sample_rate: u32, duration_seconds: f64) -> Self {
        Self {
            inner: PreRollBuffer::new(num_channels, sample_rate, duration_seconds),
            channels: num_channels,
        }
    }

    /// Appends interleaved frames to pre-roll buffer.
    pub fn append_interleaved(&mut self, input: &[f32]) {
        if self.channels == 0 || input.is_empty() {
            return;
        }
        let num_frames = input.len() / self.channels;
        let mut deinterleaved = vec![Vec::with_capacity(num_frames); self.channels];
        for frame in 0..num_frames {
            let base = frame * self.channels;
            for ch in 0..self.channels {
                deinterleaved[ch].push(input[base + ch]);
            }
        }
        let refs: Vec<&[f32]> = deinterleaved.iter().map(|c| c.as_slice()).collect();
        self.inner.append(&refs);
    }

    /// Drains all buffered pre-roll audio as an interleaved vector.
    pub fn drain_interleaved(&mut self) -> Vec<f32> {
        let planar = self.inner.drain();
        if planar.is_empty() || planar[0].is_empty() {
            return Vec::new();
        }
        let num_frames = planar[0].len();
        let mut interleaved = Vec::with_capacity(num_frames * self.channels);
        for frame in 0..num_frames {
            for ch in planar.iter().take(self.channels) {
                interleaved.push(ch[frame]);
            }
        }
        interleaved
    }

    /// Number of frames currently in pre-roll buffer.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clears pre-roll buffer.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

/// WebAssembly wrapper for crash-safe chunked recording stream.
#[wasm_bindgen]
pub struct WasmChunkedRecordWriter {
    inner: ChunkedRecordWriter,
    channels: usize,
}

#[wasm_bindgen]
impl WasmChunkedRecordWriter {
    /// Creates a new `WasmChunkedRecordWriter`.
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            inner: ChunkedRecordWriter::new(sample_rate, channels),
            channels: channels as usize,
        }
    }

    /// Encodes an interleaved audio block into a self-contained, crash-safe chunk.
    pub fn encode_chunk(&mut self, input: &[f32]) -> Result<Vec<u8>, JsValue> {
        if self.channels == 0 || input.is_empty() {
            return Ok(Vec::new());
        }
        let num_frames = input.len() / self.channels;
        let mut deinterleaved = vec![Vec::with_capacity(num_frames); self.channels];
        for frame in 0..num_frames {
            let base = frame * self.channels;
            for ch in 0..self.channels {
                deinterleaved[ch].push(input[base + ch]);
            }
        }
        let refs: Vec<&[f32]> = deinterleaved.iter().map(|c| c.as_slice()).collect();
        let mut out = Vec::new();
        self.inner
            .write_chunk(&mut out, &refs)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(out)
    }

    /// Recovers all valid audio chunks from a stream buffer, returning interleaved float samples.
    pub fn recover_interleaved(data: &[u8]) -> Result<Vec<f32>, JsValue> {
        let mut cursor = std::io::Cursor::new(data);
        let audio = ChunkedRecordWriter::recover_from_stream(&mut cursor)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let num_channels = audio.channels.len();
        let num_frames = audio.num_frames();
        let mut interleaved = Vec::with_capacity(num_frames * num_channels);

        for frame in 0..num_frames {
            for ch in audio.channels.iter().take(num_channels) {
                interleaved.push(ch[frame]);
            }
        }

        Ok(interleaved)
    }

    /// Converts a stream of chunked recording bytes into a standard WAV file.
    pub fn to_wav(data: &[u8]) -> Result<Vec<u8>, JsValue> {
        let mut cursor = std::io::Cursor::new(data);
        let mut wav_out = std::io::Cursor::new(Vec::new());
        ChunkedRecordWriter::convert_chunks_to_wav(&mut cursor, &mut wav_out)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(wav_out.into_inner())
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

    #[test]
    fn test_c_abi_stretch_quantum() {
        unsafe {
            let node = stretch_create(48000.0);
            assert!(!node.is_null());
            stretch_set_ratio(node, 1.25);
            stretch_set_pitch(node, 3.0);

            let in_buf = biquad_alloc_buffer(128);
            let out_buf = biquad_alloc_buffer(128);

            for i in 0..128 {
                *in_buf.add(i) = 0.5;
            }

            stretch_process_quantum(node, in_buf, out_buf, 128);
            assert!(!(*out_buf.add(127)).is_nan());

            biquad_free_buffer(in_buf, 128);
            biquad_free_buffer(out_buf, 128);
            stretch_destroy(node);
        }
    }

    #[test]
    fn test_c_abi_codec_encoders() {
        unsafe {
            let samples = vec![0.5f32; 2304];
            let mut out_len = 0usize;

            // MP3
            let mp3_ptr = codec_encode_mp3(samples.as_ptr(), samples.len(), 48000, &mut out_len);
            assert!(!mp3_ptr.is_null());
            assert!(out_len > 100);
            codec_free_buffer(mp3_ptr, out_len);

            // Opus
            let opus_ptr = codec_encode_opus(samples.as_ptr(), samples.len(), 48000, &mut out_len);
            assert!(!opus_ptr.is_null());
            assert!(out_len > 100);
            codec_free_buffer(opus_ptr, out_len);

            // Vorbis
            let vorbis_ptr =
                codec_encode_vorbis(samples.as_ptr(), samples.len(), 48000, &mut out_len);
            assert!(!vorbis_ptr.is_null());
            assert!(out_len > 100);
            codec_free_buffer(vorbis_ptr, out_len);

            // WAV Decode via C ABI
            let ch0 = vec![0.5f32; 100];
            let ch1 = vec![-0.5f32; 100];
            let wav_bytes = flux_engine::decoder::WavWriter::encode(
                &[&ch0, &ch1],
                48000,
                flux_engine::decoder::WavEncoding::Float32,
                None,
                false,
            )
            .unwrap();

            let mut out_channels = 0usize;
            let mut out_frames = 0usize;
            let mut out_sr = 0u32;
            let mut out_samples_len = 0usize;

            let audio_ptr = codec_decode_audio(
                wav_bytes.as_ptr(),
                wav_bytes.len(),
                &mut out_channels,
                &mut out_frames,
                &mut out_sr,
                &mut out_samples_len,
            );
            assert!(!audio_ptr.is_null());
            assert_eq!(out_channels, 2);
            assert_eq!(out_frames, 100);
            assert_eq!(out_sr, 48000);
            assert_eq!(out_samples_len, 200);

            // Interleaved samples should alternate ch0 and ch1
            assert_eq!(*audio_ptr, 0.5);
            assert_eq!(*audio_ptr.add(1), -0.5);

            codec_free_audio_buffer(audio_ptr, out_samples_len);
        }
    }

    #[test]
    fn test_wasm_capture_wrappers() {
        // Meter test
        let mut meter = WasmLevelMeter::new(2, 48000);
        let interleaved = vec![0.5f32, -0.5f32, 0.5f32, -0.5f32];
        meter.process_interleaved(&interleaved, 2);
        assert!((meter.get_peak_db(0) - (-6.02)).abs() < 0.1);
        assert!(!meter.is_clipped(0));

        // PreRollBuffer test
        let mut preroll = WasmPreRollBuffer::new(2, 48000, 0.1);
        assert!(preroll.is_empty());
        preroll.append_interleaved(&[0.1, 0.2, 0.3, 0.4]);
        assert_eq!(preroll.len(), 2);
        let drained = preroll.drain_interleaved();
        assert_eq!(drained, vec![0.1, 0.2, 0.3, 0.4]);
        assert!(preroll.is_empty());

        // ChunkedRecordWriter test
        let mut writer = WasmChunkedRecordWriter::new(48000, 2);
        let chunk_bytes = writer.encode_chunk(&[0.1, 0.2, 0.3, 0.4]).unwrap();
        assert!(!chunk_bytes.is_empty());
        let recovered = WasmChunkedRecordWriter::recover_interleaved(&chunk_bytes).unwrap();
        assert_eq!(recovered, vec![0.1, 0.2, 0.3, 0.4]);
        let wav = WasmChunkedRecordWriter::to_wav(&chunk_bytes).unwrap();
        assert!(wav.starts_with(b"RIFF"));
    }
}
