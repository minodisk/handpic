"""Rank the Good frames of `riffle-cli facechoice` dumps by `full_sharp`.

Usage: rank.py <fc.tsv> <prototype m_*.json> [<threshold>] [<cut rank>...]

Applies the Good rule of `focus.ts` to the frame's judged face and prints the
Good count, the Spearman against the prototype's `fu`, the rank the threshold
falls at (the rank of the first frame at or above it), the score at each cut
rank and the ranks of the two acceptance files.
"""

import csv
import json
import sys

FACE_CONFIDENCE = 0.8
ACCEPTANCE = ("_DSC1508.ARW", "_DSC1536.ARW")


def number(row, key):
    try:
        return float(row[key])
    except ValueError:
        return None


def judged_rows(path):
    with open(path, encoding="utf8", newline="") as f:
        rows = list(csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE))
    files = {}
    for r in rows:
        files.setdefault(r["file"], []).append(r)
    for file, group in files.items():
        if group[0]["af"] == "1":
            near = [r for r in group if r["set"] == "crop" and r["nearest"] == "1"]
        else:
            faces = [
                r
                for r in group
                if r["set"] == "whole" and (number(r, "score") or 0) >= FACE_CONFIDENCE
            ]
            near = sorted(
                faces, key=lambda r: number(r, "w") * number(r, "h"), reverse=True
            )[:1]
        if near:
            yield file, near[0]


def is_good(r):
    get = lambda k: number(r, k)
    values = [
        get(k) for k in ("eye_focus", "ear", "yaw", "pitch", "eye_offset", "edge_gap")
    ]
    if any(v is None for v in values):
        return False
    focus, ear, yaw, pitch, offset, gap = values
    return (
        focus >= 0.9
        and ear >= 0.25
        and abs(yaw) <= 35
        and abs(pitch) <= 45
        and offset <= 0.1
        and gap >= 0.02
    )


def ranks(values):
    order = sorted(range(len(values)), key=lambda i: values[i])
    out = [0.0] * len(values)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and values[order[j + 1]] == values[order[i]]:
            j += 1
        for k in range(i, j + 1):
            out[order[k]] = (i + j) / 2
        i = j + 1
    return out


def spearman(a, b):
    ra, rb = ranks(a), ranks(b)
    ma, mb = sum(ra) / len(ra), sum(rb) / len(rb)
    cov = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    va = sum((x - ma) ** 2 for x in ra)
    vb = sum((y - mb) ** 2 for y in rb)
    return cov / (va * vb) ** 0.5


def main():
    tsv, proto = sys.argv[1:3]
    threshold = float(sys.argv[3]) if len(sys.argv) > 3 else 0.162
    cuts = [int(c) for c in sys.argv[4:]]
    good = [(file, r) for file, r in judged_rows(tsv) if is_good(r)]
    scored = sorted(
        ((number(r, "full_sharp"), file) for file, r in good if number(r, "full_sharp") is not None),
    )
    with open(proto, encoding="utf8") as f:
        prototype = {o["file"]: o["fu"] for o in json.load(f)}
    both = [(s, prototype[file]) for s, file in scored if file in prototype]

    print(f"good {len(good)}, with a score {len(scored)}, in the prototype {len(both)}")
    print(f"spearman {spearman(*zip(*both)):.4f}")
    under = sum(s < threshold for s, _ in scored)
    print(f"first frame at or above {threshold}: rank {under + 1} ({under} under)")
    for cut in cuts:
        if cut <= len(scored):
            print(f"score at rank {cut}: {scored[cut - 1][0]:.4f}")
    for rank, (s, file) in enumerate(scored, 1):
        if file in ACCEPTANCE:
            print(f"{file}: rank {rank}, score {s:.4f}")


if __name__ == "__main__":
    main()
