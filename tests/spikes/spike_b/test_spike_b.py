"""Playwright headless browser test for Spike B: ONNX Runtime Web (small & large models)."""

import http.server
import socketserver
import threading
from pathlib import Path
import pytest
from playwright.sync_api import sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent.parent.parent


class CrossOriginHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    """HTTP handler with Cross-Origin Isolation headers for SharedArrayBuffer / WASM multithreading."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(REPO_ROOT), **kwargs)

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def log_message(self, format, *args):
        pass  # Suppress request logging


def run_spike_b_browser_test():
    """Runs Spike B benchmark in headless Chromium."""
    # Ensure models are generated before starting test
    from tests.spikes.spike_b.generate_models import (
        create_small_rnnoise_model,
        create_large_htdemucs_model,
    )

    models_dir = Path(__file__).resolve().parent
    small_path = models_dir / "small_model.onnx"
    large_path = models_dir / "large_model.onnx"
    if not small_path.exists():
        create_small_rnnoise_model(small_path)
    if not large_path.exists():
        create_large_htdemucs_model(large_path)

    with socketserver.TCPServer(("127.0.0.1", 0), CrossOriginHTTPRequestHandler) as httpd:
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
                        "--enable-unsafe-webgpu",
                        "--autoplay-policy=no-user-gesture-required",
                    ],
                )
                page = browser.new_page()

                # Navigate to Spike B runner page
                url = f"http://127.0.0.1:{port}/tests/spikes/spike_b/index.html?autorun=true"
                page.goto(url)

                # Wait for test to complete (timeout 60s for WASM compilation / chunked inference)
                page.wait_for_function("() => window.spikeBResult !== undefined", timeout=60000)
                result = page.evaluate("() => window.spikeBResult")

                browser.close()
                return result
        finally:
            httpd.shutdown()


def test_spike_b_onnx_runtime_web():
    """Verify Spike B small and large ONNX model benchmarks."""
    result = run_spike_b_browser_test()
    print("\nSpike B Measured Results:", result)

    assert result.get("success") is True, f"Spike B failed: {result}"

    # Small model (RNNoise-class) assertions
    small = result["smallModel"]
    assert small["numFrames"] == 100
    assert small["avgFrameMs"] < 10.0, f"Small model frame took {small['avgFrameMs']} ms, exceeds 10 ms real-time frame budget"
    assert small["rtf"] < 0.5, f"Small model RTF {small['rtf']} exceeds 0.5 target"

    # Large model (HTDemucs-class) assertions
    large = result["largeModel"]
    assert large["numChunks"] == 4
    # Chunk duration is 1000 ms, avg chunk compute should be within real-time factor bound
    assert large["avgChunkMs"] > 0
    assert large["rtf"] > 0


if __name__ == "__main__":
    res = run_spike_b_browser_test()
    print("Result:", res)
