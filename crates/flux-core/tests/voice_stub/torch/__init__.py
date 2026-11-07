# Test stand-in for PyTorch: just enough for flux_voice.py. No graphics chip, seeded like the real one.
from . import backends  # noqa: F401

__version__ = "2.6.0+stub"
_seed = [0]


class cuda:
    @staticmethod
    def is_available():
        return False


def manual_seed(s):
    _seed[0] = int(s)
