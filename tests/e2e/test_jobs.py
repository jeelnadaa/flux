"""End-to-end test of how long work shows up: no page redraws while a job runs, speech generation shows its progress,
failures are said out loud, and buttons whose work takes a moment say so.

Runs flux-mcp's HTTP server with a throwaway data folder and the stub voice engine (no model runs, nothing is fetched
from the internet) in a real browser.

    python tests/e2e/test_jobs.py     (needs `cargo build -p flux-mcp --release` first)
"""
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request
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


WATCH = """() => { window.__re = 0; new MutationObserver(m => { window.__re += m.length; }).observe(document.querySelector('#stage'), { childList: true }); }"""


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="flux-jobs-"))
    mir = build_catalog(tmp / "data")
    port = free_port()
    env = dict(os.environ, FLUX_DATA_DIR=str(tmp / "data"), FLUX_EXPORT_DIR=str(tmp / "export"), FLUX_NULL_AUDIO="1",
               FLUX_STUB_LOG=str(tmp / "stub.log"), FLUX_STUB_TTS_SLEEP="6")
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
        api(port, "project.new", {"name": "Jobs test"})
        api(port, "media.import", {"path": speech(tmp / "talk.wav", 8.0), "wait": True})
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

    def tts_jobs():
        return [j for j in api(port, "job.list") if j["kind"] == "tts"]

    with sync_playwright() as pw:
        b = pw.chromium.launch()
        page = b.new_page(viewport={"width": 1440, "height": 900})
        errors = []
        page.on("pageerror", lambda e: errors.append(e.stack or str(e)))

        def open_tts(text):
            page.goto("about:blank")
            page.goto(base + "#tts")
            page.wait_for_selector('[data-ui="ttsText"]')
            page.fill('[data-ui="ttsText"]', text)
            page.wait_for_selector('[data-act="tts-speak"]:not([disabled])')

        # ---------- speech shows what it's doing ----------
        open_tts("Read this out loud.")
        page.click('[data-act="tts-speak"]')
        page.wait_for_selector('[data-dl="tts"]', timeout=3000)
        assert page.locator('button:has-text("Generating…")').is_disabled()
        assert page.locator('[data-act="tts-speak"]').count() == 0, "a second click would start a second job"
        page.wait_for_selector('[data-dl="tts"] [data-dl-text]:has-text("Speaking part 1 of 1")', timeout=8000)
        assert len(tts_jobs()) == 1
        page.evaluate(WATCH)
        page.click('[data-dl="tts"] [data-act="dl-log"]')
        page.wait_for_selector('[data-dl="tts"] [role="log"]:has-text("Loading the model")', timeout=4000)
        page.screenshot(path=str(OUT / "tts-progress-1440.png"))
        assert page.evaluate("window.__re") == 0, "the page redrew while speech was being made"
        ok("Generate shows the row straight away, what the engine is doing, and a log, without redrawing")

        # ---------- the Jobs page is steady and current ----------
        page.click('[data-dl="tts"] [data-act="dl-log"]')
        page.goto(base + "#jobs")
        page.reload()
        RUN = '.lst [data-dl]:has([data-act="job-cancel"])'
        page.wait_for_selector(RUN)
        running = page.locator(RUN).get_attribute("data-dl")
        page.evaluate(WATCH)
        page.evaluate("window.__row = document.querySelector('.lst [data-dl]:has([data-act=\"job-cancel\"])'); window.__btn = window.__row.querySelector('[data-act=\"job-cancel\"]')")
        seen = set()
        for _ in range(8):
            page.wait_for_timeout(120)
            row = page.locator(RUN).inner_text()
            seen.add(row)
        assert page.evaluate("window.__re") == 0, "the Jobs page redrew itself while a job ran"
        assert page.evaluate("window.__row.isConnected && window.__btn.isConnected")
        assert any("Speaking part" in r for r in seen), seen
        ok("the Jobs page stays still while a job runs, and shows the current step")

        # A job page it wasn't following still finishes on screen.
        page.wait_for_selector(f'.lst [data-dl="{running}"]:has-text("Done")', timeout=15000)
        ok("the Jobs page notices a job finishing, whoever started it")

        # ---------- failure and cancel ----------
        open_tts("fail-this-run please")
        page.click('[data-act="tts-speak"]')
        page.wait_for_selector("text=The speech wasn't made.", timeout=15000)
        assert "fail" in page.inner_text("#stage").lower()
        assert page.locator('[data-act="tts-speak"]').count() == 1
        ok("speech that fails says so on the page and can be tried again")

        open_tts("Stop this one.")
        before = len(tts_jobs())
        page.click('[data-act="tts-speak"]')
        page.wait_for_selector('[data-dl="tts"] [data-act="tts-cancel"]')
        page.click('[data-act="tts-cancel"]')
        page.wait_for_selector('[data-act="tts-speak"]', timeout=8000)
        assert page.locator("text=The speech wasn't made.").count() == 0
        assert len(tts_jobs()) == before + 1 and sum(j["state"] == "cancelled" for j in tts_jobs()) == 1, tts_jobs()
        ok("cancelling speech stops it quietly")

        # ---------- a slow job on its own page: only the moving parts change ----------
        def slow_jobs(fail=False):
            count = {"n": 0, "rev": None}

            def handler(route):
                m = json.loads(route.request.post_data or "{}").get("method")
                if m == "transport.state" and count["n"] < 10:
                    # The real job finishes at once and changes the project; this one is still "running".
                    r = route.fetch()
                    d = r.json()
                    count["rev"] = d["result"]["rev"] if count["rev"] is None else count["rev"]
                    d["result"]["rev"] = count["rev"]
                    route.fulfill(response=r, json=d)
                    return
                if m != "job.get":
                    route.continue_()
                    return
                r = route.fetch()
                d = r.json()
                count["n"] += 1
                if count["n"] < 10:
                    d["result"].update(state="running", progress=count["n"] / 10, stage="Rendering", finished=None, error=None)
                    time.sleep(.12)
                elif fail:
                    d["result"].update(state="failed", error="the test made this fail")
                route.fulfill(response=r, json=d)

            return handler, count

        def steady(start, name):
            h, c = slow_jobs()
            page.route("**/api/call", h)
            start()
            for _ in range(60):
                if c["n"] >= 2:
                    break
                page.wait_for_timeout(50)
            page.wait_for_timeout(120)
            page.evaluate(WATCH)
            bars = "[...document.querySelectorAll('#stage .prog')].map(e => e.style.getPropertyValue('--v')).join()"
            a = page.evaluate(bars)
            page.wait_for_timeout(500)
            z = page.evaluate(bars)
            assert a != z, (name, a, z)
            assert page.evaluate("window.__re") == 0, f"{name} redrew while the job ran"
            page.wait_for_timeout(900)
            page.unroute("**/api/call", h)
            ok(f"{name}: the bar moves and the page isn't redrawn")

        page.goto("about:blank")
        page.goto(base + "#export")
        page.wait_for_selector('[data-act="export-start"]')
        steady(lambda: page.click('[data-act="export-start"]'), "export")

        page.goto("about:blank")
        page.goto(base + "#stems-setup")
        page.wait_for_selector('[data-act="stems-start"]')
        steady(lambda: page.click('[data-act="stems-start"]'), "split into stems")

        for screen, key, act in (("convert", "conv", "conv-start"), ("batch", "batch", "batch-start")):
            page.goto(base + "#home")
            page.reload()
            page.wait_for_selector("#stage a, #stage button")
            f = str(tmp / "talk.wav").replace("\\", "/")
            page.evaluate(f"FD.D.ui.{key} = Object.assign({{ files: ['{f}'], format: 'wav', bitrate: 245, sample_rate: 0, steps: [] }})")
            page.evaluate(f"Flux.render('{screen}')")
            page.wait_for_selector(f'[data-act="{act}"]:not([disabled])')
            steady(lambda: page.click(f'[data-act="{act}"]'), screen)

        # A job with no page of its own that fails says so.
        c = next(x for t in api(port, "project.get")["tracks"] for x in t["clips"])
        page.goto("about:blank")
        page.goto(base + "#edit")
        page.wait_for_selector(".clip canvas")
        page.wait_for_timeout(500)
        page.evaluate(f"FD.D.sel.clip = {c['id']}; Flux.rr()")
        page.wait_for_selector('[data-act="clip-bounce"]')
        jid = api(port, "job.list")[0]["id"]
        h, _ = slow_jobs(fail=True)

        def as_job(route):
            # Bounce finishes in one call today; this is the path for any action that hands back a job.
            if json.loads(route.request.post_data or "{}").get("method") == "clip.bounce":
                route.fulfill(json={"ok": True, "result": {"job": jid}})
            else:
                h(route)

        page.route("**/api/call", as_job)
        page.click('[data-act="clip-bounce"]')
        page.wait_for_selector("text=the test made this fail", timeout=8000)
        page.unroute("**/api/call", as_job)
        ok("a background job that fails doesn't fail in silence")

        # ---------- buttons and navigation say they're working ----------
        def held(method):
            # Holds the engine's answer until the test lets it go, so "while it's working" is certain.
            calls = []

            def handler(route):
                if json.loads(route.request.post_data or "{}").get("method") == method:
                    calls.append(route)
                else:
                    route.continue_()

            return handler, calls

        def release(calls):
            for r in calls:
                r.continue_()

        page.goto("about:blank")
        page.goto(base + "#edit")
        page.wait_for_selector(".clip canvas")
        page.wait_for_timeout(500)
        page.evaluate(f"FD.D.sel.clip = {c['id']}; Flux.rr()")
        chip = '[data-act="clip-normalize"]'
        page.wait_for_selector(chip)
        h, calls = held("clip.normalize")
        page.route("**/api/call", h)
        page.click(chip)
        page.wait_for_selector(chip + '[aria-busy="true"]', timeout=2000)
        page.evaluate(f"document.querySelector('{chip}').click()")
        page.wait_for_timeout(300)
        assert len(calls) == 1, len(calls)
        release(calls)
        page.wait_for_selector(chip + ':not([aria-busy])', timeout=5000)
        page.unroute("**/api/call", h)
        ok("a button whose work takes a moment shows it, and a second click does nothing")

        h, calls = held("analysis.loudness")
        page.route("**/api/call", h)
        page.evaluate("FD.stale('loudness'); Flux.render('master'); null")
        page.wait_for_selector("html.navbusy", state="attached", timeout=2000)
        release(calls)
        page.wait_for_selector(".h1:has-text('Master')", timeout=5000)
        assert page.locator("html.navbusy").count() == 0
        page.unroute("**/api/call", h)
        ok("a page that takes a moment to measure shows it's on the way")

        h, calls = held("tool.silence.scan")
        page.route("**/api/call", h)
        page.evaluate("FD.D.ui.silScan = null; Flux.render('fx-silence'); null")
        page.wait_for_selector("text=Scanning…", timeout=3000)
        release(calls)
        page.wait_for_selector("text=Not scanned", state="detached", timeout=5000)
        page.unroute("**/api/call", h)
        ok("the silence remover says when it's scanning")

        for w, h in ((820, 1100), (390, 844)):
            page.set_viewport_size({"width": w, "height": h})
            page.goto("about:blank")
            page.goto(base + "#jobs")
            page.wait_for_selector(".lst [data-dl]")
            page.wait_for_timeout(300)
            assert page.evaluate("document.documentElement.scrollWidth - innerWidth") <= 0, w
            page.screenshot(path=str(OUT / f"jobs-{w}.png"))
        ok("the Jobs page fits at 820 and 390 px")

        assert not errors, errors
        ok("no script errors")
        b.close()


if __name__ == "__main__":
    main()
