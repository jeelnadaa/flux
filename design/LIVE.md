# Live screen (added 2026-11-07)

The Live screen was not in the original prototype. It follows "When something is not in `design/`" in DESIGN.md.
The production screen is `apps/desktop/ui/live.js`; this note records how it is built so the next screen can copy it.

**Nearest screen copied:** Presets (`pg` home-style page, rail entry, `g4` grid of `fxc` cards) for the gallery,
and the effect panels (`ctl-grid` of `sl` sliders, `seg`, `tg`, `sel`) for the designer.

**Layout, top to bottom**

1. Page header `pg('live', 'Live', lede, body, action)`. Rail entry "Live" after Home, icon `live`.
2. Two route cards in a `g2`: Computer sound and Microphone. Each is a `.box.col` with
   Sound from / Preset / Play to (`fld` + `sel`), Start or Stop (`btn`), Bypass (`tg`), Edit preset (`btn g`),
   two `.meter` rows (In, Out) with dB readouts, and a mono stats line. A running card gets the selection ring
   (`inset 0 0 0 2px var(--tx)`) and a `pill sig` reading "Live".
3. `sec('Presets')`: `seg` (Computer sound / Microphone / Yours), search `inp`, New and Import buttons, then a `g4`
   of `fxc` cards. Card palette and effect come from the 13 themes by a hash of the preset id.
4. `sec('Designer')`: title row with actions; then `sec`s for Whole preset (`ctl-grid`), Macros, Lanes,
   Post chain and Modulators. Modules, modulators and macros are `.box.col` on `--s2` in a `.lvmods` grid
   (auto-fill, min 320 px). Lanes and the post chain have a 3 px `--tx` rule on the left.
5. The step sequencer `.lvsteps` is a row of bars filled from the bottom in `--tx` over `--s4`.

**New CSS** (in `app.css` here and `desktop.css` in the app): `.tipbox`, `.lvmods`, `.lvsteps`. No new tokens.

**Tooltips:** every control sits inside an element with `data-tip`; one `.tipbox` (role tooltip) shows after
420 ms of hover or at once on keyboard focus, and links itself with `aria-describedby`. Escape hides it.

**Checked:** 1440, 820 and 390 px; all 13 themes and 6 fonts at 390 px with no sideways scrolling
(`tests/e2e/test_live.py`).

## Voice cloning: the Script section

Composed from the kit inside `wiz` (wide): a `box col` holding the script `inp` (textarea), then three rows of
`chip` buttons labelled in mono (Sounds, Hesitations, Pauses), one chip per placeholder. After Check script, the
parts read inline: words as text, sounds as `pill sig`, hesitations as `pill ok`, pauses as a plain `pill` with the
length ("0.5 s pause"), and a mono summary line (sounds, hesitations, seconds of pauses, pieces). No new CSS.
Checked at 1440 and 390 px with no sideways scrolling.

## Voice cloning: Models, How it speaks, and the Download dialog

**Models** is a `lst box`: the voice engine first, then one row per model. A row is the kit list row: an `act`
icon (`voice` for speech, `swap` for conversion, `chip` for the engine), a `ti` with the name in bold and the note,
then a row of `pill`s (what it does, `pill sig` for sounds, languages, `pill ok` for "Suggested here" and for a
graphics chip fit, `pill warn` when it can't run). On the right: licence and size in mono, then the action:
Download or Resume (`btn`, primary when suggested), or when installed Use to speak / Use to convert (`btn p` with
`aria-pressed` when chosen) and a ghost trash `btn`. While downloading, the row wraps a full-width `prog` with the
percent and stage in mono and a ghost Cancel. Under the list, a `box col` with a `sel` for the kind of model and a
Choose folder `btn`, then the models folder path in mono, dim, 12.5 px.

**How it speaks** is a `box col`: the chosen model's name, a `seg` for the device (Auto, NVIDIA, Apple,
Processor), one `sl` per model setting (bound `cl:<model>:<key>`, value in mono), a `sel` for the language when the
model has several, a Seed `inp`, and a ghost Defaults `btn`.

**Download dialog** (`clone-get`) is the kit `dlg` over the clone page, shaped like Split into stems: a `lst box`
with the model (licence, source in mono, size) and the voice engine when it isn't installed (note, size), a
`callout ac` with the lock icon saying what downloads, how big and where, a dim 12.5 px line on permission, and a
foot with ghost Cancel and primary Download. No new CSS.

**Checked:** 1440, 820 and 390 px with no sideways scrolling; every control named and inside a `data-tip`
(`tests/e2e/test_clone_models.py`, screenshots `target/e2e/clone-*.png`).
