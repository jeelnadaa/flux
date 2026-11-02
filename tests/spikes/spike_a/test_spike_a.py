"""Playwright headless browser smoke test for Spike A: AudioWorklet + WASM Biquad Chain."""

import http.server
import socketserver
import threading
from pathlib import Path
import pytest
from playwright.sync_api import sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent.parent.parent


class QuietSimpleHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(REPO_ROOT), **kwargs)

    def log_message(self, format, *args):
        pass  # Suppress request logging


def run_spike_a_browser_test():
    """Runs Spike A smoke test in headless Chromium."""
    # Find ephemeral port
    with socketserver.TCPServer(("127.0.0.1", 0), QuietSimpleHTTPRequestHandler) as httpd:
        port = httpd.server_address[1]
        server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        server_thread.start()

        try:
            with sync_playwright() as p:
                browser = p.chromium.launch(
                    headless=True,
                    args=["--no-sandbox", "--disable-gpu", "--autoplay-policy=no-user-gesture-required"],
                )
                page = browser.new_page()

                # Navigate to Spike A runner page
                url = f"http://127.0.0.1:{port}/tests/spikes/spike_a/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.spikeAResult !== undefined", timeout=20000)
                result = page.evaluate("() => window.spikeAResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_spike_a_audioworklet_wasm():
    """Verify AudioWorklet + WASM biquad chain smoke test."""
    result = run_spike_a_browser_test()
    print("\nSpike A Measured Results:", result)

    assert result.get("success") is True, f"Spike A failed: {result}"
    assert abs(result["measuredFreqHz"] - 1000.0) <= 5.0, f"Expected 1000 Hz, got {result['measuredFreqHz']}"
    assert result["nanCount"] == 0, f"Encountered {result['nanCount']} NaNs in rendered output"
    assert result["cpuPercent"] < 30.0, f"CPU usage {result['cpuPercent']}% exceeded 30% budget"


if __name__ == "__main__":
    res = run_spike_a_browser_test()
    print("Result:", res)
