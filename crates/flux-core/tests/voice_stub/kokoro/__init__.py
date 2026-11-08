# Test stand-in for Kokoro. Checks the model and voice files, then returns a tone whose length follows the text
# and the speed, as 16-bit WAV at 24 kHz (like the real runner writes).
import json
import math
import os
import time
import wave

import _wav

FLUX_STUB = True


def flux_stub_run(req, say):
    ck = req["ckpt_dir"]
    for f in ("config.json", "kokoro-v1_0.pth"):
        if not os.path.isfile(os.path.join(ck, f)):
            raise FileNotFoundError(f"missing {f}")
    voice = req["voice"]
    if not os.path.isfile(os.path.join(ck, "voices", voice + ".pt")):
        raise FileNotFoundError(f"missing voice {voice}")
    speed = float((req.get("params") or {}).get("speed", 1.0))
    text = " ".join(s["value"] for s in req["segments"] if s["type"] == "text")
    _wav.log({"tts": "kokoro", "voice": voice, "speed": speed, "chars": len(text), "segments": req["segments"]})
    # For testing a run that takes a while, and one that fails.
    say("STATUS", "Loading the model")
    say("PROGRESS", 0.15)
    say("STATUS", "Speaking part 1 of 1")
    time.sleep(float(os.environ.get("FLUX_STUB_TTS_SLEEP", "0")))
    if "fail-this-run" in text:
        raise RuntimeError("the stub was told to fail")
    sr = 24000
    n = int(sr * 0.05 * len(text) / speed) + sum(int(sr * int(s["value"]) / 1000) for s in req["segments"] if s["type"] == "silence")
    with wave.open(req["out"], "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(b"".join(int(8000 * math.sin(2 * math.pi * 330 * i / sr)).to_bytes(2, "little", signed=True) for i in range(n)))
    say("PROGRESS", 0.9)
    say("RESULT", json.dumps({"sr": sr, "seconds": round(n / sr, 3), "device": "cpu"}))
