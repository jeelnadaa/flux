from . import _Base


class ChatterboxTurboTTS(_Base):
    needs = ("a.bin", "turbo.json")
    kind = "turbo"

    def generate(self, text, audio_prompt_path=None, temperature=0.8, top_p=0.95, top_k=1000):
        # Like the real model (transformers' sampler), a whole-number setting must arrive as an int.
        if not isinstance(top_k, int):
            raise ValueError(f"`top_k` has to be a strictly positive integer, but is {top_k}")
        return self._speak(text, audio_prompt_path=audio_prompt_path, temperature=temperature, top_p=top_p, top_k=top_k)
