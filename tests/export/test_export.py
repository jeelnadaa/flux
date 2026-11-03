"""Playwright headless browser test for Export, Presets, Normalization, and Converter."""

import http.server
import socketserver
import threading
from pathlib import Path
import pytest
from playwright.sync_api import sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent.parent


class QuietSimpleHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(REPO_ROOT), **kwargs)

    def log_message(self, format, *args):
        pass  # Suppress request logging


def run_export_browser_test():
    """Runs Export & Format Converter test in headless Chromium."""
    with socketserver.TCPServer(("127.0.0.1", 0), QuietSimpleHTTPRequestHandler) as httpd:
        port = httpd.server_address[1]
        server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        server_thread.start()

        try:
            with sync_playwright() as p:
                browser = p.chromium.launch(
                    headless=True,
                    args=[
                        "--no-sandbox",
                        "--disable-gpu",
                        "--autoplay-policy=no-user-gesture-required",
                    ],
                )
                page = browser.new_page()

                # Navigate to Export runner page
                url = f"http://127.0.0.1:{port}/tests/export/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.exportTestResult !== undefined", timeout=25000)
                result = page.evaluate("() => window.exportTestResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_export_browser():
    """Verify export presets, progress tracking, region slicing, cancellation, and converter."""
    result = run_export_browser_test()
    print("\nExport Browser Measured Results:", result)

    assert result.get("success") is True, f"Export browser test failed: {result}"
    assert result.get("presetsCount") >= 6
    assert result.get("wavBytesLength") > 44
    assert result.get("cancelledOk") is True
    assert result.get("convertedOk") is True
    assert result.get("progressCount") > 0
