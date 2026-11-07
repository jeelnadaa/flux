from . import _Base


class ChatterboxMultilingualTTS(_Base):
    needs = ("a.bin", "mtl.json")
    kind = "multilingual"

    def generate(self, text, language_id, audio_prompt_path=None, exaggeration=0.5):
        return self._speak(text, language_id=language_id, audio_prompt_path=audio_prompt_path, exaggeration=exaggeration)
