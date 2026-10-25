# Changelog

## 0.1.0 — 2026-10-25

The first Datum Atlas: about 100 referenced materials, property charts with family envelopes, and selection with limits and index lines.

**Materials**
- About 100 generic materials from every family: metals, polymers, elastomers, ceramics and glasses, composites, foams and natural materials.
- Each value is a range with its source. The datasheet numbers the references and lists them at the bottom.
- Built-in values are drafts: compiled from the named sources, not yet checked against each table or page.
- Browse: a family tree and a table you can sort, select and copy to Excel (Ctrl+C), with Sum, Average and Count.

**Your own materials**
- Add Material explains what is needed: a name, the family, density, at least one more property, low and high values in the unit shown, and where the values come from.
- Your materials are saved on this computer and appear in Browse, the datasheets, every chart and the selection. Edit or delete them, or copy a built-in material and change it.

**Property charts**
- Any two properties on log or linear axes. Each material is an ellipse spanning its ranges; each family has a smooth envelope (one per group where a family is spread out).
- Hover for the ranges, click for a short datasheet beside the chart, double-click for the full one.
- Wheel to zoom, drag to pan, Fit; guide lines of slope 1, 2 and 3; click a family in the legend to hide it.
- Export as SVG or PNG; print or save as PDF.

**Selection**
- Limits: minimum and maximum values and yes/no properties. Materials without the value fail with a reason, or can be kept and marked.
- Selection line: place it with the Line tool, drag it, type its slope; the materials above it pass. The chart shows the index and its value at the line, such as E^½/ρ = 0.00316.
- Selection box, and families or classes left out.
- Results: what passes, ranked by the index with “× best”, and why every other material fails. The stage panel beside the chart shows the counts and the best materials.

**Getting started**
- File › Home: design problems to start from (light stiff beam, strong tie, stiff panel, springs, hot and light, damage tolerant).
- Setup for the requirements; Getting Started explains the method; the Glossary explains every term.
- Every planned feature already has its place in the ribbon and the sidebar; Help › Roadmap says when it arrives.
