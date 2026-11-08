# ADR 0007: The Flux speaker and microphone

**Status:** accepted 2026-11-08. The user asked for a virtual microphone and speaker on every OS, and left the driver choice to the agent.

## Why

Capturing a real output by loopback can't stop the original playing, so the sound is heard twice: once as it was and once processed. The fix is a device apps play into that makes no sound of its own. Flux records it, processes it, and plays the result on a real output. A microphone other apps can pick needs the same kind of device.

## Decision

| OS | How | Own device? |
|----|-----|-------------|
| Linux | `pactl` loads a null sink "Flux speaker", a null sink "Flux microphone feed", and a remapped source "Flux microphone". Works on PipeWire and PulseAudio, needs no root. | Yes |
| Windows | Uses an installed virtual audio cable, found by device name. | Not yet |
| macOS | Uses an installed virtual audio device (such as a 2-channel loopback), found by device name. | Not yet |

On Linux, while the computer-sound route runs, Flux makes the Flux speaker the system output and puts the old one back when the route stops. It does so only after the route has started. Live uses cpal's PulseAudio host on Linux (the `pulseaudio` crate, MIT), because the ALSA host doesn't list individual sinks and monitors. The editor keeps the default host.

On every OS: one cable serves as both devices one at a time, two cables serve one each. A virtual device is never offered as a place to listen, which prevents feedback.

## Why no driver of our own on Windows and macOS yet

- **Windows** only loads kernel audio drivers signed through Microsoft, which needs a paid EV code-signing certificate and a Hardware Dev Center account. Test-signing mode would mean asking users to lower a system security setting.
- **macOS** needs an AudioServerPlugIn installed system-wide. For a downloaded installer to open without warnings it needs an Apple Developer ID, which is paid. It also can't be built or tested on the machine this was written on.

AGENTS.md requires the user's approval for spending, so both are queued in PROGRESS.md. Flux already looks for devices named "Flux speaker" and "Flux microphone" first, so a driver can be added later without changing Live.

Virtual cable software isn't bundled: the common macOS option is GPL, and the common Windows option is licensed for end users only. The user installs one; Flux only opens it as an audio device.

## Consequences

- If Flux crashes during a Linux route, the Flux speaker stays the system output until the user changes it or removes the devices. A normal stop or exit restores it.
- On Windows and macOS the user sets the cable as the output themselves. Flux says which device to pick.
- With no virtual device on Windows or macOS, capturing a real output still plays the original too. Live says so.
