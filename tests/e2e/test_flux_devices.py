"""End-to-end test of the Flux speaker and microphone on the Live page, in a real browser.

The sound hardware and the sound server are stood in for: FLUX_FAKE_DEVICES gives the device list and pactl_stub.py
keeps the sound server's state, so the page is driven for a computer with nothing, one with a virtual cable, and a
Linux one where Flux makes the devices. Real PipeWire, PulseAudio and cables are in the human and hardware queue.

    python tests/e2e/test_flux_devices.py     (needs `cargo build -p flux-mcp --release` first)
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

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target" / "release" / ("flux-mcp.exe" if os.name == "nt" else "flux-mcp")
OUT = ROOT / "target" / "e2e"
STUB = ROOT / "crates" / "flux-core" / "tests" / "voice_stub" / "pactl_stub.py"


def dev(i, name, default=False):
    return {"id": i, "name": name, "default": default, "channels": 2, "sample_rate": 48000}


def write(path, outs, ins):
    path.write_text(json.dumps({"host": "Test", "hosts": [], "outputs": outs, "inputs": ins}))


class Server:
    def __init__(self, tmp, name, extra):
        self.port = free_port()
        env = dict(os.environ, FLUX_DATA_DIR=str(tmp / name), FLUX_EXPORT_DIR=str(tmp / "export"), FLUX_NULL_AUDIO="1", **extra)
        self.p = subprocess.Popen([str(EXE), "serve", "--port", str(self.port), "--ui", str(ROOT / "apps/desktop/ui"), "--no-token"], env=env,
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(100):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{self.port}/api/health", timeout=1)
                break
            except Exception:
                time.sleep(0.1)
        api(self.port, "settings.set", {"values": {"onboarded": True, "headphones_checked": True}})

    def url(self):
        return f"http://127.0.0.1:{self.port}/#live"


def main():
    assert EXE.exists(), f"build flux-mcp first: {EXE}"
    OUT.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="flux-e2e-devices-"))
    fake = tmp / "devices.json"
    passed = []

    def ok(name):
        passed.append(name)
        print("ok -", name)

    with sync_playwright() as pw:
        b = pw.chromium.launch()
        page = b.new_page(viewport={"width": 1440, "height": 900})
        errors = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.on("dialog", lambda d: d.accept())
        panel = '[data-lv="flux-devices"]'
        src = '[data-route="output"] select[data-k="src"]'
        out = '[data-route="output"] select[data-k="output"]'
        micout = '[data-route="input"] select[data-k="output"]'
        real = dev("spk", "Speakers", True)
        mic = dev("m", "Microphone", True)

        # 1. No virtual device, and Flux can't make one: says what to do, offers no Flux choice.
        write(fake, [real, dev("hp", "Headphones")], [mic])
        s = Server(tmp, "none", {"FLUX_FAKE_DEVICES": str(fake)})
        try:
            page.goto(s.url())
            page.wait_for_selector(panel)
            text = page.inner_text(panel)
            assert "No virtual audio cable is installed" in text and "Not installed" in text, text
            assert "original sound keeps playing" in page.inner_text('[data-lv="missing-output"]')
            assert "call apps can't use this" in page.inner_text('[data-lv="missing-input"]')
            assert "Flux speaker" not in " ".join(page.locator(f"{src} option").all_inner_texts())
            assert page.locator(f'{panel} [data-lact="vset"]').count() == 0
            page.screenshot(path=str(OUT / "flux-devices-none-1440.png"), full_page=True)
            page.set_viewport_size({"width": 390, "height": 844})
            page.wait_for_timeout(300)
            assert page.evaluate("(() => { const m = document.querySelector('#m'); return Math.max(document.documentElement.scrollWidth - innerWidth, m.scrollWidth - m.clientWidth); })()") <= 1
            page.set_viewport_size({"width": 1440, "height": 900})
            ok("nothing virtual: the panel and both cards say it isn't installed, and offer no Flux choice")

            # The guide: says it isn't installed, gives this computer's steps, opens the maker's page, checks again.
            page.click(f'{panel} [data-lact="vguide"]')
            page.wait_for_selector('[data-lv="guide-steps"]')
            page.wait_for_timeout(1500)
            dlg = page.inner_text('[role="dialog"]')
            foot = page.evaluate("(() => { const f = document.querySelector('[role=dialog] .foot').getBoundingClientRect(), d = document.querySelector('[role=dialog] .dlg').getBoundingClientRect(); return f.bottom <= d.bottom + 1 && f.top >= d.top; })()")
            assert foot, "Check again stays on screen in a tall guide"
            here = api(s.port, "live.virtual")["platform"]
            assert "Not installed." in dlg or here == "linux", dlg
            assert "signed" not in dlg.lower() and "another company" not in dlg, "the guide only says how to use a cable"
            assert "own_driver" not in api(s.port, "live.virtual")
            page.screenshot(path=str(OUT / "virtual-cable-guide-1440.png"))
            for os_key, want in [("windows", "Run as administrator"), ("macos", "brew install blackhole-2ch"), ("linux", "sudo apt install pulseaudio-utils")]:
                page.click(f'[data-lact="vos"][data-os="{os_key}"]')
                page.wait_for_timeout(120)
                assert want in page.inner_text('[data-lv="guide-steps"]'), os_key
                assert page.get_attribute(f'[data-lact="vos"][data-os="{os_key}"]', "aria-pressed") == "true"
                for w, h in [(820, 1100), (390, 844)]:
                    page.set_viewport_size({"width": w, "height": h})
                    page.wait_for_timeout(120)
                    assert page.evaluate("(() => { const d = document.querySelector('[role=dialog] .dlg'); return Math.max(d.scrollWidth - d.clientWidth, document.documentElement.scrollWidth - innerWidth); })()") <= 1, (os_key, w)
                    if w == 390:
                        page.screenshot(path=str(OUT / f"virtual-cable-guide-{os_key}-390.png"))
                page.set_viewport_size({"width": 1440, "height": 900})
            bare = page.evaluate("[...document.querySelectorAll('[role=dialog] button, [role=dialog] select')].filter(e => !e.closest('[data-tip]')).map(e => e.textContent)")
            assert not bare, bare
            page.click(f'[data-lact="vos"][data-os="windows"]')
            page.wait_for_timeout(120)
            assert page.get_attribute('[data-act="open-url"]', "data-url") == "https://vb-audio.com/Cable/"
            page.click('[data-lact="vcheck"]')
            page.wait_for_selector('text=Still not found')
            # A cable shows up while the guide is open (after installing): Check again finds it and Live offers it.
            write(fake, [real, dev("ca", "CABLE Input (VB-Audio Virtual Cable)")], [mic, dev("cb", "CABLE Output (VB-Audio Virtual Cable)")])
            page.click('[data-lact="vcheck"]')
            page.wait_for_selector('text=Found it.')
            page.click('[data-lact="vdone"]')
            page.wait_for_selector(f'{panel} >> text=Ready')
            assert page.input_value(src) == "flux|"
            assert page.locator('[data-lv="missing-output"]').count() == 0
            ok("guide: says not installed, steps for each OS at every width, opens the maker's page, Check again finds a new cable")
        finally:
            s.p.terminate()

        # 2. A virtual cable: it becomes the Flux speaker and microphone; only real outputs are places to listen.
        cab_o, cab_i = dev("ca", "CABLE Input (VB-Audio Virtual Cable)"), dev("cb", "CABLE Output (VB-Audio Virtual Cable)")
        write(fake, [real, cab_o], [mic, cab_i])
        s = Server(tmp, "cable", {"FLUX_FAKE_DEVICES": str(fake)})
        try:
            page.goto(s.url())
            page.wait_for_selector(panel)
            assert page.input_value(src) == "flux|", page.input_value(src)
            assert page.input_value(out) == "auto"
            assert page.input_value(micout) == "flux"
            assert "ca" not in page.eval_on_selector_all(f"{out} option", "o => o.map(x => x.value)"), "the cable is not a place to listen"
            assert "CABLE Input (VB-Audio Virtual Cable) as the output" in page.inner_text('[data-lv="pick-output"]')
            assert "CABLE Output (VB-Audio Virtual Cable) as the microphone" in page.inner_text('[data-lv="pick-input"]')
            assert "one at a time" in page.inner_text(panel)
            assert page.locator(f'{panel} [data-lact="vguide"] >> text=Add a second cable').count() == 1
            page.screenshot(path=str(OUT / "flux-devices-cable-1440.png"), full_page=True)
            # Leaving the Flux speaker picks a real output again; coming back is automatic again.
            page.select_option(src, "system|spk")
            page.wait_for_timeout(200)
            assert page.input_value(out) not in ("auto", ""), page.input_value(out)
            page.select_option(src, "flux|")
            page.wait_for_timeout(200)
            assert page.input_value(out) == "auto"
            ok("one cable: the Flux speaker and microphone by default, says which to pick, listens only on real outputs")
        finally:
            s.p.terminate()

        # 3. Linux: Add them makes the devices; the page then shows them, and Remove takes them away.
        st, log = tmp / "pactl.json", tmp / "pactl.log"
        write(fake, [dev("pulseaudio:alsa_output.real", "Built-in Audio", True)], [dev("pulseaudio:alsa_input.real", "Mic", True)])
        s = Server(tmp, "linux", {"FLUX_FAKE_DEVICES": str(fake), "FLUX_PACTL": f"{sys.executable} {STUB}", "FLUX_PACTL_STATE": str(st), "FLUX_PACTL_LOG": str(log)})
        try:
            page.goto(s.url())
            page.wait_for_selector(f'{panel} [data-lact="vset"][data-v="setup"]')
            assert "can add" in page.inner_text(panel)
            page.screenshot(path=str(OUT / "flux-devices-add-1440.png"), full_page=True)
            page.click(f'{panel} [data-lact="vset"]')
            for _ in range(100):
                if st.exists() and len(json.loads(st.read_text())["modules"]) == 3:
                    break
                time.sleep(0.05)
            assert len(json.loads(st.read_text())["modules"]) == 3
            write(fake, [dev("pulseaudio:alsa_output.real", "Built-in Audio", True), dev("pulseaudio:flux_speaker", "Flux speaker"), dev("pulseaudio:flux_mic_feed", "Flux microphone feed")],
                  [dev("pulseaudio:alsa_input.real", "Mic", True), dev("pulseaudio:flux_speaker.monitor", "Monitor of Flux speaker"), dev("pulseaudio:flux_mic", "Flux microphone")])
            page.goto("about:blank")
            page.goto(s.url())
            page.wait_for_selector(f'{panel} [data-lact="vset"][data-v="remove"]')
            assert "Ready" in page.inner_text(panel)
            assert "Flux speaker as the output" in page.inner_text('[data-lv="pick-output"]')
            page.screenshot(path=str(OUT / "flux-devices-ready-1440.png"), full_page=True)

            # With no sound hardware the start fails and says so; the system output is left alone.
            page.click('[data-lact="start"][data-route="output"]')
            page.wait_for_selector('[data-route="output"] .callout, [data-route="output"] [class*="cal"]')
            assert json.loads(st.read_text())["default"] == "alsa_output.real"
            assert not any("set-default-sink" in l for l in log.read_text().splitlines())

            for w, h in [(1440, 900), (820, 1100), (390, 844)]:
                page.set_viewport_size({"width": w, "height": h})
                page.wait_for_timeout(300)
                assert page.evaluate("(() => { const m = document.querySelector('#m'); return Math.max(document.documentElement.scrollWidth - innerWidth, m.scrollWidth - m.clientWidth); })()") <= 1, w
                if w != 1440:
                    page.locator(panel).scroll_into_view_if_needed()
                    page.screenshot(path=str(OUT / f"flux-devices-{w}.png"))
            page.set_viewport_size({"width": 1440, "height": 900})

            page.click(f'{panel} [data-lact="vset"]')
            for _ in range(100):
                if not json.loads(st.read_text())["modules"]:
                    break
                time.sleep(0.05)
            assert not json.loads(st.read_text())["modules"]
            ok("Linux: Add them makes the devices, a failed start leaves the system output alone, Remove takes them away, every width fits")
        finally:
            s.p.terminate()

        assert not errors, errors
        ok("no page errors")
        b.close()
    print(f"{len(passed)} checks passed")


if __name__ == "__main__":
    main()
