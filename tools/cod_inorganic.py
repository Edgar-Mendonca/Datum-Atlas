# Reads the Crystallography Open Database archive (a tar stream on standard input, public domain)
# and keeps the inorganic structures: every .cif whose formula has no carbon–hydrogen compound
# (carbonates and carbides are kept). Writes them as <folder>/<COD number>.cif.
# Usage: curl -sL https://www.crystallography.net/archives/cod-cifs-mysql.tgz | gzip -dc | python3 tools/cod_inorganic.py cifs
import os, re, sys, tarfile

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
formula = re.compile(rb"^_chemical_formula_sum\s+(.+)$", re.M)
element = re.compile(rb"([A-Z][a-z]?)")
kept = organic = other = 0
with tarfile.open(fileobj=sys.stdin.buffer, mode="r|") as tar:
    for m in tar:
        if not (m.isfile() and m.name.endswith(".cif")): continue
        cod = os.path.basename(m.name)[:-4]
        if not cod.isdigit() or m.size > 5_000_000: other += 1; continue
        data = tar.extractfile(m).read()
        f = formula.search(data)
        els = set(element.findall(f.group(1))) if f else set()
        if not els: other += 1; continue
        if b"C" in els and b"H" in els: organic += 1; continue
        with open(os.path.join(out, cod + ".cif"), "wb") as w: w.write(data)
        kept += 1
        if kept % 20000 == 0: print(kept, "kept so far", flush=True)
print(f"{kept} inorganic kept, {organic} organic, {other} skipped")
