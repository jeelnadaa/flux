# Test stand-in for pip: records what Flux asked to install, prints pip-like lines, installs nothing.
import json
import os
import sys
import time

log = os.environ.get("FLUX_STUB_LOG")
if log:
    with open(log, "a", encoding="utf-8") as f:
        f.write(json.dumps({"pip": sys.argv[1:]}) + "\n")
for a in sys.argv[1:]:
    if "==" in a:
        print(f"Collecting {a}")
if os.environ.get("FLUX_STUB_PIP_FAIL"):
    print("ERROR: No matching distribution found", file=sys.stderr)
    sys.exit(1)
pause = float(os.environ.get("FLUX_STUB_PIP_SLEEP", "0"))
print("Downloading stub_package-1.0-py3-none-any.whl (120.0 MB)", flush=True)
for part in range(1, 5):
    print(f"Progress {part * 30_000_000} of 120000000", flush=True)
    time.sleep(pause / 4)
print("Successfully installed (stub)")
