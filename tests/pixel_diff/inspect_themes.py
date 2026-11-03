import http.server
import socketserver
import threading
from pathlib import Path
from playwright.sync_api import sync_playwright

REPO_ROOT = Path('.').resolve()

class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args): pass

def run():
    with socketserver.TCPServer(('127.0.0.1', 0), QuietHandler) as httpd:
        port = httpd.server_address[1]
        threading.Thread(target=httpd.serve_forever, daemon=True).start()

        with sync_playwright() as p:
            browser = p.chromium.launch(headless=True)
            p1 = browser.new_page(viewport={'width': 1440, 'height': 900})
            p1.goto(f'http://127.0.0.1:{port}/design/app.html#prefs-themes')
            p1.wait_for_selector('#stage .pg')
            proto_html = p1.evaluate("() => document.querySelector('#stage .pg').outerHTML")
            
            p2 = browser.new_page(viewport={'width': 1440, 'height': 900})
            p2.goto(f'http://127.0.0.1:{port}/apps/web/index.html#prefs-themes')
            p2.wait_for_selector('#stage .pg')
            prod_html = p2.evaluate("() => document.querySelector('#stage .pg').outerHTML")

            print("=== PROTO HTML FIRST 500 CHARS ===")
            print(proto_html[:500])
            print("=== PROD HTML FIRST 500 CHARS ===")
            print(prod_html[:500])
            browser.close()

if __name__ == '__main__':
    run()
