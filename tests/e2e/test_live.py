"""End-to-end test of the Live page and tooltips, in a real browser against the real engine.

Runs flux-mcp's HTTP server on a free port with a throwaway data folder, opens the desktop UI in Chromium
(Playwright), and drives it like a person: start a route on test music, read the meters, bypass, change a
built-in preset (which makes a copy), add modules, modulators, steps and macros, export and import, delete,
stop. It also renders every screen of the app and fails on any page error, checks that every control on the
Live page has a name and a tooltip, and that the page fits at 1440, 820 and 390 px.

    python tests/e2e/test_live.py            (needs `cargo build -p flux-mcp --release` first)
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
    r = json.loads(urllib.request.urlopen(req, timeout=20).read())
    if not r.get("ok"):
        raise RuntimeError(r.get("error"))
    return r["result"]


def wait(page, ms):
    page.wait_for_timeout(ms)


def main():
    assert EXE.exists(), f"build flux-mcp first: {EXE}"
    OUT.mkdir(parents=True, exist_ok=True)
    data = Path(tempfile.mkdtemp(prefix="flux-e2e-"))
    port = free_port()
    env = dict(os.environ, FLUX_DATA_DIR=str(data / "data"), FLUX_EXPORT_DIR=str(data / "export"))
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
        with sync_playwright() as pw:
            b = pw.chromium.launch()
            page = b.new_page(viewport={"width": 1440, "height": 900})
            errors = []
            page.on("pageerror", lambda e: errors.append(str(e)))
            page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
            page.goto(f"http://127.0.0.1:{port}/#live")
            page.wait_for_selector("[data-livepage]")
            page.wait_for_selector("[data-designer]", timeout=8000)
            ev = page.evaluate

            # 1. Gallery shows the built-ins for each route.
            n_out = ev("document.querySelectorAll('.fxc[data-lact=\"pick\"]').length")
            page.click(".seg[data-lseg] button:nth-child(3)")  # Microphone (first child is the indicator)
            wait(page, 300)
            n_in = ev("document.querySelectorAll('.fxc[data-lact=\"pick\"]').length")
            assert n_out >= 15 and n_in >= 15, (n_out, n_in)
            ok(f"gallery: {n_out} computer-sound and {n_in} microphone presets")

            # 2. Start the computer-sound route on test music, playing nowhere, and read the meters.
            page.select_option('[data-lchange="route"][data-route="output"][data-k="src"]', "demo-music|")
            page.select_option('[data-lchange="route"][data-route="output"][data-k="output"]', "none")
            page.click('[data-lact="start"][data-route="output"]')
            wait(page, 1500)
            st = api(port, "live.status")["routes"]["output"]
            assert st and st["running"], st
            assert st["in_rms_db"] > -40 and st["out_rms_db"] > -60, st
            lv = ev("parseFloat(document.querySelector('[data-lvm=\"output:out\"]').style.getPropertyValue('--lv'))")
            assert lv > 20, lv
            assert "Live" in page.inner_text('[data-route="output"].box .pill')
            ok(f"route runs: in {st['in_rms_db']} dB RMS, out {st['out_rms_db']} dB RMS, meter {lv:.0f}%, {st['underruns']} dropouts")

            # 2b. The microphone route runs at the same time, here on the built-in test voice.
            page.select_option('[data-lchange="route"][data-route="input"][data-k="src"]', "demo-voice|")
            page.select_option('[data-lchange="route"][data-route="input"][data-k="output"]', "none")
            page.click('[data-lact="start"][data-route="input"]')
            wait(page, 1200)
            si = api(port, "live.status")["routes"]["input"]
            assert si and si["running"] and si["out_rms_db"] > -70, si
            page.click('[data-lact="stop"][data-route="input"]')
            wait(page, 400)
            assert api(port, "live.status")["routes"]["input"] is None
            ok(f"microphone route runs alongside: out {si['out_rms_db']} dB RMS; stops")

            # 3. Bypass on and off.
            page.click('[data-lact="bypass"][data-route="output"]')
            wait(page, 400)
            assert api(port, "live.status")["routes"]["output"]["bypass"] is True
            page.click('[data-lact="bypass"][data-route="output"]')
            wait(page, 400)
            assert api(port, "live.status")["routes"]["output"]["bypass"] is False
            ok("bypass toggles")

            # 4. Pick a built-in and change a parameter with the keyboard: Flux makes the person's own copy.
            page.click(".seg[data-lseg] button:nth-child(2)")
            wait(page, 300)
            builtin = ev("FD.D.lpresets.find(p => p.builtin && p.kind === 'output' && p.modules >= 2).id")
            page.click(f'.fxc[data-id="{builtin}"]')
            wait(page, 600)
            assert ev("FD.D.lpreset.id") == builtin
            assert api(port, "live.status")["routes"]["output"]["info"]["preset"] == builtin
            assert ev("!document.querySelector('[data-lvedit]') && !document.querySelector('[data-module]')"), "editing controls stay in the editor"
            page.click('[data-lact="edit"]')
            page.wait_for_selector('[data-lvedit] [data-module]')
            wait(page, 700)
            assert ev("document.activeElement && document.activeElement.dataset.lact") == "edclose", "focus moves into the editor"
            bind = ev("document.querySelector('[data-module] .sl[data-bind^=\"lv:mp\"]').dataset.bind")
            _, _, mid, key = bind.split(":")
            before = ev(f"(() => {{ const p = FD.D.lpreset, m = p.lanes.flatMap(l => l.modules).concat(p.post).find(x => x.id === '{mid}'); return m.params['{key}'] ?? null; }})()")
            page.focus(f'[data-bind="{bind}"] .track')
            for _ in range(6):
                page.keyboard.press("ArrowRight")
            wait(page, 1500)
            mine = ev("FD.D.ui.live.sel")
            assert mine != builtin and mine.startswith("user-"), mine
            got = api(port, "live.preset.get", {"id": mine})
            m = [x for x in [y for l in got["lanes"] for y in l["modules"]] + got["post"] if x["id"] == mid][0]
            orig = api(port, "live.preset.get", {"id": builtin})
            mo = [x for x in [y for l in orig["lanes"] for y in l["modules"]] + orig["post"] if x["id"] == mid][0]
            assert m["params"][key] != mo["params"].get(key, before), (m["params"], mo["params"])
            assert api(port, "live.status")["routes"]["output"]["info"]["preset"] == mine
            ok(f"editing built-in {builtin} made {mine}; {key} {mo['params'].get(key)} -> {m['params'][key]}; the live route follows")

            # 5. Build: add a module, a modulator with steps aimed at it, a macro with a target; mute a lane if any.
            n0 = len(got["post"])
            page.select_option('[data-lchange="madd"][data-lane="post"]', "phaser")
            wait(page, 500)
            page.click('[data-lact="moadd"]')
            wait(page, 500)
            page.select_option('[data-lchange="moshape"]', "steps")
            wait(page, 500)
            opt = ev("[...document.querySelector('[data-lchange=\"mtadd\"]').options].find(o => /Phaser/.test(o.textContent) && !/Mix/.test(o.textContent)).value")
            page.select_option('[data-lchange="mtadd"]', opt)
            wait(page, 500)
            page.focus('[data-lstep][data-i="2"]')
            page.keyboard.press("End")
            wait(page, 500)
            page.click('[data-lact="maadd"]')
            wait(page, 500)
            sel = page.locator('[data-lchange="matadd"]').last
            mopt = ev("(() => { const s = [...document.querySelectorAll('[data-lchange=\"matadd\"]')].pop(); return [...s.options].find(o => /Phaser/.test(o.textContent) && /Mix/.test(o.textContent)).value; })()")
            sel.select_option(mopt)
            wait(page, 500)
            got = api(port, "live.preset.get", {"id": mine})
            assert len(got["post"]) == n0 + 1 and got["post"][-1]["kind"] == "phaser"
            md = got["mods"][-1]
            assert md["shape"] == "steps" and md["steps"][2] == 1.0 and md["targets"] and md["targets"][0]["module"] == got["post"][-1]["id"], md
            mc = got["macros"][-1]
            assert mc["targets"] and mc["targets"][0]["param"] == "mix", mc
            ok(f"built: phaser added, steps modulator ({len(md['steps'])} steps) moves it, macro '{mc['name']}' moves its mix")

            # 5b. Every control in the editor has a name and a tooltip; a tooltip shows on focus. Escape hides it and,
            # with changes made, asks before discarding; Keep editing goes back.
            missing = ev("""(() => {
              const root = document.querySelector('[data-lvedit]');
              const ctl = [...root.querySelectorAll('button, select, input, [role=slider]')];
              const name = e => (e.getAttribute('aria-label') || e.getAttribute('aria-labelledby') && document.getElementById(e.getAttribute('aria-labelledby'))?.textContent || e.textContent || (e.labels && e.labels[0] && e.labels[0].textContent) || '').trim();
              return { n: ctl.length, noName: ctl.filter(e => !name(e)).map(e => e.outerHTML.slice(0, 90)), noTip: ctl.filter(e => !e.closest('[data-tip]')).map(e => e.outerHTML.slice(0, 90)) };
            })()""")
            assert not missing["noName"], missing["noName"][:5]
            assert not missing["noTip"], missing["noTip"][:5]
            page.focus('[data-lact="lim"]')
            page.keyboard.press("Shift+Tab")
            page.keyboard.press("Tab")
            t = None
            for _ in range(20):
                wait(page, 100)
                t = ev("document.getElementById('tipbox').classList.contains('on') && document.getElementById('tipbox').textContent")
                if t:
                    break
            assert t and len(t) > 10, t
            page.keyboard.press("Escape")
            wait(page, 50)
            assert not ev("document.getElementById('tipbox').classList.contains('on') && document.getElementById('tipbox').textContent") == t, "Escape hides the tooltip"
            page.wait_for_selector('[aria-label="Discard changes?"]')
            page.click('[aria-label="Discard changes?"] .foot [data-lact="edkeep"]')
            page.wait_for_selector('[aria-label="Discard changes?"]', state="detached")
            assert ev("!!document.querySelector('[data-lvedit]')"), "Keep editing returns to the editor"
            page.set_viewport_size({"width": 1440, "height": 900})
            wait(page, 700)
            page.screenshot(path=str(OUT / "live-editor-1440.png"))
            page.set_viewport_size({"width": 390, "height": 844})
            wait(page, 500)
            over = ev("(() => { const d = document.querySelector('[data-lvedit] .dlg'); return d.scrollWidth - d.clientWidth; })()")
            assert over <= 1, ("editor sideways scroll at 390", over)
            assert ev("(() => { const f = document.querySelector('[data-lvedit] .dlg > .foot').getBoundingClientRect(), d = document.querySelector('[data-lvedit] .dlg').getBoundingClientRect(); return f.bottom <= d.bottom + 1 && f.top >= d.top; })()"), "Save and Discard stay in view"
            page.screenshot(path=str(OUT / "live-editor-390.png"))
            page.set_viewport_size({"width": 1440, "height": 900})
            wait(page, 300)
            ok(f"editor: {missing['n']} controls named and tipped; tooltip on focus: \"{t[:40]}...\"")

            # 5c. Save keeps the changes and closes the editor.
            page.click('[data-lact="edsave"]')
            page.wait_for_selector('[data-lvedit]', state="detached")
            saved = api(port, "live.preset.get", {"id": mine})
            assert saved["post"] == got["post"] and saved["mods"] == got["mods"]
            ok("Save closes the editor and keeps the preset")

            # 5d. Changes, then Escape: Flux asks first. Escape again keeps editing; Discard puts the preset back.
            page.click('[data-lact="edit"]')
            page.wait_for_selector('[data-lvedit] [data-module]')
            page.select_option('[data-lchange="madd"][data-lane="post"]', "chorus")
            wait(page, 600)
            assert len(api(port, "live.preset.get", {"id": mine})["post"]) == len(saved["post"]) + 1, "edits play while editing"
            page.keyboard.press("Escape")
            page.wait_for_selector('[aria-label="Discard changes?"]')
            wait(page, 300)
            assert ev("document.activeElement && document.activeElement.dataset.lact") == "eddiscard"
            page.screenshot(path=str(OUT / "live-editor-discard-1440.png"))
            page.keyboard.press("Escape")
            page.wait_for_selector('[aria-label="Discard changes?"]', state="detached")
            assert ev("!!document.querySelector('[data-lvedit]')"), "Escape on the question keeps editing"
            page.click('[data-lvedit] .foot [data-lact="edclose"]')
            page.wait_for_selector('[aria-label="Discard changes?"]')
            page.click('[data-lact="eddiscard"]')
            page.wait_for_selector('[data-lvedit]', state="detached")
            wait(page, 400)
            back = api(port, "live.preset.get", {"id": mine})
            assert back["post"] == saved["post"] and back["mods"] == saved["mods"] and back["macros"] == saved["macros"], "discard restores"
            ok("Escape asks before discarding; Escape again keeps editing; Discard restores the saved preset")

            # 5e. No changes: closing doesn't ask.
            page.click('[data-lact="edit"]')
            page.wait_for_selector('[data-lvedit]')
            page.keyboard.press("Escape")
            page.wait_for_selector('[data-lvedit]', state="detached")
            ok("closing an unchanged editor doesn't ask")

            # 5f. Discarding edits to a built-in deletes the copy Flux made, and the route goes back to the built-in.
            other = ev(f"FD.D.lpresets.find(p => p.builtin && p.kind === 'output' && p.id !== '{builtin}' && p.modules >= 1).id")
            n_user = len([p for p in api(port, "live.presets") if not p["builtin"]])
            page.click(f'.fxc[data-id="{other}"]')
            wait(page, 600)
            page.click('[data-lact="edit"]')
            page.wait_for_selector('[data-lvedit] [data-module]')
            b2 = ev("document.querySelector('[data-lvedit] [data-module] .sl[data-bind^=\"lv:mp\"]').dataset.bind")
            page.focus(f'[data-bind="{b2}"] .track')
            for _ in range(4):
                page.keyboard.press("ArrowRight")
            wait(page, 1500)
            assert len([p for p in api(port, "live.presets") if not p["builtin"]]) == n_user + 1
            page.keyboard.press("Escape")
            page.wait_for_selector('[aria-label="Discard changes?"]')
            page.click('[data-lact="eddiscard"]')
            page.wait_for_selector('[data-lvedit]', state="detached")
            wait(page, 500)
            assert len([p for p in api(port, "live.presets") if not p["builtin"]]) == n_user, "the copy is deleted"
            assert api(port, "live.status")["routes"]["output"]["info"]["preset"] == other
            assert ev("FD.D.ui.live.sel") == other
            ok(f"discarding edits to built-in {other} deletes the copy; the route plays {other} again")

            # Back to the preset built above for the rest of the test.
            page.click(f'.fxc[data-id="{mine}"]')
            wait(page, 600)

            # 6. Render a preview file with before and after levels.
            page.click('[data-lact="render"]')
            for _ in range(100):
                if ev("!!FD.D.ui.live.render"):
                    break
                wait(page, 200)
            r = ev("FD.D.ui.live.render")
            assert Path(r["path"]).exists() and r["out_rms_db"] > -60, r
            ok(f"preview render: {r['seconds']:.1f} s, out {r['out_rms_db']} dB RMS")

            # 7. Export and import keep everything.
            exp = api(port, "live.preset.export", {"id": mine})
            imp = api(port, "live.preset.import", {"json": exp["json"]})
            imp = imp.get("preset", imp)
            for k in ["lanes", "post", "mods", "macros", "tempo", "width"]:
                assert imp[k] == got[k], k
            ok(f"export/import round trip: {imp['id']}")

            # 8. Every control on the Live page has a name and sits inside a tooltip.
            missing = ev("""(() => {
              const root = document.querySelector('[data-livepage]');
              const ctl = [...root.querySelectorAll('button, select, input, [role=slider]')];
              const name = e => (e.getAttribute('aria-label') || e.getAttribute('aria-labelledby') && document.getElementById(e.getAttribute('aria-labelledby'))?.textContent || e.textContent || (e.labels && e.labels[0] && e.labels[0].textContent) || '').trim();
              const noName = ctl.filter(e => !name(e)).map(e => e.outerHTML.slice(0, 90));
              const noTip = ctl.filter(e => !e.closest('[data-tip]')).map(e => e.outerHTML.slice(0, 90));
              return { n: ctl.length, noName, noTip };
            })()""")
            assert not missing["noName"], missing["noName"][:5]
            assert not missing["noTip"], missing["noTip"][:5]
            ok(f"{missing['n']} controls: all named, all with tooltips")

            # 10. Fits at desktop, tablet and phone widths.
            for w, h in [(1440, 900), (820, 1180), (390, 844)]:
                page.set_viewport_size({"width": w, "height": h})
                wait(page, 400)
                over = ev("(() => { const m = document.querySelector('#m'); return Math.max(document.documentElement.scrollWidth - innerWidth, m.scrollWidth - m.clientWidth); })()")
                assert over <= 1, (w, over)
                page.screenshot(path=str(OUT / f"live-{w}.png"))
            themes = ev("Flux.THEMES.map(t => t[0])")
            fonts = ev("Flux.FONTS.map(f => f[0])")
            for i, th in enumerate(themes):
                ev(f"document.documentElement.dataset.flux = '{th}'; document.documentElement.dataset.font = '{fonts[i % len(fonts)]}'")
                wait(page, 120)
                over = ev("(() => { const m = document.querySelector('#m'); return Math.max(document.documentElement.scrollWidth - innerWidth, m.scrollWidth - m.clientWidth); })()")
                assert over <= 1, (th, over)
            page.screenshot(path=str(OUT / f"live-390-{themes[-1]}.png"))
            ev("document.documentElement.dataset.flux = 'cassette'; document.documentElement.dataset.font = 'bricolage'")
            ok(f"phone width fits in all {len(themes)} themes and {len(fonts)} fonts")
            page.set_viewport_size({"width": 1440, "height": 900})
            ok("no sideways scrolling at 1440, 820 and 390 px (screenshots in target/e2e)")

            # 11. A playing preset can't be deleted; after stopping, it can.
            ev("window.confirm = () => true")
            page.click('[data-lact="del"]')
            wait(page, 600)
            assert any(p["id"] == mine for p in api(port, "live.presets"))
            assert "playing" in (ev("FD.D.ui.live.err.ed") or ""), ev("FD.D.ui.live.err")
            page.click('[data-lact="stop"][data-route="output"]')
            wait(page, 500)
            assert api(port, "live.status")["routes"]["output"] is None
            page.click('[data-lact="del"]')
            wait(page, 600)
            assert all(p["id"] != mine for p in api(port, "live.presets")), ev("FD.D.ui.live.err")
            ok("a playing preset is protected; stop, then delete")

            # 12a. Voice cloning: a voice is only made with consent, through real file picks; scripts with sounds parse.
            def tone(path, secs):
                import math, random, struct, wave
                rnd = random.Random(3)
                with wave.open(str(path), "wb") as w:
                    w.setnchannels(1)
                    w.setsampwidth(2)
                    w.setframerate(24000)
                    w.writeframes(b"".join(struct.pack("<h", int(8000 * math.sin(i * 0.05) + rnd.randint(-300, 300))) for i in range(int(24000 * secs))))
                return str(path)
            ref = tone(data / "ref.wav", 8)
            con = tone(data / "consent.wav", 4)
            page.goto(f"http://127.0.0.1:{port}/#clone")
            page.wait_for_selector('[data-cact="create"]')
            assert page.is_disabled('[data-cact="create"]')
            page.fill('[data-cin="name"]', "Robin")
            wait(page, 600)
            for k, f in [("consent", con), ("reference", ref)]:
                with page.expect_file_chooser() as fc:
                    page.click(f'[data-cact="pick"][data-k="{k}"]')
                fc.value.set_files(f)
                wait(page, 600)
            assert page.is_disabled('[data-cact="create"]'), "consent box not ticked yet"
            page.click('[data-cact="agree"]')
            wait(page, 300)
            page.click('[data-cact="create"]')
            wait(page, 1200)
            profs = api(port, "clone.profiles")
            assert len(profs) == 1 and profs[0]["name"] == "Robin" and profs[0]["owner"] == "me", profs
            assert (data / "data" / "Voices" / profs[0]["id"] / "consent.wav").exists()
            page.fill('[data-cin="script"]', "Okay [laugh] hmm, (sighs) fine.")
            page.click('[data-cact="check"]')
            wait(page, 500)
            pills = ev("[...document.querySelectorAll('[data-cparts] .pill')].map(p => p.textContent)")
            assert pills == ["laugh", "sigh"], pills
            # Hesitation and pause placeholders, typed and inserted from the chips.
            page.fill('[data-cin="script"]', "So [laugh] [pause 1.5s] (um) yes")
            page.click('[data-cact="tag"][data-t="hmm"]')
            page.click('[data-cact="check"]')
            wait(page, 500)
            script_now = page.input_value('[data-cin="script"]')
            assert "[hmm]" in script_now, script_now
            pills = ev("[...document.querySelectorAll('[data-cparts] .pill')].map(p => p.textContent)")
            assert pills.count("hmm") == 1 and "um" in pills and "laugh" in pills and "1.5 s pause" in pills, pills
            summary = page.inner_text("[data-csum]")
            assert "1 sound" in summary and "2 hesitations" in summary and "1.5 s of pauses" in summary, summary
            for kind in ["Sounds", "Hesitations", "Pauses"]:
                assert page.locator(f"text={kind}").count() >= 1, kind
            page.fill('[data-cin="script"]', "hello [yodel]")
            page.click('[data-cact="check"]')
            wait(page, 500)
            assert "yodel" in page.inner_text(".callout.warn")
            # With no model downloaded, Speak is off and the page says what to download.
            assert page.is_disabled('[data-cact="speak"]')
            msg = page.inner_text('#stage .callout:has-text("downloaded"), #stage .callout:has-text("memory"), #stage .callout:has-text("Intel")')
            assert "Chatterbox" in msg or "memory" in msg or "Intel" in msg, msg
            ev("window.confirm = () => true")
            page.click('[data-cact="del"]')
            wait(page, 800)
            assert api(port, "clone.profiles") == []
            ok(f"voice made only with consent and both recordings; script sounds {pills}; speak says: {msg[:70]}...")

            # 12. Every screen of the app renders without a page error.
            ids = ev("Flux.S.map(s => s.id)")
            bad, untipped, total_ctl = [], [], 0
            for sid in ids:
                errors.clear()
                ev(f"Flux.render('{sid}')")
                wait(page, 120)
                if errors:
                    bad.append((sid, errors[:2]))
                bare = ev("[...document.querySelectorAll('#stage button, #stage a[href], #stage select, #stage input, #stage textarea, #stage [role=slider]')].filter(e => !e.closest('[data-tip]')).map(e => e.outerHTML.slice(0, 80))")
                untipped.extend((sid, x) for x in bare)
                total_ctl += ev("document.querySelectorAll('#stage button, #stage a[href], #stage select, #stage input, #stage textarea, #stage [role=slider]').length")
            ev("Flux.render('live')")
            assert not bad, bad
            ok(f"all {len(ids)} screens render without errors")
            assert not untipped, untipped[:8]
            ok(f"tooltips: all {total_ctl} controls on all {len(ids)} screens have one")
            b.close()
    finally:
        srv.terminate()
        try:
            srv.wait(5)
        except Exception:
            srv.kill()
        shutil.rmtree(data, ignore_errors=True)
    print(f"\n{len(passed)} passed")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print("FAILED:", repr(e))
        sys.exit(1)
