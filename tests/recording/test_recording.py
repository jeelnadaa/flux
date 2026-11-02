"""Playwright headless browser test for Recording Capture, Metering, Pre-Roll, and OPFS Chunked Writes."""

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


def run_recording_browser_test():
    """Runs Recording Capture test in headless Chromium."""
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
                        "--use-fake-ui-for-media-stream",
                        "--use-fake-device-for-media-stream",
                    ],
                )
                page = browser.new_page()
                page.on("console", lambda msg: print(f"CONSOLE: {msg.text}"))
                page.on("pageerror", lambda err: print(f"PAGEERROR: {err}"))

                # Navigate to Recording runner page
                url = f"http://127.0.0.1:{port}/tests/recording/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.recordingTestResult !== undefined", timeout=25000)
                result = page.evaluate("() => window.recordingTestResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_recording_capture_and_opfs_chunked_stream():
    """Verify microphone constraints, level metering, pre-roll ring buffer, OPFS chunked writes, and crash recovery."""
    result = run_recording_browser_test()
    print("\nRecording Measured Results:", result)

    assert result.get("success") is True, f"Recording browser test failed: {result}"
    assert result.get("meterTest") is True
    assert result.get("preRollTest") is True
    assert result.get("opfsChunkedTest") is True
    assert result.get("crashRecoveryTest") is True
    assert result.get("monitoringTest") is True


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
