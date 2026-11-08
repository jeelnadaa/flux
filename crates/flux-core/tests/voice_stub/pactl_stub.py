# Test stand-in for pactl: a tiny PipeWire/PulseAudio that keeps its modules, sinks and default sink in a JSON file
# (FLUX_PACTL_STATE), and logs every command to FLUX_PACTL_LOG. Checks the arguments the way the server would.
import json
import os
import sys

STATE = os.environ["FLUX_PACTL_STATE"]


def load():
    if os.path.exists(STATE):
        with open(STATE, encoding="utf-8") as f:
            return json.load(f)
    return {"next": 500, "modules": [], "default": "alsa_output.real"}


def arg(args, key):
    for a in args:
        if a.startswith(key + "="):
            return a.split("=", 1)[1]
    return None


def main():
    a = sys.argv[1:]
    s = load()
    with open(os.environ["FLUX_PACTL_LOG"], "a", encoding="utf-8") as f:
        f.write(json.dumps(a) + "\n")
    sinks = ["alsa_output.real"] + [arg(m["args"], "sink_name") for m in s["modules"] if m["name"] == "module-null-sink"]
    sources = ["alsa_input.real", "alsa_output.real.monitor"] + [x + ".monitor" for x in sinks[1:]] + [arg(m["args"], "source_name") for m in s["modules"] if m["name"] == "module-remap-source"]
    if a == ["info"]:
        print("Server Name: PulseAudio (on PipeWire 1.2.0)\nDefault Sink: " + s["default"] + "\nDefault Source: alsa_input.real")
    elif a[:2] == ["list", "short"]:
        if a[2] == "sinks":
            for i, n in enumerate(sinks):
                print(f"{i}\t{n}\tPipeWire\tfloat32le 2ch 48000Hz\tIDLE")
        elif a[2] == "sources":
            for i, n in enumerate(sources):
                print(f"{i}\t{n}\tPipeWire\tfloat32le 2ch 48000Hz\tIDLE")
        elif a[2] == "modules":
            print("7\tmodule-always-sink\t")
            for m in s["modules"]:
                print(f"{m['id']}\t{m['name']}\t{' '.join(m['args'])}")
    elif a[0] == "load-module":
        for p in a[2:]:
            if "properties=" in p and not ("='device.description=\"" in p and p.endswith("\"'")):
                sys.exit("bad properties: " + p)
        if a[1] == "module-remap-source" and arg(a[2:], "master") + "" not in sources:
            sys.exit("no such master")
        m = {"id": s["next"], "name": a[1], "args": a[2:]}
        s["next"] += 1
        s["modules"].append(m)
        print(m["id"])
    elif a[0] == "unload-module":
        before = len(s["modules"])
        gone = [m for m in s["modules"] if str(m["id"]) == a[1]]
        if gone and gone[0]["name"] == "module-null-sink":
            # a sink can't go while the remapped source still hangs off its monitor
            name = arg(gone[0]["args"], "sink_name")
            if any(arg(m["args"], "master") == name + ".monitor" for m in s["modules"]):
                sys.exit("still in use")
            if s["default"] == name:
                s["default"] = "alsa_output.real"
        s["modules"] = [m for m in s["modules"] if str(m["id"]) != a[1]]
        if len(s["modules"]) == before:
            sys.exit("no such module")
    elif a[0] == "set-default-sink":
        if a[1] not in sinks:
            sys.exit("no such sink")
        s["default"] = a[1]
    else:
        sys.exit("unknown command " + " ".join(a))
    with open(STATE, "w", encoding="utf-8") as f:
        json.dump(s, f)


main()
