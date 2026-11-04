import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ByokAiManager } from "../ByokAiManager.js";

describe("ByokAiManager", () => {
  it("initializes providers and supports key management", () => {
    const ai = ByokAiManager.get();
    const providers = ai.getProviders();

    assert.ok(providers.length >= 4);
    const groq = providers.find((p) => p.provider === "groq");
    assert.ok(groq);

    // Add and remove keys
    ai.addKey("groq", "mock_groq_key_alpha_1");
    ai.addKey("groq", "mock_groq_key_beta_2");
    assert.equal(ai.getKeys("groq").length, 2);
    assert.equal(ai.getActiveKey("groq"), "mock_groq_key_alpha_1");

    // Key rotation
    const nextKey = ai.rotateKey("groq");
    assert.equal(nextKey, "mock_groq_key_beta_2");

    ai.removeKey("groq", 0);
    assert.equal(ai.getKeys("groq").length, 1);
  });

  it("falls back to local transcript when no network/keys provided", async () => {
    const ai = ByokAiManager.get();
    // Test with mock blob
    const blob = new Blob(["dummy audio"], { type: "audio/wav" });
    const result = await ai.transcribeAudio(blob);

    assert.ok(result.text.length > 0);
    assert.ok(result.words.length > 0);
    assert.ok(result.providerUsed.includes("Local"));
  });

  it("synthesizes local fallback speech audio", async () => {
    const ai = ByokAiManager.get();
    const result = await ai.synthesizeSpeech("Testing Flux Speech Synthesis");

    assert.ok(result.audioBuffer.byteLength > 44);
    assert.equal(result.format, "wav");
  });
});
