"""Playwright headless browser test for Spike C: Binaural Rendering & 8D Prototype."""

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


def run_spike_c_browser_test():
    """Runs Spike C test in headless Chromium."""
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

                # Navigate to Spike C runner page
                url = f"http://127.0.0.1:{port}/tests/spikes/spike_c/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete
                page.wait_for_function("() => window.spikeCResult !== undefined", timeout=25000)
                result = page.evaluate("() => window.spikeCResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_spike_c_binaural_rendering():
    """Verify Spike C binaural 3D/8D spatializer and auto-orbit rendering."""
    result = run_spike_c_browser_test()
    print("\nSpike C Measured Results:", result)

    assert result.get("success") is True, f"Spike C failed: {result}"

    # Performance
    bench = result["benchmark"]
    assert bench["cpuPercent"] < 30.0, f"CPU usage {bench['cpuPercent']}% exceeded 30% budget"

    # Hard Right ILD
    hr = result["hardRight"]
    assert hr["ildDb"] >= 3.0, f"Hard right ILD {hr['ildDb']} dB was below 3.0 dB threshold"
    assert hr["rmsR"] > hr["rmsL"]

    # Hard Left ILD
    hl = result["hardLeft"]
    assert hl["ildDb"] >= 3.0, f"Hard left ILD {hl['ildDb']} dB was below 3.0 dB threshold"
    assert hl["rmsL"] > hl["rmsR"]

    # 8D Auto-orbit continuity & alternation
    orb = result["eightDOrbit"]
    assert orb["nanCount"] == 0, f"Encountered {orb['nanCount']} NaNs in 8D orbit render"
    assert orb["maxDeltaL"] < 0.25, f"Click detected on Left channel (delta={orb['maxDeltaL']})"
    assert orb["maxDeltaR"] < 0.25, f"Click detected on Right channel (delta={orb['maxDeltaR']})"
    assert orb["q1_rmsR"] > orb["q1_rmsL"], "Right channel should dominate in Q1 of orbit"
    assert orb["q3_rmsL"] > orb["q3_rmsR"], "Left channel should dominate in Q3 of orbit"


if __name__ == "__main__":
    res = run_spike_c_browser_test()
    print("Result:", res)
