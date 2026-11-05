"""Step 4: the exploration read of efgate.md's "Pre-registration of the
exploration" on the three rounds of blind "OK to discard?" labels: the tuning
of `(s*, τ*)` on rounds 1 and 2, the selection on round 3 and the stop rule.

Usage: python -I -B gateread.py <app-dump-dir> <discard-model-dir> <gate-dir> <strict-labels-dir> <discard-labels-dir> <model-search-dir>

<app-dump-dir> is the 27-folder app dump and <discard-model-dir> the
discard-model data folder, whose `dump\\` holds the six fresh folders, both
read by the archived gate.py's `load` / `build` (`badfilter.load` is not
called); <discard-model-dir> also holds round 3 (`manifest.tsv`, `strata.tsv`,
`labels.tsv`), `scores_model.tsv` and `scores_fresh.tsv` (`ef-fill`, and
`LR/T` for round 3's strata). Round 1's and round 2's `G0` bands come from
`scores_reject.tsv` in `2026-10-16-reject-filter-eval\\`, the sibling of
<discard-model-dir>. <gate-dir> holds Step 3's `scores_gate.tsv` and Step 2's
per-face dump `faces\\<folder>.tsv` (the judged face's |yaw|);
<strict-labels-dir> the strict sample's `manifest.tsv` only (its `labels.tsv`
is never opened); <discard-labels-dir> round 1 and <model-search-dir> round 2
(`manifest.tsv`, `strata.tsv`, `labels.tsv`, and round 2's
`scores_discard.tsv` for its `G0 ⊕ ef` bands). Prints the tables (Markdown)
and saves them as `gateread.out.md` in <gate-dir>; refuses to run when that
file exists, so the read happens once. Every value is computed before
anything is printed; the label-free checks run before any `labels.tsv` is
opened, and a failure stops the read there. Nothing is written on a failed
check.
"""

import os
import sys
import time
import warnings

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

sys.path.insert(0, HERE)
import gatescores as gs  # noqa: E402

rf, gate = bf.rf, bf.gate

# Set only by a dry run on synthetic labels: the label reproduction checks are printed but do not stop the read.
DRY_RUN = False

QS = df.QS
FAMILIES = gs.FAMILIES
GRID = {"nose": (0.149, 0.215, 0.286, 0.357), "max": (0.165, 0.232, 0.300, 0.364)}
PAIRS = [(s, t) for s in FAMILIES for t in GRID[s]]
GATES = [gs.gname("ef-gate", s, t) for s, t in PAIRS]
FILLS = [gs.gname("ef-gate+fill", s, t) for s, t in PAIRS]
REFS = ["ef", "random"]
ALL = REFS + ["ef-fill"] + GATES + FILLS
REJECT_EVAL = "2026-10-16-reject-filter-eval"
WINNER3 = "LR/T"
ROUNDS = (1, 2, 3)
TITLE = {1: "Round 1 (discard-labels)", 2: "Round 2 (model-search)", 3: "Round 3 (discard-model, fresh six)"}
POP = {1: 31259, 2: 30659, 3: 14525}
EARLIER = {1: 1200, 2: 1800}
STRATA = {1: fcr.ROUNDS[1]["strata"], 2: fcr.ROUNDS[2]["strata"], 3: ["b05", "b10", "b20", "efonly", "rest"]}
LABELS = {1: (211, 389), 2: (248, 352), 3: (127, 473)}
N_SET = {"dump": (32459, 4419), "fresh": (14525, 1795)}
# The known numbers (efgate.md, "What was known before this section"; discardread.out.md and freshread.out.md).
KNOWN = {
    1: {"base": "48.45 [42.03, 55.11] ([40.74, 53.87])", "ef": "0.777 [0.716, 0.841] ([0.719, 0.832]), unweighted 0.803",
        "rr": "16.8 [1.2, 43.3] / 14.3 [2.9, 29.9] / 28.3 [15.7, 40.2]", "fill": "0.766 [0.699, 0.828] ([0.711, 0.821]), unweighted 0.807",
        "fill_diff": "-0.011 [-0.028, +0.003] ([-0.025, +0.007])"},
    2: {"base": "42.31 [35.85, 48.91] ([33.44, 51.61])", "ef": "0.785 [0.723, 0.844] ([0.693, 0.881]), unweighted 0.764",
        "rr": "5.5 [1.1, 14.5] / 12.9 [7.2, 21.8] / 12.8 [8.9, 17.9]", "fill": "0.775 [0.714, 0.835] ([0.682, 0.875]), unweighted 0.748",
        "fill_diff": "-0.010 [-0.022, -0.001] ([-0.018, -0.002])"},
    3: {"base": "60.56 [53.81, 67.29] ([53.52, 66.79])", "ef": "0.743 [0.679, 0.800] ([0.676, 0.796]), unweighted 0.676",
        "rr": "11.3 [6.3, 17.4] / 14.6 [10.2, 19.4] / 14.8 [11.5, 18.5]", "fill": "0.744 [0.681, 0.803] ([0.672, 0.804]), unweighted 0.745",
        "fill_diff": "+0.002 [-0.008, +0.012] ([-0.007, +0.012])", "miss_fill_diff": "+0.277 [+0.131, +0.407] ([+0.210, +0.410])"},
}
# Label-free readings to reproduce: `PL` % of each set's picks q5 / q10 / q20 and the removed % of the round's population.
KNOWN_PL = {
    "dump": {"ef": ("4.44", "9.14", "15.59"), "ef-fill": ("3.91", "6.99", "12.83")},
    "fresh": {"ef": ("4.07", "7.30", "14.99"), "ef-fill": ("2.84", "5.96", "13.09"), "random": ("4.46", "10.25", "20.33"), WINNER3: ("2.56", "5.91", "14.48")},
}
KNOWN_REMOVED = {1: fcr.EXPECTED[1]["removed"], 2: fcr.EXPECTED[2]["removed"], 3: ("4.98", "9.98", "20.00")}
# gatescores.out.md: `ef` on the 33 folders, `PL` % q5 / q10 / q20 and removed %; every candidate passes at q5, none at q10.
KNOWN_33 = (("4.33", "8.61", "15.42"), ("4.96", "9.97", "19.98"), "17 / 0")
MANIFEST_TOL = {1: 1e-6, 2: 1e-6, 3: 5.1e-5}
EXACT = 1e-9
YAW_BANDS = gs.YAW_BANDS


def nm(k):
    return f"`{k}`" + (" (reference)" if k in REFS else "")


def yn(ok):
    return "yes" if ok else "no"


def pct(num, den, digits=1):
    return f"{100 * num / den:.{digits}f}" if den else "-"


def same(a, b, tol=EXACT):
    return int(((np.isnan(a) != np.isnan(b)) | (np.abs(np.nan_to_num(a) - np.nan_to_num(b)) > tol)).sum())


def load_set(key, root, gate_dir, prior_path):
    """One dump: `gate.build`'s frames, `scores_gate.tsv`, the prior scores and the judged face's |yaw| from Step 2's dump."""
    frames, missing = gate.load(root)
    X = gate.build(frames, gate.load_text(os.path.join(root, "genre_text.tsv")))
    rows = X["rows"]
    n = len(rows)
    index = {(f["folder"], f["file"]): i for i, f in enumerate(rows)}
    sg, sg_rows = rf.load_scores(os.path.join(gate_dir, "scores_gate.tsv"), X)
    prior, _ = rf.load_scores(prior_path, X)
    faces, extra, mesh_check = gs.load_faces(os.path.join(gate_dir, "faces"), set(X["folders"]), index, n)
    no_rows = sum(fr is None for fr in faces)
    if no_rows:
        raise SystemExit(f"{key}: {no_rows} frames without a row in the per-face dump")
    fv = {k: np.full(n, np.nan) for k in ("ef", "nose", "max", "window_ef", "yaw")}
    for i, fr in enumerate(faces):
        fc, _, _, _ = fcr.choose(fr)
        f = fc["af"]
        if f is not None:
            for k in fv:
                fv[k][i] = f[k]
    tb = np.random.default_rng(0).random(n)
    sc = {k: sg[k] for k in ["ef", "ef-fill"] + GATES + FILLS}
    cut_sc = {k: np.where(np.isnan(v), -np.inf, v) for k, v in sc.items()}
    cut_sc["random"] = tb
    return {
        "key": key, "X": X, "rows": rows, "n": n, "missing": missing, "index": index, "sg": sg, "sg_rows": sg_rows, "prior": prior, "fv": fv,
        "extra": extra, "mesh_check": mesh_check, "tb": tb, "pick": X["pick"], "sc": sc, "cut_sc": cut_sc,
        "fin": {k: bf.finite_by_folder(v, X) for k, v in cut_sc.items()},
        "cuts": {(k, q): bf.graded_cut(v, X, tb, q) for k, v in cut_sc.items() for q in QS},
        "stored": np.array([np.nan if f["ef"] is None else f["ef"] for f in rows]),
    }


def main(app_dump, discard_dir, gate_dir, strict_dir, r1_dir, r2_dir):
    out_path = os.path.join(gate_dir, "gateread.out.md")
    if os.path.exists(out_path):
        raise SystemExit(f"{out_path} exists: the exploration is read once")
    start = time.time()
    reject_dir = os.path.join(os.path.dirname(os.path.abspath(discard_dir)), REJECT_EVAL)
    fresh_root = os.path.join(discard_dir, "dump")
    dirs = {1: r1_dir, 2: r2_dir, 3: discard_dir}
    roots = {k: os.path.normcase(os.path.abspath(p)) for k, p in (("strict-labels", strict_dir), ("round-1", r1_dir), ("round-2", r2_dir),
                                                                  ("discard-model (round 3)", discard_dir), ("reject-filter-eval", reject_dir), ("gate", gate_dir))}
    skip = [os.path.normcase(os.path.abspath(p)) for p in (fresh_root, os.path.join(gate_dir, "faces"), os.path.join(discard_dir, "sample"))]
    opened = {k: [] for k in roots}
    opened_all = []

    def audit(event, args):
        if event == "open" and isinstance(args[0], str):
            p = os.path.normcase(os.path.abspath(args[0]))
            opened_all.append(p)
            if any(p.startswith(r + os.sep) for r in skip):
                return
            for k, r in roots.items():
                if p.startswith(r + os.sep):
                    opened[k].append(os.path.relpath(p, r))

    sys.addaudithook(audit)
    tee = bf.Tee(sys.stdout)
    sys.stdout = tee

    def fail(tables):
        print("# `eye_focus` gate and fill: the exploration read\n")
        for head, rows in tables:
            gate.table(head, rows)
        sys.stdout = tee.stream
        raise SystemExit("a check failed; nothing written")

    D = {
        "dump": load_set("dump", app_dump, gate_dir, os.path.join(discard_dir, "scores_model.tsv")),
        "fresh": load_set("fresh", fresh_root, gate_dir, os.path.join(discard_dir, "scores_fresh.tsv")),
    }
    SET = {1: D["dump"], 2: D["dump"], 3: D["fresh"]}
    load_time = time.time() - start
    print(f"loaded: {load_time:.0f} s", file=sys.stderr, flush=True)

    # ---------------------------------------------------------------- label-free
    t0 = time.time()
    d, e = D["dump"]["X"], D["fresh"]["X"]
    stored_rej, _ = rf.load_scores(os.path.join(reject_dir, "scores_reject.tsv"), d)
    sd, _ = rf.load_scores(os.path.join(r2_dir, "scores_discard.tsv"), d)
    fresh_prior = D["fresh"]["prior"]
    tb_d, tb_e = D["dump"]["tb"], D["fresh"]["tb"]

    index_d = D["dump"]["index"]
    strict_seen = np.zeros(D["dump"]["n"], dtype=bool)
    for r in bf.read_tsv(os.path.join(strict_dir, "manifest.tsv")):
        strict_seen[index_d[(r["folder"], r["file"])]] = True
    man = {rd: bf.read_tsv(os.path.join(dirs[rd], "manifest.tsv")) for rd in ROUNDS}
    strata_rows = {rd: bf.read_tsv(os.path.join(dirs[rd], "strata.tsv")) for rd in ROUNDS}
    ms = {rd: fcr.sample_of(man[rd], SET[rd]["index"], SET[rd]["X"]["folder_of"], False) for rd in ROUNDS}
    seen = {1: strict_seen.copy(), 2: strict_seen.copy(), 3: np.zeros(D["fresh"]["n"], dtype=bool)}
    seen[2][ms[1]["idx"]] = True
    pop = {rd: ~seen[rd] for rd in ROUNDS}

    cg = {q: bf.graded_cut(stored_rej["G0"], d, tb_d, q)[0] for q in QS}
    cw = {q: bf.graded_cut(sd["G0 ⊕ ef"], d, tb_d, q)[0] for q in QS}
    cl = {q: bf.graded_cut(fresh_prior[WINNER3], e, tb_e, q)[0] for q in QS}
    ce = {q: D["fresh"]["cuts"][("ef", q)][0] for q in QS}
    band = {rd: np.full(SET[rd]["n"], "control", dtype=object) for rd in ROUNDS}
    band[1][cg[20]] = "b20"
    band[1][cg[10]] = "b10"
    band[1][cg[5]] = "b05"
    band[2][cg[20] & ~cw[20]] = "g0only"
    band[2][cw[20]] = "b20"
    band[2][cw[10]] = "b10"
    band[2][cw[5]] = "b05"
    band[3][ce[20] & ~cl[20]] = "efonly"
    band[3][cl[20]] = "b20"
    band[3][cl[10]] = "b10"
    band[3][cl[5]] = "b05"

    def pl_of(x, k, q):
        return 100 * (x["cuts"][(k, q)][0] & x["pick"]).sum() / x["pick"].sum()

    res_removed = {(rd, k, q): 100 * (SET[rd]["cuts"][(k, q)][0] & pop[rd]).sum() / pop[rd].sum() for rd in ROUNDS for k in ALL for q in QS}
    N33 = sum(x["n"] for x in D.values())
    P33 = sum(int(x["pick"].sum()) for x in D.values())
    pl33 = {(k, q): 100 * sum(int((x["cuts"][(k, q)][0] & x["pick"]).sum()) for x in D.values()) / P33 for k in ALL for q in QS}
    rem33 = {(k, q): 100 * sum(int(x["cuts"][(k, q)][0].sum()) for x in D.values()) / N33 for k in ALL for q in QS}
    lf = {(k, q): pl33[(k, q)] <= df.PL_BAR and rem33[(k, q)] >= df.REMOVED_BAR for k in ALL for q in QS}
    cands17 = ["ef-fill"] + GATES + FILLS

    checks_a = []
    for key, x in D.items():
        sg, n = x["sg"], x["n"]
        rows_seen, partial, mismatch = x["mesh_check"]
        gate_diff = sum(same(sg[gs.gname("ef-gate", s, t)], np.where(sg[s] > t, sg["window_ef"], sg["ef"])) for s, t in PAIRS)
        has = ~np.isnan(sg["ef"])
        fill_diff = sum(same(sg[f][has], sg[g][has]) + same(np.isnan(sg[f]).astype(float), np.isnan(sg["ef-fill"]).astype(float)) for f, g in zip(FILLS, GATES))
        face_diff = sum(same(x["fv"][k], sg[k]) for k in ("ef", "nose", "max", "window_ef"))
        ref_ef = x["stored"] if key == "dump" else x["prior"]["ef"]
        checks_a += [
            (f"{key}: frames (with a vector) / picks / without a vector", f"{N_SET[key][0]} / {N_SET[key][1]} / 0", f"{n} / {int(x['pick'].sum())} / {x['missing']}"),
            (f"{key}: `scores_gate.tsv` rows (both dumps) / this dump's frames found", f"46984 / {n}", f"{x['sg_rows']} / {n}"),
            (f"{key}: `scores_gate.tsv` `ef` vs " + ("the stored `eye_focus`" if key == "dump" else "`scores_fresh.tsv`'s `ef`") + " (> 1e-9, or missing on one side only)",
             "0", str(same(sg["ef"], ref_ef))),
            (f"{key}: `scores_gate.tsv` `ef-fill` vs `{'scores_model.tsv' if key == 'dump' else 'scores_fresh.tsv'}`'s", "0", str(same(sg["ef-fill"], x["prior"]["ef-fill"]))),
            (f"{key}: the 8 `ef-gate` columns vs their recomputation from `ef`, `s`, `window_ef` and the grid: frames differing, summed", "0", str(gate_diff)),
            (f"{key}: the 8 `ef-gate+fill` columns vs `ef-gate` where `ef` exists, and missing exactly where `ef-fill` is: frames differing, summed", "0", str(fill_diff)),
            (f"{key}: the judged face's `ef`, `nose`, `max`, `window_ef` in the per-face dump vs `scores_gate.tsv`: frames differing, summed", "0", str(face_diff)),
            (f"{key}: per-face dump rows with partial / misplaced mesh columns", "0 / 0", f"{partial} / {mismatch}"),
            (f"{key}: frames left out of a cut, over every score and `q`", "0", str(sum(int(c[1].sum()) for c in x["cuts"].values()))),
        ]
        for k, exp in KNOWN_PL[key].items():
            got = " / ".join(f"{pl_of(x, k, q):.2f}" if k != WINNER3 else f"{100 * (cl[q] & x['pick']).sum() / x['pick'].sum():.2f}" for q in QS)
            checks_a.append((f"{key}: {nm(k)} `PL` % of the dump's picks q5 / q10 / q20 (earlier readings)", " / ".join(exp), got))
    grid_got = {s: tuple(sorted({t for s2, t in PAIRS if s2 == s})) for s in FAMILIES}
    cols_ok = all(k in D["dump"]["sg"] for k in ["ef", "nose", "max", "window_ef", "ef-fill"] + GATES + FILLS)
    checks_a += [
        ("`scores_gate.tsv` holds `ef`, `nose`, `max`, `window_ef`, `ef-fill` and the 16 gated columns of the grid as fixed (efgate.md)", "yes", yn(cols_ok and grid_got == GRID)),
        ("33 folders: `ef` `PL` % q5 / q10 / q20 ; removed % (gatescores.out.md)", " / ".join(KNOWN_33[0]) + " ; " + " / ".join(KNOWN_33[1]),
         " / ".join(f"{pl33[('ef', q)]:.2f}" for q in QS) + " ; " + " / ".join(f"{rem33[('ef', q)]:.2f}" for q in QS)),
        ("33 folders: candidates passing the label-free conditions at q5 / at q10, of 17 (gatescores.out.md)", KNOWN_33[2],
         f"{sum(lf[(k, 5)] for k in cands17)} / {sum(lf[(k, 10)] for k in cands17)}"),
    ]
    for rd in ROUNDS:
        x, m = SET[rd], ms[rd]
        mv = np.array([float(r["eye_focus"]) for r in man[rd]])
        ev = x["sc"]["ef"][m["idx"]]
        sdiff, bdiff, pdiff = fcr.strata_check(x["X"], pop[rd], band[rd], strata_rows[rd], m)
        checks_a.append((f"Round {rd}: `ef` vs `manifest.tsv`'s `eye_focus` (> {MANIFEST_TOL[rd]:g}, or missing on one side only)", "0 of 600",
                         f"{same(ev, mv, MANIFEST_TOL[rd])} of {len(m['idx'])}"))
        if rd in EARLIER:
            checks_a += [
                (f"Round {rd}: earlier-labeled frames excluded / population", f"{EARLIER[rd]} / {POP[rd]}", f"{int(seen[rd].sum())} / {int(pop[rd].sum())}"),
                (f"Round {rd}: sampled frames in an earlier sample", "0", str(int(seen[rd][m["idx"]].sum()))),
            ]
        checks_a += [
            (f"Round {rd}: `manifest.tsv` rows / distinct frames", "600 / 600", f"{len(man[rd])} / {len(set(m['idx'].tolist()))}"),
            (f"Round {rd}: `strata.tsv` rows whose population, drawn count or `π` differ when recomputed", f"0 of {len(strata_rows[rd])}", f"{sdiff} of {len(strata_rows[rd])}"),
            (f"Round {rd}: `manifest.tsv` frames whose recomputed band / `π` differ", "0 / 0", f"{bdiff} / {pdiff}"),
            (f"Round {rd}: sum of `1/π` over the sampled frames", f"{POP[rd]}.0", f"{m['w'].sum():.1f}"),
            (f"Round {rd}: `ef` removed % of the round's population q5 / q10 / q20", " / ".join(KNOWN_REMOVED[rd]), " / ".join(f"{res_removed[(rd, 'ef', q)]:.2f}" for q in QS)),
        ]
    checks_a = [(c, x, v, "pass" if x == v else "FAIL") for c, x, v in checks_a]
    early_any = sorted({p for p in opened_all if os.path.basename(p) == "labels.tsv"})
    checks_a.append(("`labels.tsv` files opened before these checks", "none", ", ".join(f"`{p}`" for p in early_any) or "none", "pass" if not early_any else "FAIL"))
    head_a = "| Check (label-free, before any `labels.tsv` is opened) | Expected | This run | Result |"
    if not all(r[3] == "pass" for r in checks_a):
        fail([(head_a, checks_a)])

    # ---------------------------------------------------------------- labels
    lab = {rd: bf.read_tsv(os.path.join(dirs[rd], "labels.tsv")) for rd in ROUNDS}
    S = {}
    for rd in ROUNDS:
        x = SET[rd]
        s = fcr.sample_of(lab[rd], x["index"], x["X"]["folder_of"], True)
        s["labels"] = {r["label"] for r in lab[rd]}
        by_name = {r["name"]: r for r in man[rd]}
        s["man_diff"] = sum(r["name"] not in by_name or r["band"] != by_name[r["name"]]["band"] or r["inclusion_prob"] != by_name[r["name"]]["inclusion_prob"]
                            or (r["folder"], r["file"]) != (by_name[r["name"]]["folder"], by_name[r["name"]]["file"]) for r in lab[rd])
        s["cs"] = df.counts(s, x["X"]["folders"], STRATA[rd])
        s["cc"] = bf.cluster_counts(s, x["X"]["folders"])
        s["all"] = np.ones(len(s["idx"]), dtype=bool)
        S[rd] = s

    warnings.simplefilter("ignore", RuntimeWarning)
    auc, res = {}, {}

    def auc_of(rd, k, part):
        if (rd, k, part) not in auc:
            s = S[rd]
            auc[(rd, k, part)] = fcr.auc_draws(s, SET[rd]["fin"][k][s["idx"]], s[part], s["cs"], s["cc"])
        return auc[(rd, k, part)]

    def res_of(rd, k, q):
        if (rd, k, q) not in res:
            s = S[rd]
            res[(rd, k, q)] = fcr.cut_metrics(s, SET[rd]["cuts"][(k, q)][0][s["idx"]], s["all"], s["cs"], s["cc"])
        return res[(rd, k, q)]

    def da(rd, k, part="all", ref="ef"):
        return fcr.diff(auc_of(rd, k, part), auc_of(rd, ref, part))

    # Tuning on rounds 1 and 2.
    tune = {}
    for s_, t in PAIRS:
        g = gs.gname("ef-gate", s_, t)
        tune[(s_, t)] = sum(auc_of(rd, g, "all")["point"] - auc_of(rd, "ef", "all")["point"] for rd in (1, 2))
        print(f"tuning {g}: {time.time() - t0:.0f} s", file=sys.stderr, flush=True)
    best = max(PAIRS, key=lambda p: (tune[p], -FAMILIES.index(p[0]), -p[1]))
    s_star, t_star = best
    tune_ties = sum(tune[p] == tune[best] for p in PAIRS) - 1
    G, F = gs.gname("ef-gate", s_star, t_star), gs.gname("ef-gate+fill", s_star, t_star)
    CANDS = [G, "ef-fill", F]
    SCORES = REFS + CANDS

    for rd in ROUNDS:
        s, x = S[rd], SET[rd]
        idx = s["idx"]
        miss = np.isnan(x["sc"]["ef"][idx])
        gated = x["sg"][s_star][idx] > t_star
        s["miss"], s["gated"], s["comp"] = miss, gated, ~miss & ~gated
        for k in SCORES:
            for part in ("all", "miss", "gated", "comp"):
                auc_of(rd, k, part)
            for q in QS:
                res_of(rd, k, q)
        print(f"round {rd}: {time.time() - t0:.0f} s", file=sys.stderr, flush=True)

    checks_b, base = [], {}
    for rd in ROUNDS:
        s = S[rd]
        w, dis, cs, cc = s["w"], s["bad"], s["cs"], s["cc"]
        b = 100 * (w * dis).sum() / w.sum()
        bci, bcci = bf.quant(bf.ratio(cs, w * dis, w)), bf.quant(bf.ratio(cc, w * dis, w))
        base[rd] = (b, bci, bcci, 100 * dis.mean())
        kn = KNOWN[rd]

        def auc_str(a):
            return fcr.auc_s(a) + f" ({fcr.cauc_s(a)})" + f", unweighted {a['unw']:.3f}"

        checks_b += [
            (f"Round {rd}: `labels.tsv` labels within {{keep, discard}}", "yes", yn(s["labels"] <= {"keep", "discard"})),
            (f"Round {rd}: labels / keep / discard", f"600 / {LABELS[rd][0]} / {LABELS[rd][1]}", f"{len(s['idx'])} / {int((~dis).sum())} / {int(dis.sum())}"),
            (f"Round {rd}: `labels.tsv` rows whose name, file, band or `π` differ from `manifest.tsv`", "0", str(s["man_diff"])),
            (f"Round {rd}: sum of `1/π` over the labeled frames", f"{POP[rd]}.0", f"{w.sum():.1f}"),
            (f"Round {rd}: discardable base rate % [stratified 95%] (cluster)", kn["base"], f"{b:.2f} [{bci[0]:.2f}, {bci[1]:.2f}] ([{bcci[0]:.2f}, {bcci[1]:.2f}])"),
            (f"Round {rd}: `ef` AUC [stratified 95%] (cluster)", kn["ef"], auc_str(auc_of(rd, "ef", "all"))),
            (f"Round {rd}: `ef` `RR` % q5 / q10 / q20 [stratified 95%]", kn["rr"], " / ".join(fcr.rr_s(res_of(rd, "ef", q)) for q in QS)),
            (f"Round {rd}: `ef-fill` AUC [stratified 95%] (cluster)", kn["fill"], auc_str(auc_of(rd, "ef-fill", "all"))),
            (f"Round {rd}: `ef-fill` - `ef` AUC [stratified 95%] (cluster)", kn["fill_diff"], fcr.eff(*da(rd, "ef-fill"), 3)),
        ]
        if "miss_fill_diff" in kn:
            checks_b.append((f"Round {rd}: no-crop-face subset, `ef-fill` - `ef` AUC [stratified 95%] (cluster)", kn["miss_fill_diff"], fcr.eff(*da(rd, "ef-fill", "miss"), 3)))
    checks_b = [(c, x, v, "pass" if x == v else "FAIL") for c, x, v in checks_b]
    allowed = {"strict-labels": {"manifest.tsv"}, "round-1": {"labels.tsv", "manifest.tsv", "strata.tsv"},
               "round-2": {"labels.tsv", "manifest.tsv", "strata.tsv", "scores_discard.tsv"},
               "discard-model (round 3)": {"labels.tsv", "manifest.tsv", "strata.tsv", "scores_model.tsv", "scores_fresh.tsv"},
               "reject-filter-eval": {"scores_reject.tsv"}, "gate": {"scores_gate.tsv"}}
    for k, ps in opened.items():
        got = sorted(set(ps))
        checks_b.append((f"Files opened under the {k} directory (outside `dump\\`, `faces\\` and `sample\\`)", "within " + ", ".join(f"`{p}`" for p in sorted(allowed[k])),
                         ", ".join(f"`{p}`" for p in got) or "none", "pass" if set(got) <= allowed[k] else "FAIL"))
    label_roots = [roots["round-1"], roots["round-2"], roots["discard-model (round 3)"]]
    stray = sorted({p for p in opened_all if (os.path.basename(p) == "labels.tsv" and os.path.dirname(p) not in label_roots) or os.path.splitext(p)[1] in (".xmp", ".dop")})
    checks_b.append(("`labels.tsv` opened outside the three rounds' folders, or any sidecar opened", "none", ", ".join(f"`{p}`" for p in stray) or "none", "pass" if not stray else "FAIL"))
    head_b = "| Check (labels) | Expected | This run | Result |"
    if not all(r[3] == "pass" for r in checks_b):
        if not DRY_RUN:
            fail([(head_a, checks_a), (head_b, checks_b)])
        print("dry run: label checks failed, continuing", file=sys.stderr, flush=True)

    # Selection on round 3 and the stop rule.
    elig = {k: (lf[(k, 5)], lf[(k, 10)]) for k in CANDS}
    pool = [k for k in CANDS if any(elig[k])]

    def r12(k):
        return auc_of(1, k, "all")["point"] + auc_of(2, k, "all")["point"]

    winner = max(pool, key=lambda k: (auc_of(3, k, "all")["point"], r12(k), -CANDS.index(k))) if pool else None
    tie3 = winner is not None and sum(auc_of(3, k, "all")["point"] == auc_of(3, winner, "all")["point"] for k in pool) > 1
    stop_pts = {rd: da(rd, winner)[0] for rd in ROUNDS} if winner else {}
    fresh = winner is not None and all(v > 0 for v in stop_pts.values())
    compute_time = time.time() - t0

    # ---------------------------------------------------------------- print
    print("# `eye_focus` gate and fill: the exploration read\n")
    print(
        "The candidates of efgate.md's \"Pre-registration of the exploration\" (Step 3's `scores_gate.tsv`) and the references `ef` (today's "
        "`eye_focus`, missing last) and random (seed 0), read against the three rounds of blind \"OK to discard?\" labels: rounds 1 and 2 tune "
        "`(s*, τ*)`, round 3 selects the winner, and the stop rule reads all three. Each round keeps its own population, `π` and strata (rounds 1 "
        "and 2 as facechoiceread.py builds them, round 3 as freshread.py does); one stratified count matrix (1,000 draws, seed 0) and one "
        "folder-cluster matrix (1,000 draws, seed 0) per round are shared by every score, so the differences against `ef` are paired. A missing "
        "score sorts last: `-inf` in the per-folder cut, the folder's lowest finite value minus 1 in the AUC. Intervals: stratified 95% in brackets, "
        "folder-cluster 95% in parentheses.\n"
    )
    print("## Checks\n")
    gate.table(head_a, checks_a)
    gate.table(head_b, checks_b)

    print("## Data\n")
    drow = []
    for key, x in D.items():
        drow.append((f"{key}: folders / frames / picks", f"{len(x['X']['folders'])} / {x['n']} / {int(x['pick'].sum())}"))
        drow.append((f"{key}: per-face dump rows of files outside the frames", str(x["extra"])))
    for rd in ROUNDS:
        s = S[rd]
        for part, title in (("miss", "no crop face"), ("gated", f"gated (`{s_star}` > {t_star:.3f})"), ("comp", "the complement")):
            mk = s[part]
            drow.append((f"Round {rd}: labels {title} / keep / discard (weighted share of the population)",
                         f"{int(mk.sum())} / {int((mk & ~s['bad']).sum())} / {int((mk & s['bad']).sum())} ({pct((s['w'] * mk).sum(), s['w'].sum())}%)"))
    gate.table("| Count | Value |", drow)
    gate.table("| Round | Discardable base rate, weighted % | Stratified 95% | Folder-cluster 95% | Unweighted % |",
               [[TITLE[rd], f"{base[rd][0]:.2f}", "[{:.2f}, {:.2f}]".format(*base[rd][1]), "[{:.2f}, {:.2f}]".format(*base[rd][2]), f"{base[rd][3]:.2f}"] for rd in ROUNDS])

    print("## 1. The tuning (rounds 1 and 2)\n")
    print(
        "Per (`s`, `τ`) of the grid as fixed, `ef-gate(s, τ)`'s weighted AUC and its paired difference against `ef` on round 1 and round 2, each "
        "on its own population and `π`, and the sum of the two AUC - `ef` points. The rule: the largest sum; ties to `nose`, then to the smaller "
        "`τ`. \"Gated labels\": the labeled frames whose judged face has `s` > `τ`.\n"
    )
    trow = []
    for s_, t in PAIRS:
        g = gs.gname("ef-gate", s_, t)
        cells = [f"`{s_}`", f"{t:.3f}"]
        for rd in (1, 2):
            gm = SET[rd]["sg"][s_][S[rd]["idx"]] > t
            cells += [int(gm.sum()), fcr.auc_s(auc_of(rd, g, "all")), fcr.eff(*da(rd, g), 3)]
        cells.append(f"{tune[(s_, t)]:+.6f}" + (" **(s\\*, τ\\*)**" if (s_, t) == best else ""))
        trow.append(cells)
    gate.table("| `s` | `τ` | Round 1 gated labels | Round 1 AUC | Round 1 AUC - `ef` | Round 2 gated labels | Round 2 AUC | Round 2 AUC - `ef` | Sum of AUC - `ef` |", trow)
    print(f"**(s\\*, τ\\*)** = (`{s_star}`, {t_star:.3f}), sum {tune[best]:+.6f}" + (f" ({tune_ties} other pair(s) tie on the sum, broken by the rule)" if tune_ties else "")
          + f". The candidates of the selection: `{G}`, `ef-fill`, `{F}`.\n")

    print("## 2. The AUCs, all labeled frames\n")
    for rd in (3, 2, 1):
        print(f"### {TITLE[rd]}" + (" (selection)" if rd == 3 else "") + "\n")
        gate.table("| Score | AUC [stratified 95%] | Cluster | Unweighted | AUC - `ef` |",
                   [[nm(k), fcr.auc_s(auc_of(rd, k, "all")), fcr.cauc_s(auc_of(rd, k, "all")), f"{auc_of(rd, k, 'all')['unw']:.3f}",
                     "" if k == "ef" else fcr.eff(*da(rd, k), 3)] for k in SCORES])
    print(f"Round 3, `{G}` - `ef-fill`: {fcr.eff(*da(3, G, ref='ef-fill'), 3)}; `{F}` - `ef-fill`: {fcr.eff(*da(3, F, ref='ef-fill'), 3)}.\n")

    print("## 3. The cuts\n")
    print(
        "Per folder, the bottom `floor(q * n / 100)` by score, ties by the seed-0 draw (`bf.graded_cut`). Weighted (`1/π`) with the stratified "
        "95% interval and the unweighted value in parentheses; the cluster interval of `RR` beside. `PL`: the share of the round's dump's picks "
        "inside the cut (rounds 1 and 2: the 27 folders' 4,419; round 3: the six folders' 1,795), label-free; removed: the share of the round's "
        "population. `RR` - `ef`: paired, stratified with the cluster interval in parentheses.\n"
    )
    for rd in (3, 2, 1):
        print(f"### {TITLE[rd]}\n")
        crow = []
        for k in SCORES:
            for q in QS:
                r = res_of(rd, k, q)
                crow.append([nm(k), f"q{q}", f"{r['n_rem']} / {r['n_dis']}", f"{bf.iv(r['point']['DP'], *r['ci']['DP'])} ({bf.f1(r['unw']['DP'])})",
                             f"{fcr.rr_s(r)} ({bf.f1(r['unw']['RR'])})", "[{:.1f}, {:.1f}]".format(*r["cci"]["RR"]), f"{pl_of(SET[rd], k, q):.2f}",
                             f"{res_removed[(rd, k, q)]:.2f}", "" if k == "ef" else fcr.eff(*fcr.diff(r, res_of(rd, "ef", q), "RR"))])
        gate.table("| Score | Cut | Sampled removed / discard | `DP` % | `RR` % | `RR` cluster | `PL` % | Removed % | `RR` - `ef`, points |", crow)

    print("## 4. The subsets and the label × gated cross-tab\n")
    print(
        f"A reading beside; it decides nothing. The labeled frames of each round split three ways: **no crop face** (no `ef`), **gated** (the "
        f"judged face's `{s_star}` > {t_star:.3f}) and **the complement** (`ef` present, not gated). The AUC is read on each subset (the score is "
        "still the full one; only the labeled frames read are restricted). On the no-crop-face subset `ef` and `ef-gate` are the folder's lowest "
        "finite value minus 1 on every frame, so they order folders only; on the complement every gate equals `ef`. `ef-gate+fill` also gates "
        "filled faces, which sit in the no-crop-face subset.\n"
    )
    for rd in (3, 2, 1):
        s = S[rd]
        w, dis, cs = s["w"], s["bad"], s["cs"]
        print(f"### {TITLE[rd]}\n")
        for part, title in (("miss", "No crop face"), ("gated", "Gated"), ("comp", "The complement")):
            print(f"{title} ({int(s[part].sum())} labels):\n")
            gate.table("| Score | AUC [stratified 95%] | Cluster | Unweighted | AUC - `ef` |",
                       [[nm(k), fcr.auc_s(auc_of(rd, k, part)), fcr.cauc_s(auc_of(rd, k, part)), f"{auc_of(rd, k, part)['unw']:.3f}",
                         "" if k == "ef" else fcr.eff(*da(rd, k, part), 3)] for k in SCORES])
        idx = s["idx"]
        ev, wv = SET[rd]["sc"]["ef"][idx], SET[rd]["sg"]["window_ef"][idx]
        yaw = np.abs(SET[rd]["fv"]["yaw"][idx])

        def grow(label, mk):
            ci = bf.quant(bf.ratio(cs, w * dis * mk, w * mk)) if mk.any() else (np.nan, np.nan)
            return [label, int(mk.sum()), int((mk & ~dis).sum()), int((mk & dis).sum()), pct((w * mk).sum(), w.sum()),
                    f"{pct((w * dis * mk).sum(), (w * mk).sum())} [{ci[0]:.1f}, {ci[1]:.1f}]" if mk.any() else "-", pct((mk & dis).sum(), mk.sum())]

        xrow = [grow("All labels", s["all"]), grow("No crop face", s["miss"]), grow("Gated", s["gated"]), grow("The complement", s["comp"])]
        for lbl, mk in (("lower than", wv < ev), ("equal to", wv == ev), ("higher than", wv > ev)):
            xrow.append(grow(f"Gated, `window_ef` {lbl} `ef`", s["gated"] & mk))
        for lbl, lo, hi in YAW_BANDS:
            band_m = (yaw >= lo) & (yaw < hi)
            xrow.append(grow(f"Gated, \\|yaw\\| {lbl}", s["gated"] & band_m))
        xrow.append(grow("Gated, no pose", s["gated"] & np.isnan(yaw)))
        for lbl, lo, hi in YAW_BANDS:
            band_m = (yaw >= lo) & (yaw < hi)
            xrow.append(grow(f"The complement, \\|yaw\\| {lbl}", s["comp"] & band_m))
        xrow.append(grow("The complement, no pose", s["comp"] & np.isnan(yaw)))
        gate.table("| Labels | n | Keep | Discard | Weighted share of the population % | Weighted discard % [stratified 95%] | Unweighted discard % |", xrow)

    print("## 5. The selection rule and the stop rule applied\n")
    print(
        f"Selection: among `{G}`, `ef-fill` and `{F}`, those whose q5 or q10 cut passes the label-free conditions on all 33 folders (`PL` <= "
        f"{df.PL_BAR:.0f}% of the {P33} picks, removed >= {df.REMOVED_BAR}% of the {N33} frames) are eligible; the winner is the eligible one with "
        "the highest weighted AUC on round 3, ties by the sum of rounds 1 and 2's AUCs. Stop rule: the winner goes to the fresh round only if its "
        "AUC - `ef` point is above zero on round 3 and on each of rounds 1 and 2; otherwise Steps 5-6 are skipped.\n"
    )
    vrow = []
    for k in CANDS:
        vrow.append([nm(k), f"{pl33[(k, 5)]:.2f} / {rem33[(k, 5)]:.2f}: {yn(elig[k][0])}", f"{pl33[(k, 10)]:.2f} / {rem33[(k, 10)]:.2f}: {yn(elig[k][1])}",
                     yn(any(elig[k])), f"{auc_of(3, k, 'all')['point']:.4f}", f"{r12(k):.4f}"] + [f"{da(rd, k)[0]:+.6f}" for rd in ROUNDS])
    gate.table("| Candidate | q5 `PL` % / removed %: passes | q10 `PL` % / removed %: passes | Eligible | Round 3 AUC | Rounds 1 + 2 AUC | Round 1 AUC - `ef` | Round 2 AUC - `ef` | Round 3 AUC - `ef` |", vrow)
    if winner is None:
        print("**Winner**: none (no candidate passes the label-free conditions at q5 or q10).\n")
        print("**Stop rule**: no winner, so no fresh round; Steps 5-6 are skipped and the Decision is \"nothing, keep `eye_focus`\".\n")
    else:
        print(f"**Winner**: {nm(winner)}, round 3 AUC {auc_of(3, winner, 'all')['point']:.4f}" + (" (a tie on round 3, broken by rounds 1 + 2)" if tie3 else "") + ".\n")
        pts = ", ".join(f"round {rd} {stop_pts[rd]:+.6f}" for rd in ROUNDS)
        print(f"**Stop rule**: the winner's AUC - `ef` points are {pts}; " + ("all above zero, so the winner goes to the fresh round (Steps 5-6).\n" if fresh else
              "not above zero on all three, so no fresh round; Steps 5-6 are skipped and the Decision is \"nothing, keep `eye_focus`\".\n"))

    print("## Wall time\n")
    gate.table("| Stage | Seconds |", [("Load (both dumps, `scores_gate.tsv`, prior scores, per-face dump)", f"{load_time:.0f}"),
                                       ("Cuts, checks, metrics and bootstraps", f"{compute_time:.0f}")])
    print(f"Total {time.time() - start:.0f} s.\n")
    sys.stdout = tee.stream
    with open(out_path, "w", encoding="utf-8", newline="") as out:
        out.write("".join(tee.parts))
    print("gateread: done", file=sys.stderr, flush=True)


if __name__ == "__main__":
    if len(sys.argv) != 7:
        raise SystemExit(__doc__)
    sys.stdout.reconfigure(encoding="utf-8")
    main(*sys.argv[1:7])
