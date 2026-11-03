import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT))

import http.server
import socketserver
import threading
from playwright.sync_api import sync_playwright
from tests.pixel_diff.comparator import compare_images

class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(REPO_ROOT), **kwargs)
    def log_message(self, format, *args):
        pass

def run():
    artifacts_dir = REPO_ROOT / 'tests' / 'pixel_diff' / 'artifacts'
    artifacts_dir.mkdir(parents=True, exist_ok=True)

    with socketserver.TCPServer(('127.0.0.1', 0), QuietHandler) as httpd:
        port = httpd.server_address[1]
        server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        server_thread.start()

        with sync_playwright() as p:
            browser = p.chromium.launch(
                headless=True,
                args=['--no-sandbox', '--disable-gpu', '--font-render-hinting=none'],
            )

            screens = ['kit', 'prefs-themes', 'prefs-fonts']

            for screen in screens:
                print(f"=== Screen: {screen} ===")
                for width, height, name in [(1440, 900, 'desktop'), (820, 1024, 'tablet'), (390, 844, 'phone')]:
                    # 1. Prototype
                    page_proto = browser.new_page(viewport={'width': width, 'height': height})
                    page_proto.goto(f'http://127.0.0.1:{port}/design/app.html#{screen}')
                    page_proto.wait_for_selector('#stage .pg')
                    page_proto.evaluate("""() => {
                        const rv = document.getElementById('rv');
                        if (rv) rv.remove();
                        const frame = document.getElementById('frame');
                        if (frame) frame.style.inset = '0';
                        const m = document.getElementById('m');
                        if (m) m.scrollTop = 0;
                    }""")
                    page_proto.wait_for_timeout(500)
                    proto_img = artifacts_dir / f'proto_{screen}_{name}_{width}.png'
                    page_proto.locator('#stage').screenshot(path=str(proto_img))
                    page_proto.close()

                    # 2. Production
                    page_prod = browser.new_page(viewport={'width': width, 'height': height})
                    page_prod.goto(f'http://127.0.0.1:{port}/apps/web/index.html#{screen}')
                    page_prod.wait_for_selector('#stage .pg')
                    page_prod.evaluate("""() => {
                        const m = document.getElementById('m');
                        if (m) m.scrollTop = 0;
                    }""")
                    page_prod.wait_for_timeout(500)
                    prod_img = artifacts_dir / f'prod_{screen}_{name}_{width}.png'
                    page_prod.locator('#stage').screenshot(path=str(prod_img))
                    page_prod.close()

                    diff_img = artifacts_dir / f'diff_{screen}_{name}_{width}.png'
                    res = compare_images(proto_img, prod_img, diff_img)
                    print(f"[{name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, mean delta: {res['mean_delta']:.2f}")

            browser.close()
            httpd.shutdown()

if __name__ == '__main__':
    run()
