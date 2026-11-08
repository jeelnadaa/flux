# Flux shim for phonemizer (GPL, wraps eSpeak NG). Importing it works; making a backend fails, so Kokoro's reader
# turns its eSpeak fallback off and Flux's own fallback spells unknown words out. Flux code, MIT.
from . import backend  # noqa: F401
