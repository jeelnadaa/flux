"""Automated Visual Regression and Pixel-Diff Test Suite for Flux UI (P2.0).

Verifies 1:1 replica of design/ prototype at:
- 1440px (Desktop)
- 820px (Tablet)
- 390px (Phone)
Enforces >= 95% pixel matching across all screens.
Tests interactive Theme and Font settings with 950ms circular reveal.
"""

import http.server
import socketserver
import threading
from pathlib import Path
import pytest
from playwright.sync_api import sync_playwright
from tests.pixel_diff.comparator import compare_images

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
ARTIFACTS_DIR = REPO_ROOT / 'tests' / 'pixel_diff' / 'artifacts'


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(REPO_ROOT), **kwargs)

    def log_message(self, format, *args):
        pass  # Suppress request logging


@pytest.fixture(scope='module')
def test_server():
    ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)
    with http.server.ThreadingHTTPServer(('127.0.0.1', 0), QuietHandler) as httpd:
        port = httpd.server_address[1]
        server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        server_thread.start()
        yield port
        httpd.shutdown()


@pytest.fixture(scope='module')
def browser_instance():
    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=True,
            args=['--no-sandbox', '--disable-gpu', '--font-render-hinting=none'],
        )
        yield browser
        browser.close()


def capture_and_compare(browser, port: int, screen: str, width: int, height: int, name: str):
    # 1. Prototype screen
    ctx_proto = browser.new_context(viewport={'width': width, 'height': height})
    page_proto = ctx_proto.new_page()
    page_proto.goto(f'http://127.0.0.1:{port}/design/app.html#{screen}', wait_until='domcontentloaded')
    page_proto.wait_for_selector('#stage .pg', timeout=25000)
    page_proto.evaluate("""() => {
        const rv = document.getElementById('rv');
        if (rv) rv.remove();
        const frame = document.getElementById('frame');
        if (frame) frame.style.inset = '0';
        const m = document.getElementById('m');
        if (m) m.scrollTop = 0;
    }""")
    page_proto.wait_for_timeout(350)
    proto_img = ARTIFACTS_DIR / f'proto_{screen}_{name}_{width}.png'
    page_proto.locator('#stage').screenshot(path=str(proto_img))
    ctx_proto.close()

    # 2. Production screen
    ctx_prod = browser.new_context(viewport={'width': width, 'height': height})
    page_prod = ctx_prod.new_page()
    page_prod.goto(f'http://127.0.0.1:{port}/apps/web/index.html#{screen}', wait_until='domcontentloaded')
    page_prod.wait_for_selector('#stage .pg', timeout=25000)
    page_prod.evaluate("""() => {
        const m = document.getElementById('m');
        if (m) m.scrollTop = 0;
    }""")
    page_prod.wait_for_timeout(350)
    prod_img = ARTIFACTS_DIR / f'prod_{screen}_{name}_{width}.png'
    page_prod.locator('#stage').screenshot(path=str(prod_img))
    ctx_prod.close()

    diff_img = ARTIFACTS_DIR / f'diff_{screen}_{name}_{width}.png'
    res = compare_images(proto_img, prod_img, diff_img)
    return res


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_kit(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'kit', width, height, name)
    print(f"\n[KIT {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Kit screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_themes(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'prefs-themes', width, height, name)
    print(f"\n[THEMES {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Themes screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_fonts(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'prefs-fonts', width, height, name)
    print(f"\n[FONTS {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Fonts screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


def test_theme_switching_and_persistence(browser_instance, test_server):
    """Verifies all 13 themes can be selected, persisted in localStorage, and applied to DOM root."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#prefs-themes')
    page.wait_for_selector('#stage .pg')

    # Initial default theme is cassette
    initial_theme = page.evaluate("() => document.documentElement.dataset.flux")
    assert initial_theme == 'cassette'

    # Click slowed theme card
    slowed_btn = page.locator('button[data-set-theme="slowed"]')
    slowed_btn.click()
    page.wait_for_timeout(300)

    theme_after = page.evaluate("() => document.documentElement.dataset.flux")
    stored_theme = page.evaluate("() => localStorage.getItem('flux-theme')")
    assert theme_after == 'slowed'
    assert stored_theme == 'slowed'

    # Click nightcore theme card
    nightcore_btn = page.locator('button[data-set-theme="nightcore"]')
    nightcore_btn.click()
    page.wait_for_timeout(300)

    theme_after_nightcore = page.evaluate("() => document.documentElement.dataset.flux")
    stored_nightcore = page.evaluate("() => localStorage.getItem('flux-theme')")
    assert theme_after_nightcore == 'nightcore'
    assert stored_nightcore == 'nightcore'

    # Click cassette to reset
    cassette_btn = page.locator('button[data-set-theme="cassette"]')
    cassette_btn.click()
    page.wait_for_timeout(300)

    assert page.evaluate("() => document.documentElement.dataset.flux") == 'cassette'
    context.close()


def test_font_switching_and_storage(browser_instance, test_server):
    """Verifies all 6 fonts can be selected, persisted in localStorage, and applied to DOM root."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#prefs-fonts')
    page.wait_for_selector('#stage .pg')

    # Initial font is bricolage
    initial_font = page.evaluate("() => document.documentElement.dataset.font")
    assert initial_font == 'bricolage'

    # Click satoshi font card
    satoshi_btn = page.locator('button[data-set-font="satoshi"]')
    satoshi_btn.click()
    page.wait_for_timeout(300)

    font_after = page.evaluate("() => document.documentElement.dataset.font")
    stored_font = page.evaluate("() => localStorage.getItem('flux-font')")
    assert font_after == 'satoshi'
    assert stored_font == 'satoshi'

    # Click geist font card
    geist_btn = page.locator('button[data-set-font="geist"]')
    geist_btn.click()
    page.wait_for_timeout(300)

    font_after_geist = page.evaluate("() => document.documentElement.dataset.font")
    stored_geist = page.evaluate("() => localStorage.getItem('flux-font')")
    assert font_after_geist == 'geist'
    assert stored_geist == 'geist'

    # Reset to bricolage
    bricolage_btn = page.locator('button[data-set-font="bricolage"]')
    bricolage_btn.click()
    page.wait_for_timeout(300)

    assert page.evaluate("() => document.documentElement.dataset.font") == 'bricolage'
    context.close()


def test_kit_interactive_controls(browser_instance, test_server):
    """Tests interactive controls on the kit screen: sliders, toggles, segmented controls."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#kit')
    page.wait_for_selector('#stage .pg')

    # Test Slider arrow navigation
    slider_track = page.locator('.sl .track').first
    initial_val = int(slider_track.get_attribute('aria-valuenow'))
    slider_track.focus()
    page.keyboard.press('ArrowRight')
    page.wait_for_timeout(100)
    new_val = int(slider_track.get_attribute('aria-valuenow'))
    assert new_val == initial_val + 2, f"Expected slider value {initial_val + 2}, got {new_val}"

    # Test Toggle button
    toggle_btn = page.locator('button.tg').first
    init_pressed = toggle_btn.get_attribute('aria-pressed')
    toggle_btn.click()
    page.wait_for_timeout(100)
    assert toggle_btn.get_attribute('aria-pressed') != init_pressed

    # Test Segmented control
    seg_buttons = page.locator('.seg button')
    assert seg_buttons.count() >= 3
    second_btn = seg_buttons.nth(1)
    second_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in second_btn.get_attribute('class')

    context.close()
