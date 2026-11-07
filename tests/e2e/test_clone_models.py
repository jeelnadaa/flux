"""End-to-end test of voice model downloads and choices on the Voice cloning page, in a real browser.

Runs flux-mcp's HTTP server with a throwaway data folder whose catalog points at a local mirror (a small HTTP server
in this process) and at the stub Python runtime in crates/flux-core/tests/voice_stub, so nothing is fetched from
the internet and no real model runs. Then drives the page: the Download dialog shows the size, licence and folder
before anything is fetched; downloading shows progress and installs the voice engine; choosing models, device and
settings; speaking a script onto the project; deleting. Checks names, tooltips and widths, and saves screenshots.

    python tests/e2e/test_clone_models.py     (needs `cargo build -p flux-mcp --release` first)
"""
import hashlib
import http.server
import io
import json
import os
import shutil
import socket
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
import urllib.request
import wave
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target" / "release" / ("flux-mcp.exe" if os.name == "nt" else "flux-mcp")
OUT = ROOT / "target" / "e2e"
STUB = ROOT / "crates" / "flux-core" / "tests" / "voice_stub"
REV = "0123456789abcdef0123456789abcdef01234567"


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


def api(port, method, params=None):
    body = json.dumps({"method": method, "params": params or {}}).encode()
    req = urllib.request.Request(f"http://127.0.0.1:{port}/api/call", body, {"Content-Type": "application/json"})
    r = json.loads(urllib.request.urlopen(req, timeout=30).read())
    if not r.get("ok"):
        raise RuntimeError(r.get("error"))
    return r["result"]


def tone(path, secs, hz=200.0):
    import math
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(24000)
        w.writeframes(b"".join(int(9000 * math.sin(2 * math.pi * hz * i / 24000)).to_bytes(2, "little", signed=True) for i in range(int(24000 * secs))))
    return str(path)


def platform_key():
    import platform as pf
    osn = {"win32": "windows", "darwin": "macos"}.get(sys.platform, "linux")
    arch = {"amd64": "x86_64", "x86_64": "x86_64", "arm64": "aarch64", "aarch64": "aarch64"}.get(pf.machine().lower(), pf.machine().lower())
    return f"{osn}-{arch}"


def mirror(files):
    """A local HTTP server for the catalog's files, slowed a little so progress can be seen."""
    class H(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            data = files.get(self.path)
            if data is None:
                self.send_response(404)
                self.end_headers()
                return
            self.send_response(200)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            for i in range(0, len(data), 64 * 1024):
                self.wfile.write(data[i:i + 64 * 1024])
                time.sleep(0.03)

        def log_message(self, *a):
            pass

    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), H)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv


def build_catalog(data_dir):
    shared = os.urandom(1_500_000)
    turbo = b'{"kind":"turbo"}'
    english = b'{"kind":"english"}'
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as t:
        info = tarfile.TarInfo("python/README.txt")
        body = b"a portable Python would be here"
        info.size = len(body)
        t.addfile(info, io.BytesIO(body))
    pyarc = buf.getvalue()
    base = f"/test/voice/resolve/{REV}/"
    files = {base + "a.bin": shared, base + "turbo.json": turbo, base + "english.json": english, "/py.tar.gz": pyarc}
    srv = mirror(files)
    hub = f"http://127.0.0.1:{srv.server_address[1]}"
    f = lambda p, b: {"path": p, "size": len(b), "sha256": hashlib.sha256(b).hexdigest()}
    cat = {
        "hub": hub,
        "repos": {"r": {"repo": "test/voice", "rev": REV}},
        "models": [
            {"id": "t-turbo", "name": "Test Turbo", "role": "speech", "loader": "turbo", "repo": "r", "licence": "MIT", "languages": ["en"], "sounds": True, "min_vram_gb": 4.0,
             "note": "A test speech model that performs sounds.", "files": [f("a.bin", shared), f("turbo.json", turbo)],
             "params": [{"key": "temperature", "label": "Variety", "min": 0.05, "max": 2.0, "step": 0.05, "default": 0.8, "tip": "Higher is livelier and less predictable."}]},
            {"id": "t-english", "name": "Test English", "role": "speech", "loader": "english", "repo": "r", "licence": "MIT", "languages": ["en"], "sounds": False, "min_vram_gb": 6.0,
             "note": "A test speech model without sounds.", "files": [f("a.bin", shared), f("english.json", english)],
             "params": [{"key": "exaggeration", "label": "Emotion", "min": 0.25, "max": 2.0, "step": 0.05, "default": 0.5, "tip": "How expressive the delivery is."}]},
            {"id": "t-vc", "name": "Test Converter", "role": "convert", "loader": "vc", "repo": "r", "licence": "MIT", "languages": [], "sounds": False, "min_vram_gb": 6.0,
             "note": "A test voice converter.", "files": [f("a.bin", shared)]},
        ],
        "runtime": {
            "id": "voice-engine", "name": "Voice engine", "version": "test-1", "note": "Python and PyTorch, installed once for every model.",
            "python": {platform_key(): {"url": hub + "/py.tar.gz", "size": len(pyarc), "sha256": hashlib.sha256(pyarc).hexdigest(), "exe": "python/README.txt"}},
            "steps": [{"name": "Installing PyTorch", "args": ["torch==2.6.0"], "index": {"cpu": "https://example.invalid/cpu", "cuda": "https://example.invalid/cu124", "mps": "https://example.invalid/cpu"}},
                      {"name": "Installing Chatterbox", "args": ["chatterbox-tts==0.1.7"]}],
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
    tmp = Path(tempfile.mkdtemp(prefix="flux-e2e-clone-"))
    mir = build_catalog(tmp / "data")
    port = free_port()
    env = dict(os.environ, FLUX_DATA_DIR=str(tmp / "data"), FLUX_EXPORT_DIR=str(tmp / "export"), FLUX_NULL_AUDIO="1", FLUX_STUB_LOG=str(tmp / "stub.log"))
    srv = subprocess.Popen([str(EXE), "serve", "--port", str(port), "--ui", str(ROOT / "apps/desktop/ui"), "--no-token"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    passed = []

    def ok(name):
        passed.append(name)
        print("ok -", name)

    try:
        for _ in range(100):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/api/health", timeout=1)
                break
            except Exception:
                time.sleep(0.1)
        api(port, "settings.set", {"values": {"onboarded": True, "headphones_checked": True}})
        api(port, "project.new", {"name": "Cloning test"})
        api(port, "clone.profile.create", {"name": "Robin", "owner": "me", "consent": True, "consent_audio": tone(tmp / "c.wav", 4), "reference": tone(tmp / "r.wav", 8)})
        with sync_playwright() as pw:
            b = pw.chromium.launch()
            page = b.new_page(viewport={"width": 1440, "height": 900})
            errors = []
            page.on("pageerror", lambda e: errors.append(str(e)))
            page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
            page.on("dialog", lambda d: d.accept())
            page.goto(f"http://127.0.0.1:{port}/#clone")
            page.wait_for_selector('[data-cact="dl-ask"][data-id="t-turbo"]')
            ev = page.evaluate

            # 1. Every model is listed with its size and licence; nothing has been fetched; speaking is off.
            text = page.inner_text("#stage")
            for name in ["Test Turbo", "Test English", "Test Converter", "Voice engine"]:
                assert name in text, name
            assert "MIT · 2 MB" in text, text[:800]
            assert page.is_disabled('[data-cact="speak"]')
            page.screenshot(path=str(OUT / "clone-models-1440.png"))
            ok("models listed with licence and size; speak is off until one is downloaded")

            # 2. The Download dialog says what, how big, which licence and where, before anything is fetched.
            page.click('[data-cact="dl-ask"][data-id="t-turbo"]')
            page.wait_for_selector('[role="dialog"] [data-cact="dl-go"]')
            page.wait_for_timeout(600)
            dlg = page.inner_text('[role="dialog"]')
            assert "Download Test Turbo" in dlg and "MIT licence" in dlg and "Models" in dlg and "Voice engine" in dlg, dlg
            assert not (tmp / "data" / "Models" / "files").exists(), "nothing downloaded before the yes"
            page.screenshot(path=str(OUT / "clone-download-dialog-1440.png"))
            page.click('[role="dialog"] a[data-go="clone"] >> text=Cancel')
            page.wait_for_selector('[data-cact="dl-ask"][data-id="t-turbo"]')
            assert not (tmp / "data" / "Models" / "files").exists()
            ok("dialog shows size, licence and folder; Cancel fetches nothing")

            # 3. Download: progress with a Cancel button, then installed, checked, and chosen for speech.
            page.click('[data-cact="dl-ask"][data-id="t-turbo"]')
            page.wait_for_selector('[role="dialog"] [data-cact="dl-go"]')
            page.click('[role="dialog"] [data-cact="dl-go"]')
            page.wait_for_selector('[data-cact="dl-cancel"][data-id="t-turbo"]', timeout=10000)
            assert page.locator('[role="progressbar"]').count() >= 1
            page.screenshot(path=str(OUT / "clone-downloading-1440.png"))
            page.wait_for_selector('[data-cact="use-model"][data-id="t-turbo"]', timeout=60000)
            m = api(port, "clone.models")
            assert m["runtime"]["installed"] and next(x for x in m["models"] if x["id"] == "t-turbo")["installed"]
            assert next(x for x in m["models"] if x["id"] == "t-vc")["installed"], "the converter shares the file"
            ok("downloaded with progress; engine installed; shared files count for the converter")

            # 4. Choose: the model, the device, a setting (slider), the seed.
            api(port, "clone.settings.set", {"speech": "t-turbo"})
            page.reload()
            try:
                page.wait_for_selector('[data-bind="cl:t-turbo:temperature"]', timeout=8000)
            except Exception:
                print(page.inner_text("#stage")[:3000])
                raise
            page.click('.seg[data-cseg="device"] button:has-text("Processor")')
            page.wait_for_timeout(500)
            assert api(port, "clone.settings.get")["settings"]["device"] == "cpu"
            track = page.locator('[data-bind="cl:t-turbo:temperature"] [role=slider]')
            track.focus()
            for _ in range(5):
                page.keyboard.press("ArrowLeft")
            page.wait_for_timeout(900)
            t = api(port, "clone.settings.get")["params"]["t-turbo"]["temperature"]
            assert t < 0.8, t
            page.fill('[data-cin="seed"]', "7")
            page.wait_for_timeout(900)
            assert api(port, "clone.settings.get")["settings"]["seed"] == 7
            ok(f"device, Variety slider ({t:.2f}) and seed saved from the page")

            # 5. Speak onto the project, with progress, labelled synthetic.
            page.click('[data-cact="use"][data-id="robin"]')
            page.fill('[data-cin="script"]', "[hmm] So [laugh] [pause] yes")
            page.click('[data-cact="speak"]')
            page.wait_for_selector('.callout:has-text("Labelled synthetic")', timeout=30000)
            msg = page.inner_text('.callout:has-text("Labelled synthetic")')
            assert "Generated speech" in msg, msg
            p = api(port, "project.get")
            assert any(t["name"] == "Generated speech" for t in p["tracks"])
            page.screenshot(path=str(OUT / "clone-spoken-1440.png"))
            ok(f"spoke onto the project: {msg[:60]}")

            # 6. Every control named and with a tooltip; fits at 1440, 820 and 390 px.
            missing = ev("""(() => {
              const root = document.querySelector('#stage');
              const ctl = [...root.querySelectorAll('button, select, input, textarea, [role=slider]')];
              const name = e => (e.getAttribute('aria-label') || e.getAttribute('aria-labelledby') && document.getElementById(e.getAttribute('aria-labelledby'))?.textContent || e.textContent || (e.labels && e.labels[0] && e.labels[0].textContent) || '').trim();
              return { n: ctl.length, noName: ctl.filter(e => !name(e)).map(e => e.outerHTML.slice(0, 90)), noTip: ctl.filter(e => !e.closest('[data-tip]')).map(e => e.outerHTML.slice(0, 90)) };
            })()""")
            assert not missing["noName"], missing["noName"][:5]
            assert not missing["noTip"], missing["noTip"][:5]
            ok(f"{missing['n']} controls: all named, all with tooltips")
            for w, h in [(1440, 900), (820, 1180), (390, 844)]:
                page.set_viewport_size({"width": w, "height": h})
                page.mouse.move(0, 0)
                ev("document.activeElement && document.activeElement.blur(); document.getElementById('tipbox') && document.getElementById('tipbox').classList.remove('on')")
                page.wait_for_timeout(400)
                over = ev("(() => { const m = document.querySelector('#m') || document.scrollingElement; return Math.max(document.documentElement.scrollWidth - innerWidth, m.scrollWidth - m.clientWidth); })()")
                assert over <= 1, (w, over)
                page.screenshot(path=str(OUT / f"clone-models-{w}.png"), full_page=True)
            page.set_viewport_size({"width": 1440, "height": 900})
            ok("no sideways scrolling at 1440, 820 and 390 px")

            # 7. Delete keeps what another downloaded model shares.
            page.click('[data-cact="dl-del"][data-id="t-turbo"]')
            page.wait_for_selector('[data-cact="dl-ask"][data-id="t-turbo"]', timeout=10000)
            m = api(port, "clone.models")
            assert not next(x for x in m["models"] if x["id"] == "t-turbo")["installed"]
            assert next(x for x in m["models"] if x["id"] == "t-vc")["installed"]
            ok("deleted the model; the converter's shared file stayed")

            assert not errors, errors[:3]
            ok("no page errors")
            b.close()
    finally:
        srv.terminate()
        try:
            srv.wait(5)
        except Exception:
            srv.kill()
        mir.shutdown()
        shutil.rmtree(tmp, ignore_errors=True)
    print(f"\n{len(passed)} passed")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print("FAILED:", repr(e))
        sys.exit(1)
