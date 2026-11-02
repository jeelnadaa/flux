# DESIGN.md: How Flux Looks and Moves

Confirmed by the user on 2026-11-03. **The prototype in `design/` is the specification.** Production UI must be a 1:1 replica of it. Where this file and the prototype disagree, the prototype wins for looks and this file wins for rules.

## See it

```bash
python -m http.server 4173 --directory design
```

| Page | What it is |
|---|---|
| http://localhost:4173/landing.html | The landing page |
| http://localhost:4173/app.html | Every app screen. Add `#screen-id` to open one |
| `app.html#kit` | Component kit: buttons, sliders, cards, type, every shared part |
| `app.html#prefs-themes` | All 13 themes |
| `app.html#prefs-fonts` | All 6 fonts |

The bar across the top of `app.html` (`#rv`, with Desktop, Tablet, Phone and the screen arrows) is review chrome. Do not ship it. In the product, themes and fonts are chosen on the Settings screens.

## What 1:1 means

Port the prototype's code. Do not redraw it from memory or from screenshots.

- **Reuse the CSS.** Move `design/app.css`, `design/themes.css` and `design/fonts.css` into the app as they are. Split them into modules if you want, but keep every value: sizes, gaps, radii, weights, tracking, easings and durations.
- **Keep the markup shape.** Turn the template strings in `design/app-core.js` and `design/screens-*.js` into React components with the same elements, classes, order and copy.
- **Keep the visuals.** Port the behavior of `design/viz.js` for waveforms, spectrograms, curves and the stage. Replace its seeded fake data with real data, and keep the colors and drawing style.
- **Keep the words.** Use the prototype's copy, moved into i18n strings.
- **Add no UI library.** No Tailwind, shadcn, MUI or other component kit, and no default browser styling. They change the look.
- **Prove it.** For each screen you build, screenshot the prototype and your screen at 1440, 820 and 390 px in the same theme and font, and compare them with a pixel-diff test. Differences must be explained by real data replacing sample data. If you can't explain a difference, it's a bug.

**The only allowed changes** are real data, real behavior, accessibility fixes, and bugs found in the prototype. Record each deviation in `PROGRESS.md` with the reason. Anything else needs the user's approval.

## Character

Flux looks like its effects: flat, loud grounds and heavy type that reacts to sound. It should never look like a generic dashboard.

- Flat color grounds. No gradients as decoration, no glass, no soft shadows on cards.
- `1.5px` solid ink rules, ink pills, and inverse blocks where ink becomes the ground.
- Heavy display type, with mono for every number, unit and small label.
- Solid color blocks wherever a plain panel would be easy.
- Short sentences in plain words. Name what the thing does.

## Type

All fonts are bundled in `design/fonts/` and declared in `design/fonts.css`. The app makes no font requests to other hosts.

| Font | `data-font` | Role | Axes | Licence |
|---|---|---|---|---|
| **Bricolage Grotesque** | `bricolage` | **Default**, plus the logo and the whole landing page | Weight 200-800, width 75-100, optical size 12-96 | SIL OFL 1.1 |
| Satoshi | `satoshi` | Option | Weight 300-900 | ITF Free Font License (review queued) |
| Archivo | `archivo` | Option | Weight 100-900, width 62-125 | SIL OFL 1.1 |
| Schibsted Grotesk | `schibsted` | Option | Weight 400-900 | SIL OFL 1.1 |
| Geist | `geist` | Option | Weight 100-900 | SIL OFL 1.1 |
| Anybody | `anybody` | Option | Weight 100-900, width 50-150 | SIL OFL 1.1 |
| Geist Mono | none | Always: numbers, units, meta labels, timecode | Weight 100-900 | SIL OFL 1.1 |

Rules:

- Set the choice with `data-font` on the root. Save it in `localStorage` under `flux-font`. No attribute means Bricolage Grotesque.
- Apply a font to one block with class `ft-<id>`.
- Never name a font in a component. Use `var(--sans)`, `var(--mono)` and `var(--brand)`.
- The logo always uses `--brand`. The landing page always uses Bricolage Grotesque; only the app follows the setting.
- Display type uses the per-font tokens: `--trk` (added to tracking), `--dsz` (size scale), `--kin-lo` and `--kin-hi` (kinetic weight range). Any new display style must use them, like `.h1` in `app.css`.
- Load the current font before splitting display type into letters. Letter widths are measured once and locked in `em`.

## Themes

There are 13 themes, one for each effect card on the landing page. **Cassette is the default.** Set one with `data-flux` on the root and save it under `flux-theme`. Apply one to a block with class `pal-<id>`. Never use the `data-theme` attribute for this.

| Theme | `data-flux` | Ground | Ink | Signal | Accent text |
|---|---|---|---|---|---|
| Cassette (default) | `cassette` | `#DCCBA3` | `#1E1A10` | `#B8350A` | `#8A2606` |
| Slowed | `slowed` | `#142A6E` | `#CFE0FF` | `#FFB84D` | `#9DBDFF` |
| Nightcore | `nightcore` | `#FF2E78` | `#17030B` | `#FFF04D` | `#47001E` |
| Lo-fi | `lofi` | `#C8A179` | `#2A1A0C` | `#7A2400` | `#5C1B00` |
| Vinyl | `vinyl` | `#111013` | `#EDE6D6` | `#FF5A1C` | `#FF8A5C` |
| Underwater | `underwater` | `#04424C` | `#9CF0E4` | `#FFB38A` | `#6FE9D7` |
| Old radio | `radio` | `#E8A321` | `#1B1000` | `#A81C08` | `#6E1500` |
| Stutter | `stutter` | `#F4F1E6` | `#0B0B10` | `#E63900` | `#B82E00` |
| Tape stop | `tapestop` | `#D4342A` | `#FFF6F1` | `#FFD84D` | `#FFFAD9` |
| Reverse | `reverse` | `#E7E3FF` | `#21115C` | `#6B2CF5` | `#4B1FC2` |
| Cathedral | `cathedral` | `#17140F` | `#F2E7CF` | `#E8B04A` | `#E8B04A` |
| Next door | `nextdoor` | `#8B8C86` | `#111210` | `#7A1400` | `#3D0A00` |
| Robot | `robot` | `#0D2B21` | `#7CFFB2` | `#FFE14D` | `#7CFFB2` |

`design/themes.py` generates `themes.css` and `themes.json`, and it exits 1 on any contrast miss. Never edit `themes.css` by hand and never write a literal color in a component. The full token set for every theme is in `themes.json`:

| Tokens | Use |
|---|---|
| `--ground`, `--ink` | The theme's two defining colors |
| `--s0` to `--s4` | Surfaces, from the ground outward |
| `--tx`, `--tx2`, `--tx3` | Text, strongest to quietest |
| `--ln`, `--ln2` | Quiet lines |
| `--ac`, `--ac-ink`, `--ac-t`, `--ac-soft` | Actions: fill, text on fill, accent text, soft hover |
| `--sig`, `--sig-on`, `--sig-i` | Signal color: on the ground, text on it, and on ink blocks |
| `--ok`, `--warn`, `--clip` | Status |
| `--t1` to `--t8`, `--t1-on` to `--t8-on` | Track colors, and text on them |

## Motion

Motion is part of the design. Don't skip it or replace it with a simpler version.

| Motion | How | Values |
|---|---|---|
| Spring | CSS `linear()` | `--spring` in `app.css` |
| Standard easing | CSS | `--ez`: `cubic-bezier(.16,1,.3,1)` |
| Screen change | View Transitions | Old `.28s`, new `.5s`, using `--ez` |
| Theme or font change | Circular clip-path reveal from the click point | `950ms`, see `reveal()` in `app.js` |
| Kinetic display type | Per-letter weight and `scaleY` in `requestAnimationFrame`, reacting to pointer and playback | `tick` in `app.js` |
| Entrances | Staggered children in `.in` | `--n` per child |
| Segmented controls | Ink pill slides, with no animation on first paint | `placeInd()` in `app.js` |
| Logo | The "l" bar pulses between `scaleY(.88)` and `1.12` | Never lower: below that it reads "fiux" |

Landing motion lives in `design/landing.js`: GSAP 3.12.5 with ScrollTrigger, Lenis 1.1.13, a WebGL2 hero, canvas scenes and a WebAudio HRTF synth. Create ScrollTrigger pins in page order.

- Respect `prefers-reduced-motion`: show the final state with no kinetic loops.
- Skip view transitions while the page is hidden, and catch their rejected promises.
- Animate only `transform`, `opacity`, `clip-path`, and font weight on display type. Hold 60 fps.

## Landing sound

Sound is on by default. Browsers block audio until the first click, tap or key press, so it starts on that gesture and a hint shows until then. The button label shows the current state: "Sound on" when it's on. The choice is saved under `flux-sound`.

## Layout

The app is a grid: top bar, left rail, main area and transport. Layout responds to its own width with container queries on `#stage`, so check 1440, 820 and 390 px. The page body must never scroll sideways. Only tables, timelines and mixers may scroll inside their own containers.

## Every screen

The prototype has 87 app screens plus the landing page. Each id opens at `app.html#id`.

**First run** (6): `boot` Checking your browser, `unsupported` Limited browser, `welcome` Welcome and mode, `hpcheck` Headphone check, `hrtf` Pick your head shape, `storage-ask` Keep work safe

**Home** (8): `home` Home, `new` New project and templates, `empty` Empty project, `library` Projects, `import` Importing a file, `record` Record, `task` Simple mode: Make it 8D, `task-slowed` Simple mode: Slowed + reverb

**Workspaces** (5): `edit` Edit: timeline, `automation` Edit: automation lanes, `spectral` Spectral: waveform and repair, `mix` Mix: mixer, `history` Visual history

**Effects** (9): `fxbrowser` Effect browser, `rack` Effect chain, `fx-eq` Equalizer, `fx-comp` Dynamics, `fx-time` Speed and pitch, `fx-clean` Noise, hum and clicks, `fx-silence` Silence remover, `fx-reverb` Reverb and delay, `meters` Meters

**Spatial** (4): `stage` Sound stage: 3D and 8D, `rooms` Rooms, `roomdesign` Room designer, `hall` Row 10 and Next door

**Creative** (4): `creative` Slowed, nightcore, lo-fi, `textures` Textures and media, `glitch` Glitch and rhythm, `voice` Voice changer

**AI** (16): `aitools` AI tools, `magic` Magic, `stems-setup` Stems: set up, `stems-run` Stems: processing, `stems` Stems: results, `cleanup` Voice cleanup, `script` Transcript editor, `fillers` Filler word review, `analysis` Tempo and key, `master` Auto-master, `tts` Text to speech, `dub` Translate and dub, `dubreview` Dub review, `clone` Voice clone consent, `consent` Cloud consent, `jobs` Jobs

**Output** (5): `export` Export, `exporting` Exporting, `exported` Export finished, `export-sub` Export stems, subtitles, project, `export-project` Save project file

**Tools** (5): `convert` Format converter, `batch` Batch processing, `ringtone` Ringtone maker, `macros` Actions, `presets` Presets

**Settings** (11): `prefs-themes` Themes, `prefs-fonts` Fonts, `prefs-general` General, `prefs-audio` Audio, `prefs-keys` Keyboard shortcuts, `prefs-privacy` Privacy, `prefs-models` AI models, `prefs-storage` Storage, `prefs-access` Accessibility, `prefs-account` Account, `prefs-about` About

**Help** (6): `help` Guides, `trouble` Troubleshooting, `diag` Diagnostic report, `legal` Privacy policy, `legal-terms` Terms, `legal-credits` Attributions

**System** (8): `kit` Component kit, `palette` Command palette, `shortcuts` Shortcut sheet, `recovery` Crash recovery, `relink` Missing media, `quota` Storage almost full, `error` Export failed, `offline` Working offline

## When something is not in `design/`

Never improvise a screen, state or component. Plain defaults and generic dashboard styling are defects.

1. Find the nearest existing screen and copy its layout, spacing and motion.
2. Build only from the kit (`app.html#kit`) and the tokens. New tokens go in `themes.py`, which must still pass its contrast checks.
3. Match the traits under "Character".
4. Write in the same plain voice.
5. Check it in all 13 themes and all 6 fonts at 1440, 820 and 390 px. It must have no horizontal overflow, keep readable contrast, and work with reduced motion on.
6. Screenshot it beside the nearest existing screen. If it looks plainer or quieter than its neighbor, it isn't finished.
7. Add the new screen to `design/` so the next one has something to copy, and note it in `PROGRESS.md`.

If a design decision would change how the product behaves or what it means, stop and ask the user.

## Accessibility

WCAG 2.2 AA applies in every theme and font. Everything is reachable by keyboard with a visible focus state. Kinetic letters stay readable to screen readers through `aria-label` on the heading.
