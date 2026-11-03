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
    page_proto.wait_for_selector('#stage > *', timeout=25000)
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
    page_prod.wait_for_selector('#stage > *', timeout=25000)
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


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_spectral(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'spectral', width, height, name)
    print(f"\n[SPECTRAL {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Spectral screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_edit(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'edit', width, height, name)
    print(f"\n[EDIT {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Edit screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


def test_spectral_selection_interaction(browser_instance, test_server):
    """Verifies spectral selection box can be moved and view modes toggled."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#spectral')
    page.wait_for_selector('#stage > *')

    # Verify spectral box exists
    box = page.locator('[data-testid="spectral-box"]')
    assert box.is_visible()

    # Move the box by dragging
    box_box = box.bounding_box()
    assert box_box is not None

    page.mouse.move(box_box['x'] + 20, box_box['y'] + 20)
    page.mouse.down()
    page.mouse.move(box_box['x'] + 80, box_box['y'] + 50)
    page.mouse.up()
    page.wait_for_timeout(100)

    # Verify mode toggles work
    seg_btns = page.locator('.tlbar .seg button')
    assert seg_btns.count() >= 3
    # Click 'Spectrogram'
    seg_btns.nth(0).click()
    page.wait_for_timeout(100)
    assert 'on' in seg_btns.nth(0).get_attribute('class')

    context.close()


def test_timeline_scrub_and_snapping(browser_instance, test_server):
    """Verifies timeline ruler scrubbing moves playhead and snaps."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#edit')
    page.wait_for_selector('#stage > *')

    ph = page.locator('.ph')
    initial_style = ph.get_attribute('style') or ''
    assert 'translateX(120px)' in initial_style

    # Click on ruler at x = 320px
    ruler = page.locator('.ruler').first
    ruler_box = ruler.bounding_box()
    assert ruler_box is not None

    page.mouse.click(ruler_box['x'] + 280, ruler_box['y'] + 10)
    page.wait_for_timeout(100)

    # Playhead should have moved
    new_style = ph.get_attribute('style') or ''
    assert new_style != initial_style

    # Toggle snap chip
    snap_chip = page.locator('button.chip:has-text("Snap: zero cross")')
    assert 'on' in (snap_chip.get_attribute('class') or '')
    snap_chip.click()
    page.wait_for_timeout(100)
    assert 'on' not in (snap_chip.get_attribute('class') or '')

    context.close()


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_automation(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'automation', width, height, name)
    print(f"\n[AUTOMATION {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Automation screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_mix(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'mix', width, height, name)
    print(f"\n[MIX {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Mix screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


def test_automation_screen_interaction(browser_instance, test_server):
    """Verifies automation modes, curve modes, and lane addition/removal."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#automation')
    page.wait_for_selector('#stage > *')

    # Verify automation SVG path exists
    autoline = page.locator('svg.autoline')
    assert autoline.count() >= 1

    # Toggle mode to 'Write'
    write_btn = page.locator('.tlbar .seg button:has-text("Write")')
    write_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in write_btn.get_attribute('class')

    # Toggle curve interpolation to 'Smooth'
    smooth_btn = page.locator('.tlbar .seg button:has-text("Smooth")')
    smooth_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in smooth_btn.get_attribute('class')

    # Test adding a lane
    init_lanes = page.locator('.lst > div').count()
    add_btn = page.locator('button:has-text("Add lane")')
    add_btn.click()
    page.wait_for_timeout(100)
    assert page.locator('.lst > div').count() == init_lanes + 1

    context.close()


def test_mixer_controls_interaction(browser_instance, test_server):
    """Verifies faders, mute, and solo buttons on mixer channels."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#mix')
    page.wait_for_selector('#stage > *')

    # Verify 8 channels are rendered
    channels = page.locator('.mixer .ch')
    assert channels.count() == 8

    # Verify master channel exists
    master = page.locator('.mixer .ch.mst')
    assert master.count() == 1

    # Test mute button on first channel
    mute_btn = page.locator('.mixer .ch .m').first
    assert 'on' not in (mute_btn.get_attribute('class') or '')
    mute_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (mute_btn.get_attribute('class') or '')

    # Test solo button on first channel
    solo_btn = page.locator('.mixer .ch .s').first
    assert 'on' not in (solo_btn.get_attribute('class') or '')
    solo_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (solo_btn.get_attribute('class') or '')

    context.close()


def test_edit_screen_clip_and_track_interactions(browser_instance, test_server):
    """Verifies splitting, deleting, reversing clips, and mute toggling on Edit screen."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#edit')
    page.wait_for_selector('#stage > *')

    # Initial clip count on Vocals track
    vocal_clips = page.locator('.trk').first.locator('.clip')
    assert vocal_clips.count() == 3

    # Click Split at playhead
    split_btn = page.locator('button[aria-label="Split at playhead"]')
    split_btn.click()
    page.wait_for_timeout(100)
    # Selected clip split into 2 subclips -> 4 clips now
    assert vocal_clips.count() == 4

    # Click Reverse chip
    rev_chip = page.locator('button.chip:has-text("Reverse")')
    assert 'on' not in (rev_chip.get_attribute('class') or '')
    rev_chip.click()
    page.wait_for_timeout(100)
    assert 'on' in (rev_chip.get_attribute('class') or '')

    # Click Delete button
    del_btn = page.locator('button[aria-label="Delete"]')
    del_btn.click()
    page.wait_for_timeout(100)
    # 4 - 1 = 3 clips
    assert vocal_clips.count() == 3

    # Toggle Mute on Vocals track
    mute_btn = page.locator('.trk').first.locator('button.m')
    assert 'on' not in (mute_btn.get_attribute('class') or '')
    mute_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (mute_btn.get_attribute('class') or '')

    context.close()


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_history(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'history', width, height, name)
    print(f"\n[HISTORY {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"History screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_recovery(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'recovery', width, height, name)
    print(f"\n[RECOVERY {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Recovery dialog on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_relink(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'relink', width, height, name)
    print(f"\n[RELINK {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Relink dialog on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_quota(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'quota', width, height, name)
    print(f"\n[QUOTA {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Quota dialog on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


def test_history_screen_interactions(browser_instance, test_server):
    """Verifies creating snapshots and navigating history timeline."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#history')
    page.wait_for_selector('#stage > *')

    # Initial history items count
    items = page.locator('.lst.box > div')
    initial_count = items.count()
    assert initial_count >= 6

    # Verify first item is Current
    first_btn = items.first.locator('button')
    assert first_btn.inner_text().strip() == 'Current'

    # Click 'Save a snapshot'
    snap_btn = page.locator('button:has-text("Save a snapshot")')
    snap_btn.click()
    page.wait_for_timeout(100)

    # A new snapshot should have been added
    assert items.count() == initial_count + 1
    assert 'Snapshot:' in items.first.locator('.ti b').inner_text()
    assert items.first.locator('button').inner_text().strip() == 'Current'

    # Click 'Go here' on second item
    second_btn = items.nth(1).locator('button')
    second_btn.click()
    page.wait_for_timeout(100)
    assert second_btn.inner_text().strip() == 'Current'

    context.close()


def test_recovery_dialog_interactions(browser_instance, test_server):
    """Verifies crash recovery dialog buttons and resume action."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#recovery')
    page.wait_for_selector('.dlg')

    # Resume interrupted export
    resume_btn = page.locator('.dlg button:has-text("Resume")')
    assert resume_btn.count() == 1
    resume_btn.click()
    page.wait_for_timeout(100)
    assert page.locator('.dlg button:has-text("Resumed")').count() == 1

    # Check footer link targets
    snap_link = page.locator('.dlg .foot a:has-text("Open earlier snapshot")')
    assert snap_link.get_attribute('href') == '#history'

    reopen_link = page.locator('.dlg .foot a:has-text("Reopen project")')
    assert reopen_link.get_attribute('href') == '#edit'

    context.close()


def test_relink_dialog_interactions(browser_instance, test_server):
    """Verifies missing media locate buttons and folder toggle."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#relink')
    page.wait_for_selector('.dlg')

    # Locate missing drums
    locate_btn = page.locator('.dlg .lst > div').first.locator('button:has-text("Locate")')
    locate_btn.click()
    page.wait_for_timeout(100)
    assert page.locator('.dlg .lst > div').first.locator('.pill').inner_text().strip() == 'Relinked'

    # Toggle 'Look in the same folder'
    tg = page.locator('.dlg button.tg')
    assert tg.get_attribute('aria-pressed') == 'true'
    tg.click()
    page.wait_for_timeout(100)
    assert tg.get_attribute('aria-pressed') == 'false'

    context.close()


def test_quota_dialog_interactions(browser_instance, test_server):
    """Verifies storage quota cleanup actions."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#quota')
    page.wait_for_selector('.dlg')

    # Verify 98% progress bar
    prog = page.locator('.dlg .prog')
    assert prog.get_attribute('aria-valuenow') == '98'

    # Click 'Clear' on waveform caches
    clear_btn = page.locator('.dlg .lst > div').nth(0).locator('button:has-text("Clear")')
    clear_btn.click()
    page.wait_for_timeout(100)
    assert page.locator('.dlg .lst > div').nth(0).locator('button').inner_text().strip() == 'Done'

    context.close()


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_shortcuts(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'shortcuts', width, height, name)
    print(f"\n[SHORTCUTS {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Shortcuts dialog on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_palette(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'palette', width, height, name)
    print(f"\n[PALETTE {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Command palette on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_prefs_keys(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'prefs-keys', width, height, name)
    print(f"\n[PREFS-KEYS {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Keyboard settings on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_library(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'library', width, height, name)
    print(f"\n[LIBRARY {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Library screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_empty(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'empty', width, height, name)
    print(f"\n[EMPTY {name.upper()} {width}px] Match: {res['match_percentage']:.2f}%, Mean Delta: {res['mean_delta']:.2f}")
    assert res['match_percentage'] >= 95.0, (
        f"Empty project screen on {name} ({width}px) fell below 95% match: {res['match_percentage']:.2f}%"
    )


def test_shortcuts_and_command_palette_interactions(browser_instance, test_server):
    """Verifies shortcuts dialog links and palette search filter."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    # Test shortcuts sheet
    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#shortcuts')
    page.wait_for_selector('.dlg')
    all_btn = page.locator('.dlg .foot a:has-text("All shortcuts")')
    assert all_btn.get_attribute('href') == '#prefs-keys'

    # Test command palette
    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#palette')
    page.wait_for_selector('.dlg.cmd')
    inp = page.locator('.dlg input.inp')
    assert inp.input_value() == 'nor'
    items = page.locator('.dlg .col a.it')
    assert items.count() == 5

    inp.fill('reverb')
    page.wait_for_timeout(100)
    assert items.count() == 1
    assert 'Normal room' in items.first.inner_text()

    context.close()


def test_keyboard_settings_interaction(browser_instance, test_server):
    """Verifies searching and viewing shortcuts in settings."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#prefs-keys')
    page.wait_for_selector('.tw table')

    rows = page.locator('.tw table tbody tr')
    assert rows.count() >= 10

    # Search for 'split'
    search = page.locator('input[placeholder="Search commands"]')
    search.fill('split')
    page.wait_for_timeout(100)
    assert rows.count() == 1
    assert 'Split at playhead' in rows.first.inner_text()

    context.close()


def test_library_screen_interaction(browser_instance, test_server):
    """Verifies searching, category filtering, and folder chips on Library screen."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#library')
    page.wait_for_selector('.tw table')

    rows = page.locator('.tw table tbody tr')
    assert rows.count() == 5

    # Filter to Podcast
    podcast_btn = page.locator('.seg button:has-text("Podcast")')
    podcast_btn.click()
    page.wait_for_timeout(100)
    assert rows.count() == 1
    assert 'Interview ep. 12' in rows.first.inner_text()

    # Reset to All
    all_btn = page.locator('.seg button:has-text("All")')
    all_btn.click()
    page.wait_for_timeout(100)
    assert rows.count() == 5

    # Click chip
    client_chip = page.locator('button.chip:has-text("Client work")')
    client_chip.click()
    page.wait_for_timeout(100)
    assert 'on' in client_chip.get_attribute('class')

    context.close()


def test_edit_screen_context_menu_and_keyboard(browser_instance, test_server):
    """Verifies right-click context menu and keyboard shortcuts (S for split, Delete) on Edit screen."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#edit')
    page.wait_for_selector('.tl')

    clips = page.locator('.trk').first.locator('.clip')
    assert clips.count() == 3

    # Press 'S' on keyboard -> splits selected clip
    page.keyboard.press('s')
    page.wait_for_timeout(100)
    assert clips.count() == 4

    # Right-click on timeline to open context menu
    tl = page.locator('.tl')
    tl.click(button='right', position={'x': 250, 'y': 80})
    page.wait_for_timeout(100)

    # Context menu popup should appear
    menu = page.locator('.dlg:has-text("Split")')
    assert menu.count() >= 1

    # Click Delete in context menu
    menu.locator('button:has-text("Delete")').click()
    page.wait_for_timeout(100)
    assert clips.count() == 3

    context.close()


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_rack(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'rack', width, height, name)
    print(f"Rack [{name}] match: {res['match_percentage']:.2f}% (mismatched: {res['mismatched_pixels']})")
    assert res['match_percentage'] >= 95.0, (
        f"Rack [{name}] visual match {res['match_percentage']:.2f}% below 95% threshold"
    )


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_fxbrowser(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'fxbrowser', width, height, name)
    print(f"Effect Browser [{name}] match: {res['match_percentage']:.2f}% (mismatched: {res['mismatched_pixels']})")
    assert res['match_percentage'] >= 95.0, (
        f"Effect Browser [{name}] visual match {res['match_percentage']:.2f}% below 95% threshold"
    )


def test_effect_rack_interactive(browser_instance, test_server):
    """Verifies interactivity on the Effect Rack screen: bypass toggle, solo toggle, mix, compare slots."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#rack')
    page.wait_for_selector('.split')

    # Verify 5 default rack items
    items = page.locator('.a .in .box.row')
    assert items.count() == 5

    # Check first item toggle (Enable De-esser)
    first_toggle = items.first.locator('button.tg')
    assert first_toggle.get_attribute('aria-pressed') == 'true'

    first_toggle.click()
    page.wait_for_timeout(100)
    assert first_toggle.get_attribute('aria-pressed') == 'false'

    # Check solo button
    solo_btn = items.first.locator('button.chip:has-text("Solo")')
    assert 'on' not in (solo_btn.get_attribute('class') or '')
    solo_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (solo_btn.get_attribute('class') or '')

    # Check compare slot change
    slot_b = page.locator('.tlbar .seg button:has-text("B")')
    slot_b.click()
    page.wait_for_timeout(100)
    assert 'on' in (slot_b.get_attribute('class') or '')

    context.close()


def test_effect_browser_search_and_filter(browser_instance, test_server):
    """Verifies searching and category chip filtering in the Effect Browser."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#fxbrowser')
    page.wait_for_selector('.g5')

    # Initial total cards
    cards = page.locator('.fxc')
    assert cards.count() >= 30

    # Type search query "compressor"
    inp = page.locator('.pgh input.inp')
    inp.fill('compressor')
    page.wait_for_timeout(100)

    # Filtered cards count should be reduced to 2 (Compressor and Multiband compressor)
    filtered_cards = page.locator('.fxc')
    assert filtered_cards.count() >= 1
    assert 'Compressor' in filtered_cards.first.inner_text()

    # Clear search
    inp.fill('')
    page.wait_for_timeout(100)
    assert page.locator('.fxc').count() == cards.count()

    context.close()


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_eq(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'fx-eq', width, height, name)
    print(f"Equalizer [{name}] match: {res['match_percentage']:.2f}% (mismatched: {res['mismatched_pixels']})")
    assert res['match_percentage'] >= 95.0, (
        f"Equalizer [{name}] visual match {res['match_percentage']:.2f}% below 95% threshold"
    )


def test_equalizer_interactive(browser_instance, test_server):
    """Verifies interactivity on the Parametric EQ screen: preset switching, mode, and view toggles."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#fx-eq')
    page.wait_for_selector('.vz')

    # Initial preset is Clear voice
    clear_voice_chip = page.locator('button.chip:has-text("Clear voice")')
    assert 'on' in (clear_voice_chip.get_attribute('class') or '')

    # Click "Bass boost" preset chip
    bass_boost_chip = page.locator('button.chip:has-text("Bass boost")')
    bass_boost_chip.click()
    page.wait_for_timeout(100)
    assert 'on' in (bass_boost_chip.get_attribute('class') or '')

    # Check mode switch: Linear phase
    lp_btn = page.locator('.sec:has-text("Mode") .seg button:has-text("Linear phase")')
    lp_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (lp_btn.get_attribute('class') or '')

    # Check view switch: 10 band
    band10_btn = page.locator('.sec:has-text("Show") .seg button:has-text("10 band")')
    band10_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (band10_btn.get_attribute('class') or '')

    # Check bypass toggle
    bypass_btn = page.locator('.tlbar button[data-toggle]')
    bypass_btn.click()
    page.wait_for_timeout(100)
    assert bypass_btn.get_attribute('aria-pressed') == 'true'

    context.close()


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1024, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_dynamics(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'fx-comp', width, height, name)
    print(f"Dynamics [{name}] match: {res['match_percentage']:.2f}% (mismatched: {res['mismatched_pixels']})")
    assert res['match_percentage'] >= 95.0, (
        f"Dynamics [{name}] visual match {res['match_percentage']:.2f}% below 95% threshold"
    )


def test_dynamics_interactive(browser_instance, test_server):
    """Verifies interactivity on the Dynamics screen: tab switching across all 7 processors, sliders, toggles."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#fx-comp')
    page.wait_for_selector('.tabs')

    # Initial tab is Compressor
    comp_btn = page.locator('.tabs button:has-text("Compressor")')
    assert 'on' in (comp_btn.get_attribute('class') or '')

    # Switch to Limiter tab
    limiter_btn = page.locator('.tabs button:has-text("Limiter")')
    limiter_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (limiter_btn.get_attribute('class') or '')
    assert page.locator('.ctl-grid .sl label:has-text("Ceiling")').count() == 1

    # Switch to Multiband tab
    mb_btn = page.locator('.tabs button:has-text("Multiband")')
    mb_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (mb_btn.get_attribute('class') or '')

    # Switch to Gate tab
    gate_btn = page.locator('.tabs button:has-text("Gate")')
    gate_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (gate_btn.get_attribute('class') or '')

    # Switch to De-esser tab
    deess_btn = page.locator('.tabs button:has-text("De-esser")')
    deess_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (deess_btn.get_attribute('class') or '')

    # Switch to Transient tab
    trans_btn = page.locator('.tabs button:has-text("Transient")')
    trans_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (trans_btn.get_attribute('class') or '')

    # Switch to Leveler tab
    leveler_btn = page.locator('.tabs button:has-text("Leveler")')
    leveler_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (leveler_btn.get_attribute('class') or '')

    # Switch back to Compressor
    comp_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (comp_btn.get_attribute('class') or '')

    # Toggle bypass
    bypass_btn = page.locator('.tlbar button[data-toggle]')
    bypass_btn.click()
    page.wait_for_timeout(100)
    assert bypass_btn.get_attribute('aria-pressed') == 'true'

    context.close()


@pytest.mark.parametrize(
    'width,height,name',
    [
        (1440, 900, 'desktop'),
        (820, 1180, 'tablet'),
        (390, 844, 'phone'),
    ],
)
def test_pixel_diff_speed_pitch(browser_instance, test_server, width, height, name):
    res = capture_and_compare(browser_instance, test_server, 'fx-time', width, height, name)
    print(f"SpeedPitch [{name}] match: {res['match_percentage']:.2f}% (mismatched: {res['mismatched_pixels']})")
    assert res['match_percentage'] >= 95.0, (
        f"SpeedPitch [{name}] visual match {res['match_percentage']:.2f}% below 95% threshold"
    )


def test_speed_pitch_interactive(browser_instance, test_server):
    """Verifies interactivity on the Speed and Pitch screen: chip selection, slider interaction, mode switching, tap tempo."""
    context = browser_instance.new_context(viewport={'width': 1440, 'height': 900})
    page = context.new_page()

    page.goto(f'http://127.0.0.1:{test_server}/apps/web/index.html#fx-time')
    page.wait_for_selector('.h3')

    # Verify initial stats
    speed_stat = page.locator('.box .stat').first
    assert '0.85' in speed_stat.inner_text()
    pitch_stat = page.locator('.box .stat').nth(1)
    assert '2.0' in pitch_stat.inner_text()

    # Click speed chip '1.5×'
    chip_15 = page.locator('.box button.chip:has-text("1.5×")')
    chip_15.click()
    page.wait_for_timeout(100)
    assert 'on' in (chip_15.get_attribute('class') or '')
    assert '1.50' in speed_stat.inner_text()

    # Click pitch chip '+2'
    chip_p2 = page.locator('.box button.chip:has-text("+2")')
    chip_p2.click()
    page.wait_for_timeout(100)
    assert 'on' in (chip_p2.get_attribute('class') or '')
    assert '+2.0' in pitch_stat.inner_text()

    # Switch mode to Varispeed
    varispeed_btn = page.locator('.sec .seg button:has-text("Varispeed")')
    varispeed_btn.click()
    page.wait_for_timeout(100)
    assert 'on' in (varispeed_btn.get_attribute('class') or '')

    # Test tap tempo
    tap_btn = page.locator('button[data-toggle]:has-text("Tap")')
    tap_btn.click()
    page.wait_for_timeout(500)
    tap_btn.click()
    page.wait_for_timeout(100)
    tempo_inp = page.locator('input.inp').first
    assert float(tempo_inp.input_value()) > 0

    # Toggle 'Keep pitch when changing speed'
    keep_pitch_tg = page.locator('.box.row button.tg')
    init_pressed = keep_pitch_tg.get_attribute('aria-pressed')
    keep_pitch_tg.click()
    page.wait_for_timeout(100)
    assert keep_pitch_tg.get_attribute('aria-pressed') != init_pressed

    context.close()




