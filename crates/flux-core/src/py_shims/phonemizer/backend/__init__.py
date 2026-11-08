# Flux shim, MIT. See phonemizer/__init__.py.
from . import espeak  # noqa: F401


class EspeakBackend:
    def __init__(self, *_args, **_kwargs):
        raise RuntimeError("eSpeak NG isn't installed with Flux")
