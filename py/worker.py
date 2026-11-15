"""Datum Atlas's Python worker: the app starts it once and sends one JSON request per line on
stdin; each answer is one JSON line on stdout ({"ok": true, ...} or {"ok": false, "error": ...}).
The Rust engine stays in charge; Python draws graphs (and, later, runs the heavier solvers)."""
import json
import logging
import os
import sys
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
logging.getLogger("matplotlib").setLevel(logging.ERROR)


def clean(v):
    """The reply with every NaN or infinity made None (JSON has no such numbers)."""
    if isinstance(v, float):
        return v if v == v and v not in (float("inf"), float("-inf")) else None
    if isinstance(v, dict):
        return {k: clean(x) for k, x in v.items()}
    if isinstance(v, (list, tuple)):
        return [clean(x) for x in v]
    return v


def main():
    out = sys.stdout
    sys.stdout = sys.stderr  # stray prints from libraries must not break the protocol
    import matplotlib
    import numpy
    from atlas import graph

    def handle(q):
        cmd = q.get("cmd")
        if cmd == "ping":
            return {"python": sys.version.split()[0], "numpy": numpy.__version__, "matplotlib": matplotlib.__version__}
        if cmd == "render":
            return graph.render(q["spec"])
        if cmd == "export":
            return graph.export(q["spec"], q.get("fmt", "png"), q.get("dpi", 300))
        raise ValueError(f"Unknown request {cmd!r}")

    # Warm up (imports, font list, first figure) before saying ready.
    graph.render({"series": []})
    out.write(json.dumps({"ok": True, "ready": True}) + "\n")
    out.flush()
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            r = {"ok": True, **handle(json.loads(line))}
        except Exception as e:  # report every failure to the app instead of dying
            r = {"ok": False, "error": f"{type(e).__name__}: {e}", "trace": traceback.format_exc(limit=4)}
        try:
            line = json.dumps(r, allow_nan=False, default=str)
        except ValueError:  # a NaN or infinity slipped into a result: send it as empty rather than die
            line = json.dumps(clean(r), allow_nan=False, default=str)
        out.write(line + "\n")
        out.flush()


if __name__ == "__main__":
    main()
