# Test stand-in for Chatterbox. Each model checks its folder holds what Flux downloaded, records every call to
# FLUX_STUB_LOG, and returns a tone whose length follows the text, so tests can measure the result.
import json
import math
import os

__version__ = "0.1.7+stub"


def _log(entry):
    p = os.environ.get("FLUX_STUB_LOG")
    if p:
        with open(p, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry) + "\n")


def _tone(seconds, hz=220.0, sr=24000):
    import torch
    amp = 0.3 + 0.01 * (torch._seed[0] % 10)
    return [amp * math.sin(2 * math.pi * hz * i / sr) for i in range(int(seconds * sr))]


class _Base:
    sr = 24000
    needs = ()
    kind = ""

    @classmethod
    def from_local(cls, ckpt_dir, device):
        missing = [f for f in cls.needs if not os.path.isfile(os.path.join(ckpt_dir, f))]
        if missing:
            raise FileNotFoundError(f"missing {missing[0]}")
        _log({"load": cls.kind, "device": device, "offline": os.environ.get("HF_HUB_OFFLINE")})
        return cls()

    def _speak(self, text, **kw):
        if "explode" in text:
            raise RuntimeError("the stub model was asked to fail")
        _log({"generate": self.kind, "text": text, **kw})
        return _tone(0.02 * len(text))
