# Builds data/xray.json from the photo-atomic sublibrary of ENDF/B-VIII.0 (the Livermore Evaluated
# Photon Data Library, EPDL97, as distributed by the National Nuclear Data Center for the U.S.
# Department of Energy): for each element Z = 1 to 98,
#   - the coherent scattering form factor F(s) (MF 27 / MT 502), s = sin(θ)/λ in Å⁻¹ (the library's
#     x = sin(θ/2)/λ with θ its scattering angle, i.e. our 2θ), sampled every 0.02 Å⁻¹ from 0 to 2;
#   - the anomalous scattering factors f′ (MT 506) and f″ (MT 505) at the Kα1 energy of each tube anode.
# Usage: python tools/xray_data.py <folder with photoat-*.endf> data/xray.json
import json, math, re, sys, glob, os

ANODES = {"Cu": 1.5405929, "Co": 1.788996, "Mo": 0.70931715, "Cr": 2.289726, "Fe": 1.936041, "Ag": 0.55942178}
HC = 12398.41984  # eV·Å
STEP, N = 0.02, 101

def fnum(s):
    s = s.strip()
    if not s: return 0.0
    m = re.match(r'^([+-]?\d*\.?\d*)([+-]\d+)$', s)
    return float(m.group(1)) * 10 ** int(m.group(2)) if m else float(s)

def section(path, mf, mt):
    rows = [l[:66] for l in open(path) if len(l) >= 75 and l[70:72].strip() == str(mf) and l[72:75].strip() == str(mt)]
    if not rows: return None
    ctrl = [rows[1][i:i + 11] for i in range(0, 66, 11)]
    nr, np_ = int(fnum(ctrl[4])), int(fnum(ctrl[5]))
    data = rows[2 + math.ceil(2 * nr / 6):]
    vals = [fnum(r[i:i + 11]) for r in data for i in range(0, 66, 11)][:2 * np_]
    return list(zip(vals[0::2], vals[1::2]))

def interp(pts, x, log=False):
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        if x0 <= x <= x1:
            if x1 == x0: return y1
            if log and x0 > 0 and y0 > 0 and y1 > 0:
                return math.exp(math.log(y0) + (math.log(y1) - math.log(y0)) * (math.log(x) - math.log(x0)) / (math.log(x1) - math.log(x0)))
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    return pts[-1][1]

out = {"source": "epdl97-endf8", "step": STEP, "n": N, "anodes": ANODES, "elements": {}}
for path in sorted(glob.glob(os.path.join(sys.argv[1], "photoat-*.endf"))):
    z = int(re.search(r'photoat-(\d+)_', path).group(1))
    if z > 98: continue
    ff = section(path, 27, 502)
    f1, f2 = section(path, 27, 506), section(path, 27, 505)
    assert ff and abs(ff[0][1] - z) < 1e-6, (z, ff[:2])
    f0 = [round(interp(ff, i * STEP, log=i * STEP > ff[1][0]), 4) for i in range(N)]
    e = {"f0": f0}
    if f1 and f2:
        e["f1"] = {a: round(interp(f1, HC / l), 4) for a, l in ANODES.items()}
        e["f2"] = {a: round(interp(f2, HC / l), 4) for a, l in ANODES.items()}
    out["elements"][str(z)] = e
json.dump(out, open(sys.argv[2], "w"), separators=(",", ":"))
print(len(out["elements"]), "elements")
