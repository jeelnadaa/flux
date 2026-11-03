"""Playwright headless browser test for Analysis Foundations: Peak Pyramids, Loudness, and Stereo vectorscope."""

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


def run_analysis_browser_test():
    """Runs Analysis Foundations test in headless Chromium."""
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

                # Navigate to Analysis runner page
                url = f"http://127.0.0.1:{port}/tests/analysis/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.analysisTestResult !== undefined", timeout=25000)
                result = page.evaluate("() => window.analysisTestResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_analysis_foundations_browser():
    """Verify peak pyramids, loudness measurement, phase correlation, and goniometer in browser."""
    result = run_analysis_browser_test()
    print("\nAnalysis Browser Measured Results:", result)

    assert result.get("success") is True, f"Analysis browser test failed: {result}"
    assert result.get("pyramidBytes") > 32
    assert result.get("queriedBinsCount") > 0
    assert abs(result.get("phaseCorrelation") - 1.0) < 0.001
    assert result.get("goniometerPointsCount") > 0
