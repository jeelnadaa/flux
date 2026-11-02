#!/usr/bin/env python3
"""Generates reference WAV, WAVEX, RF64, and AIFF fixtures using libsndfile (via python soundfile)

and records the libsndfile-decoded float32 samples into a JSON manifest for bit-exact verification.
"""

import json
from pathlib import Path
import numpy as np
import soundfile as sf


def main():
    out_dir = Path("tests/fixtures/libsndfile")
    out_dir.mkdir(parents=True, exist_ok=True)

    n_frames = 200
    t = np.linspace(0, 0.1, n_frames, endpoint=False, dtype=np.float32)
    # Channel 0: 440 Hz tone, Channel 1: 1000 Hz tone
    ch0 = 0.8 * np.sin(2 * np.pi * 440 * t)
    ch1 = 0.6 * np.cos(2 * np.pi * 1000 * t)
    stereo_signal = np.stack([ch0, ch1], axis=1)  # shape (200, 2)
    mono_signal = ch0.reshape(-1, 1)  # shape (200, 1)

    cases = [
        ("ref_wav_pcm16.wav", stereo_signal, 48000, "WAV", "PCM_16"),
        ("ref_wav_pcm24.wav", stereo_signal, 48000, "WAV", "PCM_24"),
        ("ref_wav_pcm32.wav", stereo_signal, 44100, "WAV", "PCM_32"),
        ("ref_wav_float32.wav", stereo_signal, 48000, "WAV", "FLOAT"),
        ("ref_wav_float64.wav", stereo_signal, 96000, "WAV", "DOUBLE"),
        ("ref_wav_u8.wav", mono_signal, 22050, "WAV", "PCM_U8"),
        ("ref_wavex_pcm24.wav", stereo_signal, 48000, "WAVEX", "PCM_24"),
        ("ref_rf64_pcm24.rf64", stereo_signal, 48000, "RF64", "PCM_24"),
        ("ref_aiff_s8.aiff", mono_signal, 22050, "AIFF", "PCM_S8"),
        ("ref_aiff_pcm16.aiff", stereo_signal, 44100, "AIFF", "PCM_16"),
        ("ref_aiff_pcm24.aiff", stereo_signal, 48000, "AIFF", "PCM_24"),
        ("ref_aiff_pcm32.aiff", stereo_signal, 96000, "AIFF", "PCM_32"),
        ("ref_aiff_float32.aiff", stereo_signal, 48000, "AIFF", "FLOAT"),
    ]

    manifest = {}

    for filename, sig, sr, fmt, subtype in cases:
        filepath = out_dir / filename
        sf.write(str(filepath), sig, sr, format=fmt, subtype=subtype)
        decoded, read_sr = sf.read(str(filepath), dtype="float32", always_2d=True)
        assert read_sr == sr

        manifest[filename] = {
            "format": fmt,
            "subtype": subtype,
            "sample_rate": sr,
            "channels": int(decoded.shape[1]),
            "frames": int(decoded.shape[0]),
            "channel_samples": [decoded[:, ch].tolist() for ch in range(decoded.shape[1])],
        }

    with open(out_dir / "manifest.json", "w") as f:
        json.dump(manifest, f, indent=2)

    print(f"Generated {len(cases)} libsndfile fixtures in {out_dir}")


if __name__ == "__main__":
    main()
