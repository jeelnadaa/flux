# AGENTS.md — How to Work on Flux

Read this file fully at the start of every session. It overrides your defaults.

Flux is a production-grade, non-destructive audio editor. **You are building the web app (PWA) only.** Quality is measured, not assumed.

| File | Purpose | When you touch it |
|------|---------|-------------------|
| [PLAN.md](PLAN.md) | What to build and how good it must be | Read the relevant phase before starting. Change it only with the user's approval. |
| [PROGRESS.md](PROGRESS.md) | Feature checklist, current focus, queues, decision log | Read first every session. Update on every successful feature. |
| [README.md](README.md) | What Flux does today, for humans | Update on every successful feature. |
| [DESIGN.md](DESIGN.md) | How Flux looks and moves; points at the prototype in `design/` | Read before any UI work. Change it only with the user's approval. |

## The rules

1. **Work in phases, in order.** Finish Phase 0 before Phase 1, and so on. The next task is the first unchecked, unblocked item in [PROGRESS.md](PROGRESS.md). Do not jump ahead because something looks more interesting.
2. **One feature at a time.** A feature is one PROGRESS.md item. Do not mix features in a commit.
3. **Test every feature properly before committing.** See "What properly tested means".
4. **If it passes, commit it. If it fails, do not commit it.**
5. **Never push.** No `git push`, no adding remotes, no pull requests. The user decides when anything leaves this machine.
6. **Update README.md and PROGRESS.md on every successful feature,** in the same commit as the code.
7. **Web app only.** Your work ends at Phase 9. Do not start the Windows app (Phase 10), create `apps/desktop`, or add Tauri until the user explicitly asks. Do not build native macOS, Linux or mobile apps at all. When Phase 9 is done, report back and stop.

## The work loop

For each feature:

1. **Orient.** Read PROGRESS.md, then the feature's section in PLAN.md, including its tests and exit criteria. Run `git status` and `git log --oneline -5`. If the working tree is dirty with work you do not recognize, stop and ask.
2. **Write the tests from PLAN.md.** Prefer writing them first. Each test must be able to fail: confirm it fails before the feature exists or when you break the feature on purpose.
3. **Implement** the smallest complete version of the feature, matching surrounding code.
4. **Run the feature's tests, then `just ci`** (the full local gate: format, lint, unit, property, golden, null, integration, E2E, license scan, WASM build).
5. **Exercise it for real.** Anything a user can see or hear must be run in the actual app, not only in unit tests. For UI, drive it with Playwright and look at a screenshot. For audio, measure the rendered output.
6. **On pass:** tick the item in PROGRESS.md, add a session log line, update README.md, and commit everything together.
7. **On failure:** fix the cause and go to step 4. After 3 honest attempts at the same failure, follow "When blocked".
8. **Next feature.**

## What properly tested means

Apply every layer that fits the feature. PLAN.md §5 and each phase's Tests section are the source of truth.

| Feature type | Required evidence |
|--------------|-------------------|
| DSP node or effect | Registered in the shared harness so all 13 universal invariants run (PLAN.md §5.2), plus the phase's specific measurements on real rendered samples |
| Decoder, encoder, file format | Round trip against reference tools, malformed-input cases, fuzz target |
| Editing command | Property test: undo then redo gives an identical project hash; exact sample positions |
| Storage or project format | Kill-mid-write recovery test, migration fixture, hostile-input test |
| UI | Playwright E2E of the real flow, keyboard path, axe-core with 0 serious violations, visual snapshot at desktop and phone widths, empty/loading/error states |
| AI feature | Chunking invariance, cancel, memory cap, fallback path, and metric on the available data; on-device mode makes zero network requests |
| Cloud path | Against the local mock, including failure, retry and deletion |

**These are never acceptable:**

- Deleting, skipping, or weakening a test to get a pass
- Loosening a threshold or regenerating a golden file without an ADR (PLAN.md §5.5)
- Mocking the thing under test
- Audio tests that assert only that code ran or a file exists
- Reporting a pass you did not observe in this session's output
- `--no-verify`, or disabling lint rules to get green

## Gates you cannot clear yourself

PLAN.md §5.4 defines four gate types. You clear **[A] Automated** only.

- **[D] Dataset:** run what is available, then add the full run to the "Human & hardware queue" in PROGRESS.md.
- **[R] Real device:** commit on proxy results and queue the real-device check.
- **[H] Human judgment** (listening panels, usability studies, legal review, pen-test): queue it.

Never simulate, estimate, or self-certify these. You have no ears: never write "sounds good". A queued gate does not block your commit, but the phase can only reach **Agent-complete**, not **Released**.

## Committing

Commit only when all tests are green, on the current branch. Initialize git in Phase 0 if the repo is missing.

```
feat(P1.1-a): decode WAV and AIFF, bit-exact against libsndfile

Tests: 42 unit, 6 property, 1 fuzz target; just ci green.
Queued: none.
```

- Type is `feat`, `fix`, `test`, `perf`, `refactor`, `docs` or `chore`. Scope is the PROGRESS.md item ID.
- Stage files by name. Check `git diff --staged` before committing.
- Never commit secrets, model weights, datasets, or large audio. Only small synthetic fixtures belong in git.
- Never rewrite history that is already committed (`--amend`, `rebase`, `reset --hard`, force anything) unless the user asks.

## Finishing a phase

When every item in a phase is ticked:

1. Run `just ci` and every exit criterion in that phase of PLAN.md. Fix or report anything missing.
2. Update the phase table in PROGRESS.md to **Agent-complete** (or **Released** if nothing is queued).
3. Rewrite the status section of README.md for what now works.
4. Commit, then create a local tag: `git tag phase-N-agent-complete`.
5. Tell the user what shipped, the measured numbers, and what is in the human queue. Then continue unless they asked you to stop.

## When blocked

After 3 honest attempts on the same failure, or when something external is missing:

1. Save partial work with `git stash push -u -m "blocked: <ID>"`. Failing code is not committed.
2. Mark the item `[!]` in PROGRESS.md and add it to "Blocked": what you tried, the exact error, and what would unblock it.
3. Commit that PROGRESS.md change and move to the next item that does not depend on it.

## Stop and ask the user first

- Any spending: paid APIs, cloud GPUs, licences, domains
- Adding a dependency or model whose licence is not clearly permitted (PLAN.md §4). Record every addition in `LICENSES.md`.
- Changing a decision in PLAN.md §1, or dropping or re-scoping a feature
- Anything leaving this machine: pushing, publishing, deploying, sending audio to a real external service
- Voice cloning, voice conversion, or anything imitating a real person
- Deleting user data or files you did not create

For a technical choice inside the plan's boundaries, decide yourself and record it as an ADR in `docs/adr/`.

## Non-negotiable engineering rules

Details are in PLAN.md §0 and §2.

- **Audio thread:** no allocation, locks, I/O, or logging. Parameters arrive by lock-free queue and are smoothed.
- **One engine:** the same DSP code runs in real time and offline. Output must null.
- **Non-destructive:** source audio is never modified. Every edit is an undoable command.
- **32-bit float internally.** Dither only at export. Sample-accurate positions, never pixels or milliseconds.
- **Deterministic:** randomness uses seeded PRNGs stored in the project.
- **Never lose work:** journal every committed command. Treat every opened file as hostile.
- **Privacy:** on-device by default. Cloud is opt-in per feature. Never log or transmit audio, transcripts, or file names.
- **No real people or trademarks** in presets or UI copy. All synthetic speech is labeled.
- **Accessibility and i18n from the first UI commit:** keyboard reachable, WCAG 2.2 AA, no hard-coded strings.

## Design

**Read [DESIGN.md](DESIGN.md) before touching any UI.** The prototype in `design/` is the specification. Every production screen must be a 1:1 replica of it, in layout, spacing, type, color, motion and copy.

- Port the prototype's CSS and markup. Do not redraw screens from memory. Do not add a UI library.
- For every screen, compare your build against the prototype with a pixel-diff at 1440, 820 and 390 px. Differences must be explained by real data replacing sample data. Any other difference is a bug, and the feature isn't done.
- Cassette is the default theme and Bricolage Grotesque is the default font. All 13 themes and all 6 fonts ship.
- If something isn't in `design/`, follow "When something is not in `design/`" in DESIGN.md. Never improvise.
- Do not change the design because something is easier another way. If it can't be done, stop and ask the user.

## Keeping state across sessions

Assume the next session remembers nothing. PROGRESS.md must always say what is done, what is next, what is blocked, and why. Keep log lines short and factual, with measured numbers rather than adjectives.
