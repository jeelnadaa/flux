# Flux Developer Guide: Preset Format (.fluxpreset)

- **Specification Version:** 1.0.0
- **File Extension:** `.fluxpreset`
- **MIME Type:** `application/vnd.flux.preset+json`

---

## 1. Overview
A `.fluxpreset` is a deterministic, versioned JSON document representing an effect processor configuration or multi-effect rack chain. All parameter ranges are normalized or documented in SI units (Hz, dB, seconds, ratios) and are validated against a strict schema.

---

## 2. JSON Schema

```json
{
  "$schema": "https://flux.audio/schemas/preset-v1.json",
  "version": 1,
  "id": "com.flux.presets.vocal_clarity",
  "name": "Vocal Clarity",
  "category": "Vocals",
  "author": "Flux Audio",
  "description": "High-pass cleanup, presence lift, and gentle optical compression for dialogue and vocals.",
  "target": "track",
  "effects": [
    {
      "nodeType": "eq",
      "bypass": false,
      "mix": 1.0,
      "parameters": {
        "bands": [
          { "type": "highpass", "frequency": 80.0, "q": 0.707, "gainDb": 0.0, "enabled": true },
          { "type": "peaking", "frequency": 350.0, "q": 1.4, "gainDb": -2.5, "enabled": true },
          { "type": "peaking", "frequency": 3200.0, "q": 1.0, "gainDb": 3.0, "enabled": true },
          { "type": "highshelf", "frequency": 10000.0, "q": 0.707, "gainDb": 2.0, "enabled": true }
        ]
      }
    },
    {
      "nodeType": "compressor",
      "bypass": false,
      "mix": 1.0,
      "parameters": {
        "thresholdDb": -18.0,
        "ratio": 3.0,
        "attackMs": 15.0,
        "releaseMs": 120.0,
        "kneeDb": 6.0,
        "autoMakeup": true
      }
    }
  ]
}
```

---

## 3. Schema Fields

| Field | Type | Description |
|---|---|---|
| `version` | Integer | Preset schema format version (currently `1`). |
| `id` | String | Reverse-DNS unique identifier for the preset. |
| `name` | String | Human-readable title displayed in the Effect Browser. |
| `category` | String | Filtering category: `Vocals`, `Drums`, `Bass`, `Master`, `Spatial`, `Creative`. |
| `target` | String | Scope of preset: `clip`, `track`, `master`. |
| `effects` | Array | Ordered array of DSP nodes executed sequentially. |

---

## 4. Validation & Error Handling

Flux validates all loaded presets before execution:
1. **Schema Check:** Verifies presence of mandatory fields (`version`, `id`, `name`, `effects`).
2. **Bounds Check:** Ensures all numeric parameters fall within hardware safety limits (e.g. frequencies in $10\text{--}24000$ Hz, gains in $-60\text{--}+24$ dB).
3. **Hostile Input Protection:** Prototype pollution keys (`__proto__`, `constructor`) are stripped.
4. **Graceful Fallback:** If an effect node references an unavailable module, Flux marks that specific node bypassed while safely loading the remainder of the chain.
