# Small WAV helpers shared by the test stand-ins (the real packages bring their own audio code).
import array
import json
import os
import struct
import sys


def log(entry):
    p = os.environ.get("FLUX_STUB_LOG")
    if p:
        with open(p, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry) + "\n")


def read(path):
    with open(path, "rb") as f:
        b = f.read()
    i, fmt, data, sub = 12, None, None, None
    while i + 8 <= len(b):
        cid, n = b[i:i + 4], struct.unpack("<I", b[i + 4:i + 8])[0]
        if cid == b"fmt ":
            fmt = struct.unpack("<HHIIHH", b[i + 8:i + 24])
            sub = struct.unpack("<H", b[i + 32:i + 34])[0] if n >= 40 else None
        elif cid == b"data":
            data = b[i + 8:i + 8 + n]
        i += 8 + n + (n & 1)
    tag, ch, sr, _, _, bits = fmt
    if tag == 0xFFFE:  # WAVE_FORMAT_EXTENSIBLE: the real format is the first two bytes of the subformat GUID
        tag = sub
    assert tag == 3 and bits == 32, "Flux sends 32-bit float WAV"
    a = array.array("f")
    a.frombytes(data)
    if sys.byteorder == "big":
        a.byteswap()
    return a, ch, sr


def write(path, interleaved, ch, sr):
    a = array.array("f", interleaved)
    if sys.byteorder == "big":
        a.byteswap()
    raw = a.tobytes()
    with open(path, "wb") as f:
        f.write(b"RIFF" + struct.pack("<I", 36 + len(raw)) + b"WAVE")
        f.write(b"fmt " + struct.pack("<IHHIIHH", 16, 3, ch, sr, sr * ch * 4, ch * 4, 32))
        f.write(b"data" + struct.pack("<I", len(raw)) + raw)
