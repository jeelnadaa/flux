"""End-to-end test of Smart tools (choosing models for stems, transcripts and speech), the transcript editor and
filler review, in a real browser.

Runs flux-mcp's HTTP server with a throwaway data folder whose catalog points at a local mirror (a small HTTP server
in this process) and at the stub Python runtime in crates/flux-core/tests/voice_stub, so nothing is fetched from the
internet and no real model runs. Online engines are turned on but never run, so no audio leaves the computer. A key
goes into the system keychain under a throwaway service name and is forgotten again before the test ends.

    python tests/e2e/test_smart_tools.py     (needs `cargo build -p flux-mcp --release` first)
"""
import hashlib
import io
import json
import math
import os
import struct
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request
import uuid
import wave
from pathlib import Path

from playwright.sync_api import sync_playwright

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_clone_models import free_port, api, mirror, platform_key  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target" / "release" / ("flux-mcp.exe" if os.name == "nt" else "flux-mcp")
OUT = ROOT / "target" / "e2e"
STUB = ROOT / "crates" / "flux-core" / "tests" / "voice_stub"
REV = "0123456789abcdef0123456789abcdef01234567"
# Keys a developer may have set; the test must not pick them up.
KEY_VARS = ["OPENAI_API_KEY", "GROQ_API_KEY", "DEEPGRAM_API_KEY", "ELEVENLABS_API_KEY", "XI_API_KEY", "REPLICATE_API_TOKEN", "HF_TOKEN"]


def speech(path, secs=4.0):
    """A stereo 48 kHz tone with gaps, standing in for a voice recording."""
    sr = 48000
    with wave.open(str(path), "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(sr)
        fr = bytearray()
        for i in range(int(sr * secs)):
            v = int(8000 * math.sin(2 * math.pi * 180 * i / sr)) if (i // 6000) % 2 == 0 else 0
            fr += struct.pack("<hh", v, v)
        w.writeframes(bytes(fr))
    return str(path)


def build_catalog(data_dir):
    blobs = {
        ("w", "config.json"): b'{"whisper":1}', ("w", "model.bin"): os.urandom(300_000),
        ("d", "w.th"): os.urandom(200_000), ("d", "w6.th"): os.urandom(200_000),
        ("k", "config.json"): b'{"kokoro":1}', ("k", "kokoro-v1_0.pth"): os.urandom(200_000),
        ("k", "voices/af_heart.pt"): os.urandom(500), ("k", "voices/bm_george.pt"): os.urandom(500),
    }
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as t:
        body = b"a portable Python would be here"
        info = tarfile.TarInfo("python/README.txt")
        info.size = len(body)
        t.addfile(info, io.BytesIO(body))
    pyarc = buf.getvalue()
    yaml, yaml6 = b"models: ['w']\n", b"models: ['w6']\n"
    files = {f"/test/{r}/resolve/{REV}/{p}": b for (r, p), b in blobs.items()}
    files.update({"/raw/tdemucs.yaml": yaml, "/raw/tdemucs_6s.yaml": yaml6, "/py.tar.gz": pyarc})
    srv = mirror(files)
    hub = f"http://127.0.0.1:{srv.server_address[1]}"
    sha = lambda b: hashlib.sha256(b).hexdigest()
    f = lambda r, p: {"path": p, "size": len(blobs[(r, p)]), "sha256": sha(blobs[(r, p)])}
    url = lambda p, b: {"path": p, "size": len(b), "sha256": sha(b), "url": f"{hub}/raw/{p}"}
    demucs_pkgs = [{"name": "Installing Demucs", "args": ["--no-deps", "demucs==4.0.1"]}]
    cat = {
        "hub": hub,
        "repos": {"w": {"repo": "test/w", "rev": REV}, "d": {"repo": "test/d", "rev": REV}, "k": {"repo": "test/k", "rev": REV}},
        "models": [
            {"id": "t-whisper", "name": "Test Whisper", "role": "transcript", "loader": "whisper", "repo": "w", "licence": "MIT", "tier": "light",
             "languages": ["en", "fr"], "sounds": False, "min_vram_gb": 0.0, "note": "A test transcriber.",
             "files": [f("w", "config.json"), f("w", "model.bin")], "install": [{"name": "Installing Whisper", "args": ["--no-deps", "faster-whisper==1.2.1"]}]},
            {"id": "t-demucs", "name": "Test Demucs", "role": "stems", "loader": "demucs", "repo": "d", "licence": "MIT", "tier": "balanced",
             "languages": [], "sounds": False, "min_vram_gb": 0.0, "note": "A test four-stem model.",
             "files": [url("tdemucs.yaml", yaml), f("d", "w.th")], "install": demucs_pkgs},
            {"id": "t-demucs-6s", "name": "Test Demucs 6", "role": "stems", "loader": "demucs", "repo": "d", "licence": "MIT", "tier": "extra",
             "languages": [], "sounds": False, "min_vram_gb": 0.0, "note": "A test six-stem model.",
             "files": [url("tdemucs_6s.yaml", yaml6), f("d", "w6.th")], "install": demucs_pkgs},
            {"id": "t-kokoro", "name": "Test Kokoro", "role": "tts", "loader": "kokoro", "repo": "k", "licence": "Apache-2.0", "tier": "light",
             "languages": ["en"], "sounds": False, "min_vram_gb": 0.0, "note": "A test speech model.",
             "files": [f("k", "config.json"), f("k", "kokoro-v1_0.pth"), f("k", "voices/af_heart.pt"), f("k", "voices/bm_george.pt")],
             "voices": [{"id": "af_heart", "name": "Heart", "language": "en-US"}, {"id": "bm_george", "name": "George", "language": "en-GB"}],
             "install": [{"name": "Installing Kokoro", "args": ["--no-deps", "kokoro==0.9.4"]}]},
        ],
        "runtime": {
            "id": "voice-engine", "name": "Voice engine", "version": "test-1", "note": "Python and PyTorch, installed once for every model.",
            "python": {platform_key(): {"url": hub + "/py.tar.gz", "size": len(pyarc), "sha256": sha(pyarc), "exe": "python/README.txt"}},
            "steps": [{"name": "Installing PyTorch", "args": ["torch==2.6.0"]}],
            "approx_mb": {"cpu": 1, "cuda": 1, "mps": 1},
            "python_override": sys.executable,
            "pythonpath": str(STUB),
        },
    }
    (data_dir / "Models").mkdir(parents=True, exist_ok=True)
    (data_dir / "Models" / "catalog.override.json").write_text(json.dumps(cat, indent=1))
    return srv


def main():
    assert EXE.exists(), f"build flux-mcp first: {EXE}"
    OUT.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="flux-e2e-smart-"))
    mir = build_catalog(tmp / "data")
    port = free_port()
    service = "Flux E2E " + uuid.uuid4().hex[:8]
    env = {k: v for k, v in os.environ.items() if k not in KEY_VARS and not (k.startswith("FLUX_") and k.endswith("_KEY"))}
    env.update(FLUX_DATA_DIR=str(tmp / "data"), FLUX_EXPORT_DIR=str(tmp / "export"), FLUX_NULL_AUDIO="1",
               FLUX_STUB_LOG=str(tmp / "stub.log"), FLUX_STUB_PIP_SLEEP="0", FLUX_KEYRING_SERVICE=service)
    srv = subprocess.Popen([str(EXE), "serve", "--port", str(port), "--ui", str(ROOT / "apps/desktop/ui"), "--no-token"], env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    passed = []

    def ok(name):
        passed.append(name)
        print("ok -", name)

    def role(r):
        return next(x for x in api(port, "ai.engines")["roles"] if x["role"] == r)

    try:
        for _ in range(100):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/api/health", timeout=1)
                break
            except Exception:
                time.sleep(0.1)
        api(port, "settings.set", {"values": {"onboarded": True, "headphones_checked": True}})
        api(port, "project.new", {"name": "Smart tools test"})
        api(port, "media.import", {"path": speech(tmp / "talk.wav"), "wait": True})
        with sync_playwright() as pw:
            b = pw.chromium.launch()
            page = b.new_page(viewport={"width": 1440, "height": 900})
            errors = []
            page.on("pageerror", lambda e: errors.append(str(e)))
            page.on("console", lambda m: errors.append(m.text) if m.type == "error" and "favicon" not in m.text and "404" not in m.text else None)
            page.on("dialog", lambda d: d.accept())
            requests = []
            page.on("request", lambda r: requests.append(r.url))
            page.goto(f"http://127.0.0.1:{port}/#prefs-models")
            page.wait_for_selector('[data-sact="dl-ask"][data-id="t-whisper"]')

            # 1. Every engine is listed by role with its tier; the built-in ones run with nothing downloaded.
            text = page.inner_text("#stage")
            for s in ["Stems", "Transcripts", "Text to speech", "Online services", "Built-in split", "Test Demucs", "Test Demucs 6",
                      "Test Whisper", "Test Kokoro", "Whisper Large v3, free online", "Whisper Large v3 Turbo on Groq", "Free, no key", "Needs a key",
                      "Specialist", "Online"]:
                assert s in text, s
            assert page.locator('[data-sact="use"][data-id="spectral"][aria-pressed="true"]').count() == 1, "the built-in split runs out of the box"
            assert role("stems")["choice"] == "spectral"
            assert not any("huggingface" in u or "replicate" in u or "groq" in u for u in requests)
            page.screenshot(path=str(OUT / "smart-tools-1440.png"), full_page=True)
            ok("engines listed by role and tier; built-in split in use; nothing fetched")

            # 2. Download asks first (size, licence, folder); Cancel fetches nothing; Download installs and it can be used.
            page.click('[data-sact="dl-ask"][data-id="t-whisper"]')
            page.wait_for_selector('[role="dialog"] [data-sact="dl-go"]:not([disabled])')
            page.wait_for_timeout(500)
            dlg = page.inner_text('[role="dialog"]')
            assert "Download Test Whisper" in dlg and "MIT licence" in dlg and "Models" in dlg, dlg
            page.screenshot(path=str(OUT / "smart-download-dialog-1440.png"))
            page.click('[role="dialog"] a[data-go="prefs-models"] >> text=Cancel')
            page.wait_for_selector('[data-sact="dl-ask"][data-id="t-whisper"]')
            assert not (tmp / "data" / "Models" / "files").exists(), "nothing downloaded before the yes"
            page.click('[data-sact="dl-ask"][data-id="t-whisper"]')
            page.click('[role="dialog"] [data-sact="dl-go"]:not([disabled])')
            page.wait_for_selector('[data-sact="use"][data-id="t-whisper"]', timeout=60000)
            assert "Downloaded and checked." in page.inner_text("#stage")
            ok("download dialog first; Cancel fetches nothing; download installs the model")

            # 3. Turning on a free online engine says what is sent and where, then uses it; nothing is sent yet.
            page.click('[data-sact="cloud-ask"][data-id="cloud-free-whisper"]')
            page.wait_for_selector('[role="dialog"] [data-sact="cloud-go"]')
            page.wait_for_timeout(500)
            dlg = page.inner_text('[role="dialog"]')
            assert "Turn on Whisper Large v3, free online" in dlg and "This runs online." in dlg and "Free" in dlg, dlg
            assert page.locator('[role="dialog"] [data-sin="key"]').count() == 0, "a free engine asks for no key"
            page.screenshot(path=str(OUT / "smart-cloud-dialog-1440.png"))
            page.click('[role="dialog"] [data-sact="cloud-go"]')
            page.wait_for_selector('[data-sact="use"][data-id="cloud-free-whisper"][aria-pressed="true"]')
            assert role("transcript")["choice"] == "cloud-free-whisper"
            page.click('[data-sact="use"][data-id="t-whisper"]')
            page.wait_for_selector('[data-sact="use"][data-id="t-whisper"][aria-pressed="true"]')
            assert role("transcript")["choice"] == "t-whisper"
            ok("free online engine: dialog says what is sent, no key; Use switches back to the local one")

            # 4. An engine that needs a key: the key field, a refusal without one, then the keychain, then Forget.
            page.click('[data-sact="cloud-ask"][data-id="cloud-groq-whisper"]')
            page.wait_for_selector('[role="dialog"] [data-sin="key"]')
            assert page.get_attribute('[role="dialog"] [data-sin="key"]', "type") == "password"
            page.click('[role="dialog"] [data-sact="cloud-go"]')
            page.wait_for_selector('text=Paste the key first')
            page.fill('[role="dialog"] [data-sin="key"]', "e2e-not-a-real-key")
            page.screenshot(path=str(OUT / "smart-key-dialog-1440.png"))
            page.press('[role="dialog"] [data-sin="key"]', "Enter")
            try:
                page.wait_for_selector('[data-sact="use"][data-id="cloud-groq-whisper"][aria-pressed="true"]')
            except Exception:
                print(page.inner_text("body")[:1500])
                raise
            assert "Key in the system keychain" in page.inner_text("#stage")
            assert "e2e-not-a-real-key" not in (tmp / "data" / "settings.json").read_text() if (tmp / "data" / "settings.json").exists() else True
            for p in (tmp / "data").rglob("*.json"):
                assert "e2e-not-a-real-key" not in p.read_text(errors="ignore"), p
            page.click('[data-sact="key-del"][data-p="groq"]')
            page.wait_for_selector('[data-sact="key-ask"][data-p="groq"]')
            assert next(p for p in api(port, "ai.engines")["providers"] if p["id"] == "groq")["key"] == "missing"
            page.click('[data-sact="use"][data-id="t-whisper"]')
            page.wait_for_selector('[data-sact="use"][data-id="t-whisper"][aria-pressed="true"]')
            ok("keyed engine: password field, refuses an empty key, keeps it out of Flux's files, Forget removes it")

            # 5. The transcript editor: make a transcript with the chosen engine, words with fillers marked, select and cut one.
            page.goto(f"http://127.0.0.1:{port}/#script")
            page.wait_for_selector('[data-sact="tr-make"]:not([disabled])')
            page.screenshot(path=str(OUT / "transcript-empty-1440.png"))
            page.click('[data-sact="tr-make"]')
            page.wait_for_selector('.wd[data-sact="tr-word"]', timeout=60000)
            words = page.locator('.wd').all_inner_texts()
            assert words == ["So", "um", "I", "I", "think", "uh", "it", "works."], words
            assert page.locator('.wd.fl').count() >= 3, "um, uh and the repeated I are marked"
            assert "Test Whisper" in page.inner_text("aside")
            page.screenshot(path=str(OUT / "transcript-1440.png"))
            before = api(port, "project.get")
            page.locator('.wd', has_text="think").focus()
            page.keyboard.press("Enter")
            page.wait_for_selector('[data-sact="tr-del"]')
            assert '"think"' in page.inner_text("aside")
            page.click('[data-sact="tr-del"]')
            page.wait_for_selector('text=Undo brings it back')
            after = api(port, "project.get")
            assert json.dumps(after) != json.dumps(before), "the cut changed the project"
            assert page.locator('text=The project changed since this transcript').count() == 1
            page.click('.wd >> text=works.')
            page.wait_for_selector('[data-sact="tr-del"][disabled]')  # positions are out of date until the transcript is updated
            ok("transcript made with the chosen engine; words and fillers shown; keyboard select; Delete cuts the word")

            # 6. Filler review: lists the fillers, filters by kind, and removing them cuts the audio.
            page.click('[data-sact="tr-make"]')
            page.wait_for_selector('text=The project changed since this transcript', state="detached", timeout=60000)
            page.goto(f"http://127.0.0.1:{port}/#fillers")
            page.wait_for_selector('[data-sact="fl-remove"]')
            n = page.locator('[data-sact="fl-toggle"]').count()
            assert n >= 2, n
            page.click('[data-sact="fl-kind"][data-k="filler"]')
            page.wait_for_timeout(200)
            assert page.locator('[data-sact="fl-toggle"]').count() <= n
            page.click('[data-sact="fl-kind"][data-k=""]')
            page.screenshot(path=str(OUT / "fillers-1440.png"))
            p0 = api(port, "project.get")
            page.click('[data-sact="fl-remove"]')
            page.wait_for_selector('text=Undo brings them back', timeout=20000)
            assert json.dumps(api(port, "project.get")) != json.dumps(p0)
            assert page.is_disabled('[data-sact="fl-remove"]'), "no second removal from an out-of-date list"
            ok("filler review lists, filters and removes")

            # 7. Stems: the setup page offers tiles from the engines that are installed, and the Specialist six-stem
            #    option only once it is downloaded. Speech: the engine picker lists the usable engines.
            page.goto(f"http://127.0.0.1:{port}/#stems-setup")
            page.wait_for_selector('[data-act="stem-tile"]')
            assert page.locator('[data-act="stem-tile"][data-id="spectral"]').count() == 1
            page.screenshot(path=str(OUT / "stems-setup-1440.png"))
            page.goto(f"http://127.0.0.1:{port}/#tts")
            page.wait_for_selector('[data-act-change="tts-engine"]')
            opts = page.locator('[data-act-change="tts-engine"] option').all_inner_texts()
            assert any("voices" in o for o in opts), opts
            page.screenshot(path=str(OUT / "tts-engine-1440.png"))
            ok("stems tiles and the speech engine picker follow the installed engines")

            # A computer with no speech voices: the picker still names the engine, and the page says what's missing.
            def no_voices(route):
                m = json.loads(route.request.post_data or "{}").get("method")
                if m not in ("ai.engines", "tts.voices"):
                    route.continue_()
                    return
                r = route.fetch()
                d = r.json()
                if m == "tts.voices":
                    d["result"]["voices"] = []
                else:
                    tts = next(x for x in d["result"]["roles"] if x["role"] == "tts")
                    tts["engines"] = [dict(e, installed=False) for e in tts["engines"]]
                route.fulfill(response=r, json=d)

            page.route("**/api/call", no_voices)
            page.goto("about:blank")
            page.goto(f"http://127.0.0.1:{port}/#tts")
            page.wait_for_selector('text=No speech voices were found')
            none = page.locator('[data-act-change="tts-engine"] option').all_inner_texts()
            assert none == ["This computer's voices"], none
            page.unroute("**/api/call", no_voices)
            page.goto("about:blank")
            page.goto(f"http://127.0.0.1:{port}/#tts")
            page.wait_for_selector('[data-act-change="tts-engine"]')
            ok("with no speech voices, the picker names the engine and the page says what's missing")

            # 8. Every width: no sideways scroll; every control has a name and a tooltip.
            for w, h in [(1440, 900), (820, 1100), (390, 844)]:
                page.set_viewport_size({"width": w, "height": h})
                for screen in ["prefs-models", "script", "fillers"]:
                    page.goto(f"http://127.0.0.1:{port}/#{screen}")
                    page.wait_for_timeout(700)
                    over = page.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth")
                    assert over <= 1, (screen, w, over)
                    if screen == "prefs-models":
                        assert "Undo brings them back" not in page.inner_text("#stage"), "a transcript message leaked onto Smart tools"
                    bare = page.evaluate("""[...document.querySelectorAll('#stage button:not([disabled]), #stage select, #stage input, #stage [role=button]')]
                        .filter(e => e.offsetParent && !e.closest('[data-tip]') && !e.getAttribute('aria-label') && !(e.textContent||'').trim()).length""")
                    assert bare == 0, (screen, w, bare)
                    if w != 1440:
                        page.screenshot(path=str(OUT / f"{'smart-tools' if screen == 'prefs-models' else screen}-{w}.png"), full_page=True)
            ok("no sideways scroll at 1440, 820 and 390; every control named")

            # 9. At phone width, every theme and font still fits.
            themes = page.evaluate("Flux.THEMES.map(t => t[0])")
            fonts = page.evaluate("Flux.FONTS.map(f => f[0])")
            fit = "(() => { const m = document.querySelector('#m'); return Math.max(document.documentElement.scrollWidth - innerWidth, m ? m.scrollWidth - m.clientWidth : 0); })()"
            for screen in ["prefs-models", "script"]:
                page.goto(f"http://127.0.0.1:{port}/#{screen}")
                page.wait_for_timeout(600)
                for i, th in enumerate(themes):
                    page.evaluate(f"document.documentElement.dataset.flux = '{th}'; document.documentElement.dataset.font = '{fonts[i % len(fonts)]}'")
                    page.wait_for_timeout(120)
                    assert page.evaluate(fit) <= 1, (screen, th)
                page.screenshot(path=str(OUT / f"{'smart-tools' if screen == 'prefs-models' else screen}-390-{themes[-1]}.png"))
                page.evaluate("document.documentElement.dataset.flux = 'cassette'; document.documentElement.dataset.font = 'bricolage'")
            ok(f"phone width fits in all {len(themes)} themes and {len(fonts)} fonts")

            assert not errors, errors
            ok("no page errors")
            b.close()
    finally:
        srv.terminate()
        mir.shutdown()
        if os.name == "nt":
            # Nothing may be left behind in the keychain.
            out = subprocess.run(["cmdkey", "/list"], capture_output=True, text=True).stdout
            assert service not in out, "a test key was left in the keychain"
    print(f"{len(passed)} checks passed")


if __name__ == "__main__":
    main()
