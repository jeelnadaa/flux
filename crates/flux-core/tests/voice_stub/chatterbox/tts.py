from . import _Base


class ChatterboxTTS(_Base):
    needs = ("a.bin", "english.json")
    kind = "english"

    def generate(self, text, audio_prompt_path=None, exaggeration=0.5, cfg_weight=0.5):
        return self._speak(text, audio_prompt_path=audio_prompt_path, exaggeration=exaggeration, cfg_weight=cfg_weight)
