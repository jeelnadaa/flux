"""Generates design/themes.css from the 13 landing-card palettes and fails on any contrast miss.
Run: python design/themes.py   Every token is a literal value, so a theme can also be applied to any block with class pal-<id>."""
import math, json, os, sys

# id, name, ground, ink, signal, accent text. Grounds are the landing wall's card colors, unchanged.
THEMES = [
    ("cassette", "Cassette", "#DCCBA3", "#1E1A10", "#B8350A", "#8A2606"),
    ("slowed", "Slowed", "#142A6E", "#CFE0FF", "#FFB84D", "#9DBDFF"),
    ("nightcore", "Nightcore", "#FF2E78", "#17030B", "#FFF04D", "#47001E"),
    ("lofi", "Lo-fi", "#C8A179", "#2A1A0C", "#7A2400", "#5C1B00"),
    ("vinyl", "Vinyl", "#111013", "#EDE6D6", "#FF5A1C", "#FF8A5C"),
    ("underwater", "Underwater", "#04424C", "#9CF0E4", "#FFB38A", "#6FE9D7"),
    ("radio", "Old radio", "#E8A321", "#1B1000", "#A81C08", "#6E1500"),
    ("stutter", "Stutter", "#F4F1E6", "#0B0B10", "#E63900", "#B82E00"),
    ("tapestop", "Tape stop", "#D4342A", "#FFF6F1", "#FFD84D", "#FFFAD9"),
    ("reverse", "Reverse", "#E7E3FF", "#21115C", "#6B2CF5", "#4B1FC2"),
    ("cathedral", "Cathedral", "#17140F", "#F2E7CF", "#E8B04A", "#E8B04A"),
    ("nextdoor", "Next door", "#8B8C86", "#111210", "#7A1400", "#3D0A00"),
    ("robot", "Robot", "#0D2B21", "#7CFFB2", "#FFE14D", "#7CFFB2"),
]
DEFAULT = "cassette"
TRACK_HUES = [265, 35, 180, 85, 320, 140, 5, 225]


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))


def hexs(c):
    return "#%02X%02X%02X" % tuple(round(min(1, max(0, v)) * 255) for v in c)


def mix(a, b, t):
    """a blended toward b by t, in sRGB, the same way CSS color-mix does it."""
    ca, cb = rgb(a), rgb(b)
    return hexs(tuple(x + (y - x) * t for x, y in zip(ca, cb)))


def lum(h):
    f = lambda v: v / 12.92 if v <= .03928 else ((v + .055) / 1.055) ** 2.4
    r, g, b = map(f, rgb(h))
    return .2126 * r + .7152 * g + .0722 * b


def cr(a, b):
    la, lb = lum(a), lum(b)
    return (max(la, lb) + .05) / (min(la, lb) + .05)


def oklch(L, C, H):
    a, b = C * math.cos(math.radians(H)), C * math.sin(math.radians(H))
    l, m, s = L + .3963377774 * a + .2158037573 * b, L - .1055613458 * a - .0638541728 * b, L - .0894841775 * a - 1.2914855480 * b
    l, m, s = l ** 3, m ** 3, s ** 3
    lin = (4.0767416621 * l - 3.3077115913 * m + .2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - .3413193965 * s, -.0041960863 * l - .7034186147 * m + 1.7076147010 * s)
    if any(v < -.001 or v > 1.001 for v in lin):
        return None
    g = lambda v: 12.92 * v if v <= .0031308 else 1.055 * max(v, 0) ** (1 / 2.4) - .055
    return hexs(tuple(g(max(0, min(1, v))) for v in lin))


def fit(hue, chroma, surfaces, need, prefer):
    """The color nearest lightness `prefer` at this hue that clears `need` against every surface."""
    best, best_d = None, 9
    for i in range(0, 101):
        L = i / 100
        for c in (chroma, chroma * .8, chroma * .6, chroma * .4, chroma * .2, 0):
            col = oklch(L, c, hue)
            if col and all(cr(col, s) >= need for s in surfaces):
                d = abs(L - prefer) + (chroma - c) * .6
                if d < best_d:
                    best, best_d = col, d
                break
    return best


def dim(ink, ground, surfaces, need):
    """Pull the ink toward the ground as far as contrast allows."""
    for k in range(60, -1, -2):
        c = mix(ink, ground, k / 100)
        if all(cr(c, s) >= need for s in surfaces):
            return c
    return ink


def build(t):
    id_, name, g, ink, sig, act = t
    if lum(g) < .08:
        s = [g, mix(g, ink, .06), mix(g, ink, .10), mix(g, ink, .17), mix(g, ink, .28)]
    elif lum(ink) > lum(g):
        s = [g, mix(g, "#000000", .16), mix(g, "#000000", .30), mix(g, "#000000", .42), mix(g, "#000000", .54)]
    else:
        s = [g, mix(g, "#FFFFFF", .30), mix(g, "#FFFFFF", .50), mix(g, ink, .12), mix(g, ink, .24)]
    dark = lum(g) < lum(ink)
    pref = .78 if dark else .42
    core = s[:3]
    tok = {f"--s{i}": s[i] for i in range(5)}
    a = lambda c, o: c + "%02X" % round(o * 255)
    tok |= {"--ground": g, "--ink": ink, "--tx": ink, "--tx2": dim(ink, g, core, 5.2), "--tx3": dim(ink, g, core, 4.6), "--ln": a(ink, .2), "--ln2": a(ink, .42), "--ac": ink, "--ac-ink": g, "--ac-t": act, "--ac-soft": a(ink, .13), "--sig": sig}
    tok["--sig-on"] = "#FFFFFF" if cr("#FFFFFF", sig) >= cr("#0B0B10", sig) else "#0B0B10"
    tok["--sig-i"] = sig if cr(sig, ink) >= 3 else fit(40, .17, [ink], 3, .78 if lum(ink) < .3 else .4)
    tok["--ok"] = fit(155, .15, core, 4.5, pref)
    tok["--warn"] = fit(75, .15, core, 4.5, pref)
    tok["--clip"] = fit(25, .19, core, 4.5, pref)
    for i, h in enumerate(TRACK_HUES, 1):
        c = fit(h, .17, core, 3.1, pref)
        tok[f"--t{i}"] = c
        tok[f"--t{i}-on"] = "#FFFFFF" if cr("#FFFFFF", c) >= cr("#0B0B10", c) else "#0B0B10"
    tok["color-scheme"] = "dark" if dark else "light"
    return tok


def check(id_, t):
    bad = []
    def need(fg, surfaces, n):
        for sf in surfaces:
            if cr(t[fg], t[sf]) < n:
                bad.append(f"{id_}: {fg} on {sf} = {cr(t[fg], t[sf]):.2f} < {n}")
    need("--tx", ["--s0", "--s1", "--s2", "--s3"], 4.5)
    need("--tx2", ["--s0", "--s1", "--s2"], 4.5)
    need("--tx3", ["--s0", "--s1", "--s2"], 4.5)
    need("--ac-t", ["--s0", "--s1", "--s2"], 4.5)
    need("--sig", ["--s0"], 3)
    for k in ("--ok", "--warn", "--clip"):
        need(k, ["--s0", "--s1", "--s2"], 4.5)
    for i in range(1, 9):
        need(f"--t{i}", ["--s0", "--s1", "--s2"], 3)
        if cr(t[f"--t{i}"], t[f"--t{i}-on"]) < 4.5:
            bad.append(f"{id_}: --t{i}-on = {cr(t[f'--t{i}'], t[f'--t{i}-on']):.2f} < 4.5")
    if cr(t["--sig"], t["--sig-on"]) < 4.5:
        bad.append(f"{id_}: --sig-on = {cr(t['--sig'], t['--sig-on']):.2f} < 4.5")
    if cr(t["--sig-i"], t["--ink"]) < 3:
        bad.append(f"{id_}: --sig-i on ink = {cr(t['--sig-i'], t['--ink']):.2f} < 3")
    return bad


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    css, data, bad = ["/* Generated by themes.py. Do not edit by hand. Default theme: " + DEFAULT + ". Use data-flux on the root, or class pal-<id> on any block. */"], {}, []
    for t in THEMES:
        tok = build(t)
        bad += check(t[0], tok)
        data[t[0]] = {"name": t[1], "tokens": tok}
        decl = ";".join(f"{k}:{v}" for k, v in tok.items())
        sel = f':root[data-flux="{t[0]}"],.pal-{t[0]}'
        if t[0] == DEFAULT:
            sel = ":root," + sel
        css.append(f"{sel}{{{decl}}}")
    open(os.path.join(here, "themes.css"), "w", encoding="utf-8").write("\n".join(css) + "\n")
    open(os.path.join(here, "themes.json"), "w", encoding="utf-8").write(json.dumps(data, indent=1) + "\n")
    print(len(THEMES), "themes written")
    if bad:
        print("\n".join(bad))
        sys.exit(1)
    print("all contrast checks pass")


main()
