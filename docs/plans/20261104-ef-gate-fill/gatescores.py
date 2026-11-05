"""Step 3: every candidate of efgate.md's "Pre-registration of the exploration"
on every frame of the 33 folders, the `τ` grid fixed by its percentile rule,
with label-free readings only.

Usage: python -I -B gatescores.py <app-dump-dir> <discard-model-dir> <gate-dir>

<app-dump-dir> is the 27-folder app dump (`features\\`, `embed.tsv`,
`genre_text.tsv`) and <discard-model-dir> the discard-model data folder, whose
`dump\\` holds the six fresh folders (same layout, with the old per-face dump
`dump\\faces\\`) and whose `scores_model.tsv` / `scores_fresh.tsv` hold
`ef-fill`; both dumps are read by the archived gate.py's `load` / `build`
(`badfilter.load` is not called). The 27 folders' old per-face dump is
`2026-10-23-face-choice\\faces\\`, the sibling of <discard-model-dir>.
<gate-dir> holds Step 2's per-face dump `faces\\<folder>.tsv`. Writes
`scores_gate.tsv` (`folder`, `file`, `ef`, `nose`, `max`, `window_ef` of the
judged face, then one column per candidate, higher keeps, `nan` where
missing) into <gate-dir>, prints the readings (Markdown) and saves them as
`gatescores.out.md` there. No label is read: no `labels.tsv` and no sidecar
is opened (an audit hook records every file opened); the picks are the
dumps' own, as `gate.load` derives them. A failed check stops the run before
anything is written.
"""

import os
import sys
import time

import numpy as np

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
PLANS = os.path.dirname(HERE)
ARCHIVED = PLANS if os.path.basename(PLANS) == "_archived" else os.path.join(PLANS, "_archived")
sys.path.insert(0, os.path.join(ARCHIVED, "20261017-bad-photo-labels"))
import badfilter as bf  # noqa: E402

sys.path.insert(0, os.path.join(ARCHIVED, "20261018-discard-labels"))
import discardfilter as df  # noqa: E402

sys.path.insert(0, os.path.join(ARCHIVED, "20261023-face-choice"))
import facechoiceread as fcr  # noqa: E402

rf, gate = bf.rf, bf.gate

QS = df.QS
FAMILIES = ["nose", "max"]
GRID_PCTS = [80.0, 90.0, 95.0, 97.5]
DIST_PCTS = [50.0, 80.0, 90.0, 95.0, 97.5, 99.0]
EYE_REF = 0.10
MIN_FACE = np.float32(60.0)
YAW_BANDS = [("0–15", 0, 15), ("15–30", 15, 30), ("30–60", 30, 60), ("60+", 60, np.inf)]
MESH_COLS = ["mesh_left_eye", "mesh_right_eye", "mesh_nose", "eye_offset", "nose_offset", "max_offset"]
FACE_CHOICE = "2026-10-23-face-choice"
EXACT, FEAT4 = 1e-9, 5.1e-5
DUMPS = {
    "dump": {"title": "27-folder dump", "n": 32459, "picks": 4419, "ef_tol": EXACT, "eo_tol": EXACT, "scores": "scores_model.tsv",
             "missing": {"ef": "4022", "af-whole": "1446", "ef-fill": "1002"}, "miss_unit": "frames", "pl": {"ef": ("4.44", "9.14", "15.59")}},
    "fresh": {"title": "six fresh folders", "n": 14525, "picks": 1795, "ef_tol": FEAT4, "eo_tol": FEAT4, "scores": "scores_fresh.tsv",
              "missing": {"ef": "9.00", "ef-fill": "1.18"}, "miss_unit": "%", "pl": {"ef": ("4.07", "7.30", "14.99"), "ef-fill": ("2.84", "5.96", "13.09")}},
}


def fnum(v):
    return np.nan if v in ("-", "err", "") else float(v)


def load_faces(path, folders, index, n):
    """`facechoiceread.load_faces` with Step 2's columns on each face, over the dump's own folders, and the mesh-column check over every face row."""
    frames = [None] * n
    extra, rows_seen, partial, mismatch = 0, 0, 0, 0
    for name in sorted(os.listdir(path)):
        if not name.endswith(".tsv") or name[:-4] not in folders:
            continue
        for r in bf.read_tsv(os.path.join(path, name)):
            if r["face"] != "-":
                rows_seen += 1
                dash = [r[c] == "-" for c in MESH_COLS]
                partial += any(dash) and not all(dash)
                small = max(np.float32(r["w"]), np.float32(r["h"])) < MIN_FACE
                mismatch += all(dash) != (r["scored"] == "window" and small)
            i = index.get((r["folder"], r["file"]))
            if i is None:
                extra += 1
                continue
            if frames[i] is None:
                frames[i] = {"af": r["af"] == "1", "crop": [], "whole": []}
            if r["face"] == "-":
                continue
            w, h = float(r["w"]), float(r["h"])
            frames[i][r["set"]].append({
                "box": (float(r["x"]), float(r["y"]), w, h),
                "area": float(np.float32(w) * np.float32(h)),
                "score": float(r["score"]),
                "conf": bool(np.float32(r["score"]) >= fcr.FLOOR),
                "nearest": r["nearest"] == "1",
                "ef": float(r["eye_focus"]),
                "nose": fnum(r["nose_offset"]),
                "eye": fnum(r["eye_offset"]),
                "max": fnum(r["max_offset"]),
                "window_ef": float(r["window_ef"]),
                "yaw": fnum(r["yaw"]),
            })
    return frames, extra, (rows_seen, partial, mismatch)


def field(face, key):
    return np.nan if face is None else face[key]


def gap(a, b):
    m = ~np.isnan(a) & ~np.isnan(b)
    return float(np.max(np.abs(a[m] - b[m]))) if m.any() else 0.0


def differ(a, b, tol):
    return int(((np.isnan(a) != np.isnan(b)) | (np.abs(np.nan_to_num(a) - np.nan_to_num(b)) > tol)).sum())


def pct(num, den, digits=2):
    return f"{100 * num / den:.{digits}f}" if den else "-"


def yn(ok):
    return "yes" if ok else "no"


def gname(kind, s, tau):
    return f"{kind}({s},{tau:.3f})"


def build_dump(key, root, old_faces, scores_path, gate_dir):
    """One dump: `gate.build`'s frames, the judged and filled faces from Step 2's dump, and its checks."""
    cfg = DUMPS[key]
    frames, missing = gate.load(root)
    d = gate.build(frames, gate.load_text(os.path.join(root, "genre_text.tsv")))
    rows = d["rows"]
    n = len(rows)
    index = {(f["folder"], f["file"]): i for i, f in enumerate(rows)}
    faces_of, extra, mesh_check = load_faces(os.path.join(gate_dir, "faces"), set(d["folders"]), index, n)
    old_of, _ = fcr.load_faces(old_faces, index, n)
    no_rows = sum(fr is None for fr in faces_of)
    no_old = sum(fr is None for fr in old_of)
    if no_rows or no_old:
        raise SystemExit(f"{key}: {no_rows} frames without a row in the new face dump, {no_old} in the old one")

    af_face, fill_face = [None] * n, [None] * n
    af, afw, old_af, old_afw = (np.full(n, np.nan) for _ in range(4))
    has_af = np.zeros(n, dtype=bool)
    for i in range(n):
        fc, vc, _, _ = fcr.choose(faces_of[i])
        af_face[i], af[i], afw[i] = fc["af"], vc["af"], vc["af-whole"]
        fill_face[i] = fc["af"] if fc["af"] is not None else fc["af-whole"]
        has_af[i] = faces_of[i]["af"]
        _, ov, _, _ = fcr.choose(old_of[i])
        old_af[i], old_afw[i] = ov["af"], ov["af-whole"]
    ef = af
    ef_fill = np.where(np.isnan(ef), afw, ef)
    stored = np.array([np.nan if f["ef"] is None else f["ef"] for f in rows])

    feat_eo = {}
    for name in sorted(os.listdir(os.path.join(root, "features"))):
        if name.endswith(".tsv"):
            for r in bf.read_tsv(os.path.join(root, "features", name)):
                feat_eo[(name[:-4], r["file"])] = fnum(r["eye_offset"])
    feat_eo = np.array([feat_eo[(f["folder"], f["file"])] for f in rows])

    v = {k: np.array([field(f, k) for f in af_face]) for k in ("nose", "eye", "max", "window_ef", "yaw")}
    vf = {k: np.array([field(f, k) for f in fill_face]) for k in ("nose", "max", "window_ef")}
    prior, n_prior = rf.load_scores(scores_path, d)
    return {
        "d": d, "rows": rows, "n": n, "missing": missing, "extra": extra, "pick": d["pick"], "tb": np.random.default_rng(0).random(n),
        "has_af": has_af, "ef": ef, "ef_fill": ef_fill, "afw": afw, "stored": stored, "old_af": old_af, "old_afw": old_afw,
        "feat_eo": feat_eo, "v": v, "vf": vf, "prior_fill": prior["ef-fill"], "prior_ef": prior.get("ef"), "n_prior": n_prior,
        "mesh_check": mesh_check, "filled": np.isnan(ef) & ~np.isnan(ef_fill),
    }


def main(app_dump, discard_dir, gate_dir):
    start = time.time()
    opened = []

    def audit(event, args):
        if event == "open" and isinstance(args[0], str):
            opened.append(os.path.normcase(os.path.abspath(args[0])))

    sys.addaudithook(audit)
    tee = bf.Tee(sys.stdout)
    sys.stdout = tee

    fc_faces = os.path.join(os.path.dirname(os.path.abspath(discard_dir)), FACE_CHOICE, "faces")
    fresh_root = os.path.join(discard_dir, "dump")
    D = {
        "dump": build_dump("dump", app_dump, fc_faces, os.path.join(discard_dir, "scores_model.tsv"), gate_dir),
        "fresh": build_dump("fresh", fresh_root, os.path.join(fresh_root, "faces"), os.path.join(discard_dir, "scores_fresh.tsv"), gate_dir),
    }
    load_time = time.time() - start
    print(f"loaded: {load_time:.0f} s", file=sys.stderr, flush=True)

    t0 = time.time()

    def cat(f):
        return np.concatenate([f(D[k]) for k in D])

    N = sum(x["n"] for x in D.values())
    pick = cat(lambda x: x["pick"])
    ef = cat(lambda x: x["ef"])
    meshed = {s: ~np.isnan(ef) & ~np.isnan(cat(lambda x, s=s: x["v"][s])) for s in FAMILIES}
    dist = {s: np.percentile(cat(lambda x, s=s: x["v"][s])[meshed[s]], DIST_PCTS) for s in FAMILIES}
    grid = {s: [round(float(np.percentile(cat(lambda x, s=s: x["v"][s])[meshed[s]], p)), 3) for p in GRID_PCTS] for s in FAMILIES}
    pairs = [(s, tau) for s in FAMILIES for tau in grid[s]]

    cands = ["ef-fill"] + [gname("ef-gate", s, t) for s, t in pairs] + [gname("ef-gate+fill", s, t) for s, t in pairs]
    for x in D.values():
        sc = {"ef": x["ef"], "ef-fill": x["ef_fill"]}
        gated, gated_fill = {}, {}
        for s, tau in pairs:
            g = x["v"][s] > tau
            gf = x["vf"][s] > tau
            sc[gname("ef-gate", s, tau)] = np.where(g, x["v"]["window_ef"], x["ef"])
            sc[gname("ef-gate+fill", s, tau)] = np.where(gf, x["vf"]["window_ef"], x["ef_fill"])
            gated[(s, tau)], gated_fill[(s, tau)] = g, gf
        gated[("eye", EYE_REF)] = x["v"]["eye"] > EYE_REF
        x["sc"], x["gated"], x["gated_fill"] = sc, gated, gated_fill
        x["cut_sc"] = {k: np.where(np.isnan(v), -np.inf, v) for k, v in sc.items()}
        x["cut_sc"]["random"] = x["tb"]
        x["cuts"] = {(k, q): bf.graded_cut(v, x["d"], x["tb"], q) for k, v in x["cut_sc"].items() for q in QS}

    print(f"scores and cuts: {time.time() - t0:.0f} s", file=sys.stderr, flush=True)

    # ---------------------------------------------------------------- checks
    checks = []
    for key, x in D.items():
        cfg, t = DUMPS[key], DUMPS[key]["title"]
        rows_seen, partial, mismatch = x["mesh_check"]
        arr = {"ef": x["ef"], "af-whole": x["afw"], "ef-fill": x["ef_fill"]}
        miss = {k: str(int(np.isnan(arr[k]).sum())) if cfg["miss_unit"] == "frames" else pct(np.isnan(arr[k]).sum(), x["n"]) for k in cfg["missing"]}
        checks += [
            (f"{t}: frames (with a vector) / picks", f"{cfg['n']} / {cfg['picks']}", f"{x['n']} / {int(x['pick'].sum())}",
             (x["n"], int(x["pick"].sum())) == (cfg["n"], cfg["picks"])),
            (f"{t}: `af` vs the stored `eye_focus` (features TSV): frames differing (> {cfg['ef_tol']:g}, or missing on one side only); largest gap",
             "0", f"{differ(x['ef'], x['stored'], cfg['ef_tol'])}; {gap(x['ef'], x['stored']):.2e}", differ(x["ef"], x["stored"], cfg["ef_tol"]) == 0),
            (f"{t}: `af` / `af-whole` vs the old per-face dump's: frames differing (> {EXACT:g}, or missing on one side only); largest gaps", "0 / 0",
             f"{differ(x['ef'], x['old_af'], EXACT)} / {differ(x['afw'], x['old_afw'], EXACT)}; {gap(x['ef'], x['old_af']):.2e} / {gap(x['afw'], x['old_afw']):.2e}",
             differ(x["ef"], x["old_af"], EXACT) == 0 and differ(x["afw"], x["old_afw"], EXACT) == 0),
            (f"{t}: `ef-fill` vs `{cfg['scores']}`'s: frames differing (> {EXACT:g}, or missing on one side only); largest gap", "0",
             f"{differ(x['ef_fill'], x['prior_fill'], EXACT)}; {gap(x['ef_fill'], x['prior_fill']):.2e}", differ(x["ef_fill"], x["prior_fill"], EXACT) == 0),
            (f"{t}: the judged face's `eye_offset` vs the features TSV's (> {cfg['eo_tol']:g}, or missing on one side only); largest gap", "0",
             f"{differ(x['v']['eye'], x['feat_eo'], cfg['eo_tol'])}; {gap(x['v']['eye'], x['feat_eo']):.2e}", differ(x["v"]["eye"], x["feat_eo"], cfg["eo_tol"]) == 0),
            (f"{t}: face rows with some but not all mesh columns `-` / with the mesh columns `-` not exactly where `scored` is `window` and the box's longer side is under 60 px (of the face rows)",
             f"0 / 0 (of {rows_seen})", f"{partial} / {mismatch} (of {rows_seen})", partial == 0 and mismatch == 0),
            (f"{t}: " + " / ".join(f"`{k}`" for k in cfg["missing"]) + f" missing {cfg['miss_unit']} (earlier readings)", " / ".join(cfg["missing"].values()),
             " / ".join(miss.values()), miss == cfg["missing"]),
            (f"{t}: frames left out of a cut, over every score and `q`", "0", str(sum(int(c[1].sum()) for c in x["cuts"].values())),
             all(not c[1].any() for c in x["cuts"].values())),
        ]
        if x["prior_ef"] is not None:
            checks.append((f"{t}: `ef` vs `{cfg['scores']}`'s `ef`: frames differing (> {EXACT:g}, or missing on one side only)", "0",
                           str(differ(x["ef"], x["prior_ef"], EXACT)), differ(x["ef"], x["prior_ef"], EXACT) == 0))
        for k, exp in cfg["pl"].items():
            got = tuple(f"{100 * (x['cuts'][(k, q)][0] & x['pick']).sum() / x['pick'].sum():.2f}" for q in QS)
            checks.append((f"{t}: `{k}` `PL` % of its picks q5 / q10 / q20 (earlier readings)", " / ".join(exp), " / ".join(got), got == exp))
    checks.append(("Frames / picks over the 33 folders", "46984 / 6214", f"{N} / {int(pick.sum())}", (N, int(pick.sum())) == (46984, 6214)))
    dup = [s for s in FAMILIES if len(set(grid[s])) != len(grid[s])]
    checks.append(("The four `τ` of each family distinct after rounding", "yes", yn(not dup), not dup))
    compute_time = time.time() - t0

    def opened_check():
        labels = sorted({p for p in opened if os.path.basename(p) == "labels.tsv"})
        sidecars = sorted({p for p in opened if os.path.splitext(p)[1] in (".xmp", ".dop")})
        return labels, sidecars

    labels_open, sidecars_open = opened_check()
    checks.append(("`labels.tsv` files opened anywhere", "none", ", ".join(f"`{p}`" for p in labels_open) or "none", not labels_open))
    checks.append(("Sidecars (`.xmp` / `.dop`) opened anywhere", "none", ", ".join(f"`{p}`" for p in sidecars_open) or "none", not sidecars_open))
    ok = all(c[3] for c in checks)

    # ---------------------------------------------------------------- print
    print("# `eye_focus` gate and fill: the candidate scores and the label-free readings\n")
    print(
        "Every candidate of efgate.md's \"Pre-registration of the exploration\" on every frame of the 33 folders (the 27-folder dump and the six "
        "fresh folders, each through the archived `gate.load` / `gate.build`), from Step 2's per-face dump. `ef` is the judged face's (`af`, "
        "`facechoiceread.choose`) `eye_focus`; `ef-gate(s, τ)` takes that face's `window_ef` where its `s` > `τ`, else `ef`; `ef-fill` fills a "
        "missing `ef` with the `af-whole` face's; `ef-gate+fill(s, τ)` gates the face behind `ef-fill`'s value, the filled face included. No "
        "label is read; the picks are the dumps' own.\n"
    )
    print("## Checks\n")
    gate.table("| Check (label-free) | Expected | This run | Result |", [(c, e, v, "pass" if k else "FAIL") for c, e, v, k in checks])
    if not ok:
        sys.stdout = tee.stream
        raise SystemExit("a check failed; nothing written")

    print("## Data\n")
    drow = []
    for key, x in D.items():
        t = DUMPS[key]["title"]
        drow += [
            (f"{t}: folders / frames / picks / frames without a vector (dropped)", f"{len(x['d']['folders'])} / {x['n']} / {int(x['pick'].sum())} / {x['missing']}"),
            (f"{t}: face-dump rows of files outside the frames (copies or frames without a vector)", str(x["extra"])),
            (f"{t}: frames with a trusted AF point", f"{int(x['has_af'].sum())} ({pct(x['has_af'].sum(), x['n'])}%)"),
        ]
    drow.append(("33 folders: frames / picks", f"{N} / {int(pick.sum())}"))
    for s in FAMILIES:
        drow.append((f"33 folders: meshed `af` faces with `{s}` (frames with `ef` whose judged face has a mesh)", f"{int(meshed[s].sum())} ({pct(meshed[s].sum(), N)}% of frames)"))
    gate.table("| Count | Value |", drow)

    print("## The distribution and the grid\n")
    print(
        "The percentiles of `s` over the meshed `af` faces of the 33 folders (`numpy.percentile`, linear interpolation). The grid: the 80th, "
        "90th, 95th and 97.5th percentiles rounded to 3 decimals (efgate.md, \"The `τ` grid rule\"); a face is gated where `s` > `τ`.\n"
    )
    gate.table("| `s` | n | " + " | ".join(f"p{p:g}" for p in DIST_PCTS) + " |",
               [[f"`{s}`", int(meshed[s].sum())] + [f"{v:.4f}" for v in dist[s]] for s in FAMILIES])
    gate.table("| `s` | " + " | ".join(f"`τ` (p{p:g})" for p in GRID_PCTS) + " |", [[f"`{s}`"] + [f"{t:.3f}" for t in grid[s]] for s in FAMILIES])

    def gsum(k, fill=False):
        return np.concatenate([(x["gated_fill"] if fill else x["gated"])[k] for x in D.values()])

    win = cat(lambda x: x["v"]["window_ef"])
    yaw = np.abs(cat(lambda x: x["v"]["yaw"]))
    filled = cat(lambda x: x["filled"])
    keys = pairs + [("eye", EYE_REF)]

    def kname(k):
        return f"`eye_offset` > {EYE_REF:.2f} (reference)" if k[0] == "eye" else f"`{k[0]}` > {k[1]:.3f}"

    print("## Gated shares\n")
    print(
        "Per (`s`, `τ`), the frames whose judged (`af`) face is gated, as a share of the 33 folders' frames, of the meshed `af` faces and of "
        "the picks; for `ef-gate+fill`, the filled faces (no crop face, the `af-whole` face's value) gated beside them. `eye_offset` > 0.10 "
        "gating is a reference only, not a candidate.\n"
    )
    grow = []
    for k in keys:
        g = gsum(k)
        base = meshed["nose"] if k[0] != "max" else meshed["max"]
        r = [kname(k), int(g.sum()), pct(g.sum(), N), pct(g.sum(), base.sum()), int((g & pick).sum()), pct((g & pick).sum(), pick.sum())]
        if k[0] == "eye":
            r += ["-", "-"]
        else:
            gf = gsum(k, True) & filled
            r += [int(gf.sum()), int((gf & pick).sum())]
        grow.append(r)
    gate.table("| Gate | Gated `af` faces | % of frames | % of meshed `af` faces | Gated picks | % of picks | Gated filled faces (`ef-gate+fill`) | Of them picks |", grow)

    print("### By |yaw| band\n")
    print("The meshed `af` faces by the |yaw| of their head pose (`[lo, hi)` degrees), and per gate the share of the band gated. \"No pose\": meshed faces without a pose.\n")
    bands = [(lbl, (yaw >= lo) & (yaw < hi)) for lbl, lo, hi in YAW_BANDS] + [("no pose", np.isnan(yaw))]
    base = meshed["nose"]
    brow = [["Meshed `af` faces"] + [f"{int((bm & base).sum())}" for _, bm in bands]]
    for k in keys:
        g = gsum(k)
        brow.append([kname(k)] + [f"{int((g & bm & base).sum())} ({pct((g & bm & base).sum(), (bm & base).sum(), 1)}%)" for _, bm in bands])
    gate.table("| Gate | " + " | ".join(lbl for lbl, _ in bands) + " |", brow)
    print("The gated faces' spread over the bands (% of the gate's gated faces):\n")
    srow = []
    for k in keys:
        g = gsum(k)
        srow.append([kname(k)] + [pct((g & bm).sum(), g.sum(), 1) for _, bm in bands])
    gate.table("| Gate | " + " | ".join(lbl for lbl, _ in bands) + " |", srow)

    print("### The window against the mesh on the gated faces\n")
    print("Among each gate's gated `af` faces, the share whose `window_ef` is lower / equal / higher than their `ef`, and the share with `window_ef` = 0 (Decision 8: kept as 0).\n")
    wrow = []
    for k in keys:
        g = gsum(k)
        wrow.append([kname(k), int(g.sum()), pct((g & (win < ef)).sum(), g.sum(), 1), pct((g & (win == ef)).sum(), g.sum(), 1),
                     pct((g & (win > ef)).sum(), g.sum(), 1), pct((g & (win == 0)).sum(), g.sum(), 1)])
    gate.table("| Gate | Gated | Lower % | Equal % | Higher % | `window_ef` = 0 % |", wrow)

    print("## Missing shares\n")
    print("The share of frames each score leaves missing (sorted last), per dump and over the 33 folders. A gate changes no missing value.\n")
    mrow = []
    for k in ["ef"] + cands:
        cells = [f"`{k}`"]
        for x in D.values():
            cells.append(f"{int(np.isnan(x['sc'][k]).sum())} ({pct(np.isnan(x['sc'][k]).sum(), x['n'])}%)")
        tot = sum(int(np.isnan(x["sc"][k]).sum()) for x in D.values())
        cells.append(f"{tot} ({pct(tot, N)}%)")
        mrow.append(cells)
    gate.table("| Score | 27-folder dump | Six fresh folders | 33 folders |", mrow)

    print("## Label-free cuts on the 33 folders\n")
    print(
        f"Per folder, the bottom `floor(q * n / 100)` frames by the score, a missing score last, ties by each dump's seed-0 draw "
        f"(`bf.graded_cut`). Removed: the share of the 33 folders' {N} frames inside the cut; `PL`: the share of their {int(pick.sum())} picks "
        f"inside it. The selection rule's label-free conditions: `PL` <= {df.PL_BAR:.0f}% and removed >= {df.REMOVED_BAR}% at q5 or q10. "
        "`ef` and random (seed 0) are references.\n"
    )
    lrow, verdict = [], {}
    for k in ["ef", "random"] + cands:
        for q in QS:
            rem = sum(int(x["cuts"][(k, q)][0].sum()) for x in D.values())
            pl_n = sum(int((x["cuts"][(k, q)][0] & x["pick"]).sum()) for x in D.values())
            r_pct, pl = 100 * rem / N, 100 * pl_n / pick.sum()
            verdict[(k, q)] = (pl <= df.PL_BAR, r_pct >= df.REMOVED_BAR)
            lrow.append([f"`{k}`" + (" (reference)" if k in ("ef", "random") else ""), f"q{q}", rem, f"{r_pct:.2f}", pl_n, f"{pl:.2f}", yn(verdict[(k, q)][0]), yn(verdict[(k, q)][1])])
    gate.table("| Score | Cut | Frames removed | Removed % | Picks removed | `PL` % | `PL` <= 5% | Removed >= 4.5% |", lrow)
    gate.table("| Candidate | q5 passes | q10 passes | Eligible (q5 or q10) |",
               [[f"`{k}`", yn(all(verdict[(k, 5)])), yn(all(verdict[(k, 10)])), yn(all(verdict[(k, 5)]) or all(verdict[(k, 10)]))] for k in cands])

    out_tsv = os.path.join(gate_dir, "scores_gate.tsv")
    with open(out_tsv, "w", encoding="utf-8", newline="") as out:
        out.write("folder\tfile\tef\tnose\tmax\twindow_ef\t" + "\t".join(cands) + "\n")
        for x in D.values():
            for i, f in enumerate(x["rows"]):
                vals = [x["ef"][i], x["v"]["nose"][i], x["v"]["max"][i], x["v"]["window_ef"][i]] + [x["sc"][k][i] for k in cands]
                out.write(f"{f['folder']}\t{f['file']}\t" + "\t".join(repr(float(v)) for v in vals) + "\n")

    print("## Files opened\n")
    print("Every file this run opened outside the Python installation and the plan folders, by directory (the audit hook), with the `labels.tsv` and sidecar check repeated after the write.\n")
    labels_open, sidecars_open = opened_check()
    py_roots = tuple(os.path.normcase(os.path.abspath(p)) for p in (sys.prefix, sys.base_prefix, PLANS))
    by_dir = {}
    for p in opened:
        if p.startswith(py_roots):
            continue
        by_dir.setdefault(os.path.dirname(p), set()).add(os.path.basename(p))
    def names(ns):
        if len(ns) <= 4:
            return ", ".join(f"`{x}`" for x in sorted(ns))
        kinds = {}
        for x in ns:
            kind = ".output.txt" if x.endswith(".output.txt") else os.path.splitext(x)[1]
            kinds[kind] = kinds.get(kind, 0) + 1
        return ", ".join(f"{c} `{k}`" for k, c in sorted(kinds.items()))

    gate.table("| Directory | Files | Names |", [[f"`{dp}`", len(ns), names(ns)] for dp, ns in sorted(by_dir.items())])
    gate.table("| Check | Result |", [("`labels.tsv` opened anywhere", "none" if not labels_open else ", ".join(labels_open)),
                                       ("Sidecars opened anywhere", "none" if not sidecars_open else ", ".join(sidecars_open))])

    print("## Wall time\n")
    gate.table("| Stage | Seconds |", [("Load (both dumps, both per-face dumps, features, prior scores)", f"{load_time:.0f}"), ("Scores, cuts and checks", f"{compute_time:.0f}")])
    print(f"Total {time.time() - start:.0f} s.\n")
    sys.stdout = tee.stream
    with open(os.path.join(gate_dir, "gatescores.out.md"), "w", encoding="utf-8", newline="") as out:
        out.write("".join(tee.parts))
    if labels_open or sidecars_open:
        raise SystemExit("a label or sidecar file was opened")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    sys.stdout.reconfigure(encoding="utf-8")
    main(*sys.argv[1:4])
