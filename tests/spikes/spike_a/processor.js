/* global AudioWorkletProcessor, registerProcessor, WebAssembly */
// AudioWorklet processor for Spike A: WASM Biquad Chain with 128-sample render quantum

class FluxBiquadProcessor extends AudioWorkletProcessor {
  constructor(_options) {
    super();
    this.wasmInstance = null;
    this.chainPtr = null;
    this.outBufPtr = null;
    this.initialized = false;
    this.phase = 0.0;
    this.sampleRate = 48000.0;

    this.port.onmessage = async (event) => {
      const data = event.data;
      if (data.type === "INIT") {
        try {
          this.sampleRate = data.sampleRate || 48000.0;
          const wasmModule = data.wasmModule;
          const importObj = {
            __wbindgen_placeholder__: new Proxy({}, { get: () => () => {} }),
            __wbindgen_externref_xform__: new Proxy({}, { get: () => () => {} }),
          };
          this.wasmInstance = await WebAssembly.instantiate(wasmModule, importObj);
          const exp = this.wasmInstance.exports;

          // Create 8-stage biquad chain
          this.chainPtr = exp.biquad_create(this.sampleRate, 8);

          // Configure stages:
          // Stage 0: 1 kHz peaking filter (+3 dB gain)
          exp.biquad_configure_peaking(this.chainPtr, 0, 1000.0, 1.0, 3.0);
          // Stage 1: Highpass 40 Hz
          exp.biquad_configure_highpass(this.chainPtr, 1, 40.0, 0.707);
          // Stage 2: Lowpass 18 kHz
          exp.biquad_configure_lowpass(this.chainPtr, 2, 18000.0, 0.707);

          this.outBufPtr = exp.biquad_alloc_buffer(128);
          this.initialized = true;
          this.port.postMessage({ type: "READY" });
        } catch (err) {
          this.port.postMessage({ type: "ERROR", error: err.message });
        }
      }
    };
  }

  process(_inputs, outputs, _parameters) {
    if (!this.initialized || !this.chainPtr) {
      return true;
    }

    const exp = this.wasmInstance.exports;
    const output = outputs[0];
    const channel0 = output[0];
    const frames = channel0 ? channel0.length : 128;

    // Synthesize 1 kHz tone into outBufPtr
    const memoryF32 = new Float32Array(exp.memory.buffer, this.outBufPtr, frames);
    const omega = (2.0 * Math.PI * 1000.0) / this.sampleRate;
    for (let i = 0; i < frames; i++) {
      memoryF32[i] = Math.sin(this.phase);
      this.phase += omega;
      if (this.phase >= 2.0 * Math.PI) {
        this.phase -= 2.0 * Math.PI;
      }
    }

    // Process in-place through WASM biquad chain
    exp.biquad_process_quantum(this.chainPtr, this.outBufPtr, this.outBufPtr, frames);

    // Transfer to AudioWorklet output channel
    if (channel0) {
      channel0.set(memoryF32);
    }

    return true;
  }
}

registerProcessor("flux-biquad-processor", FluxBiquadProcessor);
