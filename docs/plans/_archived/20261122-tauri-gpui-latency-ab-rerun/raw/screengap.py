"""Per press, the time from the image painted to the DWM present on screen.

Pairs press i of a run's latprobe CSV with the i-th `page` line carrying
`keypressToPixels` of its log (GPUI stdout, Tauri app log), for the 12 used
runs, and prints p50 / p95 per run and the median of the three runs.
"""
import re, statistics, sys
from pathlib import Path

out = Path(sys.argv[1])
ST = re.compile(r"(\w+)=([\d.]+)ms")
USED = {
    "ARW": {"A": ["A1-try1", "A2-try1", "A3-try1"], "B": ["B1-try2", "B2-try1", "B3-try4"]},
    "RAF": {"A": ["A1-try3", "A2-try1", "A3-try2"], "B": ["B1-try1", "B2-try3", "B3-try5"]},
}

def pct(v, p):
    s = sorted(v); return s[min(int(len(s) * p), len(s) - 1)]

def pages(name, app):
    log = out / (f"{name}.stdout.txt" if app == "A" else f"{name}.app.log")
    rows = []
    for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
        at = line.find("page ")
        if at < 0 or "keypressToPixels=" not in line: continue
        rows.append({k: float(v) for k, v in ST.findall(line[at:])})
    return rows

def screen(name):
    lines = (out / f"{name}.csv").read_text().splitlines()[1:]
    return [float(l.split(",")[1]) for l in lines]

def corr(x, y):
    mx, my = statistics.fmean(x), statistics.fmean(y)
    sxy = sum((a - mx) * (b - my) for a, b in zip(x, y))
    sx = sum((a - mx) ** 2 for a in x) ** .5; sy = sum((b - my) ** 2 for b in y) ** .5
    return sxy / (sx * sy)

for folder, apps in USED.items():
    for app, slots in apps.items():
        print(f"\n## {folder} {'GPUI' if app == 'A' else 'Tauri'}")
        per = []
        for slot in slots:
            name = f"{folder}-{slot}"
            rows, scr = pages(name, app), screen(name)
            assert len(rows) == len(scr) == 60, (name, len(rows), len(scr))
            k2p = [r["keypressToPixels"] for r in rows]
            m = {}
            if app == "A":
                painted = [r["keypressToPixels"] - r["frameWait"] - r["present"] for r in rows]
                m["gap"] = [s - p for s, p in zip(scr, painted)]
                m["frameWait"] = [r["frameWait"] for r in rows]
                m["present"] = [r["present"] for r in rows]
                m["toDwm"] = [s - k for s, k in zip(scr, k2p)]
            else:
                m["gap"] = [s - k for s, k in zip(scr, k2p)]
                m["afterFrame2"] = [s - r["frame2"] for s, r in zip(scr, rows)]
            # The pairing check: screen against keypressToPixels, aligned and shifted by one press.
            c0 = corr(scr, k2p); c1 = corr(scr[1:], k2p[:-1]); c2 = corr(scr[:-1], k2p[1:])
            neg = sum(g < 0 for g in m["gap"])
            print(name, " ".join(f"{k}={pct(v, .5):.2f}/{pct(v, .95):.2f}" for k, v in m.items()),
                  f"min_gap={min(m['gap']):.2f} neg={neg} r={c0:.2f} r_shift={c1:.2f},{c2:.2f}")
            per.append(m)
        print("median", " ".join(
            f"{k}={statistics.median(pct(m[k], .5) for m in per):.2f}/{statistics.median(pct(m[k], .95) for m in per):.2f}"
            for k in per[0]))
