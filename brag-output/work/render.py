# Renders video.html frame by frame (each frame a pure function of time) with headless Chromium.
# python render.py stills 0.5 2.2 ...   -> stills/<t>.png
# python render.py all                  -> frames/%04d.png at 30 fps for 22 s
import sys, time
from pathlib import Path
from playwright.sync_api import sync_playwright

HERE = Path(__file__).parent
FPS, DUR = 30, 22.0
methods = (HERE / 'methods.txt').read_text().strip()

with sync_playwright() as p:
    b = p.chromium.launch(args=['--allow-file-access-from-files'])
    page = b.new_page(viewport={'width': 1920, 'height': 1080})
    page.add_init_script(f"window.METHODS = {methods!r};")
    page.goto((HERE / 'video.html').as_uri())
    page.evaluate('window.ready')
    page.evaluate('document.fonts.ready')
    page.evaluate('render(0)')
    time.sleep(0.5)
    if sys.argv[1] == 'stills':
        out = HERE / 'stills'; out.mkdir(exist_ok=True)
        for s in sys.argv[2:]:
            page.evaluate(f'render({float(s)})')
            page.screenshot(path=str(out / f'{s}.png'))
    else:
        out = HERE / 'frames'; out.mkdir(exist_ok=True)
        n = int(FPS * DUR)
        t0 = time.time()
        for f in range(n):
            page.evaluate(f'render({f / FPS})')
            page.screenshot(path=str(out / f'{f:04d}.png'))
            if f % 60 == 0: print(f, f'{time.time() - t0:.0f}s', flush=True)
    b.close()
