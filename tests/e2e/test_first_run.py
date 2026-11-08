"""End-to-end test of first run in a real browser: it shows once, the sound checks play with no project open, and
Settings can run the sound-profile steps again and come back.

    python tests/e2e/test_first_run.py     (needs `cargo build -p flux-mcp --release` first)
"""
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target" / "release" / ("flux-mcp.exe" if os.name == "nt" else "flux-mcp")
OUT = ROOT / "target" / "e2e"


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


def main():
    assert EXE.exists(), f"build flux-mcp first: {EXE}"
    OUT.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="flux-e2e-first-"))
    port = free_port()
    env = dict(os.environ, FLUX_DATA_DIR=str(tmp / "data"), FLUX_EXPORT_DIR=str(tmp / "export"), FLUX_NULL_AUDIO="1")
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
        assert not api(port, "settings.get")["onboarded"]
        with sync_playwright() as pw:
            b = pw.chromium.launch()
            page = b.new_page(viewport={"width": 1440, "height": 900})
            errors = []
            page.on("pageerror", lambda e: errors.append(str(e)))
            page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
            base = f"http://127.0.0.1:{port}/"

            # Build machines have no sound output and desktops do, and first run differs. The test sets which one it is.
            sound = {"state": "ok"}

            def caps(route):
                if '"app.capabilities"' not in (route.request.post_data or ""):
                    route.continue_()
                    return
                r = route.fetch()
                d = r.json()
                next(x for x in d["result"] if x["id"] == "engine")["state"] = sound["state"]
                route.fulfill(response=r, json=d)

            page.route("**/api/call", caps)

            page.goto(base)
            page.wait_for_selector('text=Checking what this computer can do.')
            for _ in range(30):
                if api(port, "settings.get")["onboarded"]:
                    break
                time.sleep(0.1)
            assert api(port, "settings.get")["onboarded"], "first run is marked seen when it opens"
            ok("first run opens on a fresh install and is marked seen at once")

            # Close Flux halfway (on the welcome step) and start again: first run doesn't come back.
            page.click('text=Continue')
            page.wait_for_selector('text=Hello.')
            page.goto("about:blank")
            page.goto(base)
            page.wait_for_selector('text=What are we doing to it?')
            assert page.locator('text=Checking what this computer can do.').count() == 0
            ok("starting again goes home, not back into first run")

            # A computer with no sound output is told so, and can carry on.
            sound["state"] = "warn"
            page.goto("about:blank")
            page.goto(base + "#boot")
            page.wait_for_selector('text=Checking what this computer can do.')
            page.click('a.btn:has-text("Continue"), button:has-text("Continue")')
            page.wait_for_selector("text=Flux can't find a sound output.")
            assert page.locator('text=Hello.').count() == 0
            page.click('text=Continue without sound')
            page.wait_for_selector('text=Hello.')
            sound["state"] = "ok"
            page.goto("about:blank")
            page.goto(base)
            page.wait_for_selector('text=What are we doing to it?')
            ok("with no sound output, first run says so and carries on without sound")

            # First-run sound steps, no project open: the tones play, no error dialog.
            page.goto(base + "#hpcheck")
            page.wait_for_selector('[data-act="tone"][data-kind="left"]')
            page.click('[data-act="tone"][data-kind="left"]')
            page.click('[data-act="tone"][data-kind="move"]')
            page.wait_for_timeout(600)
            assert page.locator("text=That didn't work").count() == 0
            page.click('[data-go="hrtf"]')
            page.wait_for_selector('[data-act="hrtf"][data-v="A"]')
            page.click('[data-act="hrtf"][data-v="A"]')
            page.wait_for_timeout(600)
            assert page.locator("text=That didn't work").count() == 0
            assert api(port, "settings.get")["hrtf_profile"] == "A"
            ok("headphone tones and profile tones play with no project open")

            # From Settings: the same steps, 2 of them, and both ways out lead back to Settings.
            page.goto(base + "#prefs-audio")
            page.wait_for_selector('text=Sound profile')
            assert page.locator('text=Profile A, head width 19.5 cm').count() == 1
            page.wait_for_timeout(900)
            page.screenshot(path=str(OUT / "prefs-sound-profile-1440.png"), full_page=True)
            page.click('a:has-text("Set up again"), button:has-text("Set up again")')
            page.wait_for_selector('h1:has-text("headphones the right way round")')
            assert page.get_attribute('.dots', 'aria-label') == "Step 1 of 2"
            page.click('a.btn:has-text("Back")')
            page.wait_for_selector('text=Sound profile')
            assert page.evaluate("location.hash") == "#prefs-audio"
            ok("Back from the headphone check returns to Settings")

            page.click('a:has-text("Set up again"), button:has-text("Set up again")')
            page.wait_for_selector('[data-go="hrtf"]')
            page.click('[data-go="hrtf"]')
            page.wait_for_selector('[data-act="hrtf"][data-v="E"]')
            assert page.get_attribute('.dots', 'aria-label') == "Step 2 of 2"
            page.click('[data-act="hrtf"][data-v="E"]')
            page.wait_for_selector('text=Use Profile E')
            page.click('text=Use Profile E')
            page.wait_for_selector('text=Profile E, head width 19.5 cm')
            assert page.evaluate("location.hash") == "#prefs-audio"
            assert api(port, "settings.get")["hrtf_profile"] == "E"
            ok("picking a new profile from Settings saves it and returns to Settings")

            for w in (820, 390):
                page.set_viewport_size({"width": w, "height": 900})
                page.wait_for_timeout(900)
                over = page.evaluate("document.documentElement.scrollWidth - innerWidth")
                assert over <= 0, (w, over)
                page.screenshot(path=str(OUT / f"prefs-sound-profile-{w}.png"), full_page=True)
            ok("Settings fits at 820 and 390 px")

            assert not errors, errors[:3]
            ok("no page errors")
            b.close()
    finally:
        srv.terminate()
        try:
            srv.wait(5)
        except Exception:
            srv.kill()
        shutil.rmtree(tmp, ignore_errors=True)
    print(f"\n{len(passed)} passed")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print("FAILED:", repr(e))
        sys.exit(1)
