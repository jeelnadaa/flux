# Connecting an agent to Flux (MCP)

`flux-mcp` exposes every Flux command as an MCP tool over stdio. Tool names are the method names with `.` replaced by
`_` (for example `project_get`, `media_import`, `export_start`).

## Modes

| Command | What it does |
|---|---|
| `flux-mcp` | Drives the running Flux app if it is open; otherwise runs the engine headless on the same data folder. |
| `flux-mcp --headless` | Always runs its own engine (no window, no sound by default). |
| `flux-mcp --attach` | Only drives the running app; tool calls fail while the app is closed. |
| `flux-mcp serve --port 7341 [--ui DIR] [--no-token]` | HTTP API on 127.0.0.1 (`POST /api/call {method, params}`), for scripts and tests. |

The app publishes its control port and a random token in `control.json` in the Flux data folder. Only programs
running as you can read it. The server listens on 127.0.0.1 only and rejects foreign `Host` and `Origin` headers.

## Client configuration

Run `flux-mcp config` to print a block with the absolute path of your binary. The general shape:

```json
{ "mcpServers": { "flux": { "command": "/path/to/flux-mcp", "args": [] } } }
```

- **Claude Code:** `claude mcp add flux -- /path/to/flux-mcp`
- **Claude Desktop, Cursor, VS Code and others:** add the block above to the client's MCP settings file.

The desktop app ships `flux-mcp` next to its executable. Settings → Agents (MCP) shows the exact path.

## Working with Flux

- Positions are in samples. Most position parameters also accept `<name>_s` in seconds (`at_s: 1.5`).
- Every change is undoable with `history_undo`, and `history_list` shows the stack.
- Long tasks (`media_import`, `tool_stems`, `export_start`, `batch_start`, `tts_speak`) return a job id. Pass
  `wait: true`, or poll `job_get {id, wait_s}`.
- Errors come back as tool results with `isError: true` and a code: `bad_params`, `not_found`, `empty`,
  `unavailable` or `internal`.
- `mcp_status` tells you whether you are driving the app or running headless.

Example session:

```text
media_import {path: "C:/audio/interview.wav", wait: true}
tool_cleanup {strength: 0.6}
tool_master_apply {target: -16}
export_start {format: "mp3", wait: true}
```

## Live

`live_*` tools run the Live page: computer sound or a microphone through a preset to any output.

```text
live_presets {}                                   # built-ins and yours
live_devices {}                                   # device ids
live_start {route: "output", preset: "out-8d-orbit", output: "<device id>"}
live_preset_create {from: "out-8d-orbit", name: "My orbit"}             # returns the new id
live_module_add {preset: "<new id>", kind: "phaser", lane: "post"}
live_render {preset: "<new id>", source: "demo-music", seconds: 8}      # audition offline
live_status {}                                    # levels, dropouts
```

Built-in presets are read-only: copy one with `live_preset_create {from}` first. `live_kinds` lists every effect,
parameter and modulator shape with a description.

## Voice cloning

`clone_*` tools need a consented voice. `clone_status` says which model this computer gets and whether it is
ready; `clone_models` lists every model with its size, licence, role and whether it is downloaded. Downloads need
the person's yes: without `confirm: true`, `clone_model_download` changes nothing and returns `confirm_required`
with the size, licence and folder to show them. Scripts use placeholders: sounds (`[laugh]`, `(sighs)`, `*chuckles*`), hesitations said in the voice
(`[hmm]`, `[um]`, or plain words) and pauses (`[pause]`, `[long pause]`, `[pause 2s]`). Check a script first:

```text
clone_tags {}
clone_script {text: "[hmm] So I opened it [laugh] [pause] and it was empty."}
clone_model_download {model: "chatterbox-turbo"}                 # confirm_required: ask the person first
clone_model_download {model: "chatterbox-turbo", confirm: true}  # job; installs the voice engine too
clone_settings_set {speech: "chatterbox-turbo", device: "auto", seed: 7, params: {"chatterbox-turbo": {temperature: 0.7}}}
clone_speak {voice: "robin", text: "...", wait: true}               # onto the Generated speech track
clone_convert {voice: "robin", path: "C:/me.wav", model: "chatterbox-vc"}   # your take, re-voiced, timing kept
clone_model_add_local {path: "D:/my-finetune", loader: "turbo"}      # a compatible folder of your own
clone_model_delete {model: "chatterbox"}                            # keeps files another model shares
```

`not_installed` says what to download; `cancelled` downloads resume when started again.

`consent_required` means the voice doesn't exist or the consent recording is missing.

## Privacy

Flux never uploads audio. Diagnostics (`system_diagnostics`) contain no audio, transcripts or file names.
