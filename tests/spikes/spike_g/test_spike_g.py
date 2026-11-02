"""Playwright headless browser test for Spike G: Storage Durability & Journal Crash Recovery."""

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


def run_spike_g_browser_test():
    """Runs Spike G test in headless Chromium."""
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

                # Navigate to Spike G runner page
                url = f"http://127.0.0.1:{port}/tests/spikes/spike_g/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.spikeGResult !== undefined", timeout=25000)
                result = page.evaluate("() => window.spikeGResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_spike_g_storage_durability_and_journal_recovery():
    """Verify Spike G storage quota, persistence API, and journal recovery from killed tab."""
    result = run_spike_g_browser_test()
    print("\nSpike G Measured Results:", result)

    assert result.get("success") is True, f"Spike G failed: {result}"

    # Durability & Quota
    dur = result["durability"]
    assert dur["persistSupported"] is True
    assert dur["quotaBytes"] > 0, f"Expected non-zero storage quota, got {dur['quotaBytes']}"

    # Journal crash recovery
    rec = result["journalRecovery"]
    assert rec["validCommandsCount"] == 5
    assert rec["recoveredCommandsCount"] == 5, f"Expected 5 recovered commands, got {rec['recoveredCommandsCount']}"
    assert rec["discardedCorruptedLinesCount"] == 1, "Expected 1 discarded partial line from mid-write kill"
    assert rec["recoveryTimeMs"] < 100.0, f"Recovery took {rec['recoveryTimeMs']} ms, exceeded 100 ms target"


if __name__ == "__main__":
    res = run_spike_g_browser_test()
    print("Result:", res)
