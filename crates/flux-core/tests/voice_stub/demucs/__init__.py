# Test stand-in for Demucs. Checks the model folder holds the definition Flux downloaded and the audio is stereo
# float at 44.1 kHz, then splits by fixed gains that sum back to the input: vocals 1/2, drums 1/4, bass and other
# 1/8 each (6 stems: other 1/16, guitar and piano 1/32 each).
import json
import os

import _wav

FLUX_STUB = True


def flux_stub_run(req, say):
    name = req["name"]
    if not os.path.isfile(os.path.join(req["ckpt_dir"], name + ".yaml")):
        raise FileNotFoundError(f"missing {name}.yaml")
    a, ch, sr = _wav.read(req["input"])
    if ch != 2 or sr != 44100:
        raise RuntimeError(f"expected 44.1 kHz stereo, got {ch} channels at {sr}")
    gains = {"drums": 0.25, "bass": 0.125, "other": 0.125, "vocals": 0.5}
    if name.endswith("6s"):
        gains = {"drums": 0.25, "bass": 0.125, "other": 0.0625, "vocals": 0.5, "guitar": 0.03125, "piano": 0.03125}
    _wav.log({"stems": name, "device": req.get("device"), "frames": len(a) // ch})
    os.makedirs(req["out"], exist_ok=True)
    for i, (stem, g) in enumerate(gains.items()):
        _wav.write(os.path.join(req["out"], stem + ".wav"), [g * v for v in a], ch, sr)
        say("PROGRESS", round((i + 1) / len(gains), 3))
    say("RESULT", json.dumps({"device": "cpu", "stems": list(gains), "sr": sr}))
