"""Per-file summary of a `handpic-cli lumascore` table.

    mise x -- python luma_stats.py <table.tsv>

Prints, over the `af` rows, the preview sizes, the median / mean / p10 / p90
of `gray_ms`, `luma_ms` and the saving `gray_ms - luma_ms`, the relative
score difference `|sharp_luma - sharpness| / sharpness` (median, max, and the
share where the luma score is higher), and the rows where `stored` is set
against `sharpness`. Standard library only.
"""

import csv
import statistics
import sys
from collections import Counter


def pct(values, p):
    values = sorted(values)
    at = (len(values) - 1) * p
    lo = int(at)
    hi = min(lo + 1, len(values) - 1)
    return values[lo] + (values[hi] - values[lo]) * (at - lo)


def summary(name, values):
    print(
        f"{name}: n={len(values)} median {statistics.median(values):.3f} "
        f"mean {statistics.fmean(values):.3f} p10 {pct(values, 0.1):.3f} "
        f"p90 {pct(values, 0.9):.3f}"
    )


def main(path):
    with open(path, newline="", encoding="utf-8") as f:
        rows = [r for r in csv.DictReader(f, delimiter="\t") if r["af"] == "af"]
    print("sizes:", dict(Counter(f"{r['width']}x{r['height']}" for r in rows)))
    gray = [float(r["gray_ms"]) for r in rows]
    luma = [float(r["luma_ms"]) for r in rows]
    summary("gray_ms", gray)
    summary("luma_ms", luma)
    summary("saving_ms", [g - l for g, l in zip(gray, luma)])
    scored = [r for r in rows if r["sharpness"] != "-" and r["sharp_luma"] != "-"]
    rel = [
        abs(float(r["sharp_luma"]) - float(r["sharpness"])) / float(r["sharpness"])
        for r in scored
        if float(r["sharpness"]) > 0
    ]
    higher = sum(1 for r in scored if float(r["sharp_luma"]) > float(r["sharpness"]))
    print(
        f"relative difference: n={len(rel)} median {100 * statistics.median(rel):.3f}% "
        f"max {100 * max(rel):.3f}%; luma higher on {higher}/{len(scored)}"
    )
    for r in rows:
        if r["stored"] != "-":
            print(f"check {r['file']}: stored {r['stored']} sharpness {r['sharpness']} equal {r['stored'] == r['sharpness']}")


if __name__ == "__main__":
    main(sys.argv[1])
