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
4. `sec('Designer')`: the chosen preset's name, description, pills and actions (Use it, Edit, Preview file, Copy,
   Export, Delete). Edit opens the editor, a `dlg wide` over the page (the kit's `.ov` + `.dlg`): name and
   description, Randomize, then `sec`s for Whole preset (`ctl-grid`), Macros, Lanes, Post chain and Modulators,
   and a footer kept at the bottom of the dialog (sticky, `--s0`, 1.5 px `--ln2` rule above) with Discard and Save.
   Modules, modulators and macros are `.box.col` on `--s2` in a `.lvmods` grid (auto-fill, min 320 px). Lanes and
   the post chain have a 3 px `--tx` rule on the left. Changes play as they are made. Discard, the close button
   and Escape ask first when anything changed, with a small `dlg` ("Discard changes?": Keep editing, Discard;
   focus on Discard, Escape keeps editing), then put the preset back as it was when the editor opened.
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

## Smart tools (Settings), Split into stems engines, Text to speech engine menu

**Smart tools** (`prefs-models`, `apps/desktop/ui/smart.js`) is a Settings page built with `Flux.pref` like
Privacy and Storage. Nearest screens copied: the voice cloning Models list (above) and Settings > Privacy. Top: a
dim lede with the used size in mono, a `callout chip` naming the graphics chip and memory. Then one `sec` per role
(Stems, Transcripts, Text to speech): the reason for the suggestion in dim text, then a `lst box` with one kit list
row per engine (`act` icon: `chip` built in, `disk` model, `globe` online), `ti` with name, licence and size or
provider and Free / Needs a key, the note, then `pill`s for the tier (Light, Balanced, Best, Specialist, Online),
`pill ok` "Suggested here", `pill` Free and Text only. On the right: In use (`btn p`, `aria-pressed`), Use,
Download or Resume (primary when suggested), Turn on (`btn` with `globe`), or a ghost trash `btn`. A `sel` for the
voice or the language sits under its role. **Online services** is a `lst box` with one row per provider: `lock`
icon, where the key comes from, and Add key or Forget key. Dialogs (`smart-get`, `smart-cloud`, `smart-key`) are the
kit `dlg` shaped like the cloning Download dialog: a `lst box` row, then `callout ac` (download) or `callout warn`
with the globe saying what is sent where, a password `inp` for a key with where to get one in mono, and a foot of
ghost Cancel and primary action. No new CSS.

**Split into stems** keeps the prototype dialog; the tiles now come from the installed engines (built-in, Balanced,
Best), and a fourth `seg` option "6 stems" appears when the six-stem model is downloaded. **Text to speech** keeps
the prototype; the aside gains a `sec('Engine')` with a `sel` and a "Run on" `pill`.

**Transcript editor** and **Filler review** (`script`, `fillers`) are the prototype screens from
`design/screens-c.js`, wired to real data: words are `.wd` spans (`fl` filler, `low` low confidence, `cur` selected),
the aside shows the selection with Delete and Listen, Accuracy, and "Transcribed with" plus an On device or Online
`pill`. When there is no transcript, the page is the kit `pg` with a lede and Make a transcript. Out-of-date
transcripts show a `callout warn` and disable Delete and Remove until updated.

**Checked:** 1440, 820 and 390 px with no sideways scrolling, and at 390 px in all 13 themes and 6 fonts; every
control named and inside a `data-tip` (`tests/e2e/test_smart_tools.py`, screenshots `target/e2e/smart-*.png`,
`transcript-*.png`, `fillers-*.png`, `stems-setup-1440.png`, `tts-engine-1440.png`).

## Live: Flux speaker and microphone

Sits under the two route cards, a `box col` shaped like them: a `h3` title and a dim 13 px note on the left, and on
the right a `pill ok` "Ready" or a `pill` "Not set up", then Add them (`btn p`) or Remove (ghost, trash) where Flux
makes the devices itself. In the route cards, Sound from gains "Flux speaker: apps you send to it" first, Play to
gains "Your usual output" (computer sound) and "Flux microphone: for call apps" (microphone). A dim 12.5 px line at
the bottom of the card says which device to pick in other apps. Virtual devices are never offered as a place to
listen. No new CSS. **Checked:** 1440, 820 and 390 px with no sideways scrolling (`tests/e2e/test_flux_devices.py`,
screenshots `target/e2e/flux-devices-*.png`).

**Virtual cable guide** (`live-cable`) is a kit `dlg wide` over Live, with the editor's sticky foot (`stickfoot`). A row
of `chip`s picks the OS (this computer first selected), then a `callout warn` "Not installed" or a `callout check` with
what was found, a dim line on what it is, a `lst box` of numbered steps (`act mono` number, `ti` with the title in bold,
links as `btn` with `globe`, commands in mono with a ghost Copy), and a dim line on one cable and removal. Foot: ghost Done, primary Check
again. When nothing is installed, the panel shows `pill warn` "Not installed" and each route card a dim line with a
ghost How to install. **Checked:** 1440, 820 and 390 px for each OS (`virtual-cable-guide-*.png`).

## Download log (a running download's row)

The row on Smart tools and Voice cloning gains **Show log**, a ghost button before Cancel, built with `Flux.btn`. It
opens a `pre.mono.box` under the row, like the release notes box in the Update ready dialog. Under it a `.dim` line
appears after 20 s without output. While a job runs only the row's meter and text change; the page isn't redrawn.
Code: `dlRow`, `dlPatch` and `dlToggle` in `apps/desktop/ui/app-core.js`.

## Editor: typed values, timeline zoom, clip position

**Typed values.** Clicking a slider's value swaps it for a small `inp mono svin` (24 px high, right-aligned, in the
same spot). A fader's dB line does the same. Enter applies, Esc cancels, leaving the box applies. A value that can't
be read gets a 2 px `--sig` outline. A dotted underline on hover marks a value as editable.

**Timeline bar.** The single zoom button becomes ghost minus, a mono readout such as `2.4×`, ghost plus, and ghost
Fit, at the end of the existing `tlbar`. `--lw` on `.tl` sets the lane width from `--z`.

**Inspector.** With a clip selected, the Start and Len text becomes two `fld`s with mono `inp`s in the same `g2`, with
a dim 12.5 px line on the arrow keys under them, and a ghost `x` Deselect button beside the track pill.

**Motion.** A redraw of the same screen adds `.rr` to the stage, which stops `.in` entrances and `.prog` growth from
playing again. They still play when arriving at a screen. Checked at 1440, 820 and 390 px in the Cassette theme
(`tests/e2e/test_editor.py`, screenshots `target/e2e/editor-timeline-*.png`). Other themes and fonts use the same tokens.

## Work in progress: Text to speech, Jobs, busy states

**Text to speech.** Under the button, the download row from Smart tools (`Flux.dlRow`) with a ghost danger Cancel; the
button reads Generating… and is disabled. A failure is a `callout bad` in the same place. **Jobs.** A running row adds
the ghost Show log button before Cancel and the current step in its dim line. **Busy.** A button or chip waiting on its
work takes `aria-busy` (55% opacity, progress cursor). A page taking over 200 ms to measure shows a 3 px `--sig` shimmer
across the top of the window. Checked at 1440, 820 and 390 px (`tests/e2e/test_jobs.py`).

## Work in progress: Assets and listening first

**Assets** (rail item in both rails, between the first two groups; box icon drawn at the rail's 24 px and stroke) copies `pg`/`pr` with
`pgh`. Section Generated holds **take cards**: a `box col` with a `lst` title row and Synthetic pill, the **player**, then a row of
`select.inp` (project), primary Add to assets, ghost danger Discard. Section In a project is a project `select`, Add a file, and a `lst box`
of asset rows: ghost icon Listen, title and dim line, Add to timeline, ghost icon trash. The same rows appear in the Edit panel (six, newest
first) and the empty project page. **Player**: Listen button, `.plbar` (a `.prog` bar in a 24 px hit area, `role=slider`, arrows move 5 s),
mono time. The take card follows Text to speech and the clone Script box. Text to speech has the clone page's tag chips, copied.
Checked at 1440, 820 and 390 px (`tests/e2e/test_assets.py`).
