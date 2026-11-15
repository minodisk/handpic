# The Low threshold `LOW_BELOW`

## Rule

`LOW_BELOW` is the largest threshold on a 0.01 grid under which at most 2%
of the held-out 3-star frames fall (the user's choice at planning). A frame
whose `full_eye_sharp` is under it is Low. The four held-out folders choose
the value; the fresh folder (`2026-06-05-focus-sample`) only reports. The
data and the labels are described in [`experiment.md`](experiment.md).

## Result

**`LOW_BELOW = 0.110`**

With 154 held-out 3-star frames, 2% allows 3 of them under the threshold.
Their four lowest values are 0.0698, 0.0886, 0.1034 and 0.1171, so any
threshold in (0.1034, 0.1171] meets the rule with 3/154 (1.9%). The 0.01
grid gives 0.110. A finer grid would allow 0.115, but that already puts 2 of
the 27 fresh 3-star frames (0.1135, 0.1138) under it, against none at 0.110.

At 0.110:

| Frames under `LOW_BELOW` | Held-out | Fresh |
| --- | --- | --- |
| 3 stars (share) | 3/154 (1.9%) | 0/27 (0.0%) |
| 2 stars (share) | 5/39 (12.8%) | 1/18 (5.6%) |
| 1 star (recall) | 13/88 (14.8%) | 5/13 (38.5%) |

For comparison, today's Low (the cue's `NotCandidate`, from the preview's
`eye_focus`):

| Today's Low | Held-out | Fresh |
| --- | --- | --- |
| 3 stars | 1/154 | 0/27 |
| 2 stars | 2/39 | 2/18 |
| 1 star | 3/88 | 6/13 |

So the new Low catches more of the held-out 1-star frames (13 against 3) for
2 more 3-star frames, and one fewer on the fresh folder (5 against 6). It
stays "clearly soft", not "most misses". The planning-time run of the same
join (0.110: held-out 3 / 5 / 13, fresh 3 stars 0/27 and 1 star 5/13) agrees
with this one.

The denominators are the frames with a judged face. Every held-out rated
frame has one; 6 fresh rated frames (3 stars 1, 2 stars 1, 1 star 4) do not
and stay Neutral under any threshold.

## The table `low_threshold.py` printed

`python low_threshold.py`, reading [`frames.tsv`](frames.tsv):

```text
held-out: rated 3/2/1 stars 154, 39, 88; without a judged face (left out, Neutral) 0, 0, 0
fresh: rated 3/2/1 stars 28, 19, 17; without a judged face (left out, Neutral) 1, 1, 4

Frames with full_eye_sharp under the threshold (count / frames with a judged face)

| threshold | held-out 3 stars | held-out 2 stars | held-out 1 star | fresh 3 stars | fresh 2 stars | fresh 1 star |
|---|---|---|---|---|---|---|
| 0.050 | 0/154 | 0/39 | 0/88 | 0/27 | 0/18 | 0/13 |
| 0.060 | 0/154 | 0/39 | 0/88 | 0/27 | 0/18 | 1/13 |
| 0.070 | 1/154 | 0/39 | 3/88 | 0/27 | 0/18 | 1/13 |
| 0.080 | 1/154 | 1/39 | 5/88 | 0/27 | 1/18 | 1/13 |
| 0.090 | 2/154 | 2/39 | 7/88 | 0/27 | 1/18 | 3/13 |
| 0.100 | 2/154 | 3/39 | 11/88 | 0/27 | 1/18 | 5/13 |
| 0.110 | 3/154 | 5/39 | 13/88 | 0/27 | 1/18 | 5/13 |
| 0.120 | 5/154 | 5/39 | 19/88 | 2/27 | 2/18 | 5/13 |
| 0.130 | 8/154 | 8/39 | 28/88 | 3/27 | 4/18 | 6/13 |
| 0.140 | 10/154 | 11/39 | 35/88 | 4/27 | 4/18 | 8/13 |
| 0.150 | 19/154 | 13/39 | 42/88 | 6/27 | 8/18 | 8/13 |
| 0.160 | 28/154 | 14/39 | 46/88 | 7/27 | 10/18 | 9/13 |
| 0.162 | 29/154 | 14/39 | 47/88 | 7/27 | 10/18 | 9/13 |
| 0.170 | 32/154 | 16/39 | 53/88 | 7/27 | 11/18 | 10/13 |

LOW_BELOW = 0.110 (at most 2% of 154 held-out 3-star frames under it)
held-out: 3-star share 1.9%, 2-star share 12.8%, 1-star recall 14.8%
fresh: 3-star share 0.0%, 2-star share 5.6%, 1-star recall 38.5%

Today's Low (state NotCandidate), for comparison
held-out: 3/2/1 stars 1/154 | 2/39 | 3/88
fresh: 3/2/1 stars 0/27 | 2/18 | 6/13
```

The 0.162 row is `SOFT_BELOW`, the Good cut: 29/154 held-out 3-star frames
are under it and so not Good.
