import sys
import http.server
import socketserver
import threading
from pathlib import Path
from playwright.sync_api import sync_playwright

REPO_ROOT = Path('.').resolve()
sys.path.insert(0, str(REPO_ROOT))

class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        pass

def inspect():
    with socketserver.TCPServer(('127.0.0.1', 0), QuietHandler) as httpd:
        port = httpd.server_address[1]
        threading.Thread(target=httpd.serve_forever, daemon=True).start()

        with sync_playwright() as p:
            browser = p.chromium.launch(headless=True)
            # Themes Phone
            p1 = browser.new_page(viewport={'width': 390, 'height': 844})
            p1.goto(f'http://127.0.0.1:{port}/design/app.html#prefs-themes')
            p1.wait_for_selector('#stage .pg')
            p1.evaluate("""() => {
                const rv = document.getElementById('rv');
                if (rv) rv.remove();
                const frame = document.getElementById('frame');
                if (frame) frame.style.inset = '0';
            }""")
            p1.wait_for_timeout(300)
            proto_th = p1.evaluate("""() => ({
                scroll: document.getElementById('m').scrollTop,
                h1: document.querySelector('.h1').getBoundingClientRect(),
                cards: document.querySelector('.g3').getBoundingClientRect(),
                firstCard: document.querySelector('.g3 button, .g3 .fxc').getBoundingClientRect()
            })""")
            p1.close()

            p2 = browser.new_page(viewport={'width': 390, 'height': 844})
            p2.goto(f'http://127.0.0.1:{port}/apps/web/index.html#prefs-themes')
            p2.wait_for_selector('#stage .pg')
            p2.wait_for_timeout(300)
            prod_th = p2.evaluate("""() => ({
                scroll: document.getElementById('m').scrollTop,
                h1: document.querySelector('.h1').getBoundingClientRect(),
                cards: document.querySelector('.g3').getBoundingClientRect(),
                firstCard: document.querySelector('.g3 button, .g3 .fxc').getBoundingClientRect()
            })""")
            p2.close()

            print('THEMES PHONE PROTO:', proto_th)
            print('THEMES PHONE PROD:', prod_th)

            # Fonts Desktop
            p3 = browser.new_page(viewport={'width': 1440, 'height': 900})
            p3.goto(f'http://127.0.0.1:{port}/design/app.html#prefs-fonts')
            p3.wait_for_selector('#stage .pg')
            p3.evaluate("""() => {
                const rv = document.getElementById('rv');
                if (rv) rv.remove();
                const frame = document.getElementById('frame');
                if (frame) frame.style.inset = '0';
            }""")
            p3.wait_for_timeout(500)
            artifacts_dir = REPO_ROOT / 'tests' / 'pixel_diff' / 'artifacts'
            p3_img = artifacts_dir / 'proto_fonts_test_1440.png'
            p3.locator('#stage').screenshot(path=str(p3_img))
            p3.close()

            p4 = browser.new_page(viewport={'width': 1440, 'height': 900})
            p4.goto(f'http://127.0.0.1:{port}/apps/web/index.html#prefs-fonts')
            p4.wait_for_selector('#stage .pg')
            p4.wait_for_timeout(500)
            p4_img = artifacts_dir / 'prod_fonts_test_1440.png'
            p4.locator('#stage').screenshot(path=str(p4_img))
            p4.close()

            diff_img = artifacts_dir / 'diff_fonts_test_1440.png'
            from tests.pixel_diff.comparator import compare_images
            res = compare_images(p3_img, p4_img, diff_img)
            print('FRESH FONTS DESKTOP MATCH:', res['match_percentage'], res['mean_delta'])

            browser.close()

if __name__ == '__main__':
    inspect()
