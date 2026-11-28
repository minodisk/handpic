"""Step 1: the cue readings of the 2026-10-12 dump against the user's own
culling (kept = still on disk, culled = gone), as pre-registered in plan.md.

Usage: python -I -B culling.py <app-dump-dir> <strict-dir> <discard-dir> <confirm-dir> <out-dir>

<app-dump-dir> is the app dump of 2026-10-12, whose `features\\*.tsv` give the
frames and their readings; <strict-dir>, <discard-dir> and <confirm-dir> hold
the `labels.tsv` of the three earlier labeling rounds. Lists the dump's
library folders (path, size, `mtime_ns`; no library file is opened), writes
`labels.tsv` and the two library listings into <out-dir> and prints every
table (Markdown) on stdout.

Standard library only.
"""

import csv
import hashlib
import os
import random
import sys
import time
from datetime import datetime

LIBRARY = r"D:\Photos\2026"
PITCH_CUT = 45.0
EAR_CUT = 0.137
YAW_CUT = 35.0
FOCUS_CUT = 0.9
SHARP_SHARE = 10
DRAWS = 1000
SEED = 0
RATIO_BAR = 0.5
SHARE_BAR = 0.25
SHARE_Q_BAR = 0.35
KEPT_SHARE_BAR = 0.05
PICK_ALONE = "2026-08-29"
SIDECARS = (".xmp", ".dop")
MISSING = "-"
PRIMARY = "pitch45"
READINGS = [
    (PRIMARY, "`|pitch|` > 45"),
    ("pitch_up", "`pitch` > 45"),
    ("pitch_down", "`pitch` < -45"),
    ("closed", "EAR <= 0.137"),
    ("yaw35", "`|yaw|` > 35"),
    ("focus_low", "`eye_focus` < 0.9"),
    ("no_focus", "no `eye_focus`"),
    ("sharp10", "bottom sharpness decile"),
]
ROUNDS = [("strict", "bad"), ("discard", "discard"), ("model-search", "discard")]
KNOWN_FOLDERS = {
    "2026-07-22": (189, 32, 32, 18),
    "2026-07-26": (64, 7, 7, 1),
    "2026-07-30": (936, 105, 105, 34),
    "2026-08-01": (3401, 463, 463, 84),
    "2026-08-22": (2677, 267, 267, 41),
    "2026-09-13-b": (1795, 232, 232, 13),
    "2026-09-19": (2134, 282, 287, 51),
}
KNOWN_BINS = {PRIMARY: 242, "pitch_up": 16, "pitch_down": 226, "closed": 937, "yaw35": 2810, "focus_low": 3062,
              "no_focus": 1294}
KNOWN_NO_PITCH = 3855


# Follows `under` of docs/plans/_archived/20261128-cue-labels/sample.py.
def under(path, root):
    p = os.path.normcase(os.path.realpath(path))
    r = os.path.normcase(os.path.realpath(root))
    return p == r or p.startswith(r.rstrip(os.sep) + os.sep)


# Follows `listing` of docs/plans/_archived/20261128-cue-labels/sample.py.
def listing(folders):
    out = {}
    for folder in folders:
        for root, _, files in os.walk(os.path.join(LIBRARY, folder)):
            for name in files:
                p = os.path.join(root, name)
                st = os.stat(p)
                out[p] = (st.st_size, st.st_mtime_ns)
    return out


# Follows `write_listing` of docs/plans/_archived/20261128-cue-labels/sample.py.
def write_listing(path, entries):
    with open(path, "w", encoding="utf-8", newline="") as out:
        out.write("path\tsize\tmtime_ns\n")
        for p in sorted(entries):
            size, mtime = entries[p]
            out.write(f"{p}\t{size}\t{mtime}\n")


# Follows `changed` of docs/plans/_archived/20261128-cue-labels/sample.py.
def changed(before, after):
    return sorted(p for p in set(before) | set(after) if before.get(p) != after.get(p))


# Follows `read_tsv` of docs/plans/_archived/20261128-cue-labels/sample.py.
def read_tsv(path):
    with open(path, encoding="utf-8", newline="") as f:
        return list(csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE))


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        h.update(f.read())
    return h.hexdigest()


def number(v):
    return None if v == MISSING else float(v)


def cell(c):
    return str(c).replace("|", r"\|")


def table(head, rows):
    """`head` is the header row, its columns split at ` | `; a `|` in a name or a cell is escaped for the table."""
    head = head.strip()[2:-2].split(" | ")
    print("| " + " | ".join(cell(h) for h in head) + " |")
    print("|" + " --- |" * len(head))
    for r in rows:
        print("| " + " | ".join(cell(c) for c in r) + " |")
    print()


def pct(x):
    return MISSING if x is None else f"{100 * x:.1f}%"


def num(x, digits=3):
    return MISSING if x is None else f"{x:.{digits}f}"


def day(ns):
    return datetime.fromtimestamp(ns / 1e9).strftime("%Y-%m-%d")


def quantile(sorted_values, q):
    """numpy's default (linear) quantile."""
    if not sorted_values:
        return None
    h = (len(sorted_values) - 1) * q
    lo = int(h)
    hi = min(lo + 1, len(sorted_values) - 1)
    return sorted_values[lo] + (h - lo) * (sorted_values[hi] - sorted_values[lo])


def pick_of(r):
    return r["xmp"] == "Pick" or r["dop"] == "Pick"


def value_of(r):
    for v in ("Pick", "Reject", "None"):
        if v in (r["xmp"], r["dop"]):
            return v
    return MISSING


def bins_of(frames, keys):
    """Each reading's firing set over `keys`; the sharpness decile is taken per folder over the folder's frames in `keys`."""
    out = {name: set() for name, _ in READINGS}
    per_folder = {}
    for k in keys:
        r = frames[k]
        pitch, ear, yaw, focus = number(r["pitch"]), number(r["ear"]), number(r["yaw"]), number(r["eye_focus"])
        if pitch is not None and abs(pitch) > PITCH_CUT:
            out[PRIMARY].add(k)
        if pitch is not None and pitch > PITCH_CUT:
            out["pitch_up"].add(k)
        if pitch is not None and pitch < -PITCH_CUT:
            out["pitch_down"].add(k)
        if ear is not None and ear <= EAR_CUT:
            out["closed"].add(k)
        if yaw is not None and abs(yaw) > YAW_CUT:
            out["yaw35"].add(k)
        if focus is not None and focus < FOCUS_CUT:
            out["focus_low"].add(k)
        if focus is None:
            out["no_focus"].add(k)
        per_folder.setdefault(k[0], []).append(k)
    for folder, ks in per_folder.items():
        scored = sorted((number(frames[k]["sharpness"]), k[1]) for k in ks if number(frames[k]["sharpness"]) is not None)
        for _, file in scored[: SHARP_SHARE * len(ks) // 100]:
            out["sharp10"].add((folder, file))
    return out


def cells(keys, label, fired):
    """(bin frames, bin positive, complement frames, complement positive) per folder."""
    out = {}
    for k in keys:
        c = out.setdefault(k[0], [0, 0, 0, 0])
        i = 0 if k in fired else 2
        c[i] += 1
        c[i + 1] += label[k]
    return out


def ratio_of(c):
    bn, bk, cn, ck = c
    if bn == 0 or ck == 0:
        return None
    return (bk / bn) / (ck / cn)


def summed(per_folder, folders):
    s = [0, 0, 0, 0]
    for f in folders:
        c = per_folder.get(f, (0, 0, 0, 0))
        for i in range(4):
            s[i] += c[i]
    return s


def draw_matrix(n):
    rng = random.Random(SEED)
    return [[rng.randrange(n) for _ in range(n)] for _ in range(DRAWS)]


def reading(per_folder, folders, matrix):
    total = summed(per_folder, folders)
    bn, bk, cn, ck = total
    bs = bk / bn if bn else None
    cs = ck / cn if cn else None
    ratios, bss, diffs, dropped = [], [], [], 0
    for row in matrix:
        c = summed(per_folder, [folders[i] for i in row])
        r = ratio_of(c)
        if r is None:
            dropped += 1
            continue
        ratios.append(r)
        bss.append(c[1] / c[0])
        diffs.append(100 * (c[3] / c[2] - c[1] / c[0]))
    ratios.sort()
    bss.sort()
    diffs.sort()
    lofo = [ratio_of(summed(per_folder, [g for g in folders if g != f])) for f in folders] if len(folders) > 1 else []
    lofo = [x for x in lofo if x is not None]
    return {
        "bn": bn, "bk": bk, "cn": cn, "ck": ck, "bs": bs, "cs": cs, "ratio": ratio_of(total),
        "diff": None if bs is None or cs is None else 100 * (cs - bs),
        "ratio_q": (quantile(ratios, 0.025), quantile(ratios, 0.975)),
        "bs_q": (quantile(bss, 0.025), quantile(bss, 0.975)),
        "diff_q": (quantile(diffs, 0.025), quantile(diffs, 0.975)),
        "dropped": dropped, "lofo": (min(lofo), max(lofo)) if lofo else (None, None),
    }


def interval(q, fmt):
    return MISSING if q[0] is None else f"{fmt(q[0])} to {fmt(q[1])}"


def points(x):
    return MISSING if x is None else f"{x:.1f}"


def reading_rows(results, label_total):
    rows = []
    for name, title in READINGS:
        x = results[name]
        rows.append([title, x["bn"], x["bk"], pct(x["bs"]), interval(x["bs_q"], pct), x["cn"], x["ck"], pct(x["cs"]),
                     num(x["ratio"]), interval(x["ratio_q"], num), f"{num(x['lofo'][0])} to {num(x['lofo'][1])}"
                     if x["lofo"][0] is not None else MISSING,
                     pct(None if x["bs"] is None else 1 - x["bs"]), pct(None if x["cs"] is None else 1 - x["cs"]),
                     points(x["diff"]), interval(x["diff_q"], points), pct(x["bk"] / label_total if label_total else None),
                     x["dropped"]])
    return rows


READING_HEAD = ("| Bin | Bin frames | Bin {p} | Bin {p} share | 95% | Complement frames | Complement {p} | Complement {p} share "
                "| Ratio | 95% | Leave-one-folder-out | Bin {n} rate | Complement {n} rate | Difference (points) | 95% "
                "| Share of all {p} frames in the bin | Dropped draws |")


def per_folder_rows(per_folder, folders):
    rows = []
    for f in folders:
        bn, bk, cn, ck = per_folder.get(f, (0, 0, 0, 0))
        rows.append([f"`{f}`", bn, bk, pct(bk / bn if bn else None), cn, ck, pct(ck / cn if cn else None),
                     num(ratio_of((bn, bk, cn, ck)))])
    return rows


PER_FOLDER_HEAD = "| Folder | Bin frames | Bin {p} | Bin {p} share | Complement frames | Complement {p} | Complement {p} share | Ratio |"


def bars(x, label_total):
    bar1 = x["ratio"] is not None and x["ratio"] <= RATIO_BAR and x["ratio_q"][1] is not None and x["ratio_q"][1] < 1
    bar2 = (bar1 and x["bs"] <= SHARE_BAR and x["bs_q"][1] <= SHARE_Q_BAR and x["bk"] / label_total <= KEPT_SHARE_BAR)
    return bar1, bar2


def main(dump, strict_dir, discard_dir, confirm_dir, out_dir):
    start = time.time()
    if under(out_dir, LIBRARY):
        raise SystemExit(f"refusing to write under the library {LIBRARY}: {out_dir}")
    os.makedirs(out_dir, exist_ok=True)

    features = os.path.join(dump, "features")
    dump_folders = sorted(n[: -len(".tsv")] for n in os.listdir(features) if n.endswith(".tsv"))
    frames, columns = {}, None
    for folder in dump_folders:
        for r in read_tsv(os.path.join(features, folder + ".tsv")):
            assert r["folder"] == folder
            key = (r["folder"], r["file"])
            assert key not in frames
            frames[key] = r
            columns = columns or list(r)
    rest_columns = [c for c in columns if c not in ("folder", "file")]

    before = listing(dump_folders)
    write_listing(os.path.join(out_dir, "library.before.tsv"), before)
    folder_mtime = {f: os.stat(os.path.join(LIBRARY, f)).st_mtime_ns for f in dump_folders}
    top = {f: {} for f in dump_folders}
    for p in before:
        parent, name = os.path.split(p)
        folder = os.path.basename(parent)
        if os.path.normcase(parent) == os.path.normcase(os.path.join(LIBRARY, folder)) and folder in top:
            top[folder][name.lower()] = p

    kept_all = {k: int(k[1].lower() in top[k[0]]) for k in frames}
    in_dump = {f: 0 for f in dump_folders}
    on_disk = {f: 0 for f in dump_folders}
    for k, v in kept_all.items():
        in_dump[k[0]] += 1
        on_disk[k[0]] += v
    culled_folders = [f for f in dump_folders if 0 < on_disk[f] < in_dump[f]]
    population = sorted(k for k in frames if k[0] in culled_folders)
    kept = {k: kept_all[k] for k in population}
    pick = {k: int(pick_of(r)) for k, r in frames.items()}

    labels_path = os.path.join(out_dir, "labels.tsv")
    with open(labels_path, "w", encoding="utf-8", newline="") as out:
        out.write("\t".join(["folder", "file", "kept"] + rest_columns) + "\n")
        for k in population:
            out.write("\t".join([k[0], k[1], str(kept[k])] + [frames[k][c] for c in rest_columns]) + "\n")

    print("# Culling cues: the readings\n")
    print("## Folders\n")
    rows = []
    for f in dump_folders:
        role = "culled" if f in culled_folders else ("gone (left out)" if on_disk[f] == 0 else "intact (left out)")
        picks = sum(pick[k] for k in frames if k[0] == f)
        p45 = sum(1 for k in frames if k[0] == f and number(frames[k]["pitch"]) is not None
                  and abs(number(frames[k]["pitch"])) > PITCH_CUT)
        known = KNOWN_FOLDERS.get(f)
        rows.append([f"`{f}`", role, in_dump[f], on_disk[f], picks, p45,
                     " / ".join(str(v) for v in known) if known else MISSING, day(folder_mtime[f])])
    table("| Folder | Role | Dump frames | On disk | Dump picks | `|pitch|` > 45 | Known (frames / on disk / picks / pitch) "
          "| Folder mtime |", rows)
    n_kept = sum(kept.values())
    print(f"Culled folders: {len(culled_folders)} ({', '.join(culled_folders)}), {len(population)} frames, {n_kept} kept. "
          f"Known: {len(KNOWN_FOLDERS)} folders, 11196 frames, 1388 kept. "
          f"Same folders as known: {'yes' if culled_folders == sorted(KNOWN_FOLDERS) else 'no'}. "
          f"Left out: {len(dump_folders) - len(culled_folders)} folders, "
          f"{sum(in_dump[f] for f in dump_folders if f not in culled_folders)} frames.\n")
    print("## The first number\n")
    print(f"Kept share of the population: {n_kept} of {len(population)}, {pct(n_kept / len(population))} "
          f"(culled {pct(1 - n_kept / len(population))}).\n")

    print("## The label's own check: kept against the dump's sidecar value, per folder\n")
    print("A frame's value is `Pick` when its `xmp` or `dop` says `Pick`, else `Reject`, else `None`, else `-`.\n")
    rows = []
    for f in culled_folders:
        ks = [k for k in population if k[0] == f]
        row = [f"`{f}`"]
        for side in (1, 0):
            row += [sum(1 for k in ks if kept[k] == side and value_of(frames[k]) == v) for v in ("Pick", "Reject", "None", MISSING)]
        rows.append(row)
    rows.append(["all"] + [sum(r[i] for r in rows) for i in range(1, 9)])
    table("| Folder | Kept `Pick` | Kept `Reject` | Kept `None` | Kept `-` | Culled `Pick` | Culled `Reject` | Culled `None` "
          "| Culled `-` |", rows)

    print("## Bin sizes in the population against \"What is known\"\n")
    fired = bins_of(frames, population)
    no_pitch = sum(1 for k in population if number(frames[k]["pitch"]) is None)
    table("| Bin | This run | Known |", [[t, len(fired[n]), KNOWN_BINS.get(n, MISSING)] for n, t in READINGS]
          + [["no `pitch`", no_pitch, KNOWN_NO_PITCH]])

    print("## Sidecars of the kept frames and folder mtimes, from the listing\n")
    rows = []
    for f in culled_folders:
        ks = [k for k in population if k[0] == f and kept[k]]
        row = [f"`{f}`"]
        for ext in SIDECARS:
            times = []
            for k in ks:
                for name in (k[1] + ext, os.path.splitext(k[1])[0] + ext):
                    p = top[f].get(name.lower())
                    if p is not None:
                        times.append(before[p][1])
                        break
            row += [len(times), f"{day(min(times))} to {day(max(times))}" if times else MISSING]
        row.append(day(folder_mtime[f]))
        rows.append(row)
    table("| Folder | Kept frames' `.xmp` | `.xmp` mtime range | Kept frames' `.dop` | `.dop` mtime range | Folder mtime |", rows)

    print("## The primary and the secondary readings on the culled label\n")
    print(f"Population: the {len(culled_folders)} culled folders. Kept = on disk. Intervals: {DRAWS}-draw folder-cluster "
          f"bootstrap, seed {SEED}, one draw matrix for every reading of this population, 2.5% and 97.5% quantiles; a draw "
          "whose bin is empty or whose complement has no kept frame is dropped and counted. The ratio is the bin's kept "
          "share over the complement's; the difference is the bin's culled rate minus the complement's.\n")
    matrix = draw_matrix(len(culled_folders))
    per = {n: cells(population, kept, fired[n]) for n, _ in READINGS}
    results = {n: reading(per[n], culled_folders, matrix) for n, _ in READINGS}
    table(READING_HEAD.format(p="kept", n="culled"), reading_rows(results, n_kept))

    x = results[PRIMARY]
    bar1, bar2 = bars(x, n_kept)
    print("### Bars, primary reading (`|pitch|` > 45)\n")
    table("| Condition | Value | Met |", [
        ["Bar 1: ratio <= 0.5", num(x["ratio"]), "yes" if x["ratio"] is not None and x["ratio"] <= RATIO_BAR else "no"],
        ["Bar 1: ratio 97.5% quantile < 1", num(x["ratio_q"][1]), "yes" if x["ratio_q"][1] is not None and x["ratio_q"][1] < 1 else "no"],
        ["**Bar 1**", MISSING, "**met**" if bar1 else "**not met**"],
        ["Bar 2: bin kept share <= 25%", pct(x["bs"]), "yes" if x["bs"] <= SHARE_BAR else "no"],
        ["Bar 2: its 97.5% quantile <= 35%", pct(x["bs_q"][1]), "yes" if x["bs_q"][1] <= SHARE_Q_BAR else "no"],
        ["Bar 2: bin holds <= 5% of all kept frames", pct(x["bk"] / n_kept), "yes" if x["bk"] / n_kept <= KEPT_SHARE_BAR else "no"],
        ["**Bar 2** (needs bar 1)", MISSING, "**met**" if bar2 else "**not met**"],
    ])
    print(f"Follow-up rule: branch {1 if bar1 else 2}.\n")
    print("Secondary readings in the bars' form (not a pass on their own):\n")
    table("| Bin | Bar 1 form | Bar 2 form |", [[t, *("yes" if b else "no" for b in bars(results[n], n_kept))] for n, t in READINGS[1:]])

    print("### Per folder, culled label\n")
    for n, t in READINGS:
        print(f"{t}:\n")
        table(PER_FOLDER_HEAD.format(p="kept"), per_folder_rows(per[n], culled_folders))

    print("## Secondary readings on the pick label\n")
    print("A dump pick against the rest, the same bins; the sharpness decile is taken over each folder's dump frames.\n")
    pick_folders = [f for f in dump_folders if any(pick[k] for k in frames if k[0] == f)]
    for title, folders in ((f"`{PICK_ALONE}` alone", [PICK_ALONE]),
                           (f"every dump folder with at least one pick ({len(pick_folders)} folders)", pick_folders)):
        keys = sorted(k for k in frames if k[0] in folders)
        n_pick = sum(pick[k] for k in keys)
        f_bins = bins_of(frames, keys)
        p_per = {n: cells(keys, pick, f_bins[n]) for n, _ in READINGS}
        p_matrix = draw_matrix(len(folders))
        p_res = {n: reading(p_per[n], folders, p_matrix) for n, _ in READINGS}
        print(f"### {title}\n")
        print(f"{len(keys)} frames, {n_pick} picks ({pct(n_pick / len(keys))}). "
              + ("One cluster: the bootstrap draws the one folder every time, so its interval is the point."
                 if len(folders) == 1 else f"Own {DRAWS}-draw matrix over these {len(folders)} folders, seed {SEED}.") + "\n")
        table(READING_HEAD.format(p="pick", n="non-pick"), reading_rows(p_res, n_pick))
        if len(folders) > 1:
            print(f"Per folder, `|pitch|` > 45 on the pick label:\n")
            table(PER_FOLDER_HEAD.format(p="pick"), per_folder_rows(p_per[PRIMARY], folders))

    print("## Earlier labels against kept and culled\n")
    print("Read beside the culling, not pooled into it. Weighted shares use each round's `1/π` (`inclusion_prob`).\n")
    rows = []
    for (name, bad), d in zip(ROUNDS, (strict_dir, discard_dir, confirm_dir)):
        lab = [r for r in read_tsv(os.path.join(d, "labels.tsv")) if (r["folder"], r["file"]) in kept]
        for side, side_name in ((1, "kept"), (0, "culled")):
            rs = [r for r in lab if kept[(r["folder"], r["file"])] == side]
            nb = sum(1 for r in rs if r["label"] == bad)
            w = sum(1 / float(r["inclusion_prob"]) for r in rs)
            wb = sum(1 / float(r["inclusion_prob"]) for r in rs if r["label"] == bad)
            rows.append([name, side_name, len(rs), nb, len(rs) - nb, pct(nb / len(rs) if rs else None),
                         f"{w:.1f}", pct(wb / w if w else None)])
    table("| Round | Side | Labeled frames | `bad` / `discard` | Other | Share | Weighted frames | Weighted share |", rows)

    after = listing(dump_folders)
    write_listing(os.path.join(out_dir, "library.after.tsv"), after)
    lost = changed(before, after)
    added = sorted(set(after) - set(before))
    gone = sorted(set(before) - set(after))
    print("## Checks\n")
    table("| Check | Expected | This run |", [
        (f"Library files ({len(dump_folders)} folders, recursive) before / after", "equal", f"{len(before)} / {len(after)}"),
        ("Library paths changed / added / removed", "0 / 0 / 0", f"{len(lost) - len(added) - len(gone)} / {len(added)} / {len(gone)}"),
        ("`labels.tsv` rows", len(population), len(population)),
        ("`labels.tsv` SHA-256", MISSING, sha256(labels_path)),
    ])
    for p in lost[:20]:
        print(f"- `{p}`")
    print(f"Total {time.time() - start:.0f} s.", file=sys.stderr)


if __name__ == "__main__":
    if len(sys.argv) != 6:
        raise SystemExit(__doc__)
    sys.stdout.reconfigure(encoding="utf-8")
    main(*sys.argv[1:])
