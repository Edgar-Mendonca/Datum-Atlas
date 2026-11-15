"""Checks for the graph module and the worker's protocol (python -m unittest discover -s py/tests)."""
import base64
import json
import os
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
from atlas import graph  # noqa: E402


class Graph(unittest.TestCase):
    def test_render_gives_svg_and_a_map_for_hover(self):
        spec = {"size": [600, 300], "x": {"label": "Strain (%)"}, "y": {"label": "Stress (MPa)"},
                "series": [{"name": "A", "x": [0, 1, 2, 3], "y": [0, 100, 150, 160]}, {"name": "B", "kind": "scatter", "x": [1, 2], "y": [90, None]}]}
        r = graph.render(spec)
        self.assertTrue(r["svg"].lstrip().startswith("<?xml") or "<svg" in r["svg"][:400])
        self.assertIn("Stress (MPa)", r["svg"])
        self.assertAlmostEqual(r["w"], 600, delta=1)
        x0, y0, x1, y1 = r["box"]
        self.assertTrue(0 <= x0 < x1 <= r["w"] and 0 <= y0 < y1 <= r["h"])
        self.assertEqual(len(r["points"]), 5)  # the None point is left out
        self.assertLessEqual(r["xlim"][0], 0)

    def test_log_axes_and_marks(self):
        r = graph.render({"x": {"log": True}, "y": {"log": True}, "series": [{"kind": "scatter", "x": [1e3, 1e5], "y": [400, 300], "open": [False, True]}],
                          "marks": [{"x": 1e4, "y": 350, "text": "here"}], "lines": [{"y": 320}]})
        self.assertTrue(r["xlog"] and r["ylog"])

    def test_bad_values_never_break_it(self):
        for s in ({}, {"series": [{"x": [], "y": []}]}, {"series": [{"x": [1, float("nan")], "y": [float("inf"), 2]}]}, {"x": {"min": 5, "max": 1}}):
            graph.render(s)

    def test_export_for_publication(self):
        spec = {"series": [{"x": [0, 1], "y": [0, 1]}]}
        self.assertTrue(base64.b64decode(graph.export(spec, "pdf")["data"]).startswith(b"%PDF"))
        self.assertTrue(base64.b64decode(graph.export(spec, "png", 150)["data"]).startswith(b"\x89PNG"))
        self.assertIn(b"<svg", base64.b64decode(graph.export(spec, "svg")["data"]))


class Worker(unittest.TestCase):
    def test_protocol(self):
        p = subprocess.run([sys.executable, "-X", "utf8", os.path.join(os.path.dirname(HERE), "worker.py")],
                           input='{"cmd":"ping"}\n{"cmd":"nope"}\nnot json\n{"cmd":"render","spec":{"series":[{"x":[0,1],"y":[0,1]}]}}\n',
                           capture_output=True, text=True, timeout=120)
        lines = [json.loads(x) for x in p.stdout.splitlines()]
        self.assertTrue(lines[0]["ready"])
        self.assertIn("matplotlib", lines[1])
        self.assertFalse(lines[2]["ok"])
        self.assertFalse(lines[3]["ok"])
        self.assertTrue(lines[4]["ok"] and "svg" in lines[4])


if __name__ == "__main__":
    unittest.main()
