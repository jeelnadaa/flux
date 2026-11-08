# Flux voice runner. Flux starts this with its own Python to run its downloaded models on this computer: voice
# cloning (Chatterbox), text to speech (Kokoro, or Chatterbox's built-in voice), transcripts (Whisper, through
# faster-whisper) and stems (Demucs).
#
#   python flux_voice.py check            -> one JSON line: what the runtime can do
#   python flux_voice.py run request.json -> "PROGRESS <0..1>" lines, then "RESULT <json>" (or "ERROR <message>")
#
# Models load only from the folders Flux downloaded and checked; the Hub is switched off, so nothing is fetched or
# sent while speaking. Output is 16-bit mono WAV; Flux labels it synthetic afterwards. Chatterbox adds its own
# inaudible watermark.
import array
import inspect
import json
import os
import struct
import sys
import wave

# Stand-ins for modules some packages import but Flux never calls (they would bring in LGPL code): go last, so a
# real package always wins.
sys.path.append(os.path.join(os.path.dirname(os.path.abspath(__file__)), "flux_shims"))

os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")


def say(kind, value):
    sys.stdout.write(f"{kind} {value}\n")
    sys.stdout.flush()


def pick_device(torch, want):
    has_cuda = bool(torch.cuda.is_available())
    has_mps = bool(getattr(getattr(torch, "backends", None), "mps", None) and torch.backends.mps.is_available())
    if want == "cuda" and not has_cuda:
        raise RuntimeError("This computer's graphics chip can't be used by the voice engine. Choose Processor or Auto.")
    if want == "mps" and not has_mps:
        raise RuntimeError("Apple graphics cores aren't available here. Choose Processor or Auto.")
    if want in ("cuda", "mps", "cpu"):
        return want
    return "cuda" if has_cuda else "mps" if has_mps else "cpu"


def check():
    import torch
    import chatterbox
    from chatterbox import tts_turbo, tts, mtl_tts, vc  # noqa: F401  every loader must import
    import perth
    if getattr(perth, "PerthImplicitWatermarker", None) is None:
        # perth hides its import errors; without the watermark the models refuse to load.
        from perth.perth_net.perth_net_implicit.perth_watermarker import PerthImplicitWatermarker  # noqa: F401  raises the real error
    out = {
        "ok": True,
        "python": sys.version.split()[0],
        "torch": getattr(torch, "__version__", "?"),
        "chatterbox": getattr(chatterbox, "__version__", None) or _dist_version("chatterbox-tts"),
        "cuda": bool(torch.cuda.is_available()),
        "mps": bool(getattr(getattr(torch, "backends", None), "mps", None) and torch.backends.mps.is_available()),
    }
    print(json.dumps(out))


def _dist_version(name):
    try:
        from importlib.metadata import version
        return version(name)
    except Exception:
        return None


def load(loader, ckpt, device):
    if loader == "turbo":
        from chatterbox.tts_turbo import ChatterboxTurboTTS as M
    elif loader == "english":
        from chatterbox.tts import ChatterboxTTS as M
    elif loader == "multilingual":
        from chatterbox.mtl_tts import ChatterboxMultilingualTTS as M
    elif loader == "vc":
        from chatterbox.vc import ChatterboxVC as M
    else:
        raise RuntimeError(f"unknown model type {loader}")
    return M.from_local(ckpt, device)


def samples(wav):
    x = wav
    if hasattr(x, "detach"):
        x = x.detach().cpu()
    if hasattr(x, "squeeze"):
        x = x.squeeze()
    x = x.tolist() if hasattr(x, "tolist") else list(x)
    if x and isinstance(x[0], list):  # (channels, n): keep the first
        x = x[0]
    return x


def write_wav(path, xs, sr):
    a = array.array("h", (int(max(-1.0, min(1.0, v)) * 32767) for v in xs))
    if sys.byteorder == "big":
        a.byteswap()
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(int(sr))
        w.writeframes(a.tobytes())


def call(fn, **kw):
    # Pass only what this model's generate() accepts, so one request shape fits every loader.
    # Settings arrive from JSON as floats; whole-number settings (top_k) go back to int, by the model's own default.
    sig = inspect.signature(fn)
    ps = sig.parameters
    for k, v in list(kw.items()):
        d = ps[k].default if k in ps else None
        if isinstance(v, float) and isinstance(d, int) and not isinstance(d, bool):
            kw[k] = int(round(v))
    if any(p.kind == p.VAR_KEYWORD for p in ps.values()):
        return fn(**kw)
    return fn(**{k: v for k, v in kw.items() if k in ps})


# ---------------------------------------------------------------- audio files

def read_wav(path):
    """32-bit float or 16-bit WAV -> (interleaved array('f'), channels, rate)."""
    with open(path, "rb") as f:
        b = f.read()
    if b[0:4] != b"RIFF" or b[8:12] != b"WAVE":
        raise RuntimeError("not a WAV file")
    i, fmt, data, sub = 12, None, None, None
    while i + 8 <= len(b):
        cid, n = b[i:i + 4], struct.unpack("<I", b[i + 4:i + 8])[0]
        if cid == b"fmt ":
            fmt = struct.unpack("<HHIIHH", b[i + 8:i + 24])
            if n >= 40:  # WAVE_FORMAT_EXTENSIBLE carries the real format in its subformat GUID
                sub = struct.unpack("<H", b[i + 32:i + 34])[0]
        elif cid == b"data":
            data = b[i + 8:i + 8 + n]
        i += 8 + n + (n & 1)
    if not fmt or data is None:
        raise RuntimeError("WAV without format or data")
    tag, ch, sr, _, _, bits = fmt
    if tag == 0xFFFE and sub is not None:
        tag = sub
    if tag == 3 and bits == 32:
        a = array.array("f")
        a.frombytes(data[: len(data) // 4 * 4])
    elif tag == 1 and bits == 16:
        h = array.array("h")
        h.frombytes(data[: len(data) // 2 * 2])
        a = array.array("f", (v / 32768.0 for v in h))
    else:
        raise RuntimeError("Flux sends 32-bit float or 16-bit WAV")
    if sys.byteorder == "big":
        a.byteswap()
    return a, ch, sr


def write_wav_f32(path, interleaved, ch, sr):
    a = array.array("f", interleaved)
    if sys.byteorder == "big":
        a.byteswap()
    raw = a.tobytes()
    with open(path, "wb") as f:
        f.write(b"RIFF" + struct.pack("<I", 36 + len(raw)) + b"WAVE")
        f.write(b"fmt " + struct.pack("<IHHIIHH", 16, 3, ch, int(sr), int(sr) * ch * 4, ch * 4, 32))
        f.write(b"data" + struct.pack("<I", len(raw)) + raw)


def stub(mod):
    """Test stand-ins (crates/flux-core/tests/voice_stub) answer the request themselves."""
    return getattr(mod, "FLUX_STUB", False)


# ---------------------------------------------------------------- transcripts (faster-whisper)

def op_transcribe(req):
    import torch  # first: its CUDA libraries are the ones CTranslate2 uses
    import faster_whisper
    if stub(faster_whisper):
        return faster_whisper.flux_stub_run(req, say)
    from faster_whisper import WhisperModel
    import numpy as np
    device = pick_device(torch, req.get("device", "auto"))
    if device == "mps":
        device = "cpu"  # CTranslate2 has no Apple graphics backend
    model = WhisperModel(req["ckpt_dir"], device=device, compute_type="float16" if device == "cuda" else "int8", local_files_only=True)
    say("PROGRESS", 0.1)
    a, ch, sr = read_wav(req["input"])
    x = np.frombuffer(a.tobytes(), dtype=np.float32).reshape(-1, ch).mean(axis=1)
    if sr != 16000:
        raise RuntimeError("Flux sends 16 kHz audio for transcripts")
    lang = req.get("language") or None
    segs, info = model.transcribe(x, language=None if lang in (None, "", "auto") else lang, word_timestamps=True,
                                  beam_size=int(round(req.get("params", {}).get("beam_size", 5))), vad_filter=False)
    total = max(float(info.duration), 0.001)
    out = []
    for sg in segs:
        out.append({"start": round(sg.start, 3), "end": round(sg.end, 3), "text": sg.text.strip(),
                    "words": [{"start": round(w.start, 3), "end": round(w.end, 3), "word": w.word, "p": round(w.probability, 3)} for w in (sg.words or [])]})
        say("PROGRESS", round(min(0.99, 0.1 + 0.89 * sg.end / total), 3))
    with open(req["out"], "w", encoding="utf-8") as f:
        json.dump({"language": info.language, "language_probability": round(info.language_probability, 3), "segments": out}, f)
    say("RESULT", json.dumps({"device": device, "language": info.language, "segments": len(out)}))


# ---------------------------------------------------------------- stems (Demucs)

def op_stems(req):
    import torch
    import demucs
    if stub(demucs):
        return demucs.flux_stub_run(req, say)
    from pathlib import Path
    import numpy as np
    import demucs.apply as A
    from demucs.pretrained import get_model
    device = pick_device(torch, req.get("device", "auto"))
    model = get_model(req["name"], repo=Path(req["ckpt_dir"]))
    model.eval()
    a, ch, sr = read_wav(req["input"])
    if sr != model.samplerate or ch != model.audio_channels:
        raise RuntimeError(f"Flux sends {model.audio_channels} channels at {model.samplerate} Hz for this model")
    wav = torch.from_numpy(np.frombuffer(a.tobytes(), dtype=np.float32).reshape(-1, ch).T.copy())
    ref = wav.mean(0)
    mean, std = ref.mean(), ref.std() + 1e-8
    passes = [len(getattr(model, "models", [model]))]
    done = [0]

    class Bar:  # demucs reports progress through tqdm; count its chunks instead
        def __init__(self, it, **kw):
            self.it = list(it)

        def __iter__(self):
            n = max(1, len(self.it))
            for i, x in enumerate(self.it):
                yield x
                say("PROGRESS", round(min(0.98, 0.05 + 0.93 * (done[0] + (i + 1) / n) / passes[0]), 3))
            done[0] += 1

    A.tqdm.tqdm = Bar
    with torch.no_grad():
        out = A.apply_model(model, ((wav - mean) / std)[None], device=device, shifts=1, split=True, overlap=0.25, progress=True, num_workers=0)[0]
    out = (out * std + mean).cpu().numpy()
    names = list(model.sources)
    os.makedirs(req["out"], exist_ok=True)
    for k, name in enumerate(names):
        write_wav_f32(os.path.join(req["out"], name + ".wav"), out[k].T.reshape(-1).astype(np.float32).tolist(), ch, sr)
    say("RESULT", json.dumps({"device": device, "stems": names, "sr": sr}))


# ---------------------------------------------------------------- text to speech (Kokoro, or Chatterbox's own voice)

LETTERS = {"a": "ˈA", "b": "bˈi", "c": "sˈi", "d": "dˈi", "e": "ˈi", "f": "ˈɛf", "g": "ʤˈi", "h": "ˈAʧ", "i": "ˈI",
           "j": "ʤˈA", "k": "kˈA", "l": "ˈɛl", "m": "ˈɛm", "n": "ˈɛn", "o": "ˈO", "p": "pˈi", "q": "kjˈu", "r": "ˈɑɹ",
           "s": "ˈɛs", "t": "tˈi", "u": "jˈu", "v": "vˈi", "w": "dˈʌbᵊlju", "x": "ˈɛks", "y": "wˈI", "z": "zˈi"}


class SpellOut:
    """Words the reader doesn't know are spelled out, letter by letter, rather than skipped."""

    def __call__(self, token):
        ps = [LETTERS[c] for c in token.text.lower() if c in LETTERS]
        return (" ".join(ps) if ps else None), 1


def op_tts(req):
    import torch
    loader = req["loader"]
    segs = req["segments"]
    seed = int(req.get("seed", 0))
    params = dict(req.get("params") or {})
    out = []
    if loader == "kokoro":
        import kokoro
        if stub(kokoro):
            return kokoro.flux_stub_run(req, say)
        from kokoro import KModel, KPipeline
        device = pick_device(torch, req.get("device", "auto"))
        ck = req["ckpt_dir"]
        km = KModel(repo_id="hexgrad/Kokoro-82M", config=os.path.join(ck, "config.json"), model=os.path.join(ck, "kokoro-v1_0.pth")).to(device).eval()
        voice = req["voice"]
        pipe = KPipeline(lang_code="b" if voice.startswith("b") else "a", repo_id="hexgrad/Kokoro-82M", model=km)
        g2p = getattr(pipe, "g2p", None)
        if g2p is not None and getattr(g2p, "fallback", None) is None:
            g2p.fallback = SpellOut()
        vp = os.path.join(ck, "voices", voice + ".pt")
        sr = 24000
        say("PROGRESS", 0.15)
        for i, s in enumerate(segs):
            if s["type"] == "silence":
                out.extend([0.0] * int(sr * int(s["value"]) / 1000))
            else:
                torch.manual_seed(seed + i)
                for r in pipe(s["value"], voice=vp, speed=float(params.get("speed", 1.0))):
                    if r.audio is not None:
                        out.extend(r.audio.detach().cpu().tolist())
            say("PROGRESS", round(0.15 + 0.8 * (i + 1) / len(segs), 3))
    else:
        device = pick_device(torch, req.get("device", "auto"))
        model = load(loader, req["ckpt_dir"], device)
        sr = int(getattr(model, "sr", 24000))
        say("PROGRESS", 0.15)
        for i, s in enumerate(segs):
            if s["type"] == "silence":
                out.extend([0.0] * int(sr * int(s["value"]) / 1000))
            else:
                torch.manual_seed(seed + i)
                out.extend(samples(call(model.generate, **dict(params, text=s["value"]))))
            say("PROGRESS", round(0.15 + 0.8 * (i + 1) / len(segs), 3))
    write_wav(req["out"], out, sr)
    say("RESULT", json.dumps({"sr": sr, "seconds": round(len(out) / sr, 3), "device": device}))


def run(req_path):
    with open(req_path, encoding="utf-8") as f:
        req = json.load(f)
    op = req.get("op")
    if op == "transcribe":
        return op_transcribe(req)
    if op == "stems":
        return op_stems(req)
    if op == "tts":
        return op_tts(req)
    import torch
    device = pick_device(torch, req.get("device", "auto"))
    say("PROGRESS", 0.02)
    model = load(req["loader"], req["ckpt_dir"], device)
    sr = int(getattr(model, "sr", 24000))
    say("PROGRESS", 0.15)
    seed = int(req.get("seed", 0))
    out = []
    if req["op"] == "convert":
        torch.manual_seed(seed)
        wav = call(model.generate, audio=req["input"], target_voice_path=req["reference"])
        out = samples(wav)
        say("PROGRESS", 0.95)
    else:
        segs = req["segments"]
        params = dict(req.get("params") or {})
        if req["loader"] == "multilingual":
            params["language_id"] = req.get("language", "en")
        prompted = False
        for i, s in enumerate(segs):
            if s["type"] == "silence":
                out.extend([0.0] * int(sr * int(s["value"]) / 1000))
            else:
                torch.manual_seed(seed + i)
                kw = dict(params, text=s["value"])
                if not prompted:
                    kw["audio_prompt_path"] = req["reference"]
                    prompted = True
                out.extend(samples(call(model.generate, **kw)))
            say("PROGRESS", round(0.15 + 0.8 * (i + 1) / len(segs), 3))
    write_wav(req["out"], out, sr)
    say("RESULT", json.dumps({"sr": sr, "seconds": round(len(out) / sr, 3), "device": device}))


if __name__ == "__main__":
    try:
        if sys.argv[1:2] == ["check"]:
            check()
        elif sys.argv[1:2] == ["run"] and len(sys.argv) == 3:
            run(sys.argv[2])
        else:
            raise SystemExit("usage: flux_voice.py check | run request.json")
    except SystemExit:
        raise
    except Exception as e:  # one line Flux can show; no audio, text or file names in it
        msg = f"{type(e).__name__}: {e}".splitlines()[0][:400]
        say("ERROR", msg)
        sys.exit(1)
