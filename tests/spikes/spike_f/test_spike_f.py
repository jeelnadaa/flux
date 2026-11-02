"""Playwright headless browser test for Spike F: Audio Encoders & Signalsmith Stretcher."""

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


def run_spike_f_browser_test():
    """Runs Spike F test in headless Chromium."""
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

                # Navigate to Spike F runner page
                url = f"http://127.0.0.1:{port}/tests/spikes/spike_f/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.spikeFResult !== undefined", timeout=25000)
                result = page.evaluate("() => window.spikeFResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_spike_f_encoders_and_stretcher():
    """Verify Spike F stretcher benchmark, WASM encoders, and WebCodecs AAC."""
    result = run_spike_f_browser_test()
    print("\nSpike F Measured Results:", result)

    assert result.get("success") is True, f"Spike F failed: {result}"

    # Stretcher benchmark
    stretcher = result["stretcher"]
    assert stretcher["cpuPercent"] < 30.0, f"Stretcher CPU {stretcher['cpuPercent']}% exceeded 30% budget"
    assert stretcher["nanCount"] == 0, f"Found {stretcher['nanCount']} NaNs in stretcher output"

    # WASM Encoders
    codecs = result["codecs"]
    assert codecs["mp3"]["hasSyncWord"] is True, "MP3 bitstream missing syncword"
    assert codecs["mp3"]["encodedBytes"] > 100

    assert codecs["opus"]["isOggOpus"] is True, "Opus bitstream missing OggS or OpusHead"
    assert codecs["opus"]["encodedBytes"] > 100

    assert codecs["vorbis"]["isOggVorbis"] is True, "Vorbis bitstream missing OggS or vorbis header"
    assert codecs["vorbis"]["encodedBytes"] > 100

    # WebCodecs AAC
    webcodecs = result["webcodecs"]
    assert webcodecs["supported"] is True, "WebCodecs AudioEncoder not supported in Chromium"
    assert webcodecs["aacSupported"] is True, "AAC (mp4a.40.2) not supported by AudioEncoder"
    assert webcodecs["aacChunkCount"] > 0, "No encoded AAC chunks produced"
    assert webcodecs["aacTotalBytes"] > 0


if __name__ == "__main__":
    res = run_spike_f_browser_test()
    print("Result:", res)
