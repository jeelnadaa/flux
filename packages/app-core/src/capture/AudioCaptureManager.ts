/**
 * AudioCaptureManager - High-fidelity microphone capture, device routing,
 * input metering, pre-roll buffering, and crash-safe chunked streaming to OPFS.
 */

export interface CaptureConfig {
  deviceId?: string;
  channelCount: number;
  sampleRate: number;
  rawProcessing: boolean;
  preRollSeconds: number;
  chunkDurationMs: number;
}

export interface MeterReading {
  peakDb: number[];
  rmsDb: number[];
  clipped: boolean[];
  clipHeld: boolean[];
}

export interface MonitoringStatus {
  enabled: boolean;
  estimatedLatencyMs: number;
  warning?: string;
}

export interface ChunkHeader {
  magic: string; // 'FCHK'
  sequence: number;
  sampleRate: number;
  channels: number;
  numFrames: number;
}

export class AudioCaptureManager {
  private config: CaptureConfig;
  private audioContext: AudioContext | null = null;
  private mediaStream: MediaStream | null = null;
  private sourceNode: MediaStreamAudioSourceNode | null = null;
  private monitorGainNode: GainNode | null = null;
  private isRecording = false;
  private isMonitoring = false;

  // Level metering state
  private lastPeaks: number[];
  private lastRms: number[];
  private lastClipped: boolean[];
  private lastClipHeld: boolean[];
  private clipHoldTimers: number[];

  // Pre-roll ring buffer (planar float arrays)
  private preRollBuffers: Float32Array[] = [];
  private preRollCapacityFrames = 0;
  private preRollWriteIndex = 0;
  private preRollCount = 0;

  // Chunked recorder state
  private chunkSequence = 0;
  private pendingChunkSamples: Float32Array[] = [];
  private framesPerChunk = 0;
  private opfsWriter: WritableStreamDefaultWriter<Uint8Array> | null = null;
  private recordedChunks: Uint8Array[] = [];
  private totalRecordedFrames = 0;

  constructor(config?: Partial<CaptureConfig>) {
    this.config = {
      channelCount: 1,
      sampleRate: 48000,
      rawProcessing: true, // echoCancellation: false, noiseSuppression: false, autoGainControl: false
      preRollSeconds: 2.0,
      chunkDurationMs: 500,
      ...config,
    };

    const channels = this.config.channelCount;
    this.lastPeaks = new Array(channels).fill(-120.0);
    this.lastRms = new Array(channels).fill(-120.0);
    this.lastClipped = new Array(channels).fill(false);
    this.lastClipHeld = new Array(channels).fill(false);
    this.clipHoldTimers = new Array(channels).fill(0);

    this.initPreRollBuffer();
    this.framesPerChunk = Math.round((this.config.sampleRate * this.config.chunkDurationMs) / 1000);
  }

  private initPreRollBuffer(): void {
    this.preRollCapacityFrames = Math.round(this.config.sampleRate * this.config.preRollSeconds);
    this.preRollBuffers = [];
    for (let ch = 0; ch < this.config.channelCount; ch++) {
      this.preRollBuffers.push(new Float32Array(this.preRollCapacityFrames));
    }
    this.preRollWriteIndex = 0;
    this.preRollCount = 0;
  }

  /**
   * Enumerate connected audio input devices.
   */
  public static async getAvailableDevices(): Promise<MediaDeviceInfo[]> {
    if (typeof navigator === "undefined" || !navigator.mediaDevices?.enumerateDevices) {
      return [];
    }
    const devices = await navigator.mediaDevices.enumerateDevices();
    return devices.filter((d) => d.kind === "audioinput");
  }

  /**
   * Initialize microphone capture stream.
   */
  public async initialize(audioCtx?: AudioContext): Promise<void> {
    if (typeof navigator === "undefined" || !navigator.mediaDevices?.getUserMedia) {
      return;
    }

    if (audioCtx) {
      this.audioContext = audioCtx;
    } else if (!this.audioContext && typeof AudioContext !== "undefined") {
      this.audioContext = new AudioContext({ sampleRate: this.config.sampleRate });
    }

    await this.startStream();
  }

  private async startStream(): Promise<void> {
    const audioConstraints: MediaTrackConstraints = {
      channelCount: { ideal: this.config.channelCount },
      sampleRate: { ideal: this.config.sampleRate },
    };

    if (this.config.deviceId) {
      audioConstraints.deviceId = { exact: this.config.deviceId };
    }

    if (this.config.rawProcessing) {
      // PLAN.md §1.2: echoCancellation, noiseSuppression, autoGainControl off by default for raw studio capture
      audioConstraints.echoCancellation = false;
      audioConstraints.noiseSuppression = false;
      audioConstraints.autoGainControl = false;
    } else {
      audioConstraints.echoCancellation = true;
      audioConstraints.noiseSuppression = true;
      audioConstraints.autoGainControl = true;
    }

    this.mediaStream = await navigator.mediaDevices.getUserMedia({ audio: audioConstraints });

    if (this.audioContext) {
      this.sourceNode = this.audioContext.createMediaStreamSource(this.mediaStream);
      this.monitorGainNode = this.audioContext.createGain();
      this.monitorGainNode.gain.value = this.isMonitoring ? 1.0 : 0.0;
      this.sourceNode.connect(this.monitorGainNode);
      this.monitorGainNode.connect(this.audioContext.destination);
    }
  }

  /**
   * Switch the active audio input device seamlessly.
   */
  public async setInputDevice(deviceId: string): Promise<void> {
    this.config.deviceId = deviceId;
    if (this.mediaStream) {
      for (const track of this.mediaStream.getTracks()) {
        track.stop();
      }
    }
    await this.startStream();
  }

  /**
   * Toggle raw vs voice-processed capture.
   */
  public async setRawProcessing(raw: boolean): Promise<void> {
    if (this.config.rawProcessing === raw) return;
    this.config.rawProcessing = raw;
    if (this.mediaStream) {
      for (const track of this.mediaStream.getTracks()) {
        track.stop();
      }
      await this.startStream();
    }
  }

  /**
   * Software monitoring toggle with latency calculation and acoustic feedback warning.
   */
  public setMonitoring(enabled: boolean): MonitoringStatus {
    this.isMonitoring = enabled;
    if (this.monitorGainNode) {
      this.monitorGainNode.gain.value = enabled ? 1.0 : 0.0;
    }

    const baseLatency = this.audioContext ? (this.audioContext.baseLatency || 0.01) * 1000 : 10;
    const outputLatency = this.audioContext
      ? ((this.audioContext as AudioContext & { outputLatency?: number }).outputLatency || 0.02) *
        1000
      : 20;
    const estimatedLatencyMs = Math.round(baseLatency + outputLatency);

    let warning: string | undefined;
    if (enabled && estimatedLatencyMs > 50) {
      warning = `High monitoring latency (${estimatedLatencyMs} ms). Use headphones to prevent acoustic feedback.`;
    }

    return {
      enabled,
      estimatedLatencyMs,
      warning,
    };
  }

  /**
   * Processes an incoming audio block (called by AudioWorklet or ScriptProcessor/test feed).
   */
  public processAudioBlock(channelBuffers: Float32Array[]): void {
    const channels = Math.min(channelBuffers.length, this.config.channelCount);
    const numFrames = channelBuffers[0].length;

    // 1. Level metering
    for (let ch = 0; ch < channels; ch++) {
      const samples = channelBuffers[ch];
      let sumSq = 0.0;
      let peak = 0.0;
      let clipped = false;

      for (let i = 0; i < numFrames; i++) {
        const s = samples[i];
        const abs = Math.abs(s);
        if (abs > peak) peak = abs;
        sumSq += s * s;
        if (abs >= 0.99999) {
          clipped = true;
        }
      }

      const peakDb = peak > 1e-6 ? 20.0 * Math.log10(peak) : -120.0;
      const rmsDb = sumSq > 1e-12 ? 10.0 * Math.log10(sumSq / numFrames) : -120.0;

      this.lastPeaks[ch] = peakDb;
      this.lastRms[ch] = rmsDb;
      this.lastClipped[ch] = clipped;

      if (clipped) {
        this.lastClipHeld[ch] = true;
        this.clipHoldTimers[ch] = 96000; // 2 seconds hold at 48 kHz
      } else if (this.clipHoldTimers[ch] > 0) {
        this.clipHoldTimers[ch] -= numFrames;
        if (this.clipHoldTimers[ch] <= 0) {
          this.lastClipHeld[ch] = false;
        }
      }
    }

    // 2. Pre-roll ring buffer update
    for (let ch = 0; ch < channels; ch++) {
      const src = channelBuffers[ch];
      const dst = this.preRollBuffers[ch];
      let writeIdx = this.preRollWriteIndex;

      for (let i = 0; i < numFrames; i++) {
        dst[writeIdx] = src[i];
        writeIdx = (writeIdx + 1) % this.preRollCapacityFrames;
      }
    }
    this.preRollWriteIndex = (this.preRollWriteIndex + numFrames) % this.preRollCapacityFrames;
    this.preRollCount = Math.min(this.preRollCapacityFrames, this.preRollCount + numFrames);

    // 3. Recording sink
    if (this.isRecording) {
      if (this.pendingChunkSamples.length === 0) {
        for (let ch = 0; ch < channels; ch++) {
          this.pendingChunkSamples.push(new Float32Array(this.framesPerChunk));
        }
      }

      let framesCopied = 0;
      while (framesCopied < numFrames) {
        const currentInPending = this.totalRecordedFrames % this.framesPerChunk;
        const availableInChunk = this.framesPerChunk - currentInPending;
        const toCopy = Math.min(availableInChunk, numFrames - framesCopied);

        for (let ch = 0; ch < channels; ch++) {
          this.pendingChunkSamples[ch].set(
            channelBuffers[ch].subarray(framesCopied, framesCopied + toCopy),
            currentInPending,
          );
        }

        framesCopied += toCopy;
        this.totalRecordedFrames += toCopy;

        if (currentInPending + toCopy >= this.framesPerChunk) {
          this.flushPendingChunk();
        }
      }
    }
  }

  /**
   * Flushes pending chunk into a crash-safe binary chunk (FCHK).
   */
  private flushPendingChunk(actualFrames?: number): void {
    if (this.pendingChunkSamples.length === 0) return;

    const channels = this.config.channelCount;
    const numFrames = actualFrames ?? this.framesPerChunk;
    if (numFrames === 0) return;

    const payloadBytes = numFrames * channels * 4;
    const totalChunkBytes = 32 + payloadBytes + 4; // 32-byte header + payload + 4-byte footer

    const chunk = new Uint8Array(totalChunkBytes);
    const view = new DataView(chunk.buffer);

    // Magic: 'FCHK' (0x4643484B)
    view.setUint8(0, 0x46);
    view.setUint8(1, 0x43);
    view.setUint8(2, 0x48);
    view.setUint8(3, 0x4b);

    view.setUint32(4, this.chunkSequence, true);
    view.setBigUint64(8, BigInt(this.totalRecordedFrames - numFrames), true);
    view.setUint32(16, this.config.sampleRate, true);
    view.setUint16(20, channels, true);
    view.setUint16(22, 0, true); // reserved
    view.setUint32(24, numFrames, true);

    // Interleave planar float32 audio and copy
    let offset = 32;
    for (let f = 0; f < numFrames; f++) {
      for (let ch = 0; ch < channels; ch++) {
        view.setFloat32(offset, this.pendingChunkSamples[ch][f], true);
        offset += 4;
      }
    }

    // Compute simple CRC-32 for payload
    const crc = this.computeCrc32(chunk.subarray(32, 32 + payloadBytes));
    view.setUint32(28, crc, true);

    // Footer: 'KCHF' (0x4B434846)
    view.setUint8(offset, 0x4b);
    view.setUint8(offset + 1, 0x43);
    view.setUint8(offset + 2, 0x48);
    view.setUint8(offset + 3, 0x46);

    this.chunkSequence++;
    this.recordedChunks.push(chunk);

    if (this.opfsWriter) {
      this.opfsWriter.write(chunk).catch((err) => {
        console.error("Failed to write recording chunk to OPFS:", err);
      });
    }
  }

  private computeCrc32(data: Uint8Array): number {
    let crc = 0xffffffff;
    for (let i = 0; i < data.length; i++) {
      crc ^= data[i];
      for (let j = 0; j < 8; j++) {
        crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
      }
    }
    return (crc ^ 0xffffffff) >>> 0;
  }

  /**
   * Start recording with optional direct OPFS stream writer.
   * If pre-roll is enabled, automatically prepend pre-roll audio!
   */
  public async startRecording(opfsWriter?: WritableStreamDefaultWriter<Uint8Array>): Promise<void> {
    this.opfsWriter = opfsWriter || null;
    this.recordedChunks = [];
    this.chunkSequence = 0;
    this.totalRecordedFrames = 0;
    this.isRecording = true;

    // Prepend pre-roll buffer if available
    if (this.preRollCount > 0) {
      const channels = this.config.channelCount;
      const preRollFrames = this.preRollCount;
      const preRollExtracted: Float32Array[] = [];

      const startIdx =
        (this.preRollWriteIndex - preRollFrames + this.preRollCapacityFrames) %
        this.preRollCapacityFrames;

      for (let ch = 0; ch < channels; ch++) {
        const out = new Float32Array(preRollFrames);
        const ring = this.preRollBuffers[ch];
        for (let i = 0; i < preRollFrames; i++) {
          out[i] = ring[(startIdx + i) % this.preRollCapacityFrames];
        }
        preRollExtracted.push(out);
      }

      // Record pre-roll audio immediately
      let framesDone = 0;
      while (framesDone < preRollFrames) {
        const toTake = Math.min(this.framesPerChunk, preRollFrames - framesDone);
        const subChannels: Float32Array[] = [];
        for (let ch = 0; ch < channels; ch++) {
          subChannels.push(preRollExtracted[ch].subarray(framesDone, framesDone + toTake));
        }
        this.processAudioBlock(subChannels);
        framesDone += toTake;
      }
    }
  }

  /**
   * Stop recording and return summary and concatenated chunks.
   */
  public async stopRecording(): Promise<{
    totalFrames: number;
    chunksCount: number;
    data: Uint8Array;
  }> {
    this.isRecording = false;

    // Flush any trailing samples
    const currentInPending = this.totalRecordedFrames % this.framesPerChunk;
    if (currentInPending > 0) {
      this.flushPendingChunk(currentInPending);
    }

    if (this.opfsWriter) {
      try {
        await this.opfsWriter.close();
      } catch (err) {
        console.warn("Error closing OPFS writer:", err);
      }
      this.opfsWriter = null;
    }

    const totalLength = this.recordedChunks.reduce((acc, c) => acc + c.length, 0);
    const combined = new Uint8Array(totalLength);
    let offset = 0;
    for (const chunk of this.recordedChunks) {
      combined.set(chunk, offset);
      offset += chunk.length;
    }

    return {
      totalFrames: this.totalRecordedFrames,
      chunksCount: this.chunkSequence,
      data: combined,
    };
  }

  /**
   * Recover all valid chunks from a raw stream buffer (e.g. after unexpected crash/power-loss).
   */
  public static recoverRecording(streamData: Uint8Array): Float32Array[] {
    const view = new DataView(streamData.buffer, streamData.byteOffset, streamData.byteLength);
    let cursor = 0;
    const chunkBlocks: { channels: number; numFrames: number; samples: Float32Array[] }[] = [];
    let detectedChannels = 0;

    while (cursor + 36 <= streamData.length) {
      // Check magic 'FCHK'
      const magic =
        String.fromCharCode(view.getUint8(cursor)) +
        String.fromCharCode(view.getUint8(cursor + 1)) +
        String.fromCharCode(view.getUint8(cursor + 2)) +
        String.fromCharCode(view.getUint8(cursor + 3));

      if (magic !== "FCHK") {
        break; // Corrupted or truncated chunk boundary
      }

      const channels = view.getUint16(cursor + 20, true);
      const numFrames = view.getUint32(cursor + 24, true);
      const crcExpected = view.getUint32(cursor + 28, true);
      const payloadBytes = numFrames * channels * 4;

      if (cursor + 32 + payloadBytes + 4 > streamData.length) {
        // Truncated trailing chunk; discard safely
        break;
      }

      // Verify footer 'KCHF'
      const footerOffset = cursor + 32 + payloadBytes;
      const footer =
        String.fromCharCode(view.getUint8(footerOffset)) +
        String.fromCharCode(view.getUint8(footerOffset + 1)) +
        String.fromCharCode(view.getUint8(footerOffset + 2)) +
        String.fromCharCode(view.getUint8(footerOffset + 3));

      if (footer !== "KCHF") {
        break; // Invalid chunk footer
      }

      // Check CRC32
      const payload = streamData.subarray(cursor + 32, cursor + 32 + payloadBytes);
      let crc = 0xffffffff;
      for (let i = 0; i < payload.length; i++) {
        crc ^= payload[i];
        for (let j = 0; j < 8; j++) {
          crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
        }
      }
      crc = (crc ^ 0xffffffff) >>> 0;

      if (crc !== crcExpected) {
        break; // Corrupt payload
      }

      detectedChannels = channels;
      const channelSamples: Float32Array[] = [];
      for (let ch = 0; ch < channels; ch++) {
        channelSamples.push(new Float32Array(numFrames));
      }

      let readOffset = cursor + 32;
      for (let f = 0; f < numFrames; f++) {
        for (let ch = 0; ch < channels; ch++) {
          channelSamples[ch][f] = view.getFloat32(readOffset, true);
          readOffset += 4;
        }
      }

      chunkBlocks.push({ channels, numFrames, samples: channelSamples });
      cursor = footerOffset + 4;
    }

    if (chunkBlocks.length === 0 || detectedChannels === 0) {
      return [];
    }

    const totalFrames = chunkBlocks.reduce((acc, b) => acc + b.numFrames, 0);
    const result: Float32Array[] = [];
    for (let ch = 0; ch < detectedChannels; ch++) {
      const channelOut = new Float32Array(totalFrames);
      let offset = 0;
      for (const block of chunkBlocks) {
        channelOut.set(block.samples[ch], offset);
        offset += block.numFrames;
      }
      result.push(channelOut);
    }

    return result;
  }

  /**
   * Get current meter levels.
   */
  public getLevelReading(): MeterReading {
    return {
      peakDb: [...this.lastPeaks],
      rmsDb: [...this.lastRms],
      clipped: [...this.lastClipped],
      clipHeld: [...this.lastClipHeld],
    };
  }

  /**
   * Clear clipping hold.
   */
  public clearClips(): void {
    this.lastClipHeld.fill(false);
    this.clipHoldTimers.fill(0);
  }

  /**
   * Cleanup resources.
   */
  public dispose(): void {
    this.isRecording = false;
    if (this.mediaStream) {
      for (const track of this.mediaStream.getTracks()) {
        track.stop();
      }
      this.mediaStream = null;
    }
    if (this.sourceNode) {
      this.sourceNode.disconnect();
      this.sourceNode = null;
    }
    if (this.monitorGainNode) {
      this.monitorGainNode.disconnect();
      this.monitorGainNode = null;
    }
    if (this.audioContext && this.audioContext.state !== "closed") {
      this.audioContext.close();
      this.audioContext = null;
    }
  }
}
