# Flux voice runner. Flux starts this with its own Python to clone voices on this computer.
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
import sys
import wave

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


def run(req_path):
    with open(req_path, encoding="utf-8") as f:
        req = json.load(f)
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
