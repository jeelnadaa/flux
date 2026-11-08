# Flux shim for PyAV. faster-whisper imports it to decode files, but Flux hands it decoded audio, so it is never
# used. PyAV's wheels bundle FFmpeg (LGPL), which Flux doesn't install. Flux code, MIT.


def open(*_args, **_kwargs):  # noqa: A001
    raise RuntimeError("Flux sends decoded audio; PyAV isn't installed")
