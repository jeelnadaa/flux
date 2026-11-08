# Test stand-in for faster-whisper. Checks the model folder and the audio Flux sends (16 kHz mono float), then
# "hears" a fixed sentence spread over the audio, with fillers and a repeat, so tests can check word positions.
import json
import os

import _wav

FLUX_STUB = True
WORDS = ["So", "um", "I", "I", "think", "uh", "it", "works."]


def flux_stub_run(req, say):
    if not os.path.isfile(os.path.join(req["ckpt_dir"], "model.bin")):
        raise FileNotFoundError("missing model.bin")
    a, ch, sr = _wav.read(req["input"])
    if ch != 1 or sr != 16000:
        raise RuntimeError(f"expected 16 kHz mono, got {ch} channels at {sr}")
    secs = len(a) / sr
    _wav.log({"transcribe": True, "language": req.get("language"), "device": req.get("device"), "seconds": round(secs, 3)})
    step = secs / len(WORDS)
    words = [{"start": round(i * step, 3), "end": round(i * step + step * 0.8, 3), "word": " " + w, "p": 0.9} for i, w in enumerate(WORDS)]
    say("PROGRESS", 0.5)
    with open(req["out"], "w", encoding="utf-8") as f:
        json.dump({"language": "en", "language_probability": 0.99,
                   "segments": [{"start": 0.0, "end": round(secs, 3), "text": " ".join(WORDS), "words": words}]}, f)
    say("RESULT", json.dumps({"device": "cpu", "language": "en", "segments": 1}))
