# Flux shim, MIT. See phonemizer/__init__.py.


class EspeakWrapper:
    @classmethod
    def set_library(cls, *_args):
        pass

    @classmethod
    def set_data_path(cls, *_args):
        pass
