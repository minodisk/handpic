import re, statistics, sys
from pathlib import Path
out = Path(sys.argv[1])
ST = re.compile(r"(\w+)=([\d.]+)ms")
def pct(v, p):
    s = sorted(v); return s[min(int(len(s)*p), len(s)-1)]
ok = [l.split()[0] for l in (out/"runs.txt").read_text().splitlines() if "bad=[]" in l and "warm" not in l]
res = {}
for name in ok:
    folder, rest = name.split("-", 1); app = rest[0]
    log = out/(f"{name}.stdout.txt" if app == "A" else f"{name}.app.log")
    rows = []
    for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
        at = line.find("page ")
        if at < 0 or "keypressToPixels=" not in line: continue
        rows.append({k: float(v) for k, v in ST.findall(line[at:])})
    m = {}
    if app == "A":
        m["painted"] = [r["keypressToPixels"]-r["frameWait"]-r["present"] for r in rows]
        m["nextFrame"] = [r["keypressToPixels"] for r in rows]
    else:
        m["painted"] = [r["keypressToPixels"] for r in rows]
        m["nextFrame"] = [r["frame2"] for r in rows if "frame2" in r]
        m["frame1"] = [r["frame1"] for r in rows if "frame1" in r]
    res.setdefault((folder, app), []).append((name, len(rows), m))
for (folder, app), items in sorted(res.items()):
    print(f"\n## {folder} {'GPUI' if app=='A' else 'Tauri'}")
    keys = list(items[0][2])
    for name, n, m in items:
        print(name, n, " ".join(f"{k}={pct(m[k],.5):.2f}/{pct(m[k],.95):.2f}(n={len(m[k])})" for k in keys))
    print("median", " ".join(f"{k}={statistics.median([pct(m[k],.5) for _,_,m in items]):.2f}/{statistics.median([pct(m[k],.95) for _,_,m in items]):.2f}" for k in keys))
