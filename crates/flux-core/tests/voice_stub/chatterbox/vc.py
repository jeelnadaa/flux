import os

from . import _Base, _log, _tone


class ChatterboxVC(_Base):
    needs = ("a.bin",)
    kind = "vc"

    def generate(self, audio, target_voice_path=None):
        if not os.path.isfile(audio) or not os.path.isfile(target_voice_path):
            raise FileNotFoundError("input or voice missing")
        _log({"generate": "vc", "has_input": True, "has_target": True})
        return _tone(1.5, 330.0)
