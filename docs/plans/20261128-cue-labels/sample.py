"""Step 1: the blind "OK to discard?" sample of the cue-labels round, as
shuffled RAW copies with their manifest.

Usage: python -I -B sample.py <app-dump-dir> <strict-dir> <discard-dir> <confirm-dir> <out-dir> [--check]

<app-dump-dir> is the app dump of 2026-10-12, whose `features\\*.tsv` give the
frames and their readings; <strict-dir>, <discard-dir> and <confirm-dir> hold
the `manifest.tsv` of the three earlier labeling rounds, whose frames leave
the population. Writes `manifest.tsv`, `strata.tsv` and the library listings
into <out-dir>, copies the sampled RAW files (no sidecar) into
`<out-dir>\\sample\\` and prints the tables (Markdown) and the checks on
stdout. `--check` writes the manifest and the strata again, compares them
with the first run's and copies nothing.

Standard library only. Every library file is opened read-only (`rb`, by
`shutil.copy2` and `sha256`), never written.
"""

import csv
import hashlib
import os
import random
import shutil
import sys
import time

LIBRARY = r"D:\Photos\2026"
PITCH_CUT = 45.0
EAR_CUT = 0.137
STRATA = [("pitch45", 300), ("closed", 100), ("rest", 500)]
REST_FLOOR = 8
SIDECARS = (".xmp", ".dop")
RAW_COLUMNS = ["capture_time", "subsec", "xmp", "dop", "sharpness", "state", "eye_focus", "ear", "eyes_open", "yaw", "pitch",
               "roll", "eye_offset", "edge_gap"]
HEAD = ["name", "source", "folder", "file", "draw", "stratum", "inclusion_prob"] + RAW_COLUMNS
MISSING = "-"


# Follows `under` of docs/plans/_archived/20261017-bad-photo-labels/sample.py.
def under(path, root):
    p = os.path.normcase(os.path.realpath(path))
    r = os.path.normcase(os.path.realpath(root))
    return p == r or p.startswith(r.rstrip(os.sep) + os.sep)


# Follows `allocate` of docs/plans/_archived/20261017-bad-photo-labels/sample.py.
def allocate(sizes, total, floor):
    """`total` over the cells proportional to their sizes, at least `floor` (or the whole cell) each, largest remainder."""
    fixed = {}
    while True:
        free = [k for k in sizes if k not in fixed]
        left = total - sum(fixed.values())
        pop = sum(sizes[k] for k in free)
        quota = {k: left * sizes[k] / pop for k in free}
        low = [k for k in free if quota[k] < min(floor, sizes[k])]
        if not low:
            break
        for k in low:
            fixed[k] = min(floor, sizes[k])
    out = dict(fixed)
    out.update({k: int(q) for k, q in quota.items()})
    rest = total - sum(out.values())
    for k in sorted(quota, key=lambda k: (-(quota[k] - int(quota[k])), k))[:rest]:
        out[k] += 1
    assert all(out[k] <= sizes[k] for k in sizes)
    return out


# Follows `listing` of docs/plans/_archived/20261017-bad-photo-labels/sample.py.
def listing(folders):
    out = {}
    for folder in folders:
        for root, _, files in os.walk(os.path.join(LIBRARY, folder)):
            for name in files:
                p = os.path.join(root, name)
                st = os.stat(p)
                out[p] = (st.st_size, st.st_mtime_ns)
    return out


# Follows `sha256` of docs/plans/_archived/20261017-bad-photo-labels/sample.py.
def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            b = f.read(1 << 22)
            if not b:
                return h.hexdigest()
            h.update(b)


def file_sha256(path):
    return sha256(path) if os.path.isfile(path) else "(none)"


def write_listing(path, entries):
    with open(path, "w", encoding="utf-8", newline="") as out:
        out.write("path\tsize\tmtime_ns\n")
        for p in sorted(entries):
            size, mtime = entries[p]
            out.write(f"{p}\t{size}\t{mtime}\n")


def read_listing(path):
    with open(path, encoding="utf-8", newline="") as f:
        return {r["path"]: (int(r["size"]), int(r["mtime_ns"])) for r in csv.DictReader(f, delimiter="\t")}


def changed(before, after):
    return sorted(p for p in set(before) | set(after) if before.get(p) != after.get(p))


def read_tsv(path):
    with open(path, encoding="utf-8", newline="") as f:
        return list(csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE))


def number(v):
    return None if v == MISSING else float(v)


def table(head, rows):
    print(head)
    print("|" + " --- |" * head.count("|", 1))
    for r in rows:
        print("| " + " | ".join(str(c) for c in r) + " |")
    print()


def stratum_of(r):
    pitch, ear = number(r["pitch"]), number(r["ear"])
    if pitch is not None and abs(pitch) > PITCH_CUT:
        return "pitch45"
    if ear is not None and ear <= EAR_CUT:
        return "closed"
    return "rest"


def main(dump, strict_dir, discard_dir, confirm_dir, out_dir, check):
    start = time.time()
    if under(out_dir, LIBRARY):
        raise SystemExit(f"refusing to write under the library {LIBRARY}: {out_dir}")
    sample_dir = os.path.join(out_dir, "sample")
    assert not under(sample_dir, LIBRARY)
    os.makedirs(out_dir, exist_ok=True)

    features = os.path.join(dump, "features")
    dump_folders = sorted(n[: -len(".tsv")] for n in os.listdir(features) if n.endswith(".tsv"))
    frames = {}
    for folder in dump_folders:
        for r in read_tsv(os.path.join(features, folder + ".tsv")):
            assert r["folder"] == folder
            key = (r["folder"], r["file"])
            assert key not in frames
            frames[key] = r

    before = listing(dump_folders)
    if not check:
        write_listing(os.path.join(out_dir, "library.before.tsv"), before)
    present = set(before)

    def source(key):
        return os.path.join(LIBRARY, key[0], key[1])

    in_library = {f: 0 for f in dump_folders}
    in_dump = {f: 0 for f in dump_folders}
    for key in frames:
        in_dump[key[0]] += 1
        in_library[key[0]] += source(key) in present
    intact = [f for f in dump_folders if in_library[f] == in_dump[f]]

    rounds = [("strict", strict_dir), ("discard", discard_dir), ("confirm", confirm_dir)]
    labeled = {}
    for name, d in rounds:
        for r in read_tsv(os.path.join(d, "manifest.tsv")):
            key = (r["folder"], r["file"])
            assert key in frames, key
            assert key not in labeled, key
            labeled[key] = name

    stratum = {key: stratum_of(r) for key, r in frames.items() if key[0] in intact}
    population = sorted(k for k in stratum if k not in labeled)
    removed = {s: sum(1 for k in stratum if k in labeled and stratum[k] == s) for s, _ in STRATA}

    rng = random.Random(0)
    prob, drawn, strata_rows = {}, [], []
    for name, take in STRATA[:-1]:
        cell = [k for k in population if stratum[k] == name]
        chosen = rng.sample(cell, take)
        drawn.extend(chosen)
        for k in cell:
            prob[k] = take / len(cell)
        strata_rows.append((name, MISSING, len(cell), take))
    rest_name, rest_total = STRATA[-1]
    rest_cells = {f: [k for k in population if stratum[k] == rest_name and k[0] == f] for f in intact}
    rest_alloc = allocate({f: len(c) for f, c in rest_cells.items()}, rest_total, REST_FLOOR)
    for f in intact:
        cell, take = rest_cells[f], rest_alloc[f]
        chosen = rng.sample(cell, take)
        drawn.extend(chosen)
        for k in cell:
            prob[k] = take / len(cell)
        strata_rows.append((rest_name, f, len(cell), take))
    order = list(drawn)
    rng.shuffle(order)
    n_total = sum(t for _, t in STRATA)
    assert len(set(order)) == len(order) == n_total
    assert set(prob) == set(population)
    assert not any(k in labeled for k in order)
    name_of = {k: f"{i:04d}_{k[0]}_{k[1]}" for i, k in enumerate(order, start=1)}

    print("## Folders\n")
    table(
        "| Folder | Dump frames | In the library | Intact | Labeled (strict / discard / confirm) |",
        [[f"`{f}`", in_dump[f], in_library[f], "yes" if f in intact else "no",
          " / ".join(str(sum(1 for k, v in labeled.items() if k[0] == f and v == r)) for r, _ in rounds)] for f in dump_folders],
    )
    print(f"Intact folders: {len(intact)}, {sum(in_dump[f] for f in intact)} frames. "
          f"Excluded (culled) folders: {len(dump_folders) - len(intact)}, {sum(in_dump[f] for f in dump_folders if f not in intact)} dump frames, "
          f"{sum(in_library[f] for f in dump_folders if f not in intact)} still in the library.\n")

    print("## Exclusion\n")
    table(
        "| Round | Labeled frames | In the intact folders |",
        [[r, sum(1 for v in labeled.values() if v == r), sum(1 for k, v in labeled.items() if v == r and k[0] in intact)] for r, _ in rounds]
        + [["all", len(labeled), sum(1 for k in labeled if k[0] in intact)]],
    )

    print("## Strata\n")
    srows = []
    for name, take in STRATA:
        cell = [k for k in population if stratum[k] == name]
        srows.append([f"`{name}`", sum(1 for k in stratum if stratum[k] == name), removed[name], len(cell), take,
                      sum(1 for k in order if stratum[k] == name), f"{take / len(cell):.6g}" + (" (overall)" if name == rest_name else "")])
    srows.append(["all", len(stratum), sum(removed.values()), len(population), n_total, len(order), f"{n_total / len(population):.6g}"])
    table("| Stratum | Intact frames | Removed by the exclusion | Population | Allocated | Drawn | `π` |", srows)

    print("Per folder, sampled count by stratum:\n")
    table(
        "| Folder | Population | `rest` population | Sampled | `pitch45` | `closed` | `rest` |",
        [[f"`{f}`", sum(1 for k in population if k[0] == f), len(rest_cells[f]), sum(1 for k in order if k[0] == f)]
         + [sum(1 for k in order if k[0] == f and stratum[k] == s) for s, _ in STRATA] for f in intact],
    )
    up = sum(1 for k in order if stratum[k] == "pitch45" and number(frames[k]["pitch"]) > 0)
    down = sum(1 for k in order if stratum[k] == "pitch45" and number(frames[k]["pitch"]) < 0)
    pop_up = sum(1 for k in population if stratum[k] == "pitch45" and number(frames[k]["pitch"]) > 0)
    pop_down = sum(1 for k in population if stratum[k] == "pitch45" and number(frames[k]["pitch"]) < 0)
    print(f"`pitch45` up / down: sampled {up} / {down}, population {pop_up} / {pop_down}.\n")
    weight = sum(1 / prob[k] for k in order)
    print(f"Sum of `1/π` over the sample: {weight:.6f} (population {len(population)}).\n")
    assert abs(weight - len(population)) < 1e-6

    manifest_path = os.path.join(out_dir, "manifest.tsv")
    strata_path = os.path.join(out_dir, "strata.tsv")
    old_manifest, old_strata = file_sha256(manifest_path), file_sha256(strata_path)
    with open(manifest_path, "w", encoding="utf-8", newline="") as out:
        out.write("\t".join(HEAD) + "\n")
        for i, k in enumerate(order, start=1):
            r = frames[k]
            vals = [name_of[k], source(k), k[0], k[1], i, stratum[k], f"{prob[k]:.6g}"] + [r[c] for c in RAW_COLUMNS]
            out.write("\t".join(str(v) for v in vals) + "\n")
    with open(strata_path, "w", encoding="utf-8", newline="") as out:
        out.write("stratum\tfolder\tpopulation\tdrawn\tinclusion_prob\n")
        for name, folder, size, take in strata_rows:
            out.write(f"{name}\t{folder}\t{size}\t{take}\t{take / size:.6g}\n")
    new_manifest, new_strata = sha256(manifest_path), sha256(strata_path)
    print("## Files\n")
    table("| File | SHA-256 before this run | SHA-256 written |", [["`manifest.tsv`", old_manifest, new_manifest], ["`strata.tsv`", old_strata, new_strata]])

    absent = [source(k) for k in order if source(k) not in present]
    if absent:
        raise SystemExit(f"{len(absent)} sampled sources missing, first {absent[0]}")
    need = sum(before[source(k)][0] for k in order)
    free_bytes = shutil.disk_usage(os.path.splitdrive(os.path.abspath(out_dir))[0] + os.sep).free
    print("## Disk\n")
    table("| Item | GB |", [("Sampled RAW files", f"{need / 1e9:.1f}"), ("Free on the output drive", f"{free_bytes / 1e9:.1f}")])
    sys.stdout.flush()

    if check:
        after = listing(dump_folders)
        first = read_listing(os.path.join(out_dir, "library.after.tsv"))
        print("## Checks\n")
        table(
            "| Check | Expected | This run |",
            [
                ("`manifest.tsv` equal to the first run's", "yes", "yes" if old_manifest == new_manifest else "no"),
                ("`strata.tsv` equal to the first run's", "yes", "yes" if old_strata == new_strata else "no"),
                ("Library paths changed since the first run's `library.after.tsv` / during this run", "0 / 0",
                 f"{len(changed(first, after))} / {len(changed(before, after))}"),
            ],
        )
        print(f"Total {time.time() - start:.0f} s.")
        return

    os.makedirs(sample_dir, exist_ok=True)
    t = time.time()
    copied = skipped = 0
    for i, k in enumerate(order, start=1):
        src, dst = source(k), os.path.join(sample_dir, name_of[k])
        st = os.stat(src)
        if os.path.isfile(dst):
            dt = os.stat(dst)
            if dt.st_size == st.st_size and dt.st_mtime_ns == st.st_mtime_ns:
                skipped += 1
                continue
        shutil.copy2(src, dst)
        copied += 1
        if i % 50 == 0:
            print(f"copied {i} of {n_total} ({time.time() - t:.0f} s)", file=sys.stderr, flush=True)
    copy_time = time.time() - t

    t = time.time()
    mismatched = []
    for i, k in enumerate(order, start=1):
        src, dst = source(k), os.path.join(sample_dir, name_of[k])
        if os.path.getsize(src) != os.path.getsize(dst) or sha256(src) != sha256(dst):
            mismatched.append(name_of[k])
        if i % 50 == 0:
            print(f"hashed {i} of {n_total} ({time.time() - t:.0f} s)", file=sys.stderr, flush=True)
    hash_time = time.time() - t
    after = listing(dump_folders)
    write_listing(os.path.join(out_dir, "library.after.tsv"), after)
    files = sorted(os.listdir(sample_dir))
    sidecars = [p for p in files if p.lower().endswith(SIDECARS)]
    extra = sorted(set(files) - set(name_of.values()))
    lost = changed(before, after)
    added = sorted(set(after) - set(before))
    gone = sorted(set(before) - set(after))
    used = sum(os.path.getsize(os.path.join(sample_dir, p)) for p in files)

    print("## Checks\n")
    table(
        "| Check | Expected | This run |",
        [
            ("Copies whose size or SHA-256 differs from the source", "0", len(mismatched)),
            ("`.xmp` / `.dop` files in `sample\\`", "0", len(sidecars)),
            ("Files in `sample\\` / manifest rows", f"{n_total} / {n_total}", f"{len(files)} / {len(name_of)}"),
            ("Files in `sample\\` not in the manifest", "0", len(extra)),
            (f"Library files ({len(dump_folders)} folders, recursive) before / after", "equal", f"{len(before)} / {len(after)}"),
            ("Library paths changed / added / removed", "0 / 0 / 0", f"{len(lost) - len(added) - len(gone)} / {len(added)} / {len(gone)}"),
            ("Disk use of `sample\\` (GB)", f"{need / 1e9:.1f}", f"{used / 1e9:.1f}"),
        ],
    )
    for p in (mismatched + sidecars + extra + lost)[:20]:
        print(f"- `{p}`")
    print("## Wall time\n")
    table("| Stage | Seconds |", [(f"Copy ({copied} copied, {skipped} skipped)", f"{copy_time:.0f}"), ("SHA-256 of sources and copies", f"{hash_time:.0f}")])
    print(f"Total {time.time() - start:.0f} s.")


if __name__ == "__main__":
    args = sys.argv[1:]
    check = "--check" in args
    args = [a for a in args if a != "--check"]
    if len(args) != 5:
        raise SystemExit(__doc__)
    sys.stdout.reconfigure(encoding="utf-8")
    main(*args, check)
