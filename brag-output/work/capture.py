# Captures the real Flux UI (flux-mcp serve + the desktop UI, real engine) at 1920x1080 for the launch video.
import math, os, random, struct, subprocess, sys, tempfile, time, urllib.request, wave
from pathlib import Path
sys.path.insert(0, 'D:/flux/tests/e2e')
import test_live as T
from playwright.sync_api import sync_playwright

OUT = Path(__file__).parent / 'shots'
OUT.mkdir(exist_ok=True)
data = Path(tempfile.mkdtemp())

def song(path, secs=24, sr=48000):
    # A synthetic groove: kick, hat, chord pad and a little melody, so the waveform looks like music.
    rnd = random.Random(7)
    n = int(secs * sr)
    bpm = 112; beat = 60 / bpm
    notes = [57, 60, 64, 67, 69, 67, 64, 62]
    frames = bytearray()
    for i in range(n):
        t = i / sr
        b = t / beat
        ph = b % 1
        kick = math.sin(2 * math.pi * 55 * t * (1 + 2 * math.exp(-ph * 30))) * math.exp(-ph * 9) * 0.8
        hat = (rnd.random() * 2 - 1) * math.exp(-((b * 2) % 1) * 40) * 0.15
        bar = int(b // 4) % 4
        root = [45, 41, 48, 43][bar]
        pad = sum(math.sin(2 * math.pi * 440 * 2 ** ((root + k - 69) / 12) * t) for k in (12, 16, 19)) * 0.08
        m = notes[int(b * 2) % 8]
        mel = math.sin(2 * math.pi * 440 * 2 ** ((m + 12 - 69) / 12) * t) * math.exp(-((b * 2) % 1) * 5) * 0.18
        swell = 0.6 + 0.4 * math.sin(2 * math.pi * t / 12)
        l = (kick + hat + pad * swell + mel * 0.8)
        r = (kick + hat * 0.7 + pad * swell + mel * 1.0)
        frames += struct.pack('<hh', int(max(-1, min(1, l * 0.7)) * 32767), int(max(-1, min(1, r * 0.7)) * 32767))
    with wave.open(str(path), 'wb') as w:
        w.setnchannels(2); w.setsampwidth(2); w.setframerate(sr); w.writeframes(bytes(frames))

port = T.free_port()
env = dict(os.environ, FLUX_DATA_DIR=str(data / 'data'), FLUX_EXPORT_DIR=str(data / 'export'))
srv = subprocess.Popen([str(T.EXE), 'serve', '--port', str(port), '--ui', str(T.ROOT / 'apps/desktop/ui'), '--no-token'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(100):
        try:
            urllib.request.urlopen(f'http://127.0.0.1:{port}/api/health', timeout=1); break
        except Exception:
            time.sleep(0.1)
    T.api(port, 'settings.set', {'values': {'onboarded': True, 'headphones_checked': True}})
    wav = data / 'Night Drive.wav'
    song(wav)
    T.api(port, 'media.import', {'path': str(wav), 'wait': True})
    with sync_playwright() as p:
        b = p.chromium.launch()
        page = b.new_page(viewport={'width': 1920, 'height': 1080})
        page.add_init_script("document.addEventListener('DOMContentLoaded',()=>{const s=document.createElement('style');s.textContent='.tipbox{display:none!important}';document.head.appendChild(s)})")
        def shot(route, name, before=None, scroll=None):
            page.goto(f'http://127.0.0.1:{port}/#{route}')
            time.sleep(1.5)
            if before: before()
            if scroll: page.locator(scroll).first.scroll_into_view_if_needed(); page.mouse.wheel(0, -120); time.sleep(0.4)
            page.mouse.move(1919, 1079)
            time.sleep(0.6)
            page.screenshot(path=str(OUT / f'{name}.png'))
            print('shot', name)
        shot('home', 'home')
        shot('edit', 'edit')
        shot('stage', 'stage')
        shot('creative', 'creative')
        shot('live', 'live')
        def script():
            page.fill('[data-cin="script"]', "[hmm] So I opened the box [laugh] and, um, [pause] it was empty. [long pause] [sigh] Of course it was.")
            page.click('[data-cact="check"]'); time.sleep(0.6)
        shot('clone', 'clone', script, '[data-cin="script"]')
        shot('prefs-agents', 'agents')
        shot('master', 'master')
        b.close()
finally:
    srv.kill()
