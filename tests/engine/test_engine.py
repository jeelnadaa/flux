"""Playwright headless browser test for Engine Core: Transport, Loop, Scrub, and Dither."""

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


def run_engine_browser_test():
    """Runs Engine Core test in headless Chromium."""
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

                # Navigate to Engine runner page
                url = f"http://127.0.0.1:{port}/tests/engine/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.engineTestResult !== undefined", timeout=25000)
                result = page.evaluate("() => window.engineTestResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_engine_core_browser():
    """Verify transport play/seek, seamless loop, variable speed scrub, and dither in browser."""
    result = run_engine_browser_test()
    print("\nEngine Core Measured Results:", result)

    assert result.get("success") is True, f"Engine test failed: {result}"
    assert result.get("transportTest") is True
    assert result.get("loopTest") is True
    assert result.get("scrubTest") is True
    assert result.get("ditherTest") is True
    assert result.get("wasmEngineTest") is True


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
