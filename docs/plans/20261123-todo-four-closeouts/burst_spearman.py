"""Within-burst rank agreement of two sharpness columns of `handpic-cli features`.

One table, two columns of it:

    mise x -- python burst_spearman.py one <table.tsv> <col_a> <col_b> [filter...]

Two tables, the same column of each, joined on folder and file:

    mise x -- python burst_spearman.py two <a.tsv> <b.tsv> <col> [filter...]

A filter is `col=value` or `col!=value`, checked on the rows of the (first)
table. Bursts are formed per folder over every row of the (first) table, in
capture order (`capture_time` then `subsec`), consecutive frames no more than
1000 ms apart, as `crates/app/src/burst.rs` does. A burst is used when at least
3 of its frames pass the filters and carry both values; ties get average ranks;
a burst whose values are all equal on one side has no correlation and is
counted apart. Standard library only.
"""

import csv
import datetime
import statistics
import sys

GAP_MS = 1000
MIN_FRAMES = 3
LOWEST = 10


def read(path):
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def capture_ms(row):
    try:
        t = datetime.datetime.strptime(row["capture_time"], "%Y:%m:%d %H:%M:%S")
    except (ValueError, KeyError):
        return None
    sub = row.get("subsec") or ""
    sub = sub if sub.isdigit() else ""
    sub = (sub[:3] + "000")[:3]
    epoch = datetime.datetime(1970, 1, 1)
    return int((t - epoch).total_seconds()) * 1000 + int(sub)


def bursts(rows):
    by_folder = {}
    for r in rows:
        by_folder.setdefault(r["folder"], []).append(r)
    out = []
    for folder in sorted(by_folder):
        timed = [(capture_ms(r), r) for r in by_folder[folder]]
        timed.sort(key=lambda t: (t[0] is None, t[0] or 0, t[1]["file"]))
        previous = None
        for ms, r in timed:
            if ms is not None and previous is not None and ms - previous <= GAP_MS:
                out[-1].append(r)
            else:
                out.append([r])
            previous = ms
    return out


def ranks(values):
    order = sorted(range(len(values)), key=lambda i: values[i])
    out = [0.0] * len(values)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and values[order[j + 1]] == values[order[i]]:
            j += 1
        for k in range(i, j + 1):
            out[order[k]] = (i + j) / 2 + 1
        i = j + 1
    return out


def spearman(a, b):
    ra, rb = ranks(a), ranks(b)
    ma, mb = statistics.fmean(ra), statistics.fmean(rb)
    cov = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    va = sum((x - ma) ** 2 for x in ra)
    vb = sum((y - mb) ** 2 for y in rb)
    if va == 0 or vb == 0:
        return None
    return cov / (va * vb) ** 0.5


def number(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def percentile(sorted_values, p):
    if len(sorted_values) == 1:
        return sorted_values[0]
    at = (len(sorted_values) - 1) * p
    lo = int(at)
    hi = min(lo + 1, len(sorted_values) - 1)
    return sorted_values[lo] + (sorted_values[hi] - sorted_values[lo]) * (at - lo)


def passes(row, filters):
    for f in filters:
        if "!=" in f:
            col, val = f.split("!=", 1)
            if row.get(col) == val:
                return False
        else:
            col, val = f.split("=", 1)
            if row.get(col) != val:
                return False
    return True


def main(argv):
    if len(argv) >= 4 and argv[0] == "one":
        rows = read(argv[1])
        col_a, col_b = argv[2], argv[3]
        filters = argv[4:]
        a_of = lambda r: number(r.get(col_a))
        b_of = lambda r: number(r.get(col_b))
        label = f"{argv[1]}: {col_a} vs {col_b}"
    elif len(argv) >= 4 and argv[0] == "two":
        rows = read(argv[1])
        other = {(r["folder"], r["file"]): r for r in read(argv[2])}
        col = argv[3]
        filters = argv[4:]
        a_of = lambda r: number(r.get(col))
        b_of = lambda r: number(other.get((r["folder"], r["file"]), {}).get(col))
        label = f"{col}: {argv[1]} vs {argv[2]}"
    else:
        sys.exit(__doc__)

    used = []
    flat = 0
    frames = 0
    for burst in bursts(rows):
        members = [
            (r, a_of(r), b_of(r))
            for r in burst
            if passes(r, filters) and a_of(r) is not None and b_of(r) is not None
        ]
        if len(members) < MIN_FRAMES:
            continue
        a = [m[1] for m in members]
        b = [m[2] for m in members]
        rho = spearman(a, b)
        if rho is None:
            flat += 1
            continue
        frames += len(members)
        top_a = max(range(len(a)), key=lambda i: a[i])
        top_b = max(range(len(b)), key=lambda i: b[i])
        used.append((rho, top_a == top_b, members))

    print(label)
    print(f"filters: {' '.join(filters) or '(none)'}")
    print(f"bursts used: {len(used)} ({frames} frames), skipped as flat: {flat}")
    if not used:
        return
    rhos = sorted(u[0] for u in used)
    same = sum(1 for u in used if u[1])
    print(f"spearman median: {statistics.median(rhos):.3f}")
    print(f"spearman p10: {percentile(rhos, 0.1):.3f}")
    print(f"spearman min: {rhos[0]:.3f}")
    print(f"same top frame: {same}/{len(used)} ({100 * same / len(used):.0f}%)")
    print(f"lowest {min(LOWEST, len(used))} bursts:")
    for rho, top, members in sorted(used, key=lambda u: u[0])[:LOWEST]:
        first, last = members[0][0], members[-1][0]
        print(
            f"  {rho:+.3f}  {'same' if top else 'diff'} top  {len(members)} frames  "
            f"{first['folder']}/{first['file']}..{last['file']}"
        )


if __name__ == "__main__":
    main(sys.argv[1:])
