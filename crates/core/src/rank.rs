//! The personal best-shot ranking: a pairwise logistic model over each
//! frame's image embedding (projected to `PCA_DIMS` whitened dimensions), the
//! second pass's technical values and its position in the burst, which ranks
//! the frames of one burst against each other. It never changes a judgment.
//!
//! The prior `crates/core/ranking/siglip2.json` is the image-embeddings plan's
//! Step 2 model refit on the app's stored values of 27 culled folders by the
//! plan's offline `rank_prior.py`: the PCA basis, the imputation means, the
//! standardization and the weights. `fit` refits the weights on the user's
//! own judgments in the prior's coordinates, shrunk toward the prior's
//! weights; how far a few folders of viewed frames move the model is not
//! measured. On top of those shared weights, `fit` learns one offset per
//! top-level genre (`genre::top_level`), pulled toward zero by a fixed
//! penalty, and `rank` ranks a burst of genre `g` (`genre::of_burst`) by the
//! shared weights plus `g`'s offset. The prior carries no offsets (all zero),
//! so moving `RANK_VERSION` to 2 for them only edited its `rankVersion` by
//! hand (and `rank_prior.py`'s constant); `rank_prior.py` was not re-run.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use chrono::NaiveDateTime;
use serde::Deserialize;

use crate::candidate::{candidate, FocusCandidate};
use crate::eyes::closed_probability;
use crate::genre;
use crate::pose::Pose;
use crate::Flag;

/// The version of the columns below, their order and what each reads. The
/// prior and a stored fit carry it; bump it (and re-run `rank_prior.py`) on
/// any change to them.
pub const RANK_VERSION: i64 = 2;

/// The largest gap between two frames of one burst, `BURST_GAP_MS` in
/// `burst.ts`.
pub const BURST_GAP_MS: i64 = 1000;

const TECH: usize = 10;
const POSITION: usize = 2;
pub const PCA_DIMS: usize = 32;
/// The model's columns: the technical values, the position, the PCA
/// dimensions.
pub const COLUMNS: usize = TECH + POSITION + PCA_DIMS;

/// An explicit reject counts this many times a frame merely not chosen in the
/// pairs of its burst.
pub const REJECT_WEIGHT: f64 = 2.0;

/// How much harder a genre's offset is pulled toward zero than the shared
/// weights toward the prior's: `OFFSET_SHRINK * tau / 2 * ||offset||^2` per
/// genre, `tau = 1 / (2 C)`. Chosen by the genre-ranking-offsets plan's
/// Step 1 (`docs/plans/20261014-genre-ranking-offsets/offsets.md`): the
/// highest pooled held-out AUC among the values that do no harm.
pub const OFFSET_SHRINK: f64 = 30.0;

/// The good-photo cuts of `photoTier` in `focus.ts`.
const GOOD_EYE_FOCUS: f64 = 0.9;
const GOOD_EYE_EAR: f64 = 0.25;
const GOOD_MAX_YAW: f64 = 35.0;
const GOOD_MAX_PITCH: f64 = 45.0;
const MAX_EYE_OFFSET: f64 = 0.1;
const MIN_EDGE_GAP: f64 = 0.02;

/// The reasons a score is told by, each a group of columns, with the columns
/// it sums over.
const REASONS: [(&str, std::ops::Range<usize>); 7] = [
    ("Look", TECH + POSITION..COLUMNS),
    ("Burst position", TECH..TECH + POSITION),
    ("Sharpness", 0..1),
    ("AF eye focus", 1..2),
    ("Eyes open", 2..3),
    ("Head pose", 3..6),
    ("Good photo", 6..7),
];

/// The missing-value indicators, which join the reason of the value they
/// stand for: no `eye_focus`, no eyes judgment, no pose.
const MISSING: [(usize, usize); 3] = [(7, 1), (8, 2), (9, 3)];

const FILE: &str = include_str!("../ranking/siglip2.json");

/// `model`, `embed_version` and `rank_version` are checked by a unit test, not
/// at run time.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Prior {
    /// The `embed::ModelSpec` name the basis was fit on.
    #[cfg_attr(not(test), expect(dead_code))]
    model: String,
    #[cfg_attr(not(test), expect(dead_code))]
    embed_version: i64,
    #[cfg_attr(not(test), expect(dead_code))]
    rank_version: i64,
    /// The training mean a missing technical value is set to.
    tech_mean: Vec<f64>,
    pca_mean: Vec<f32>,
    /// `PCA_DIMS` rows, each a component divided by the square root of its
    /// explained variance (whitened).
    pca_basis: Vec<Vec<f32>>,
    /// The standardization of every column.
    mean: Vec<f64>,
    scale: Vec<f64>,
    weights: Vec<f64>,
    /// scikit-learn's `C` the weights were fit at.
    c: f64,
}

fn prior() -> &'static Prior {
    static PRIOR: OnceLock<Prior> = OnceLock::new();
    PRIOR.get_or_init(|| {
        serde_json::from_str(FILE).unwrap_or_else(|e| panic!("ranking/siglip2.json: {e}"))
    })
}

/// The weights a burst is ranked by: `shared` plus the offset of the burst's
/// genre, a label absent from `offsets` (and `genre::UNKNOWN`) adding zero.
#[derive(Debug, Clone, PartialEq)]
pub struct Weights {
    pub shared: Vec<f64>,
    pub offsets: BTreeMap<String, Vec<f64>>,
}

impl Weights {
    fn of(&self, genre: &str) -> Vec<f64> {
        match self.offsets.get(genre).filter(|_| genre != genre::UNKNOWN) {
            Some(offset) => self.shared.iter().zip(offset).map(|(s, o)| s + o).collect(),
            None => self.shared.clone(),
        }
    }
}

/// The prior's weights with no offsets, what the ranking uses before any fit.
pub fn prior_weights() -> Weights {
    Weights {
        shared: prior().weights.clone(),
        offsets: BTreeMap::new(),
    }
}

/// What the ranking reads of one file: its name and capture time for the
/// bursts, the values the second pass stored for its judged face (all `None`
/// when that pass has not run at the current version) and its embedding.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub name: String,
    pub capture_time: Option<String>,
    pub subsec: Option<String>,
    pub sharpness: Option<f64>,
    pub eye_focus: Option<f64>,
    pub eyes_ear: Option<f64>,
    pub pose: Option<Pose>,
    pub eye_offset: Option<f64>,
    pub edge_gap: Option<f64>,
    /// `scan::Analysis::full_eye_sharp`.
    pub full_eye_sharp: Option<f64>,
    /// An `embed` vector at `EMBED_VERSION`; a frame without one gets no
    /// score and takes no part in a fit.
    pub vec: Option<Vec<f32>>,
}

fn capture_ms(frame: &Frame) -> Option<i64> {
    let time =
        NaiveDateTime::parse_from_str(frame.capture_time.as_deref()?, "%Y:%m:%d %H:%M:%S").ok()?;
    let subsec: String = frame
        .subsec
        .as_deref()
        .unwrap_or("")
        .chars()
        .take(3)
        .collect();
    let subsec = format!("{subsec:0<3}");
    let ms = if subsec.bytes().all(|b| b.is_ascii_digit()) {
        subsec.parse::<i64>().unwrap_or(0)
    } else {
        0
    };
    Some(time.and_utc().timestamp_millis() + ms)
}

/// The capture order of `orderFiles("capture", ...)` in `sort.ts`.
fn capture_order(a: &Frame, b: &Frame) -> Ordering {
    let by_time = match (&a.capture_time, &b.capture_time) {
        (Some(ta), Some(tb)) => ta.cmp(tb).then_with(|| {
            let (sa, sb) = (
                a.subsec.as_deref().unwrap_or(""),
                b.subsec.as_deref().unwrap_or(""),
            );
            let width = sa.len().max(sb.len());
            format!("{sa:0<width$}").cmp(&format!("{sb:0<width$}"))
        }),
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (None, None) => Ordering::Equal,
    };
    by_time.then_with(|| a.name.cmp(&b.name))
}

/// The bursts of `frames`, as `groupBursts` in `burst.ts` forms them: in
/// capture order, a frame within `BURST_GAP_MS` of the previous one joins its
/// burst, and one without a usable capture time is a burst of its own. Each
/// burst lists indices into `frames`.
pub fn bursts(frames: &[Frame]) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = (0..frames.len()).collect();
    order.sort_by(|&a, &b| capture_order(&frames[a], &frames[b]));
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut previous: Option<i64> = None;
    for i in order {
        let ms = capture_ms(&frames[i]);
        match (ms, previous, groups.last_mut()) {
            (Some(ms), Some(previous), Some(group)) if ms - previous <= BURST_GAP_MS => {
                group.push(i);
            }
            _ => groups.push(vec![i]),
        }
        previous = ms;
    }
    groups
}

fn logit(p: f64) -> f64 {
    let p = p.clamp(1e-3, 1.0 - 1e-3);
    (p / (1.0 - p)).ln()
}

/// The good-photo rule of `photoTier` in `focus.ts`, without its full-size
/// eye cut (`full_eye_sharp`): the ranking's features deliberately stay on the
/// preview's values.
fn good_photo(f: &Frame) -> bool {
    candidate(f.eye_focus) == FocusCandidate::Candidate
        && f.eye_focus.is_some_and(|p| p >= GOOD_EYE_FOCUS)
        && f.eyes_ear.is_some_and(|e| e >= GOOD_EYE_EAR)
        && f.pose
            .is_some_and(|p| p.yaw.abs() <= GOOD_MAX_YAW && p.pitch.abs() <= GOOD_MAX_PITCH)
        && f.eye_offset.is_some_and(|o| o <= MAX_EYE_OFFSET)
        && f.edge_gap.is_some_and(|g| g >= MIN_EDGE_GAP)
}

/// The technical columns of one frame of a burst whose sharpest frame scored
/// `top`, a missing value as NaN.
fn tech(f: &Frame, top: Option<f64>) -> [f64; TECH] {
    let nan = f64::NAN;
    let open = f.eyes_ear.map(|e| 1.0 - closed_probability(e));
    [
        match (f.sharpness, top) {
            (Some(s), Some(top)) if top != 0.0 => (s / top).max(1e-3).ln(),
            _ => nan,
        },
        f.eye_focus.map_or(nan, logit),
        open.unwrap_or(nan),
        f.pose.map_or(nan, |p| p.yaw.abs().min(90.0)),
        f.pose.map_or(nan, |p| p.pitch.abs().min(60.0)),
        f.pose.map_or(nan, |p| p.roll.abs().min(60.0)),
        if good_photo(f) { 1.0 } else { 0.0 },
        if f.eye_focus.is_none() { 1.0 } else { 0.0 },
        if open.is_none() { 1.0 } else { 0.0 },
        if f.pose.is_none() { 1.0 } else { 0.0 },
    ]
}

/// One frame's standardized columns.
pub type Row = [f64; COLUMNS];

/// The standardized columns of every frame with an embedding, burst by burst;
/// `None` for a frame without one.
pub fn rows(frames: &[Frame], bursts: &[Vec<usize>]) -> Vec<Option<Row>> {
    let prior = prior();
    let mut out = vec![None; frames.len()];
    for burst in bursts {
        let top = burst
            .iter()
            .filter_map(|&i| frames[i].sharpness)
            .reduce(f64::max);
        for (position, &i) in burst.iter().enumerate() {
            let f = &frames[i];
            let Some(vec) = &f.vec else {
                continue;
            };
            let mut row = [0.0; COLUMNS];
            for (j, v) in tech(f, top).into_iter().enumerate() {
                row[j] = if v.is_nan() { prior.tech_mean[j] } else { v };
            }
            row[TECH] = if burst.len() > 1 {
                position as f64 / (burst.len() - 1) as f64
            } else {
                0.0
            };
            row[TECH + 1] = if position == 0 { 1.0 } else { 0.0 };
            for (k, basis) in prior.pca_basis.iter().enumerate() {
                row[TECH + POSITION + k] = basis
                    .iter()
                    .zip(vec)
                    .zip(&prior.pca_mean)
                    .map(|((b, v), m)| f64::from(*b) * f64::from(v - m))
                    .sum();
            }
            for (j, x) in row.iter_mut().enumerate() {
                *x = (*x - prior.mean[j]) / prior.scale[j];
            }
            out[i] = Some(row);
        }
    }
    out
}

/// The genre of `burst` of `frames`, from the embeddings of its frames.
fn genre_of(frames: &[Frame], burst: &[usize]) -> &'static str {
    genre::of_burst(burst.iter().filter_map(|&i| frames[i].vec.as_deref()))
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

/// One frame's place in its burst.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Ranked {
    /// The frame's score above its burst's mean, in logits of the pairwise
    /// model: how much more likely to be the pick than an average frame of
    /// the burst.
    pub score: f64,
    /// 1-based, among the burst's frames with a score.
    pub place: usize,
    /// The burst's frames with a score.
    pub of: usize,
    /// What moved the score off the burst's mean, `(reason, logits)`,
    /// largest first by size; the logits sum to `score`.
    pub reasons: Vec<(&'static str, f64)>,
    /// The genre the burst was ranked as, `None` for `genre::UNKNOWN`.
    pub genre: Option<&'static str>,
}

/// The rank of every frame with an embedding within its burst under
/// `weights` and the offset of the burst's genre; `None` for a frame without
/// one.
pub fn rank(frames: &[Frame], weights: &Weights) -> Vec<Option<Ranked>> {
    let bursts = bursts(frames);
    let rows = rows(frames, &bursts);
    let mut out = vec![None; frames.len()];
    for burst in &bursts {
        let scored: Vec<(usize, &Row)> = burst
            .iter()
            .filter_map(|&i| rows[i].as_ref().map(|r| (i, r)))
            .collect();
        if scored.is_empty() {
            continue;
        }
        let label = genre_of(frames, burst);
        let weights = weights.of(label);
        let n = scored.len() as f64;
        let mut mean = [0.0; COLUMNS];
        for (_, row) in &scored {
            for (m, x) in mean.iter_mut().zip(row.iter()) {
                *m += x / n;
            }
        }
        let scores: Vec<f64> = scored.iter().map(|(_, r)| dot(&r[..], &weights)).collect();
        for (k, &(i, row)) in scored.iter().enumerate() {
            let part = |j: usize| weights[j] * (row[j] - mean[j]);
            let mut reasons: Vec<(&'static str, f64)> = REASONS
                .iter()
                .map(|(name, columns)| {
                    let own: f64 = columns.clone().map(part).sum();
                    let missing: f64 = MISSING
                        .iter()
                        .filter(|(_, value)| columns.contains(value))
                        .map(|&(j, _)| part(j))
                        .sum();
                    (*name, own + missing)
                })
                .collect();
            reasons.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
            out[i] = Some(Ranked {
                score: reasons.iter().map(|(_, v)| v).sum(),
                place: 1 + scores.iter().filter(|&&s| s > scores[k]).count(),
                of: scored.len(),
                reasons,
                genre: Some(label).filter(|g| *g != genre::UNKNOWN),
            });
        }
    }
    out
}

/// One folder's frames and judgments for `fit`. `viewed`, parallel to
/// `flags`, says which files the user looked at: only a viewed frame neither
/// picked nor rejected counts as a negative.
pub struct Labeled {
    pub frames: Vec<Frame>,
    pub flags: Vec<Flag>,
    pub viewed: Vec<bool>,
}

/// What `fit` found.
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    /// Its `offsets` hold every `genre::top_level` label.
    pub weights: Weights,
    /// The bursts that gave at least one pair.
    pub bursts: usize,
    /// Those bursts per genre, every `genre::top_level` label then
    /// `genre::UNKNOWN`, in that order.
    pub genres: Vec<(&'static str, usize)>,
    /// The folders that gave at least one pair.
    pub folders: usize,
}

/// One weighted pick-minus-other difference, of a burst whose genre is
/// `genre::top_level()[genre]`, or `genre::UNKNOWN` at `top_level().len()`.
struct Pair {
    diff: Row,
    weight: f64,
    genre: usize,
}

/// The pairs, the bursts that gave one per genre (indexed as `Pair.genre`)
/// and the folders that gave one.
fn pairs(folders: &[Labeled], top: &[&str]) -> (Vec<Pair>, Vec<usize>, usize) {
    let mut out = Vec::new();
    let mut bursts_used = vec![0; top.len() + 1];
    let mut folders_used = 0;
    for folder in folders {
        let bursts = bursts(&folder.frames);
        let rows = rows(&folder.frames, &bursts);
        let mut used = false;
        for burst in &bursts {
            let picks: Vec<&Row> = burst
                .iter()
                .filter(|&&i| folder.flags[i] == Flag::Pick)
                .filter_map(|&i| rows[i].as_ref())
                .collect();
            let others: Vec<(&Row, f64)> = burst
                .iter()
                .filter_map(|&i| {
                    let weight = match folder.flags[i] {
                        Flag::Pick => return None,
                        Flag::Reject => REJECT_WEIGHT,
                        Flag::None if folder.viewed[i] => 1.0,
                        Flag::None => return None,
                    };
                    rows[i].as_ref().map(|r| (r, weight))
                })
                .collect();
            let total: f64 = picks.len() as f64 * others.iter().map(|(_, w)| w).sum::<f64>();
            if total == 0.0 {
                continue;
            }
            let label = genre_of(&folder.frames, burst);
            let genre = top.iter().position(|t| *t == label).unwrap_or(top.len());
            for pick in &picks {
                for (other, weight) in &others {
                    let mut diff = [0.0; COLUMNS];
                    for (d, (p, o)) in diff.iter_mut().zip(pick.iter().zip(other.iter())) {
                        *d = p - o;
                    }
                    out.push(Pair {
                        diff,
                        weight: weight / total,
                        genre,
                    });
                }
            }
            bursts_used[genre] += 1;
            used = true;
        }
        folders_used += usize::from(used);
    }
    (out, bursts_used, folders_used)
}

/// Fit the weights on `folders`: every pick against every other frame of its
/// burst that counts as a negative (a reject at `REJECT_WEIGHT`, any other
/// frame the user viewed at 1), each burst's pairs weighing 1 in all,
/// by the pairwise logistic loss plus an L2 pull toward the prior's weights
/// as strong as the prior's own regularization (`1 / (2 C)` per burst, the
/// same objective as scikit-learn's at `C`). A pair of a burst of genre `g`
/// is scored by the shared weights plus `g`'s offset, each offset pulled
/// toward zero at `OFFSET_SHRINK` times that strength; a pair of an
/// `UNKNOWN` burst reaches the shared weights only, and a genre with no pair
/// keeps an offset of exactly zero. With no pair it returns the prior's
/// weights.
pub fn fit(folders: &[Labeled]) -> Fit {
    let prior = prior();
    let top = genre::top_level();
    let g = top.len();
    let (pairs, bursts, folders_used) = pairs(folders, &top);
    let tau = 1.0 / (2.0 * prior.c);
    let n = (1 + g) * COLUMNS;
    // The shared weights, then each genre's offset.
    let mut w = vec![0.0; n];
    w[..COLUMNS].copy_from_slice(&prior.weights);
    for _ in 0..50 {
        let weights: Vec<Vec<f64>> = (0..=g)
            .map(|k| {
                let mut v = w[..COLUMNS].to_vec();
                if k < g {
                    let offset = &w[(1 + k) * COLUMNS..(2 + k) * COLUMNS];
                    v.iter_mut().zip(offset).for_each(|(v, o)| *v += o);
                }
                v
            })
            .collect();
        // The data's gradient and Hessian per genre, `UNKNOWN` last.
        let mut grads = vec![[0.0; COLUMNS]; g + 1];
        let mut blocks = vec![vec![0.0; COLUMNS * COLUMNS]; g + 1];
        for pair in &pairs {
            let m = dot(&pair.diff, &weights[pair.genre]);
            let s = 1.0 / (1.0 + m.exp());
            for (gr, d) in grads[pair.genre].iter_mut().zip(&pair.diff) {
                *gr -= pair.weight * s * d;
            }
            let h = pair.weight * s * (1.0 - s);
            let block = &mut blocks[pair.genre];
            for j in 0..COLUMNS {
                let hj = h * pair.diff[j];
                for k in j..COLUMNS {
                    block[j * COLUMNS + k] += hj * pair.diff[k];
                }
            }
        }
        for block in &mut blocks {
            for j in 0..COLUMNS {
                for k in 0..j {
                    block[j * COLUMNS + k] = block[k * COLUMNS + j];
                }
            }
        }
        // The shared block is every genre's plus `tau I`; a genre's diagonal
        // block is its own plus `OFFSET_SHRINK * tau I`, its off-diagonal
        // blocks its own.
        let mut grad = vec![0.0; n];
        let mut hess = vec![0.0; n * n];
        for j in 0..COLUMNS {
            grad[j] = tau * (w[j] - prior.weights[j]);
            hess[j * n + j] = tau;
        }
        for (k, (gr, block)) in grads.iter().zip(&blocks).enumerate() {
            for j in 0..COLUMNS {
                grad[j] += gr[j];
                for l in 0..COLUMNS {
                    hess[j * n + l] += block[j * COLUMNS + l];
                }
            }
            if k == g {
                continue;
            }
            let at = (1 + k) * COLUMNS;
            for j in 0..COLUMNS {
                grad[at + j] = OFFSET_SHRINK * tau * w[at + j] + gr[j];
                hess[(at + j) * n + at + j] = OFFSET_SHRINK * tau;
                for l in 0..COLUMNS {
                    let h = block[j * COLUMNS + l];
                    hess[(at + j) * n + at + l] += h;
                    hess[(at + j) * n + l] = h;
                    hess[j * n + at + l] = h;
                }
            }
        }
        let step = solve(hess, grad);
        for (w, s) in w.iter_mut().zip(&step) {
            *w -= s;
        }
        if step.iter().map(|s| s * s).sum::<f64>().sqrt() < 1e-9 {
            break;
        }
    }
    let offsets = top
        .iter()
        .enumerate()
        .map(|(k, label)| {
            let at = (1 + k) * COLUMNS;
            (label.to_string(), w[at..at + COLUMNS].to_vec())
        })
        .collect();
    w.truncate(COLUMNS);
    Fit {
        weights: Weights { shared: w, offsets },
        bursts: bursts.iter().sum(),
        genres: top
            .iter()
            .copied()
            .chain([genre::UNKNOWN])
            .zip(bursts)
            .collect(),
        folders: folders_used,
    }
}

/// `x` with `a x = b` for a symmetric positive definite `a` (row-major,
/// `b.len()` square), by Cholesky.
fn solve(mut a: Vec<f64>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for j in 0..n {
        let d = (a[j * n + j] - (0..j).map(|k| a[j * n + k].powi(2)).sum::<f64>()).sqrt();
        a[j * n + j] = d;
        for i in j + 1..n {
            let s: f64 = (0..j).map(|k| a[i * n + k] * a[j * n + k]).sum();
            a[i * n + j] = (a[i * n + j] - s) / d;
        }
    }
    for i in 0..n {
        let s: f64 = (0..i).map(|k| a[i * n + k] * b[k]).sum();
        b[i] = (b[i] - s) / a[i * n + i];
    }
    for i in (0..n).rev() {
        let s: f64 = (i + 1..n).map(|k| a[k * n + i] * b[k]).sum();
        b[i] = (b[i] - s) / a[i * n + i];
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::{EMBED_VERSION, SHIPPED};

    fn frame(name: &str, time: &str, subsec: &str) -> Frame {
        Frame {
            name: name.into(),
            capture_time: Some(time.into()),
            subsec: Some(subsec.into()),
            vec: Some(vec![0.0; 768]),
            ..Frame::default()
        }
    }

    fn shared_only(shared: Vec<f64>) -> Weights {
        Weights {
            shared,
            offsets: BTreeMap::new(),
        }
    }

    fn names(frames: &[Frame], bursts: &[Vec<usize>]) -> Vec<Vec<String>> {
        bursts
            .iter()
            .map(|b| b.iter().map(|&i| frames[i].name.clone()).collect())
            .collect()
    }

    #[test]
    fn the_prior_is_made_for_the_shipped_embedding_and_these_columns() {
        let prior = prior();
        assert_eq!(prior.model, SHIPPED.name);
        assert_eq!(
            prior.embed_version, EMBED_VERSION,
            "re-run rank_prior.py when EMBED_VERSION moves"
        );
        assert_eq!(prior.rank_version, RANK_VERSION);
        assert_eq!(prior.tech_mean.len(), TECH);
        assert_eq!(prior.pca_mean.len(), 768);
        assert_eq!(prior.pca_basis.len(), PCA_DIMS);
        assert!(prior.pca_basis.iter().all(|b| b.len() == 768));
        for v in [&prior.mean, &prior.scale, &prior.weights] {
            assert_eq!(v.len(), COLUMNS);
        }
        assert!(prior.scale.iter().all(|s| *s > 0.0));
        assert!(prior.c > 0.0);
    }

    #[test]
    fn bursts_group_as_burst_ts_does() {
        let frames = vec![
            frame("c.ARW", "2026:09:19 10:00:01", "500"),
            frame("a.ARW", "2026:09:19 10:00:00", "5"),
            frame("b.ARW", "2026:09:19 10:00:00", "50"),
            frame("d.ARW", "2026:09:19 10:00:02", "51"),
            frame("e.ARW", "2026:09:19 10:00:04", "5"),
            Frame {
                name: "f.ARW".into(),
                ..Frame::default()
            },
            Frame {
                name: "g.ARW".into(),
                capture_time: Some("garbage".into()),
                ..Frame::default()
            },
        ];
        // `5` and `50` both read 10:00:00.500 and go by name; 1000 ms on to
        // 10:00:01.500 stays in the burst, 1010 ms on to 10:00:02.510 does
        // not. A time that does not parse sorts as a string, a missing one
        // last.
        assert_eq!(
            names(&frames, &bursts(&frames)),
            [
                vec!["a.ARW", "b.ARW", "c.ARW"],
                vec!["d.ARW"],
                vec!["e.ARW"],
                vec!["g.ARW"],
                vec!["f.ARW"],
            ]
        );
    }

    #[test]
    fn rows_fill_the_position_and_impute_a_missing_value() {
        let prior = prior();
        let mut frames = vec![
            frame("a.ARW", "2026:09:19 10:00:00", "0"),
            frame("b.ARW", "2026:09:19 10:00:00", "5"),
            frame("c.ARW", "2026:09:19 10:00:01", "0"),
        ];
        frames[0].sharpness = Some(50.0);
        frames[1].sharpness = Some(100.0);
        frames[2].vec = None;
        let bursts = bursts(&frames);
        let rows = rows(&frames, &bursts);
        let raw = |row: &Row, j: usize| row[j] * prior.scale[j] + prior.mean[j];
        let (a, b) = (rows[0].unwrap(), rows[1].unwrap());
        assert!((raw(&a, 0) - 0.5f64.ln()).abs() < 1e-9);
        assert!(raw(&b, 0).abs() < 1e-9, "the sharpest frame is 0");
        assert!((raw(&a, 1) - prior.tech_mean[1]).abs() < 1e-9);
        assert!((raw(&a, 7) - 1.0).abs() < 1e-9, "no eye_focus");
        assert!(raw(&a, TECH).abs() < 1e-9);
        assert!((raw(&b, TECH) - 0.5).abs() < 1e-9);
        assert!((raw(&a, TECH + 1) - 1.0).abs() < 1e-9);
        assert!(raw(&b, TECH + 1).abs() < 1e-9);
        assert!(rows[2].is_none(), "a frame without an embedding");
    }

    #[test]
    fn a_good_photo_clears_every_cut_of_photo_tier() {
        let good = Frame {
            eye_focus: Some(0.95),
            eyes_ear: Some(0.3),
            pose: Some(Pose {
                yaw: 30.0,
                pitch: -40.0,
                roll: 50.0,
            }),
            eye_offset: Some(0.05),
            edge_gap: Some(0.1),
            ..Frame::default()
        };
        assert!(good_photo(&good));
        for bad in [
            Frame {
                eye_focus: Some(0.85),
                ..good.clone()
            },
            Frame {
                eyes_ear: Some(0.2),
                ..good.clone()
            },
            Frame {
                pose: None,
                ..good.clone()
            },
            Frame {
                eye_offset: Some(0.2),
                ..good.clone()
            },
            Frame {
                edge_gap: Some(0.0),
                ..good.clone()
            },
        ] {
            assert!(!good_photo(&bad));
        }
    }

    #[test]
    fn rank_places_each_frame_in_its_burst_and_its_reasons_sum_to_its_score() {
        let mut frames = vec![
            frame("a.ARW", "2026:09:19 10:00:00", "0"),
            frame("b.ARW", "2026:09:19 10:00:00", "5"),
            frame("c.ARW", "2026:09:19 10:00:01", "0"),
            frame("d.ARW", "2026:09:19 10:00:05", "0"),
        ];
        frames[0].sharpness = Some(50.0);
        frames[1].sharpness = Some(100.0);
        frames[2].vec = None;
        let mut weights = vec![0.0; COLUMNS];
        weights[0] = 1.0;
        weights[TECH] = -0.5;
        let ranked = rank(&frames, &shared_only(weights));
        let (a, b) = (ranked[0].clone().unwrap(), ranked[1].clone().unwrap());
        assert_eq!((b.place, b.of, a.place, a.of), (1, 2, 2, 2));
        assert!(
            (a.score + b.score).abs() < 1e-9,
            "scores sit around the burst mean"
        );
        assert!(b.score > 0.0);
        for r in [&a, &b] {
            let sum: f64 = r.reasons.iter().map(|(_, v)| v).sum();
            assert!((sum - r.score).abs() < 1e-9);
            assert!(r.reasons.windows(2).all(|w| w[0].1.abs() >= w[1].1.abs()));
        }
        assert!(ranked[2].is_none());
        let d = ranked[3].clone().unwrap();
        assert_eq!((d.place, d.of), (1, 1));
        assert!(
            d.score.abs() < 1e-9,
            "a lone frame sits at its burst's mean"
        );
    }

    /// Two frames a second apart in one burst, the `sharper` one at 100.
    fn labeled(first: Flag, second: Flag, viewed: [bool; 2], sharper: usize) -> Labeled {
        let mut frames = vec![
            frame("a.ARW", "2026:09:19 10:00:00", "0"),
            frame("b.ARW", "2026:09:19 10:00:01", "0"),
        ];
        frames[sharper].sharpness = Some(100.0);
        frames[1 - sharper].sharpness = Some(20.0);
        Labeled {
            frames,
            flags: vec![first, second],
            viewed: viewed.to_vec(),
        }
    }

    #[test]
    fn a_fit_without_a_pair_is_the_prior() {
        let fit = fit(&[
            labeled(Flag::Pick, Flag::None, [true, false], 0),
            labeled(Flag::None, Flag::None, [true, true], 0),
            labeled(Flag::Pick, Flag::Pick, [true, true], 0),
        ]);
        assert_eq!(fit.weights.shared, prior_weights().shared);
        assert!(fit.weights.offsets.values().all(|o| o == &[0.0; COLUMNS]));
        assert_eq!((fit.bursts, fit.folders), (0, 0));
    }

    #[test]
    fn a_fit_counts_rejects_viewed_or_not_and_the_rest_only_when_viewed() {
        let fit = fit(&[
            labeled(Flag::Pick, Flag::Reject, [false, false], 0),
            labeled(Flag::Pick, Flag::Reject, [true, true], 0),
            labeled(Flag::Pick, Flag::None, [true, true], 0),
            labeled(Flag::Pick, Flag::None, [true, false], 0),
        ]);
        assert_eq!((fit.bursts, fit.folders), (3, 3));
    }

    #[test]
    fn an_unflagged_frame_is_a_negative_only_when_viewed() {
        let viewed = fit(&[labeled(Flag::Pick, Flag::None, [false, true], 0)]);
        assert_eq!((viewed.bursts, viewed.folders), (1, 1));
        let unviewed = fit(&[labeled(Flag::Pick, Flag::None, [true, false], 0)]);
        assert_eq!((unviewed.bursts, unviewed.folders), (0, 0));
        assert_eq!(unviewed.weights.shared, prior_weights().shared);
    }

    #[test]
    fn a_fit_leans_toward_what_the_picks_have_and_more_with_more_data() {
        let few: Vec<Labeled> = (0..5)
            .map(|_| labeled(Flag::Pick, Flag::None, [true, true], 0))
            .collect();
        let many: Vec<Labeled> = (0..500)
            .map(|_| labeled(Flag::Pick, Flag::None, [true, true], 0))
            .collect();
        let (few, many) = (fit(&few), fit(&many));
        let frames = labeled(Flag::Pick, Flag::None, [true, true], 0).frames;
        let rows = rows(&frames, &bursts(&frames));
        let (pick, other) = (rows[0].unwrap(), rows[1].unwrap());
        let margin = |w: &[f64]| dot(&pick, w) - dot(&other, w);
        assert!(margin(&few.weights.shared) > margin(&prior_weights().shared));
        assert!(margin(&many.weights.shared) > margin(&few.weights.shared));
        assert_eq!((many.bursts, many.folders), (500, 500));
    }

    /// One burst of two frames of `genre`, the first `sharper` (at 100
    /// against 20) or not, the `pick`ed one picked and the other viewed.
    fn of_genre(genre: &str, sharper: bool, pick: usize) -> Labeled {
        let vec = genre::vector_of(genre);
        let mut frames = vec![
            frame("a.ARW", "2026:09:19 10:00:00", "0"),
            frame("b.ARW", "2026:09:19 10:00:01", "0"),
        ];
        for (k, f) in frames.iter_mut().enumerate() {
            f.vec = Some(vec.clone());
            f.sharpness = Some(if (k == 0) == sharper { 100.0 } else { 20.0 });
        }
        let mut flags = vec![Flag::None; 2];
        flags[pick] = Flag::Pick;
        Labeled {
            frames,
            flags,
            viewed: vec![true; 2],
        }
    }

    /// `count` bursts of `genre` whose picks are the sharper frame (or the
    /// blurrier one), at either position in turn.
    fn bursts_of(genre: &str, count: usize, pick_sharper: bool) -> Vec<Labeled> {
        (0..count)
            .map(|i| of_genre(genre, (i % 2 == 0) == pick_sharper, i % 2))
            .collect()
    }

    /// The scores of a burst's two frames.
    fn scores(folder: &Labeled, weights: &Weights) -> (f64, f64) {
        let ranked = rank(&folder.frames, weights);
        let score = |i: usize| ranked[i].as_ref().unwrap().score;
        (score(0), score(1))
    }

    #[test]
    fn a_genre_with_no_pair_keeps_a_zero_offset_and_ranks_as_the_shared_weights() {
        let top = genre::top_level();
        let fit = fit(&bursts_of(top[0], 20, true));
        assert_eq!(fit.weights.offsets.len(), top.len());
        assert!(fit.weights.offsets[top[0]].iter().any(|o| *o != 0.0));
        let shared = shared_only(fit.weights.shared.clone());
        for label in &top[1..] {
            assert_eq!(fit.weights.offsets[*label], [0.0; COLUMNS]);
            let burst = of_genre(label, true, 0);
            let ranked = rank(&burst.frames, &fit.weights);
            assert_eq!(ranked, rank(&burst.frames, &shared));
            assert_eq!(ranked[0].as_ref().unwrap().genre, Some(*label));
        }
        assert_eq!(fit.genres[0], (top[0], 20));
        assert!(fit.genres[1..].iter().all(|(_, n)| *n == 0));
        assert_eq!(fit.genres.last().unwrap().0, genre::UNKNOWN);
    }

    #[test]
    fn a_genre_ranks_as_the_shared_model_until_it_has_many_contradicting_bursts() {
        let top = genre::top_level();
        let burst = of_genre(top[1], true, 0);
        let fit_with = |few: usize| {
            let mut folders = bursts_of(top[0], 300, true);
            folders.extend(bursts_of(top[1], few, false));
            fit(&folders)
        };
        let few = fit_with(2);
        let (a, b) = scores(&burst, &shared_only(few.weights.shared.clone()));
        assert!(a > b, "the shared model prefers the sharper frame");
        let (a, b) = scores(&burst, &few.weights);
        assert!(a > b, "two bursts leave the genre at the shared model");
        let many = fit_with(300);
        let (a, b) = scores(&burst, &shared_only(many.weights.shared.clone()));
        assert!(a > b, "the shared model still prefers the sharper frame");
        let (a, b) = scores(&burst, &many.weights);
        assert!(a < b, "300 bursts turn the genre to the blurrier frame");
    }

    #[test]
    fn an_unknown_burst_ranks_as_the_shared_weights_under_any_offsets() {
        let mut frames = vec![
            frame("a.ARW", "2026:09:19 10:00:00", "0"),
            frame("b.ARW", "2026:09:19 10:00:01", "0"),
        ];
        frames[0].sharpness = Some(100.0);
        frames[1].sharpness = Some(20.0);
        assert_eq!(genre_of(&frames, &[0, 1]), genre::UNKNOWN);
        let shared = prior_weights();
        let offsets = genre::top_level()
            .into_iter()
            .map(|label| (label.to_string(), vec![3.0; COLUMNS]))
            .chain([(genre::UNKNOWN.to_string(), vec![3.0; COLUMNS])])
            .collect();
        let with = Weights {
            offsets,
            ..shared.clone()
        };
        let ranked = rank(&frames, &with);
        assert_eq!(ranked, rank(&frames, &shared));
        assert_eq!(ranked[0].as_ref().unwrap().genre, None);
    }

    #[test]
    fn solve_inverts_a_symmetric_positive_definite_matrix() {
        for n in [COLUMNS, 6 * COLUMNS] {
            let mut a = vec![0.0; n * n];
            for i in 0..n {
                for j in 0..n {
                    a[i * n + j] = if i == j {
                        4.0
                    } else {
                        1.0 / (1 + i + j) as f64
                    };
                }
            }
            let x: Vec<f64> = (0..n).map(|i| i as f64 - 20.0).collect();
            let b: Vec<f64> = (0..n)
                .map(|i| (0..n).map(|j| a[i * n + j] * x[j]).sum())
                .collect();
            let solved = solve(a, b);
            assert!(solved.iter().zip(&x).all(|(s, x)| (s - x).abs() < 1e-9));
        }
    }
}
