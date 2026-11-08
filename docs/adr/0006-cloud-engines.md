# ADR 0006: Online engines for stems, transcripts and text to speech

Date: 2026-11-08. Status: accepted.

## Context

The user asked for cloud models next to the local ones: keyed services where a key (or another token) is needed,
and free options that need no key. Flux's rule is on-device by default; cloud is opt-in per feature, and audio,
transcripts and file names are never logged or sent anywhere the person didn't choose.

## Decision

Online engines sit in the same per-job choice as built-in and downloaded engines (`ai.engines`, `ai.engine.set`).

| Job | Free, no key | With a key |
|---|---|---|
| Transcripts | Whisper Large v3 on a public Hugging Face Space (text only, no word timing) | Groq Whisper Large v3 Turbo, OpenAI Whisper, Deepgram Nova-3 (all with word timing) |
| Text to speech | Kokoro on a public Hugging Face Space (6 voices) | OpenAI speech, ElevenLabs Flash |
| Stems | A public two-stem split Space (vocals, instrumental) | Demucs on Replicate (four stems) |

- **Never by default.** Online engines are never suggested. Choosing one needs `allow_cloud: true`, which the app
  asks for after saying what is sent and where (`sends`). Using one that wasn't turned on is refused before any
  request is made.
- **Keys.** Read from an environment variable (`FLUX_<SERVICE>_KEY` first, then the service's usual name such as
  `OPENAI_API_KEY`), else from the system keychain through the `keyring` crate (Windows Credential Manager, macOS
  Keychain, Secret Service on Linux). Flux's settings only record that a key is in the keychain. Keys are never
  returned by any method, logged or included in an error. Where there is no keychain, Flux says so and points at
  environment variables.
- **What is sent.** Only what the job needs: 16 kHz mono 16-bit WAV for transcripts (in 10-minute pieces, under the
  25 MB limit most services have), 44.1 kHz stereo 16-bit WAV for stems, the text for speech. Errors keep the
  service's own message (200 characters at most) and never the request.
- **Addresses.** HTTPS only, or a server on this computer. Each service's address can be changed, which lets the
  OpenAI engines talk to any OpenAI-compatible server (a local one, or another provider). Keys are only sent to the
  service's own address, never to the file hosts results are fetched from.
- **Free engines** are shared public demos. They can sleep, be busy or change; Flux says so in their note and turns
  their errors into "try again or choose another engine".
- **Names.** The services are named as they are (OpenAI, Groq, Deepgram, ElevenLabs, Replicate, Hugging Face) so
  people can tell where their audio goes. This is identification, not branding or endorsement; no preset imitates a
  person or a product.

## Consequences

- Tests use a local mock of each service (request shape, auth header, answers, failures). Calls to the real
  services need the person's keys and their yes to send audio, so the first real run of each is in the human
  queue.
- The free Spaces and the Replicate model version are pinned in `cloud.rs`; if one disappears, its engine fails
  with a clear message and the others keep working. Updating them is a code change.
