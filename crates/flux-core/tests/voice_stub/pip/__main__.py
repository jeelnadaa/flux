# Test stand-in for pip: records what Flux asked to install, prints pip-like lines, installs nothing.
import json
import os
import sys

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
print("Successfully installed (stub)")
