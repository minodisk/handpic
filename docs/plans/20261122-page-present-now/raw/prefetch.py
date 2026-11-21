"""p50 / p95 of `frameWait` and `keypressToPixels` per `source` of the
run-prefetch.ps1 runs' `page` lines (prefetch on, `--auto`).

Usage: python prefetch.py <out dir>
"""
import re, sys
from pathlib import Path

out = Path(sys.argv[1])

def pct(v, p):
    s = sorted(v); return s[min(int(len(s) * p), len(s) - 1)]

for folder in ["ARW", "RAF"]:
    for app in "AB":
        name = f"{folder}-prefetch-{app}"
        rows = []
        for line in (out / f"{name}.stdout.txt").read_text(encoding="utf-8", errors="replace").splitlines():
            at = line.find("page ")
            if at < 0 or "keypressToPixels=" not in line: continue
            rows.append(dict(re.findall(r"(\w+)=([\w.]+)", line[at:])))
        for source in ["prefetch", "read"]:
            r = [x for x in rows if x["source"] == source]
            if not r: continue
            fw = [float(x["frameWait"][:-2]) for x in r]
            k = [float(x["keypressToPixels"][:-2]) for x in r]
            print(name, f"source={source}", f"n={len(r)}",
                  f"frameWait={pct(fw, .5):.2f}/{pct(fw, .95):.2f}",
                  f"keypressToPixels={pct(k, .5):.2f}/{pct(k, .95):.2f}")
