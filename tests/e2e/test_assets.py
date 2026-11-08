"""End-to-end test of generating without a project, listening first, and each project's assets.

Runs flux-mcp's HTTP server with a throwaway data folder and the stub voice engine (no model runs, nothing is fetched
from the internet) in a real browser.

    python tests/e2e/test_assets.py     (needs `cargo build -p flux-mcp --release` first)
"""
import base64
import io
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request
import wave
from pathlib import Path

from playwright.sync_api import sync_playwright

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_clone_models import free_port, api  # noqa: E402
from test_smart_tools import build_catalog, speech  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target" / "release" / ("flux-mcp.exe" if os.name == "nt" else "flux-mcp")
OUT = ROOT / "target" / "e2e"


def ok(msg):
    print(f"ok - {msg}", flush=True)


def try_api(port, method, params=None):
    req = urllib.request.Request(f"http://127.0.0.1:{port}/api/call", json.dumps({"method": method, "params": params or {}}).encode(),
                                 {"Content-Type": "application/json"})
    try:
        r = json.load(urllib.request.urlopen(req))
    except urllib.error.HTTPError as e:
        r = json.load(e)
    return r


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="flux-assets-"))
    mir = build_catalog(tmp / "data")
    port = free_port()
    env = dict(os.environ, FLUX_DATA_DIR=str(tmp / "data"), FLUX_EXPORT_DIR=str(tmp / "export"), FLUX_NULL_AUDIO="1", FLUX_STUB_LOG=str(tmp / "stub.log"))
    srv = subprocess.Popen([str(EXE), "serve", "--port", str(port), "--ui", str(ROOT / "apps/desktop/ui"), "--no-token"], env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(100):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/api/health", timeout=1)
                break
            except Exception:
                time.sleep(.1)
        api(port, "settings.set", {"values": {"onboarded": True, "headphones_checked": True}})
        job = api(port, "ai.model.download", {"model": "t-kokoro", "confirm": True}).get("job")
        while api(port, "job.get", {"id": job, "wait_s": 1})["state"] == "running":
            pass
        assert api(port, "job.get", {"id": job})["state"] == "done"
        api(port, "ai.engine.set", {"role": "tts", "engine": "t-kokoro"})
        run(port, tmp)
        print("ALL OK")
    finally:
        srv.terminate()
        mir.shutdown()


def run(port, tmp):
    base = f"http://127.0.0.1:{port}/"
    take_card = '[data-take-card]'

    def takes():
        return api(port, "take.list")["takes"]

    def open_project():
        return try_api(port, "project.get").get("result")

    def stub_lines():
        return [json.loads(x) for x in (tmp / "stub.log").read_text(encoding="utf-8").splitlines() if '"tts"' in x]

    with sync_playwright() as pw:
        b = pw.chromium.launch(args=["--autoplay-policy=no-user-gesture-required"])
        page = b.new_page(viewport={"width": 1440, "height": 900})
        errors = []
        page.on("pageerror", lambda e: errors.append(e.stack or str(e)))
        page.on("console", lambda m: errors.append(m.text) if m.type == "error" and "Content Security" in m.text else None)

        def generate(text):
            page.goto("about:blank")
            page.goto(base + "#tts")
            page.wait_for_selector('[data-ui="ttsText"]')
            page.fill('[data-ui="ttsText"]', text)
            page.wait_for_selector('[data-act="tts-speak"]:not([disabled])')
            n = len(takes())
            page.click('[data-act="tts-speak"]')
            for _ in range(200):
                if len(takes()) > n:
                    break
                page.wait_for_timeout(50)
            page.wait_for_selector(take_card, timeout=8000)
            return takes()[0]

        def save_to(value, name=None):
            page.select_option(f'{take_card} [data-aact-change="to"]', value)
            if name:
                page.fill(f'{take_card} [data-aname]', name)
            page.click(f'{take_card} [data-aact="save"]')
            page.wait_for_selector('.callout:has-text("In the assets of")', timeout=6000)

        # ---------- no project needed ----------
        assert open_project() is None
        t = generate("Hello there [laugh] and then [pause 2s] goodbye.")
        assert open_project() is None, "generating opened or made a project"
        assert api(port, "library.list")["projects"] == [], "generating made a project"
        assert t["synthetic"] and t["saved"] == []
        ok("speech is generated with no project open, and no project appears")

        # ---------- [tags] on the Text to speech page ----------
        seg = stub_lines()[-1]["segments"]
        said = " ".join(s["value"] for s in seg if s["type"] == "text")
        assert "[" not in said and "laugh" not in said, seg
        assert "Hello there" in said and "goodbye" in said, seg
        assert [s["value"] for s in seg if s["type"] == "silence"] == [2000], seg
        assert t["sounds_left_out"] == 1, t
        assert t["seconds"] > 2.0, t
        assert "1 sound left out" in page.inner_text(take_card)
        ok("a [pause] becomes silence, and a sound this voice can't do is left out, not read aloud")

        page.click('[data-aact="tag"][data-t="pause"]')
        assert "[pause]" in page.input_value('[data-ui="ttsText"]')
        page.wait_for_selector("text=doesn't perform sounds")
        ok("the page offers the tags, and says what this engine does with them")

        n = len(takes())
        page.fill('[data-ui="ttsText"]', "Say [xyzzy] now")
        page.wait_for_selector('[data-act="tts-speak"]:not([disabled])')
        page.click('[data-act="tts-speak"]')
        page.wait_for_selector("text=xyzzy", timeout=4000)
        page.wait_for_timeout(300)
        assert len(takes()) == n
        page.goto("about:blank")
        ok("an unknown [tag] is refused with a reason, and nothing is generated")

        # ---------- listening ----------
        t = generate("Listen before you keep this.")
        r = api(port, "audio.preview", {"take": t["id"]})
        data = b""
        while True:
            c = api(port, "audio.preview_read", {"token": r["token"], "at": len(data)})
            data += base64.b64decode(c["data"])
            if not c["more"]:
                break
        assert len(data) == r["bytes"]
        with wave.open(io.BytesIO(data)) as w:
            pcm = w.readframes(w.getnframes())
            assert abs(w.getnframes() / w.getframerate() - t["seconds"]) < .01
        assert max(abs(int.from_bytes(pcm[i:i + 2], "little", signed=True)) for i in range(0, len(pcm), 2)) > 4000, "the preview is silent"
        ok(f"the preview is the real audio: {len(data)} bytes, {t['seconds']:.2f} s, not silent")

        page.click(f'{take_card} [data-aact="play"]')
        page.wait_for_selector(f'{take_card} [data-aact="play"]:has-text("Pause")', timeout=6000)
        page.wait_for_timeout(700)
        v = page.locator(f'{take_card} [data-aseek]').evaluate("e => parseFloat(e.style.getPropertyValue('--v'))")
        assert v > 0, v
        assert "0:0" in page.inner_text(f'{take_card} [data-pl-t]')
        page.locator(f'{take_card} [data-aseek]').focus()
        page.keyboard.press("ArrowRight")
        page.click(f'{take_card} [data-aact="play"]')
        page.wait_for_selector(f'{take_card} [data-aact="play"]:has-text("Listen")', timeout=3000)
        page.screenshot(path=str(OUT / "take-card-1440.png"))
        ok("Listen plays it in the page, the bar moves, the keyboard seeks, and it pauses")

        # ---------- save to a project, from outside any project ----------
        hostile = try_api(port, "take.save", {"id": t["id"], "project": "../evil"})
        assert not hostile["ok"], hostile
        assert not try_api(port, "audio.preview_read", {"token": "../../x"})["ok"]
        assert not try_api(port, "take.delete", {"id": "../x"})["ok"]
        ok("ids from outside are refused")

        save_to("__new", "From the shelf")
        lib = api(port, "library.list")["projects"]
        assert [p["name"] for p in lib] == ["From the shelf"], lib
        assert open_project() is None, "saving opened the project"
        got = api(port, "asset.list", {"project": lib[0]["id"]})
        assert [a["uses"] for a in got["assets"]] == [0] and got["assets"][0]["synthetic"]
        ok("saved into a new project without opening it, and it isn't on any timeline")

        # A project that's open, and another that stays closed.
        other = api(port, "project.new", {"name": "Other"})["id"]
        here = api(port, "project.new", {"name": "Here"})["id"]
        h0 = api(port, "project.get")["hash"]
        t = generate("This goes in the closed one.")
        save_to(other)
        assert api(port, "project.get")["id"] == here
        assert api(port, "project.get")["hash"] == h0, "saving to another project changed this one"
        ids = [a["id"] for a in api(port, "asset.list", {"project": other})["assets"]]
        assert ids == [t["id"]], ids
        assert page.locator(f'{take_card} [data-aact="save"]').count() == 0, "it could be saved to the same project twice"
        again = api(port, "take.save", {"id": t["id"], "project": other})
        assert again["already"] and len(api(port, "asset.list", {"project": other})["assets"]) == 1
        ok("saved into a closed project while another is open; the open one didn't change; saving twice adds one")

        # ---------- the open project's assets ----------
        t = generate("This goes in this project. " * 10)  # long enough that it cannot end by itself during a check
        save_to(here)
        p = api(port, "project.get")
        assert [m["id"] for m in p["media"]] == [t["id"]] and not any(tr["clips"] for tr in p["tracks"])
        ok("Add to assets leaves the timeline alone")

        api(port, "history.undo")
        assert api(port, "project.get")["media"] == []
        api(port, "history.redo")
        assert len(api(port, "project.get")["media"]) == 1
        ok("adding to assets can be undone")

        # An assets-only project shows them instead of hiding them behind the empty page.
        page.goto("about:blank")
        page.goto(base + "#edit")
        page.wait_for_selector(f'[data-asset="{t["id"]}"]')
        assert page.is_visible(f'[data-asset="{t["id"]}"] [data-aact="remove"]')
        page.click(f'[data-asset="{t["id"]}"] [data-aact="play"]')
        page.wait_for_selector(f'[data-asset="{t["id"]}"] [data-aseek]', timeout=6000)
        for _ in range(60):
            if page.locator(f'[data-asset="{t["id"]}"] [aria-label^="Pause"]').count():
                break
            page.wait_for_timeout(50)
        assert page.locator(f'[data-asset="{t["id"]}"] [aria-label^="Pause"]').count() == 1
        page.click(f'[data-asset="{t["id"]}"] [data-aact="play"]')
        page.click(f'[data-asset="{t["id"]}"] [data-aact="place"]')
        page.wait_for_selector(".clip canvas", timeout=6000)
        p = api(port, "project.get")
        clips = [c for tr in p["tracks"] for c in tr["clips"]]
        assert len(clips) == 1 and clips[0]["media"] == t["id"]
        assert next(tr for tr in p["tracks"] if tr["clips"])["name"] == "Generated speech"
        api(port, "history.undo")
        assert not any(tr["clips"] for tr in api(port, "project.get")["tracks"])
        api(port, "history.redo")
        ok("an asset can be heard, then put on the timeline, undoably, on the Generated speech track")

        # The Edit panel lists them too, and one in use can't be removed.
        page.goto("about:blank")
        page.goto(base + "#edit")
        page.wait_for_selector(f'aside [data-asset="{t["id"]}"]')
        assert page.locator(f'[data-asset="{t["id"]}"] [data-aact="remove"]').count() == 0
        r = try_api(port, "asset.remove", {"media": t["id"]})
        assert not r["ok"] and r["error"]["code"] == "in_use", r
        ok("the Edit panel lists assets; one on the timeline can't be removed")

        row = f'aside [data-asset="{t["id"]}"]'
        page.click(f'{row} [data-aact="play"]')
        page.wait_for_selector(f'{row} [aria-label^="Pause"]', timeout=6000)
        page.click('[data-play]')
        page.wait_for_selector(f'{row} [aria-label^="Listen"]', timeout=3000)
        page.wait_for_timeout(200)
        assert api(port, "transport.state")["playing"]
        page.click(f'{row} [data-aact="play"]')
        page.wait_for_selector(f'{row} [aria-label^="Pause"]', timeout=6000)
        for _ in range(40):
            if not api(port, "transport.state")["playing"]:
                break
            page.wait_for_timeout(50)
        assert not api(port, "transport.state")["playing"]
        page.click(f'{row} [data-aact="play"]')
        ok("the project and a preview never play over each other")

        # ---------- the Assets page ----------
        f = str(speech(tmp / "song.wav", 2.0)).replace("\\", "/")
        page.goto("about:blank")
        page.goto(base + "#assets")
        page.wait_for_selector('.h1:has-text("Assets")')
        page.evaluate(f"FD.pickFiles = async () => ['{f}']; null")
        h1 = api(port, "project.get")["hash"]
        before = sum(len(tr["clips"]) for tr in api(port, "project.get")["tracks"])
        page.click('[data-aact="addfile"]')
        page.wait_for_selector('[data-asset]:has-text("song.wav")', timeout=8000)
        assert sum(len(tr["clips"]) for tr in api(port, "project.get")["tracks"]) == before
        assert api(port, "project.get")["hash"] != h1
        song = next(a for a in api(port, "asset.list")["assets"] if a["name"] == "song.wav")
        page.click(f'[data-asset="{song["id"]}"] [data-aact="remove"]')
        page.wait_for_selector(f'[data-asset="{song["id"]}"]', state="detached", timeout=4000)
        api(port, "history.undo")
        assert any(a["id"] == song["id"] for a in api(port, "asset.list")["assets"])
        ok("the Assets page adds a file without touching the timeline, and removes it undoably")

        page.select_option('[data-aact-change="view"]', other)
        page.wait_for_selector('[data-asset]:has-text("closed one")', timeout=4000)
        assert page.locator('[data-asset] [data-aact="place"]').count() == 0
        page.click('[data-aact="addfile"]')
        for _ in range(100):
            if len(api(port, "asset.list", {"project": other})["assets"]) == 2:
                break
            page.wait_for_timeout(50)
        assert len(api(port, "asset.list", {"project": other})["assets"]) == 2
        assert api(port, "project.get")["id"] == here
        ok("a file can be added to another project from here, without opening it")

        api(port, "project.open", {"id": other})
        p = api(port, "project.get")
        assert p["missing"] == [] and len(p["media"]) == 2
        peaks = api(port, "media.peaks", {"media": p["media"][0]["id"], "start": 0, "end": p["media"][0]["frames"], "bins": 40})
        assert peaks
        ok("what was saved into a closed project is really there when it's opened")

        # ---------- with nothing open, and small screens ----------
        api(port, "project.close")
        page.goto("about:blank")
        page.goto(base + "#assets")
        page.wait_for_selector('.h1:has-text("Assets")')
        assert page.locator(take_card).count() >= 3
        page.screenshot(path=str(OUT / "assets-1440.png"))
        tid = takes()[0]["id"]
        page.click(f'[data-take-card="{tid}"] [data-aact="discard"]')
        page.wait_for_selector(f'[data-take-card="{tid}"]', state="detached", timeout=4000)
        assert all(x["id"] != tid for x in takes())
        assert any(a["id"] == tid for a in api(port, "asset.list", {"project": here})["assets"]), "discarding removed the saved copy"
        ok("Assets works with nothing open; Discard leaves copies already in projects")

        for w, h in ((820, 1100), (390, 844)):
            page.set_viewport_size({"width": w, "height": h})
            for route in ("assets", "tts"):
                page.goto("about:blank")
                page.goto(base + "#" + route)
                page.wait_for_selector("#stage .h1")
                page.wait_for_timeout(300)
                assert page.evaluate("document.documentElement.scrollWidth - innerWidth") <= 0, (route, w)
                page.screenshot(path=str(OUT / f"{route}-{w}.png"))
        ok("Assets and Text to speech fit at 820 and 390 px")

        assert not errors, errors
        ok("no script errors")
        b.close()


if __name__ == "__main__":
    main()
