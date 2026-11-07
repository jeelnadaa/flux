from . import _Base


class ChatterboxTurboTTS(_Base):
    needs = ("a.bin", "turbo.json")
    kind = "turbo"

    def generate(self, text, audio_prompt_path=None, temperature=0.8, top_p=0.95):
        return self._speak(text, audio_prompt_path=audio_prompt_path, temperature=temperature, top_p=top_p)
