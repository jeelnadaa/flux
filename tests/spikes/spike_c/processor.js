/* global AudioWorkletProcessor, registerProcessor, WebAssembly */

class FluxBinauralProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.wasmInstance = null;
    this.exports = null;
    this.binauralPtr = null;
    this.inBufPtr = null;
    this.outLBufPtr = null;
    this.outRBufPtr = null;
    this.isReady = false;

    this.port.onmessage = async (event) => {
      const { type, wasmModule, sampleRate, azimuth, orbitSpeed } = event.data;
      if (type === "INIT") {
        try {
          const importObj = {
            __wbindgen_placeholder__: new Proxy({}, { get: () => () => {} }),
            __wbindgen_externref_xform__: new Proxy({}, { get: () => () => {} }),
          };
          this.wasmInstance = await WebAssembly.instantiate(wasmModule, importObj);
          this.exports = this.wasmInstance.exports;

          this.binauralPtr = this.exports.binaural_create(sampleRate || 48000.0);
          if (azimuth !== undefined) {
            this.exports.binaural_set_azimuth(this.binauralPtr, azimuth);
          }
          if (orbitSpeed !== undefined) {
            this.exports.binaural_set_orbit_speed(this.binauralPtr, orbitSpeed);
          }

          // Allocate 128-frame buffers
          this.inBufPtr = this.exports.biquad_alloc_buffer(128);
          this.outLBufPtr = this.exports.biquad_alloc_buffer(128);
          this.outRBufPtr = this.exports.biquad_alloc_buffer(128);

          this.isReady = true;
          this.port.postMessage({ type: "READY" });
        } catch (err) {
          this.port.postMessage({ type: "ERROR", error: err.message });
        }
      } else if (type === "SET_AZIMUTH") {
        if (this.binauralPtr && this.exports) {
          this.exports.binaural_set_azimuth(this.binauralPtr, azimuth);
        }
      } else if (type === "SET_ORBIT_SPEED") {
        if (this.binauralPtr && this.exports) {
          this.exports.binaural_set_orbit_speed(this.binauralPtr, orbitSpeed);
        }
      }
    };
  }

  process(inputs, outputs, _parameters) {
    if (!this.isReady || !this.exports || !this.binauralPtr) {
      return true;
    }

    const output = outputs[0];
    if (!output || output.length === 0) return true;

    const outL = output[0];
    const outR = output.length > 1 ? output[1] : output[0];
    const frames = outL.length;

    const input = inputs[0];
    const inChannel = input && input.length > 0 ? input[0] : null;

    const mem = this.exports.memory.buffer;
    const inView = new Float32Array(mem, this.inBufPtr, frames);

    if (inChannel) {
      inView.set(inChannel);
    } else {
      inView.fill(0.0);
    }

    this.exports.binaural_process_quantum(
      this.binauralPtr,
      this.inBufPtr,
      this.outLBufPtr,
      this.outRBufPtr,
      frames,
    );

    const outLView = new Float32Array(mem, this.outLBufPtr, frames);
    const outRView = new Float32Array(mem, this.outRBufPtr, frames);

    outL.set(outLView);
    if (output.length > 1) {
      outR.set(outRView);
    }

    return true;
  }
}

registerProcessor("flux-binaural-processor", FluxBinauralProcessor);
