# Flux shim for num2words (LGPL), which Kokoro's English reader imports to say numbers. Flux installs this instead:
# English cardinals, ordinals, years and decimals. Flux code, MIT.

_ONES = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve",
         "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen"]
_TENS = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"]
_BIG = [(10 ** 12, "trillion"), (10 ** 9, "billion"), (10 ** 6, "million"), (1000, "thousand")]
_ORD = {"one": "first", "two": "second", "three": "third", "five": "fifth", "eight": "eighth", "nine": "ninth",
        "twelve": "twelfth"}


def _under_1000(n):
    out = []
    if n >= 100:
        out.append(_ONES[n // 100] + " hundred")
        n %= 100
        if n:
            out.append("and")
    if n >= 20:
        out.append(_TENS[n // 10] + ("-" + _ONES[n % 10] if n % 10 else ""))
    elif n or not out:
        out.append(_ONES[n])
    return " ".join(out)


def _cardinal(n):
    if n < 0:
        return "minus " + _cardinal(-n)
    if n < 1000:
        return _under_1000(n)
    out = []
    for size, name in _BIG:
        if n >= size:
            out.append(_cardinal(n // size) + " " + name)
            n %= size
    if n:
        out.append(("and " if n < 100 else "") + _under_1000(n))
    return " ".join(out)


def _ordinal(n):
    words = _cardinal(n)
    head, sep, last = words.rpartition(" ")
    if "-" in last:
        a, _, b = last.rpartition("-")
        last = a + "-" + _ORD.get(b, b + "th")
    elif last in _ORD:
        last = _ORD[last]
    elif last.endswith("y"):
        last = last[:-1] + "ieth"
    else:
        last += "th"
    return head + sep + last


def _year(n):
    if n < 0:
        return _year(-n) + " BC"
    hi, lo = divmod(n, 100)
    if n < 1000 or hi % 10 == 0 and lo < 10 and hi >= 20:
        return _cardinal(n).replace(" and ", " ")
    if lo == 0:
        return _cardinal(hi) + " hundred"
    return _cardinal(hi) + " " + (("oh " + _ONES[lo]) if lo < 10 else _cardinal(lo))


def num2words(number, lang="en", to="cardinal", **_kwargs):
    if isinstance(number, str):
        number = float(number) if "." in number else int(number)
    if to == "ordinal_num":
        n = int(number)
        suf = "th" if 10 <= n % 100 <= 20 else {1: "st", 2: "nd", 3: "rd"}.get(n % 10, "th")
        return f"{n}{suf}"
    if isinstance(number, float) and not number.is_integer():
        whole, _, frac = repr(abs(number)).partition(".")
        sign = "minus " if number < 0 else ""
        return sign + _cardinal(int(whole)) + " point " + " ".join(_ONES[int(d)] for d in frac)
    n = int(number)
    if to == "ordinal":
        return _ordinal(n)
    if to == "year":
        return _year(n)
    return _cardinal(n)
