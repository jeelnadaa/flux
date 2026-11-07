# Soundtrack for the Flux launch video: 120 BPM, A minor, 22 s. Everything sits on one beat grid from frame 0.
import numpy as np
from scipy.signal import butter, sosfilt, fftconvolve
from scipy.io import wavfile

SR = 48000
DUR = 22.0
N = int(SR * DUR)
BEAT = 0.5
rng = np.random.default_rng(11)
L = np.zeros(N); R = np.zeros(N)

def hz(m): return 440.0 * 2 ** ((m - 69) / 12)
def env(n, a=0.005, d=0.3, s=0.0, rel=0.05, hold=None):
    t = np.arange(n) / SR
    e = np.minimum(1, t / a) * (s + (1 - s) * np.exp(-t / d))
    if hold is not None:
        e *= np.clip((hold + rel - t) / rel, 0, 1)
    return e
def add(sig, t0, gl=1.0, gr=None):
    gr = gl if gr is None else gr
    i = int(t0 * SR)
    if i >= N: return
    n = min(len(sig), N - i)
    L[i:i + n] += sig[:n] * gl; R[i:i + n] += sig[:n] * gr
def lp(x, f, o=2): return sosfilt(butter(o, f / (SR / 2), 'low', output='sos'), x)
def hp(x, f, o=2): return sosfilt(butter(o, f / (SR / 2), 'high', output='sos'), x)
def bp(x, lo, hi): return sosfilt(butter(2, [lo / (SR / 2), hi / (SR / 2)], 'band', output='sos'), x)

def kick(g=1.0):
    n = int(0.45 * SR); t = np.arange(n) / SR
    f = 45 + 110 * np.exp(-t * 32)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t * 7) * g
def clap(g=1.0):
    n = int(0.3 * SR); x = rng.standard_normal(n)
    e = np.zeros(n)
    for k, o in enumerate([0, 0.011, 0.022]):
        i = int(o * SR); e[i:] += np.exp(-np.arange(n - i) / SR * (60 if k < 2 else 18))
    return bp(x * e, 900, 5000) * 0.5 * g
def hat(g=1.0, open_=False):
    n = int((0.25 if open_ else 0.06) * SR); x = rng.standard_normal(n)
    return hp(x, 7000) * np.exp(-np.arange(n) / SR * (14 if open_ else 70)) * 0.22 * g
def tone(m, dur, kind='saw', g=1.0, a=0.005, d=0.4, s=0.0, cut=None):
    n = int(dur * SR) + int(0.3 * SR); t = np.arange(n) / SR; f = hz(m)
    if kind == 'saw':
        x = sum(np.sin(2 * np.pi * f * k * t) / k for k in range(1, 14)) * 0.6
    elif kind == 'tri':
        x = sum(((-1) ** ((k - 1) // 2)) * np.sin(2 * np.pi * f * k * t) / (k * k) for k in range(1, 12, 2))
    elif kind == 'bell':
        x = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(2 * np.pi * f * 2.76 * t) * np.exp(-t * 6)
    else:
        x = np.sin(2 * np.pi * f * t)
    x = x * env(n, a, d, s, 0.08, dur) * g
    return lp(x, cut) if cut else x
def pad(ms, dur, g=1.0):
    n = int(dur * SR) + int(0.6 * SR); t = np.arange(n) / SR
    x = np.zeros(n)
    for m in ms:
        for det in (-0.08, 0.08):
            f = hz(m + det)
            x += sum(np.sin(2 * np.pi * f * k * t + k) / k for k in range(1, 8))
    e = np.minimum(1, t / 0.35) * np.clip((dur + 0.5 - t) / 0.5, 0, 1)
    return lp(x * e, 1800) * 0.035 * g
def riser(t0, t1, g=1.0):
    n = int((t1 - t0) * SR); t = np.arange(n) / SR; p = t / (t1 - t0)
    x = rng.standard_normal(n)
    x = sosfilt(butter(2, [300 / (SR / 2), 6000 / (SR / 2)], 'band', output='sos'), x)
    x = x * p ** 2.2 * 0.16 * g
    add(x, t0, 0.9, 0.9)
def boom(t0, g=1.0):  # a low A, in key, for scene changes
    n = int(1.6 * SR); t = np.arange(n) / SR
    f = hz(33) * (1 + 0.6 * np.exp(-t * 20))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t * 2.6) * 0.75 * g
    x += lp(rng.standard_normal(n), 400) * np.exp(-t * 9) * 0.12 * g
    add(x, t0)

CHORDS = [(57, [57, 60, 64]), (53, [53, 57, 60]), (48, [55, 60, 64]), (55, [55, 59, 62])]  # Am F C G
bar = lambda t: int(t // 2)

# Pad: the whole piece, quiet under the hook, then fuller. The last bar rings out on Am.
for b in range(11):
    root, ch = CHORDS[b % 4] if b < 10 else CHORDS[0]
    g = 0.7 if b == 0 else 1.0
    add(pad([m + 12 for m in ch], 2.0 if b < 10 else 2.0, g), b * 2.0, 0.85, 0.85)

# Hook: a hit on each line (0, 0.5, 1.0), then "Live, in 8D." at 2.0.
for i, t0 in enumerate([0, 0.5, 1.0]):
    add(kick(0.95), t0); add(tone(69 + [0, 3, 7][i], 0.35, 'tri', 0.22, d=0.25), t0, 0.8, 0.8)
add(clap(0.8), 1.5)
riser(1.5, 2.0, 0.6)
add(kick(1.0), 2.0); add(tone(81, 0.8, 'bell', 0.18, d=0.6), 2.0)

# The arpeggio: from 2.0 to 18.0. In the hook and Live scenes it circles the listener (8D).
ARP = [0, 3, 7, 12, 7, 3, 7, 10]
for k in range(int((22.0 - 2.0) / 0.25)):
    t0 = 2.0 + k * 0.25
    if 2.75 <= t0 < 3.25 or 17.8 <= t0 < 18.0 or t0 >= 21.0: continue
    root = CHORDS[bar(t0) % 4][0] if t0 < 20 else 57
    m = root + 12 + ARP[k % 8] - (0 if root < 55 else 12)
    g = 0.11 if t0 < 6 else 0.13
    if 10 <= t0 < 12: g = 0.07
    x = tone(m, 0.18, 'saw', g, d=0.12, cut=3200)
    if 2.0 <= t0 < 3.0 or 6.0 <= t0 < 10.0:
        ang = (t0 - 2.0) * 2 * np.pi * (0.75 if t0 < 3 else 0.55)
        pan = np.sin(ang); behind = np.cos(ang) < 0
        if behind: x = lp(x, 1600)
        gl, gr = np.sqrt((1 - pan) / 2), np.sqrt((1 + pan) / 2)
        d = int(abs(pan) * 0.0006 * SR)  # time difference between the ears
        xl = np.concatenate([np.zeros(d if pan > 0 else 0), x]); xr = np.concatenate([np.zeros(d if pan < 0 else 0), x])
        add(xl * gl * 2.4, t0, 1, 0); add(xr * gr * 2.4, t0, 0, 1)
    else:
        add(x, t0, 0.8, 0.8)

# Drums by scene.
for k in range(int(DUR / 0.25)):
    t0 = k * 0.25
    beat = k % 2 == 0; bi = int(round(t0 / BEAT))
    if 3.0 <= t0 < 6.0:
        if beat and bi % 2 == 0: add(kick(0.7), t0)
        if not beat: add(hat(0.6), t0, 0.7, 0.9)
    elif 6.0 <= t0 < 10.0 or 14.0 <= t0 < 20.0:
        if beat: add(kick(0.9), t0)
        if beat and bi % 2 == 1: add(clap(0.75), t0, 0.95, 0.95)
        add(hat(0.75 if not beat else 0.4, open_=(not beat and bi % 4 == 3)), t0, 0.75, 0.95)
    elif 10.0 <= t0 < 14.0:
        if t0 >= 12.0 and beat: add(kick(0.75), t0)
        if not beat: add(hat(0.45), t0, 0.8, 0.8)
        if beat and bi % 2 == 1 and t0 >= 11.0: add(clap(0.5), t0)

# Bass: eighth notes on the chord root, from the Live scene on.
for k in range(int(DUR / 0.25)):
    t0 = k * 0.25
    if not (6.0 <= t0 < 10.0 or 12.0 <= t0 < 20.5): continue
    root = CHORDS[bar(t0) % 4][0] if t0 < 20 else 57
    m = root - 24 + (12 if k % 4 == 3 else 0)
    add(tone(m, 0.2, 'saw', 0.32, a=0.004, d=0.16, s=0.2, cut=420), t0)

# Scene changes: risers in, booms on the cut.
for a, b in [(2.6, 3.0), (5.5, 6.0), (9.5, 10.0), (13.5, 14.0), (17.4, 18.0)]:
    riser(a, b, 0.8)
for t0 in [3.0, 6.0, 10.0, 14.0, 18.0]:
    boom(t0, 0.9)

# Clone scene: each tag turning into a pill plays a note of the chord.
TOTAL = 64
for end, m in [(5, 76), (33, 79), (50, 84)]:
    t0 = 10.55 + end / TOTAL * 1.55
    add(tone(m, 0.4, 'bell', 0.14, d=0.35), t0, 0.85, 0.85)
# Agent scene: a soft bell on each check mark.
for i in range(4):
    t0 = 14.75 + i * 0.62 + 0.45
    add(tone([72, 76, 79, 84][i], 0.3, 'bell', 0.1, d=0.25), t0, 0.8, 0.8)

# Outro: the Cassette iris at 19.5-20 and a final Am stab on 20.0 that rings out.
add(tone(81, 1.6, 'bell', 0.18, d=0.9), 20.0)
for m in [45, 57, 60, 64, 69]:
    add(tone(m, 1.8, 'saw', 0.06, a=0.01, d=1.2, s=0.15, cut=2400), 20.0)
add(kick(1.0), 20.0); add(hat(0.6, True), 20.0)

# One room for everything, then a gentle bus: glue, soft clip, fade at the end.
n = int(2.2 * SR); t = np.arange(n) / SR
irl = rng.standard_normal(n) * np.exp(-t * 3.2); irr = rng.standard_normal(n) * np.exp(-t * 3.2)
irl = lp(irl, 5000); irr = lp(irr, 5000)
wl = fftconvolve(L, irl)[:N]; wr = fftconvolve(R, irr)[:N]
sc = np.sqrt(np.mean(L ** 2) / max(np.mean(wl ** 2), 1e-12)) * 0.22
L2 = L + wl * sc; R2 = R + wr * sc
mix = np.stack([L2, R2], 1)
mix = hp(mix.T, 28).T
peak = np.max(np.abs(mix)); mix = mix / peak * 1.25
mix = np.tanh(mix) / np.tanh(1.25)
fade = np.clip((DUR - np.arange(N) / SR) / 0.9, 0, 1) ** 1.5
mix *= fade[:, None]
mix *= 10 ** (-1 / 20) / np.max(np.abs(mix))
rms = 20 * np.log10(np.sqrt(np.mean(mix ** 2)))
wavfile.write('music.wav', SR, (mix * 32767).astype(np.int16))
print(f'music.wav {DUR}s peak -1.0 dBFS rms {rms:.1f} dBFS')
