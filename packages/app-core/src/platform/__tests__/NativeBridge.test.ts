import { test, describe, beforeEach, afterEach } from "node:test";
import assert from "node:assert/strict";
import { NativeBridge } from "../NativeBridge.js";

describe("NativeBridge Tests", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  test("NativeBridge handles network failure gracefully without throwing", async () => {
    globalThis.fetch = async () => {
      throw new Error("Network unreachable");
    };

    const available = await NativeBridge.isNativeAvailable();
    assert.strictEqual(available, false);
    assert.strictEqual(NativeBridge.getStatus(), null);

    const decoded = await NativeBridge.decodeAudio(new ArrayBuffer(32));
    assert.strictEqual(decoded, null);

    const exported = await NativeBridge.exportAudio(new Float32Array([0.1, -0.1]), 2, 48000);
    assert.strictEqual(exported, null);
  });

  test("NativeBridge successfully decodes binary stream from native backend", async () => {
    // Construct fake binary packet:
    // [0..4] sample_rate = 44100
    // [4..8] num_channels = 2
    // [8..16] total_frames = 4
    // [16..] 4 f32s for ch0, 4 f32s for ch1
    const buffer = new ArrayBuffer(16 + 2 * 4 * 4);
    const view = new DataView(buffer);
    view.setUint32(0, 44100, true);
    view.setUint32(4, 2, true);
    view.setBigUint64(8, 4n, true);

    const f32View = new Float32Array(buffer, 16);
    f32View.set([0.1, 0.2, 0.3, 0.4, -0.1, -0.2, -0.3, -0.4]);

    globalThis.fetch = async (url: string | URL | Request) => {
      const urlStr = url.toString();
      if (urlStr.includes("/api/status")) {
        return new Response(
          JSON.stringify({
            status: "ok",
            backend: "flux-native",
            version: "0.1.0",
            sampleRate: 44100,
            platform: "windows",
            arch: "x86_64",
            simd: true,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        );
      }
      if (urlStr.includes("/api/decode")) {
        return new Response(buffer, {
          status: 200,
          headers: { "Content-Type": "application/octet-stream" },
        });
      }
      return new Response("Not found", { status: 404 });
    };

    // Force re-check
    (NativeBridge as unknown as { cachedStatus: unknown; checkAttempted: boolean }).cachedStatus =
      null;
    (NativeBridge as unknown as { checkAttempted: boolean }).checkAttempted = false;

    const available = await NativeBridge.isNativeAvailable();
    assert.strictEqual(available, true);
    assert.strictEqual(NativeBridge.getStatus()?.backend, "flux-native");

    const decoded = await NativeBridge.decodeAudio(new ArrayBuffer(10));
    assert.notStrictEqual(decoded, null);
    assert.strictEqual(decoded?.sampleRate, 44100);
    assert.strictEqual(decoded?.totalFrames, 4);
    assert.strictEqual(decoded?.channels.length, 2);
    assert.ok(Math.abs(decoded!.channels[0][0] - 0.1) < 1e-6);
  });

  test("NativeBridge successfully exports audio via native backend", async () => {
    const fakeWav = new Uint8Array([0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00]);

    globalThis.fetch = async (url: string | URL | Request) => {
      const urlStr = url.toString();
      if (urlStr.includes("/api/status")) {
        return new Response(
          JSON.stringify({
            status: "ok",
            backend: "flux-native",
            version: "0.1.0",
            sampleRate: 48000,
            platform: "windows",
            arch: "x86_64",
            simd: true,
          }),
          { status: 200 },
        );
      }
      if (urlStr.includes("/api/export")) {
        return new Response(fakeWav, {
          status: 200,
          headers: { "Content-Type": "audio/wav" },
        });
      }
      return new Response("Not found", { status: 404 });
    };

    (NativeBridge as unknown as { cachedStatus: unknown }).cachedStatus = {
      backend: "flux-native",
    };

    const exported = await NativeBridge.exportAudio(new Float32Array([0, 0]), 2, 48000, {
      format: "wav_16",
    });
    assert.notStrictEqual(exported, null);
    assert.strictEqual(exported?.length, fakeWav.length);
    assert.strictEqual(exported?.[0], 0x52); // 'R'
  });
});
