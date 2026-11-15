r"""Derive the Low threshold on `full_eye_sharp` from the user's star ratings.

Input: `frames.tsv` next to this script, and nothing else. It holds one row
per rated frame (`xmp:Rating` 1-3) of the five rated focus-sample folders,
with the columns `folder`, `file`, `rating`, `full_eye_sharp`, `eye_focus`
and `state` (`Candidate` / `NotCandidate`, today's cue); the three value
columns are empty for a frame without a judged face.

How the table was produced (2026-11-15): a scratch `riffle-cli fullfocus`
subcommand (not committed) dumped, for every frame of the folders under
`D:\Photos\tests\<folder>`, the preview's `eye_focus` and cue state and the
`full_eye_sharp` measured on the full-size JPEG for every judged face, not only
the candidates. The dump was joined by file stem with the `xmp:Rating` of the
`<stem>.xmp` sidecar in the same folder (no rating = 0 = skipped, left out).
The external dump and photos are not needed to rerun this script.

Rule (chosen by the user): `LOW_BELOW` is the largest threshold on the 0.01 grid
(0.05 to 0.17) under which at most 2% of the held-out 3-star frames fall. The held-out
folders choose it; the fresh folder only reports.

Run: `python low_threshold.py` (standard library only).
"""

import csv
import os

HELD_OUT = (
    "2026-06-14-focus-sample",
    "2026-07-18-focus-sample",
    "2026-08-01-focus-sample",
    "2026-08-22-focus-sample",
)
FRESH = ("2026-06-05-focus-sample",)
MAX_THREE_STAR_SHARE = 0.02
GRID = [round(0.05 + 0.01 * i, 2) for i in range(13)]
SOFT_BELOW = 0.162
STARS = (3, 2, 1)


def load():
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "frames.tsv")
    with open(path, encoding="utf8", newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def frames_of(rows, folders):
    return [r for r in rows if r["folder"] in folders]


def judged(frames):
    return [r for r in frames if r["full_eye_sharp"] != ""]


def totals(frames):
    return {s: sum(int(r["rating"]) == s for r in frames) for s in STARS}


def under(frames, t):
    return {
        s: sum(
            int(r["rating"]) == s
            and r["full_eye_sharp"] != ""
            and float(r["full_eye_sharp"]) < t
            for r in frames
        )
        for s in STARS
    }


def today_low(frames):
    return {
        s: sum(int(r["rating"]) == s and r["state"] == "NotCandidate" for r in frames)
        for s in STARS
    }


def cells(counts, total):
    return " | ".join(f"{counts[s]}/{total[s]}" for s in STARS)


def main():
    rows = load()
    sets = []
    for name, folders in (("held-out", HELD_OUT), ("fresh", FRESH)):
        frames = frames_of(rows, folders)
        rated = totals(frames)
        unjudged = totals([r for r in frames if r["full_eye_sharp"] == ""])
        print(
            f"{name}: rated 3/2/1 stars "
            + ", ".join(str(rated[s]) for s in STARS)
            + "; without a judged face (left out, Neutral) "
            + ", ".join(str(unjudged[s]) for s in STARS)
        )
        sets.append((name, judged(frames)))
    print()
    print("Frames with full_eye_sharp under the threshold (count / frames with a judged face)")
    print()
    print("| threshold | held-out 3 stars | held-out 2 stars | held-out 1 star | fresh 3 stars | fresh 2 stars | fresh 1 star |")
    print("|---|---|---|---|---|---|---|")
    for t in sorted(GRID + [SOFT_BELOW]):
        line = " | ".join(cells(under(f, t), totals(f)) for _, f in sets)
        print(f"| {t:.3f} | {line} |")
    held = sets[0][1]
    three = totals(held)[3]
    low = max(t for t in GRID if under(held, t)[3] <= MAX_THREE_STAR_SHARE * three)
    print()
    print(f"LOW_BELOW = {low:.3f} (at most {MAX_THREE_STAR_SHARE:.0%} of {three} held-out 3-star frames under it)")
    for name, frames in sets:
        total = totals(frames)
        u = under(frames, low)
        share = {s: u[s] / total[s] for s in STARS}
        print(
            f"{name}: 3-star share {share[3]:.1%}, 2-star share {share[2]:.1%}, 1-star recall {share[1]:.1%}"
        )
    print()
    print("Today's Low (state NotCandidate), for comparison")
    for name, frames in sets:
        total = totals(frames)
        print(f"{name}: 3/2/1 stars {cells(today_low(frames), total)}")


if __name__ == "__main__":
    main()
