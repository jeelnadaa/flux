"""End-to-end test of working in the timeline: sliders that follow the pointer and take typed values, moving and
trimming clips by drag, keyboard and typed position, scrolling and zooming the timeline, and deselecting.

Runs flux-mcp's HTTP server on a throwaway data folder with a real project (a generated WAV, imported through the
app's own method) in a real browser. Positions are read back from the engine in samples, not from the page.

    python tests/e2e/test_editor.py     (needs `cargo build -p flux-mcp --release` first)
"""
import math
import os
import struct
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

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target" / "release" / ("flux-mcp.exe" if os.name == "nt" else "flux-mcp")
OUT = ROOT / "target" / "e2e"
SR = 48000
SECS = 20


def ok(msg):
    print(f"ok - {msg}", flush=True)


def tone(path):
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        fr = bytearray()
        for i in range(SR * SECS):
            # Loud and quiet stretches, so a drawn wave has something to show at any zoom.
            fr += struct.pack("<h", int((3000 + 22000 * ((i // 2400) % 2)) * math.sin(2 * math.pi * 220 * i / SR)))
        w.writeframes(bytes(fr))
    return str(path)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="flux-editor-"))
    port = free_port()
    env = dict(os.environ, FLUX_DATA_DIR=str(tmp / "data"), FLUX_EXPORT_DIR=str(tmp / "export"), FLUX_NULL_AUDIO="1")
    srv = subprocess.Popen([str(EXE), "serve", "--port", str(port), "--ui", str(ROOT / "apps/desktop/ui"), "--no-token"], env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(100):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/api/health", timeout=1)
                break
            except Exception:
                time.sleep(.1)
        api(port, "media.import", {"path": tone(tmp / "take.wav")})
        for _ in range(200):
            try:
                if any(t["clips"] for t in api(port, "project.get")["tracks"]):
                    break
            except RuntimeError:
                pass
            time.sleep(.1)
        api(port, "track.add", {"name": "Second"})
        run(port)
        print("ALL OK")
    finally:
        srv.terminate()


def run(port):
    def project():
        return api(port, "project.get")

    def clip():
        return next(c for t in project()["tracks"] for c in t["clips"])

    def track_of(cid):
        return next(t["id"] for t in project()["tracks"] if any(c["id"] == cid for c in t["clips"]))

    with sync_playwright() as pw:
        b = pw.chromium.launch()
        page = b.new_page(viewport={"width": 1440, "height": 900})
        errors = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        api(port, "settings.set", {"values": {"onboarded": True}})
        page.goto(f"http://127.0.0.1:{port}/#edit")
        page.wait_for_selector(".clip canvas")
        page.wait_for_timeout(900)
        start0 = clip()
        cid = start0["id"]
        CLIP = f'.clip[data-clip="{cid}"]'

        def settle(ms=350):
            page.wait_for_timeout(ms)

        def click_clip():
            r = page.locator(CLIP).bounding_box()
            # The middle of the part in view: when zoomed, most of the clip is off to the sides.
            left, right = max(r["x"], 250), min(r["x"] + r["width"], 1100)
            page.mouse.click((left + right) / 2, r["y"] + r["height"] / 2)
            settle(150)

        def selected():
            return page.locator(".clip.sel").count() == 1

        def mark():
            page.evaluate("document.querySelectorAll('#stage canvas').forEach(c => c._mark = 1)")

        def same_page():
            """True when nothing on the page was rebuilt since mark()."""
            return page.evaluate("[...document.querySelectorAll('.tl canvas')].every(c => c._mark === 1)")

        # ---------- a redraw doesn't replay the screen's entrance ----------
        page.evaluate("Flux.rr()")
        page.wait_for_timeout(120)
        rise = page.evaluate("document.getAnimations().filter(a => a.animationName === 'rise' && a.playState === 'running').length")
        assert rise == 0, rise
        ok("redrawing a screen doesn't play its entrance again")

        # ---------- deselecting ----------
        click_clip()
        assert selected()
        assert page.locator('[aria-label="Deselect clip"]').count() == 1
        mark()
        settle(700)  # longer than the page's poll, so a redraw caused by the selection would have happened
        click_clip()
        assert not selected()
        settle(700)
        assert same_page(), "selecting redrew the timeline"
        assert page.locator('[aria-label="Deselect clip"]').count() == 0
        click_clip()
        page.keyboard.press("Escape")
        settle(100)
        assert not selected()
        click_clip()
        page.click('[aria-label="Deselect clip"]')
        settle(100)
        assert not selected()
        click_clip()
        lane2 = page.locator(".lane").nth(1).bounding_box()
        page.mouse.click(lane2["x"] + lane2["width"] * .5, lane2["y"] + lane2["height"] / 2)
        settle(500)
        assert not selected()
        pos = page.evaluate("FD.D.st.pos")
        want = .5 * SECS * SR * 1.04
        assert abs(pos - want) < SR * .05, (pos, want)
        ok("a selected clip deselects by clicking it, empty space, Esc or the button, without redrawing the timeline")

        # ---------- sliders ----------
        click_clip()
        GAIN = '.sl[data-bind*=":gain_db:"]'
        tr = page.locator(f"{GAIN} .track").bounding_box()
        th = page.locator(f"{GAIN} .th").bounding_box()
        cx, cy = th["x"] + th["width"] / 2, th["y"] + th["height"] / 2
        # Pressing off-centre on the handle doesn't make it jump.
        page.mouse.move(cx + 5, cy)
        page.mouse.down()
        settle(50)
        th1 = page.locator(f"{GAIN} .th").bounding_box()
        assert abs(th1["x"] + th1["width"] / 2 - cx) < 1.5, (th1, cx)
        dur = page.evaluate("[getComputedStyle(document.querySelector('.sl.dr .th')).transitionDuration, getComputedStyle(document.querySelector('.sl.dr .fill')).transitionDuration]")
        assert all(x.split(",")[-1].strip() == "0s" for x in dur), dur
        mark()
        worst = 0
        for i in range(1, 25):
            x = cx + 5 + i * 3.1
            page.mouse.move(x, cy)
            v = page.evaluate(f"parseFloat(document.querySelector('{GAIN}').style.getPropertyValue('--v'))")
            at = tr["x"] + v / 100 * tr["width"]
            worst = max(worst, abs(at - (x - 5)))
        assert worst < 1, worst
        # Shift moves it a tenth as far.
        v0 = page.evaluate(f"parseFloat(document.querySelector('{GAIN}').style.getPropertyValue('--v'))")
        page.keyboard.down("Shift")
        page.mouse.move(cx + 5 + 24 * 3.1 + 40, cy)
        page.keyboard.up("Shift")
        v1 = page.evaluate(f"parseFloat(document.querySelector('{GAIN}').style.getPropertyValue('--v'))")
        assert abs((v1 - v0) / 100 * tr["width"] - 4) < .5, (v0, v1, tr["width"])
        assert same_page(), "dragging a slider redrew the page"
        page.mouse.up()
        settle(500)
        moved = clip()["gain_db"]
        assert moved > 1, moved
        expect = -24 + 48 * v1 / 100
        assert abs(moved - expect) < .06, (moved, expect)
        # Fractional position survives the redraw, so the handle doesn't hop on release.
        v2 = page.evaluate(f"parseFloat(document.querySelector('{GAIN}').style.getPropertyValue('--v'))")
        assert abs(v2 - v1) < .15, (v1, v2)
        ok(f"a slider follows the pointer within {worst:.2f} px, doesn't jump when grabbed, Shift is fine control, release keeps its place")

        # A value can be typed.
        page.click(f"{GAIN} .sv")
        inp = page.locator(f"{GAIN} input.svin")
        assert inp.evaluate("e => e === document.activeElement")
        inp.fill("-6.5 dB")
        page.keyboard.press("Enter")
        settle(500)
        assert clip()["gain_db"] == -6.5, clip()["gain_db"]
        assert "6.5" in page.inner_text(f"{GAIN} .sv")
        # Rubbish is refused and Esc leaves it alone.
        page.click(f"{GAIN} .sv")
        inp.fill("loud")
        page.keyboard.press("Enter")
        assert inp.get_attribute("aria-invalid") == "true"
        page.keyboard.press("Escape")
        settle(150)
        assert page.locator("input.svin").count() == 0
        assert clip()["gain_db"] == -6.5
        assert selected(), "Esc in a value box deselected the clip"
        # Keyboard only: Enter on the slider opens it, Enter applies, focus comes back.
        page.focus(f"{GAIN} .track")
        page.keyboard.press("Enter")
        page.keyboard.type("3")
        page.keyboard.press("Enter")
        settle(500)
        assert clip()["gain_db"] == 3, clip()["gain_db"]
        assert page.evaluate("document.activeElement.matches('.track')")
        # Units convert: fade-in is in samples and reads in milliseconds.
        FADE = '.sl[data-bind*=":fade_in:"]'
        page.click(f"{FADE} .sv")
        page.locator(f"{FADE} input.svin").fill("100")
        page.keyboard.press("Enter")
        settle(500)
        assert abs(clip()["fade_in"] - 4800) <= 2, clip()["fade_in"]
        api(port, "clip.set", {"clip": cid, "gain_db": 0, "fade_in": 0})
        ok("typing a value works by mouse and keyboard, converts units, and refuses rubbish")

        # Typing works on the other kinds of slider too.
        page.goto(f"http://127.0.0.1:{port}/#mix")
        page.reload()
        page.wait_for_selector(".fader[data-bind]")
        page.click(".fader[data-bind^='track'] ~ [data-gtext]")
        page.locator(".ch input.svin").fill("-3")
        page.keyboard.press("Enter")
        settle(500)
        assert project()["tracks"][0]["gain_db"] == -3, project()["tracks"][0]["gain_db"]
        api(port, "track.set", {"track": project()["tracks"][0]["id"], "gain_db": 0})
        api(port, "fx.add", {"kind": "reverb", "target": "master"})
        page.goto(f"http://127.0.0.1:{port}/#fx-reverb")
        page.reload()
        page.wait_for_selector(".sl[data-bind]")
        settle(800)
        s = page.locator(".sl[data-bind^='fx']").first
        before = s.evaluate("e => e.style.getPropertyValue('--v')")
        s.locator(".sv").click()
        page.locator("input.svin").fill("10")
        page.keyboard.press("Enter")
        settle(400)
        assert s.evaluate("e => e.style.getPropertyValue('--v')") != before or "10" in s.locator(".sv").inner_text()
        ok("the mixer's faders and an effect's sliders take typed values")

        page.goto(f"http://127.0.0.1:{port}/#edit")
        page.reload()
        page.wait_for_selector(".clip canvas")
        settle(800)

        # ---------- moving a clip ----------
        c0 = clip()
        r = page.locator(CLIP).bounding_box()
        lw = page.evaluate("document.querySelector('.lane').offsetWidth")
        vlen = c0["len"] * 1.04
        x, y = r["x"] + r["width"] / 2, r["y"] + r["height"] / 2
        page.mouse.move(x, y)
        page.mouse.down()
        mark()
        worst = 0
        for i in range(1, 21):
            page.mouse.move(x + i * 6, y)
            b1 = page.locator(CLIP).bounding_box()
            worst = max(worst, abs(b1["x"] - (r["x"] + i * 6)))
        assert page.evaluate("getComputedStyle(document.querySelector('.clip.dr')).transitionDuration") == "0s"
        assert worst < 1.01, worst
        assert "0:" in page.inner_text(f"{CLIP} .cl")
        assert same_page(), "dragging a clip redrew the page"
        page.mouse.up()
        settle(500)
        moved = clip()
        want = round(120 / lw * vlen)
        assert abs(moved["start"] - want) <= vlen / lw, (moved["start"], want)
        assert selected(), "a dragged clip should end up selected"
        ok(f"a dragged clip follows the pointer within {worst:.2f} px and lands on the right sample")

        # Typed position, exact to the sample.
        start = page.locator('[data-act-change="clip-start"]')
        start.fill("0:03.000")
        page.keyboard.press("Enter")
        settle(500)
        assert clip()["start"] == 3 * SR, clip()["start"]
        start = page.locator('[data-act-change="clip-start"]')
        start.fill("2.5")
        page.keyboard.press("Enter")
        settle(500)
        assert clip()["start"] == 2.5 * SR
        start = page.locator('[data-act-change="clip-start"]')
        start.fill("soon")
        page.keyboard.press("Enter")
        settle(300)
        assert start.get_attribute("aria-invalid") == "true"
        assert clip()["start"] == 2.5 * SR
        page.reload()
        page.wait_for_selector(".clip canvas")
        settle(700)
        ok("a clip's start can be typed, as minutes:seconds or seconds, exact to the sample; rubbish is refused")

        # Keyboard: a burst of arrows is one move.
        click_clip()
        page.focus(CLIP)
        hist0 = api(port, "history.list")["current"]
        for _ in range(3):
            page.keyboard.press("ArrowRight")
        page.keyboard.press("Shift+ArrowRight")
        settle(600)
        assert clip()["start"] == 2.5 * SR + 3 * 480 + 48, clip()["start"]
        hist1 = api(port, "history.list")["current"]
        assert hist1 - hist0 == 1, (hist0, hist1)
        page.keyboard.press("Control+ArrowLeft")
        settle(600)
        assert clip()["start"] == 2.5 * SR + 3 * 480 + 48 - SR, clip()["start"]
        tracks = project()["tracks"]
        page.keyboard.press("ArrowDown")
        settle(600)
        assert track_of(cid) == tracks[1]["id"]
        page.keyboard.press("ArrowUp")
        settle(600)
        assert track_of(cid) == tracks[0]["id"]
        ok("arrow keys nudge by 10 ms, 1 ms (Shift) and 1 s (Ctrl) as one undo step per burst; up and down change track")

        # Dragging to another track.
        api(port, "clip.move", {"clip": cid, "start": 0})
        page.reload()
        page.wait_for_selector(".clip canvas")
        settle(700)
        r = page.locator(CLIP).bounding_box()
        l2 = page.locator(".lane").nth(1).bounding_box()
        page.mouse.move(r["x"] + 300, r["y"] + 40)
        page.mouse.down()
        page.mouse.move(r["x"] + 300, l2["y"] + l2["height"] / 2, steps=6)
        settle(60)
        b1 = page.locator(CLIP).bounding_box()
        assert l2["y"] - 10 < b1["y"] < l2["y"] + 12, (b1, l2)
        page.mouse.up()
        settle(500)
        assert track_of(cid) == project()["tracks"][1]["id"]
        api(port, "clip.move", {"clip": cid, "start": 0, "track": project()["tracks"][0]["id"]})
        page.reload()
        page.wait_for_selector(".clip canvas")
        settle(700)
        ok("a clip can be dragged onto another track")

        # Trimming by the edges, and typed length.
        click_clip()
        c0 = clip()
        lw = page.evaluate("document.querySelector('.lane').offsetWidth")
        r = page.locator(CLIP).bounding_box()
        page.mouse.move(r["x"] + r["width"] - 3, r["y"] + 30)
        page.mouse.down()
        page.mouse.move(r["x"] + r["width"] - 103, r["y"] + 30, steps=5)
        page.mouse.up()
        settle(500)
        c1 = clip()
        assert abs((c0["len"] - c1["len"]) - 100 / lw * c0["len"] * 1.04) <= c0["len"] * 1.04 / lw * 1.5, (c0["len"], c1["len"])
        assert c1["start"] == c0["start"]
        page.locator('[data-act-change="clip-len"]').fill("5")
        page.keyboard.press("Enter")
        settle(500)
        assert clip()["len"] == 5 * SR, clip()["len"]
        r = page.locator(CLIP).bounding_box()
        page.mouse.move(r["x"] + 3, r["y"] + 30)
        page.mouse.down()
        page.mouse.move(r["x"] + 53, r["y"] + 30, steps=4)
        page.mouse.up()
        settle(500)
        c2 = clip()
        assert c2["start"] > 0 and c2["offset"] == c2["start"], (c2["start"], c2["offset"])
        assert c2["start"] + c2["len"] == 5 * SR, c2
        ok("a clip's edges trim it, and its length can be typed")
        api(port, "clip.trim", {"clip": cid, "start": 0, "end": SECS * SR})
        page.reload()
        page.wait_for_selector(".clip canvas")
        settle(700)
        assert clip()["len"] == SECS * SR and clip()["start"] == 0

        # ---------- the timeline ----------
        tl = ".tl[data-tl]"
        m = page.evaluate(f"(() => {{ const t = document.querySelector('{tl}'); return [t.scrollWidth, t.clientWidth]; }})()")
        assert m[0] <= m[1], m
        mark()
        page.click('[aria-label="Zoom in"]')
        settle(60)
        z = page.evaluate("FD.D.ui.zoom")
        assert z > 1.5, z
        assert page.evaluate(f"document.querySelector('{tl}').scrollWidth > document.querySelector('{tl}').clientWidth")
        assert same_page(), "zooming redrew the page"
        assert "1.6" in page.inner_text("[data-zv]")
        # Ctrl and wheel zooms around the pointer.
        box = page.locator(tl).bounding_box()
        hw = page.evaluate("document.querySelector('.lane').offsetLeft")
        px = box["x"] + hw + 500
        page.mouse.move(px, box["y"] + 90)

        def time_at(x):
            return page.evaluate(f"(() => {{ const r = document.querySelector('.lane').getBoundingClientRect(); return ({x} - r.left) / r.width; }})()")

        u0 = time_at(px)
        page.keyboard.down("Control")
        for _ in range(4):
            page.mouse.wheel(0, -240)
            page.wait_for_timeout(40)
        page.keyboard.up("Control")
        settle(300)
        z2 = page.evaluate("FD.D.ui.zoom")
        assert z2 > z * 3, (z, z2)
        lw2 = page.evaluate("document.querySelector('.lane').offsetWidth")
        assert abs(time_at(px) - u0) * lw2 < 2, (u0, time_at(px))
        assert same_page(), "wheel zoom redrew the page"
        ok(f"zooming stays in place: {z:.1f}× then {z2:.1f}× around the pointer, without redrawing the page")

        # Moving along: headers stay, the ruler follows, a redraw keeps the place.
        page.evaluate(f"document.querySelector('{tl}').scrollLeft = 2500")
        settle(200)
        head = page.locator(".thd").first.bounding_box()
        assert abs(head["x"] - box["x"]) < 1, (head, box)
        ruler = page.evaluate("(() => { const c = document.querySelector('.ruler canvas'), r = c.getBoundingClientRect(); return [r.left, r.width, c.width]; })()")
        assert abs(ruler[0] - (box["x"] + hw)) < 1, ruler
        assert ruler[2] <= 2 * 4096, ruler
        page.evaluate("Flux.rr()")
        settle(300)
        assert page.evaluate(f"document.querySelector('{tl}').scrollLeft") == 2500
        # A plain mouse wheel moves along when there's nothing to scroll up or down.
        page.mouse.move(px, box["y"] + 90)
        page.mouse.wheel(0, 200)
        settle(200)
        assert page.evaluate(f"document.querySelector('{tl}').scrollLeft") > 2500
        ok("the timeline scrolls sideways with its track headers and ruler in place, and keeps its place through a redraw")

        # All the way in: a real wave still draws, canvases stay a legal size, a click lands within a pixel's samples.
        page.keyboard.press("Escape")
        for _ in range(30):
            page.click('[aria-label="Zoom in"]')
        settle(900)
        zm = page.evaluate("FD.D.ui.zoom")
        lw3 = page.evaluate("document.querySelector('.lane').offsetWidth")
        spp = SECS * SR * 1.04 / lw3
        assert 3.9 < spp < 4.2, spp
        page.evaluate(f"document.querySelector('{tl}').scrollLeft = {lw3 // 2}")
        settle(900)
        got = page.evaluate("""(() => { const c = document.querySelector('.clip canvas'), x = c.getContext('2d'), d = x.getImageData(0, 0, c.width, c.height).data; let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i]) n++; const r = c.getBoundingClientRect(), t = document.querySelector('.tl').getBoundingClientRect(); return [c.width, n, r.left <= t.left + 176 && r.right >= t.right]; })()""")
        assert got[0] <= 2 * 4096, got
        assert got[1] > 1000, got
        assert got[2], got
        ok(f"at full zoom ({spp:.2f} samples per pixel, {zm:.0f}×) the wave still draws from the real audio, in a {got[0]} px canvas")

        # The ruler drags the playhead. Seeking is exact to the zoom.
        rb = page.locator(".ruler").bounding_box()
        page.mouse.move(box["x"] + hw + 200, rb["y"] + 10)
        page.mouse.down()
        page.mouse.move(box["x"] + hw + 400, rb["y"] + 10, steps=5)
        settle(120)
        page.mouse.up()
        settle(500)
        u = (lw3 // 2 + 400) / lw3
        pos = page.evaluate("FD.D.st.pos")
        assert abs(pos - u * SECS * SR * 1.04) <= 8, (pos, u * SECS * SR * 1.04)
        ph = page.evaluate("(() => { const p = document.querySelector('[data-ph]').getBoundingClientRect(); return p.left; })()")
        assert abs(ph - (box["x"] + hw + 400)) < 2, ph
        ok("dragging the ruler moves the playhead, exact to a pixel at full zoom")

        # Playback pages the view along.
        page.click('[aria-label="Fit the whole timeline"]')
        settle(400)
        assert page.evaluate(f"document.querySelector('{tl}').scrollLeft") == 0
        assert page.evaluate("FD.D.ui.zoom") == 1
        page.keyboard.press("=")
        settle(100)
        assert page.evaluate("FD.D.ui.zoom") > 1.5
        page.keyboard.press("-")
        settle(100)
        assert page.evaluate("FD.D.ui.zoom") == 1
        for _ in range(4):
            page.keyboard.press("=")
        settle(300)
        z4 = page.evaluate("FD.D.ui.zoom")
        lw4 = page.evaluate("document.querySelector('.lane').offsetWidth")
        vw = page.evaluate(f"document.querySelector('{tl}').clientWidth") - hw
        at = int((vw * .99) / lw4 * SECS * SR * 1.04)
        api(port, "transport.seek", {"pos": at})
        page.evaluate(f"document.querySelector('{tl}').scrollLeft = 0")
        page.wait_for_timeout(2700)
        api(port, "transport.play")
        for _ in range(60):
            page.wait_for_timeout(100)
            if page.evaluate(f"document.querySelector('{tl}').scrollLeft") > 0:
                break
        api(port, "transport.pause")
        assert page.evaluate(f"document.querySelector('{tl}').scrollLeft") > 0, z4
        ok("playback brings the playhead back into view")

        # Sideways scrolling belongs to the timeline, not the page, at every width.
        for w, h in ((1440, 900), (820, 1100), (390, 844)):
            page.set_viewport_size({"width": w, "height": h})
            settle(500)
            over = page.evaluate("document.documentElement.scrollWidth - innerWidth")
            assert over <= 0, (w, over)
            assert page.evaluate(f"document.querySelector('{tl}').scrollWidth > document.querySelector('{tl}').clientWidth")
            page.screenshot(path=str(OUT / f"editor-timeline-{w}.png"))
        page.set_viewport_size({"width": 1440, "height": 900})
        settle(300)
        if not selected():
            click_clip()
        settle(300)
        assert selected()
        page.screenshot(path=str(OUT / "editor-timeline-selected.png"))
        ok("at 1440, 820 and 390 px the page never scrolls sideways")

        assert not errors, errors
        ok("no script errors")
        b.close()


if __name__ == "__main__":
    main()
