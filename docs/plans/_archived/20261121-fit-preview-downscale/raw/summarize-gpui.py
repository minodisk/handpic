"""Summarize a run-ab-gpui.ps1 output folder: per-run p50 / p95 of every stage
and of the external `screen` latency, then the medians of the p50s and of the
p95s for each build (A / B).

Usage: python summarize-gpui.py <out dir>
Percentiles: sorted index n / 2 and n * 0.95, as the native app's summary.
"""

import csv
import re
import statistics
import sys
from pathlib import Path

STAGE = re.compile(r"(\w+)=([\d.]+)ms")


def pct(values, p):
    s = sorted(values)
    return s[min(int(len(s) * p), len(s) - 1)]


def page_lines(path):
    rows = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        at = line.find("page ")
        if at < 0 or "keypressToPixels=" not in line:
            continue
        rows.append({k: float(v) for k, v in STAGE.findall(line[at:])})
    return rows


def main():
    out = Path(sys.argv[1])
    runs = {}
    for probe in sorted(out.glob("*.csv")):
        name = probe.stem
        if "warm" in name:
            continue
        ok = any(
            line.startswith(name + " ") and "bad=[]" in line
            for line in (out / "runs.txt").read_text().splitlines()
        )
        if not ok:
            continue
        folder, rest = name.split("-", 1)
        app, run = rest[0], rest[1]
        with probe.open() as f:
            rows = list(csv.DictReader(f))
        screen = [float(r["settled_ms"]) for r in rows if r["settled_ms"]]
        first = [float(r["first_change_ms"]) for r in rows if r["first_change_ms"]]
        stats = {"screen": screen, "screenFirstChange": first}
        log = out / (f"{name}.stdout.txt")
        pages = page_lines(log) if log.exists() else []
        for key in sorted({k for p in pages for k in p}):
            stats[key] = [p[key] for p in pages if key in p]
        runs.setdefault((folder, app), []).append((run, stats, len(rows)))

    for (folder, app), items in sorted(runs.items()):
        label = "A (before)" if app == "A" else "B (after)"
        keys = sorted({k for _, s, _ in items for k in s})
        print(f"\n## {folder} {label}\n")
        print("| Run | n (screen / log) | " + " | ".join(keys) + " |")
        print("|---" * (len(keys) + 2) + "|")
        for run, s, n in items:
            cells = [
                f"{pct(s[k], 0.5):.2f} / {pct(s[k], 0.95):.2f}" if s.get(k) else "-"
                for k in keys
            ]
            n_log = len(s.get("keypressToPixels", []))
            print(f"| {run} | {len(s['screen'])} / {n_log} | " + " | ".join(cells) + " |")
        med = []
        for k in keys:
            p50s = [pct(s[k], 0.5) for _, s, _ in items if s.get(k)]
            p95s = [pct(s[k], 0.95) for _, s, _ in items if s.get(k)]
            med.append(
                f"{statistics.median(p50s):.2f} / {statistics.median(p95s):.2f}" if p50s else "-"
            )
        print(f"| median of {len(items)} | | " + " | ".join(med) + " |")


if __name__ == "__main__":
    main()
