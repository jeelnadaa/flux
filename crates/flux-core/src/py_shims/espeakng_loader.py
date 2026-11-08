# Flux shim: Kokoro's text reader can fall back to eSpeak NG (GPL) for unknown words. Flux doesn't install it and
# spells unknown words out instead. Flux code, MIT.


def get_library_path():
    return ""


def get_data_path():
    return ""
