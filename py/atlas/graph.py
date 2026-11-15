"""Graphs for Datum Atlas, drawn by matplotlib from a description the app sends (a "spec"):

    {"size": [w, h] (pixels), "theme": "light"|"dark", "title": str,
     "x": {"label", "log", "min", "max"}, "y": {...},
     "series": [{"name", "kind": "line"|"scatter"|"step"|"bar"|"stem", "x": [...], "y": [...],
                 "color", "width", "dash", "marker", "size", "colors": [...], "open": [...], "labels": [...]}],
     "lines": [{"x"|"y": value, "color", "dash", "label"}], "marks": [{"x", "y", "text", "color"}],
     "legend": bool, "grid": bool}

render() returns the SVG for the window with what the window needs for hover (the plot box in
pixels, the axis limits and scales, and the points); export() returns PNG, SVG or PDF bytes for
publication, always on a white page."""
import base64
import io
import math
import re

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

PALETTE = ["#6C4AB6", "#E69F00", "#009E73", "#D55E00", "#0072B2", "#CC79A7", "#F2B233", "#56B4E9"]
THEMES = {"light": {"text": "#0C1A2B", "muted": "#5B6878", "grid": "#E3E7EE", "axes": "#3B4859"},
          "dark": {"text": "#E6EDF5", "muted": "#8D9AAB", "grid": "#2A3442", "axes": "#B8C4D2"}}
DPI = 96


def _style(theme):
    t = THEMES.get(theme, THEMES["light"])
    return {"font.family": "sans-serif", "font.sans-serif": ["IBM Plex Sans", "Segoe UI", "Arial", "Liberation Sans", "DejaVu Sans"],
            "font.size": 10, "axes.linewidth": 0.9, "axes.edgecolor": t["axes"], "axes.labelcolor": t["text"], "text.color": t["text"],
            "xtick.color": t["axes"], "ytick.color": t["axes"], "xtick.labelcolor": t["text"], "ytick.labelcolor": t["text"],
            "axes.spines.top": False, "axes.spines.right": False, "svg.fonttype": "none", "svg.hashsalt": "datum-atlas",
            "axes.unicode_minus": True, "legend.frameon": False, "figure.facecolor": "none", "axes.facecolor": "none", "savefig.facecolor": "none"}


def _finite(xs, ys):
    pts = [(float(a), float(b)) for a, b in zip(xs or [], ys or []) if a is not None and b is not None]
    return [(a, b) for a, b in pts if math.isfinite(a) and math.isfinite(b)]


def figure(spec, theme=None, size=None):
    """The figure and axes for a spec, and the points it shows (series index, x, y, label)."""
    theme = theme or spec.get("theme", "light")
    t = THEMES.get(theme, THEMES["light"])
    w, h = size or spec.get("size") or [640, 360]
    fig = plt.figure(figsize=(max(w, 120) / DPI, max(h, 90) / DPI), dpi=DPI)
    ax = fig.add_subplot(1, 1, 1)
    points = []
    xs, ys = spec.get("x") or {}, spec.get("y") or {}
    if xs.get("log"):
        ax.set_xscale("log")
    if ys.get("log"):
        ax.set_yscale("log")
    if spec.get("grid", True):
        ax.grid(True, color=t["grid"], linewidth=0.7, zorder=0)
        ax.set_axisbelow(True)
    for k, s in enumerate(spec.get("series") or []):
        pts = _finite(s.get("x"), s.get("y"))
        color = s.get("color") or PALETTE[k % len(PALETTE)]
        label = s.get("name")
        kind = s.get("kind", "line")
        if not pts:
            continue
        x, y = [p[0] for p in pts], [p[1] for p in pts]
        if kind == "scatter":
            colors = s.get("colors")
            opened = s.get("open") or []
            c = colors[: len(x)] if colors and len(colors) >= len(x) else color
            filled = [i for i in range(len(x)) if not (i < len(opened) and opened[i])]
            hollow = [i for i in range(len(x)) if i < len(opened) and opened[i]]
            if filled:
                ax.scatter([x[i] for i in filled], [y[i] for i in filled], s=s.get("size", 22), c=[c[i] for i in filled] if isinstance(c, list) else c,
                           marker=s.get("marker", "o"), edgecolors=t["axes"] if isinstance(c, list) else "none", linewidths=0.4, label=label, zorder=3)
            if hollow:
                ax.scatter([x[i] for i in hollow], [y[i] for i in hollow], s=s.get("size", 22) * 1.6, facecolors="none", edgecolors=color,
                           marker=">", linewidths=1.2, label=(label + " (run-out)") if label else None, zorder=3)
        elif kind == "bar":
            widths = [(x[1] - x[0]) * 0.92] * len(x) if len(x) > 1 else [0.8]
            ax.bar(x, y, width=s.get("barWidth", widths[0]), color=color, alpha=0.85, label=label, zorder=2, align="edge" if s.get("edge") else "center")
        elif kind == "stem":
            ax.vlines(x, 0, y, colors=color, linewidth=s.get("width", 1.8), label=label, zorder=2)
        elif kind == "step":
            ax.step(x, y, where="mid", color=color, linewidth=s.get("width", 1.6), label=label, zorder=2)
        else:
            ax.plot(x, y, color=color, linewidth=s.get("width", 1.6), linestyle=s.get("dash", "-"), marker=s.get("marker") or None,
                    markersize=s.get("size", 4), label=label, zorder=2)
        labels = s.get("labels") or []
        step = max(1, len(pts) // 1500)
        for i in range(0, len(pts), step):
            points.append([k, pts[i][0], pts[i][1], labels[i] if i < len(labels) else None])
    for ln in spec.get("lines") or []:
        kw = {"color": ln.get("color", t["muted"]), "linestyle": ln.get("dash", "--"), "linewidth": 1.0, "zorder": 1}
        if "x" in ln:
            ax.axvline(ln["x"], **kw, label=ln.get("label"))
        elif "y" in ln:
            ax.axhline(ln["y"], **kw, label=ln.get("label"))
    for m in spec.get("marks") or []:
        if m.get("x") is None or m.get("y") is None:
            continue
        c = m.get("color") or PALETTE[0]
        ax.plot([m["x"]], [m["y"]], marker=m.get("marker", "o"), markersize=m.get("size", 6), color=c, markerfacecolor=m.get("fill", c), zorder=4)
        if m.get("text"):
            ax.annotate(m["text"], (m["x"], m["y"]), textcoords="offset points", xytext=(5, 5), fontsize=8.5, color=t["text"], zorder=5)
    for axis, a in (("x", xs), ("y", ys)):
        lo, hi = a.get("min"), a.get("max")
        if lo is not None or hi is not None:
            (ax.set_xlim if axis == "x" else ax.set_ylim)(lo, hi)
    # A log axis over less than two decades reads better with plain numbers (400, not 4 × 10²).
    from matplotlib.ticker import FuncFormatter, NullFormatter
    for axis, get in ((ax.xaxis, ax.get_xlim), (ax.yaxis, ax.get_ylim)):
        lo, hi = get()
        if axis.get_scale() == "log" and lo > 0 and hi / lo < 100:
            plain = FuncFormatter(lambda v, _: f"{v:g}")
            axis.set_major_formatter(plain)
            axis.set_minor_formatter(plain if hi / lo < 10 else NullFormatter())
    ax.set_xlabel(xs.get("label", ""))
    ax.set_ylabel(ys.get("label", ""))
    if spec.get("title"):
        ax.set_title(spec["title"], fontsize=11, loc="left", color=t["text"])
    handles, names = ax.get_legend_handles_labels()
    if spec.get("legend", True) and any(names):
        ax.legend(fontsize=8.5, loc=spec.get("legendAt", "best"))
    fig.tight_layout(pad=0.6)
    return fig, ax, points


def render(spec):
    with plt.rc_context(_style(spec.get("theme", "light"))):
        fig, ax, points = figure(spec)
        fig.canvas.draw()
        w, h = fig.get_size_inches() * DPI
        b = ax.get_window_extent()
        buf = io.StringIO()
        fig.savefig(buf, format="svg", metadata={"Date": None, "Creator": "Datum Atlas"})
        plt.close(fig)
    # The window scales the picture to its panel: drop the fixed size, keep the viewBox.
    svg = buf.getvalue()
    head = re.search(r"<svg[^>]*>", svg)
    if head:
        svg = svg[:head.start()] + re.sub(r'\s(?:width|height)="[^"]*"', "", head.group(0)) + svg[head.end():]
    return {"svg": svg, "w": w, "h": h,
            # The plot box in SVG pixels (top-left origin) and the axes, so the window can turn the mouse into data values.
            "box": [b.x0, h - b.y1, b.x1, h - b.y0], "xlim": list(ax.get_xlim()), "ylim": list(ax.get_ylim()),
            "xlog": ax.get_xscale() == "log", "ylog": ax.get_yscale() == "log", "points": points,
            "names": [s.get("name") for s in spec.get("series") or []]}


def export(spec, fmt="png", dpi=300):
    """The graph for a paper: white page, light theme, 160 × 100 mm unless a size is given."""
    fmt = fmt if fmt in ("png", "svg", "pdf") else "png"
    size = spec.get("exportSize") or [160 / 25.4 * DPI, 100 / 25.4 * DPI]
    style = {**_style("light"), "figure.facecolor": "white", "axes.facecolor": "white", "savefig.facecolor": "white", "svg.fonttype": "path"}
    with plt.rc_context(style):
        fig, _, _ = figure(spec, theme="light", size=size)
        buf = io.BytesIO()
        fig.savefig(buf, format=fmt, dpi=int(dpi), metadata={"Creator": "Datum Atlas"} if fmt != "png" else None)
        plt.close(fig)
    return {"data": base64.b64encode(buf.getvalue()).decode(), "fmt": fmt}
