"""Playwright headless browser test for Spike D: 2-Hour WAV Scrubbing with OPFS & Peak Pyramid."""

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


def run_spike_d_browser_test():
    """Runs Spike D test in headless Chromium."""
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

                # Navigate to Spike D runner page
                url = f"http://127.0.0.1:{port}/tests/spikes/spike_d/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.spikeDResult !== undefined", timeout=30000)
                result = page.evaluate("() => window.spikeDResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_spike_d_opfs_streaming_and_peaks():
    """Verify Spike D 2-hour WAV file streaming in OPFS and peak pyramid zoom."""
    result = run_spike_d_browser_test()
    print("\nSpike D Measured Results:", result)

    assert result.get("success") is True, f"Spike D failed: {result}"

    # File size & metadata
    file_info = result["file"]
    expected_size = 120 * 1024 * 1024
    assert file_info["sizeBytes"] == expected_size, f"Expected {expected_size} bytes, got {file_info['sizeBytes']}"
    assert file_info["durationSeconds"] == 7200

    # Scrubbing performance
    scrub = result["scrubbing"]
    assert scrub["avgSeekMs"] < 25.0, f"Average seek latency {scrub['avgSeekMs']} ms exceeded 25 ms target"
    assert scrub["numSeeks"] == 50

    # Peak pyramid zoom
    pyr = result["pyramid"]
    assert pyr["level2Points"] > 10000
    assert pyr["q1_overviewMs"] < 15.0

    # Memory budget (PLAN.md §1.1: memory < 300 MB)
    mem = result["memory"]
    if mem.get("usedHeapMb") is not None:
        assert mem["usedHeapMb"] < 300.0, f"Heap memory {mem['usedHeapMb']} MB exceeded 300 MB limit"


if __name__ == "__main__":
    res = run_spike_d_browser_test()
    print("Result:", res)
