"""Per press, the time from the image painted to the DWM present on screen.

Adapted from the Tauri vs GPUI re-run's `screengap.py`: both builds are GPUI
here, so both sides take its GPUI branch. Pairs press i of a run's latprobe
CSV with the i-th `page` line carrying `keypressToPixels` of its stdout, for
the 12 used runs, and prints p50 / p95 per run and the median of the three
runs. `pended` is the time from the page pended (`notified`, before
`frameWait`) to the DWM present; `first` is the share of presses whose
`pended` is at most one refresh, i.e. that reached the screen at the first
DWM composition after the page was pended.

Usage: python screengap.py <out dir>
"""
import re, statistics, sys
from pathlib import Path

out = Path(sys.argv[1])
ST = re.compile(r"(\w+)=([\d.]+)ms")
REFRESH = 1000 / 59
USED = {
    "ARW": {"A": ["A1-try4", "A2-try4", "A3-try4"], "B": ["B1-try1", "B2-try1", "B3-try1"]},
    "RAF": {"A": ["A1-try2", "A2-try5", "A3-try1"], "B": ["B1-try7", "B2-try1", "B3-try1"]},
}

def pct(v, p):
    s = sorted(v); return s[min(int(len(s) * p), len(s) - 1)]

def pages(name):
    rows = []
    for line in (out / f"{name}.stdout.txt").read_text(encoding="utf-8", errors="replace").splitlines():
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
        print(f"\n## {folder} {'A (before)' if app == 'A' else 'B (after)'}")
        per, firsts = [], []
        for slot in slots:
            name = f"{folder}-{slot}"
            rows, scr = pages(name), screen(name)
            assert len(rows) == len(scr) == 60, (name, len(rows), len(scr))
            k2p = [r["keypressToPixels"] for r in rows]
            m = {}
            painted = [r["keypressToPixels"] - r["frameWait"] - r["present"] for r in rows]
            m["gap"] = [s - p for s, p in zip(scr, painted)]
            m["frameWait"] = [r["frameWait"] for r in rows]
            m["present"] = [r["present"] for r in rows]
            m["toDwm"] = [s - k for s, k in zip(scr, k2p)]
            m["pended"] = [s - (r["keypressToPixels"] - r["frameWait"] - r["paint"] - r["present"])
                           for s, r in zip(scr, rows)]
            first = sum(p <= REFRESH for p in m["pended"])
            firsts.append(first)
            # The pairing check: screen against keypressToPixels, aligned and shifted by one press.
            c0 = corr(scr, k2p); c1 = corr(scr[1:], k2p[:-1]); c2 = corr(scr[:-1], k2p[1:])
            neg = sum(g < 0 for g in m["gap"])
            print(name, " ".join(f"{k}={pct(v, .5):.2f}/{pct(v, .95):.2f}" for k, v in m.items()),
                  f"first={first}/60 min_gap={min(m['gap']):.2f} neg={neg} r={c0:.2f} r_shift={c1:.2f},{c2:.2f}")
            per.append(m)
        print("median", " ".join(
            f"{k}={statistics.median(pct(m[k], .5) for m in per):.2f}/{statistics.median(pct(m[k], .95) for m in per):.2f}"
            for k in per[0]), f"first={sum(firsts)}/{60 * len(firsts)}")
