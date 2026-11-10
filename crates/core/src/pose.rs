//! The head pose of a face from its face mesh points, ported from MediaPipe's
//! face geometry pipeline (`mediapipe/modules/face_geometry/libs`,
//! `geometry_pipeline.cc` and `procrustes_solver.cc`, Apache-2.0): the
//! screen points are unprojected through an assumed perspective camera and a
//! weighted orthogonal Procrustes fit of the canonical face onto them gives
//! the rotation, read here as yaw / pitch / roll.
//!
//! `BASIS` is copied verbatim, no change, from MediaPipe at commit
//! 212f110c65818db7f2d08c42a9539ecb2b6c0e1d: the landmark ids and weights
//! from the `procrustes_landmark_basis` entries of
//! `mediapipe/modules/face_geometry/data/geometry_pipeline_metadata_landmarks.pbtxt`,
//! the coordinates (cm) from the vertices of the same ids in its
//! `canonical_mesh` (identical to `canonical_face_model.obj`); provenance and
//! SHA-256 in `models/LICENSE-mediapipe`. Every other point has weight 0
//! and so no effect on the fit.
//!
//! The camera is MediaPipe's tasks default (`face_geometry_from_landmarks_graph.cc`:
//! origin at the top-left corner, `VERTICAL_FOV`, `NEAR`, `FAR`) over the
//! whole upright image the points are in.

/// The vertical field of view of the assumed camera, in degrees.
pub const VERTICAL_FOV: f64 = 63.0;
/// The near plane of the assumed camera, in cm.
pub const NEAR: f64 = 1.0;
/// The far plane of the assumed camera, in cm. MediaPipe sets it for its
/// projection matrix; the pose does not depend on it.
pub const FAR: f64 = 10000.0;
/// The face mesh points the pipeline reads: the 468 of the mesh, without
/// the iris points the model appends.
const MESH_POINTS: usize = 468;
/// MediaPipe's `kAbsoluteErrorEps`, the floor of the solver's checks.
const EPS: f64 = 1e-9;
/// MediaPipe's `kIsScreenLandmarkListTooCompactThreshold`: a face whose
/// normalized points all lie within this distance of their mean is skipped.
const TOO_COMPACT: f64 = 1e-3;
/// The largest roll, in degrees, a pose is returned with. Beyond it the
/// fitted face would be upside down: on the sample folders every such fit
/// (15 of 2024 judged faces) was a back of a head, an ear, a blur or a far
/// profile looking up, all three angles meaningless. A yaw past 90 degrees
/// is kept: there the mesh overshoots on a far profile, with the sign right
/// on 16 of the 18 readable such faces
/// (`docs/plans/20261007-head-pose/pose-results.md`).
const MAX_ROLL: f64 = 90.0;

/// `(landmark id, Procrustes weight, canonical position in cm)`.
const BASIS: [(usize, f64, [f64; 3]); 33] = [
    (4, 0.070909939706326, [0.000000, -0.463170, 7.586580]),
    (6, 0.032100144773722, [0.000000, 2.473255, 5.788627]),
    (10, 0.008446550928056, [0.000000, 8.261778, 4.481535]),
    (33, 0.058724168688059, [-4.445859, 2.663991, 3.173422]),
    (54, 0.007667080033571, [-6.279331, 6.615427, 1.425850]),
    (67, 0.009078059345484, [-3.523964, 8.005976, 3.729163]),
    (117, 0.009791937656701, [-5.258659, 0.945811, 2.974312]),
    (119, 0.014565368182957, [-3.300681, 0.861641, 3.872784]),
    (121, 0.018591361120343, [-1.820731, 1.467954, 4.224124]),
    (127, 0.005197994410992, [-7.743095, 2.364999, -2.005167]),
    (129, 0.120625205338001, [-1.785794, -0.978284, 4.850470]),
    (132, 0.005560018587857, [-7.270895, -2.890917, -2.252455]),
    (133, 0.05328618362546, [-1.856432, 2.585245, 3.757904]),
    (136, 0.066890455782413, [-5.085276, -7.178590, 0.714711]),
    (143, 0.014816547743976, [-6.407571, 2.236021, 1.560843]),
    (147, 0.014262833632529, [-6.234883, -1.944430, 1.663542]),
    (198, 0.025462191551924, [-1.246815, 0.230297, 5.681036]),
    (205, 0.047252278774977, [-3.832928, -1.537326, 4.137731]),
    (263, 0.058724168688059, [4.445859, 2.663991, 3.173422]),
    (284, 0.007667080033571, [6.279331, 6.615427, 1.425850]),
    (297, 0.009078059345484, [3.523964, 8.005976, 3.729163]),
    (346, 0.009791937656701, [5.258659, 0.945811, 2.974312]),
    (348, 0.014565368182957, [3.300681, 0.861641, 3.872784]),
    (350, 0.018591361120343, [1.820731, 1.467954, 4.224124]),
    (356, 0.005197994410992, [7.743095, 2.364999, -2.005167]),
    (358, 0.120625205338001, [1.785794, -0.978284, 4.850470]),
    (361, 0.005560018587857, [7.270895, -2.890917, -2.252455]),
    (362, 0.05328618362546, [1.856432, 2.585245, 3.757904]),
    (365, 0.066890455782413, [5.085276, -7.178590, 0.714711]),
    (372, 0.014816547743976, [6.407571, 2.236021, 1.560843]),
    (376, 0.014262833632529, [6.234883, -1.944430, 1.663542]),
    (420, 0.025462191551924, [1.246815, 0.230297, 5.681036]),
    (425, 0.047252278774977, [3.832928, -1.537326, 4.137731]),
];

type V3 = [f64; 3];
type M3 = [[f64; 3]; 3];

/// The orientation of a face, in degrees, as Handpic reads the rotation `R`
/// that takes MediaPipe's canonical face (looking at the camera) onto the
/// face, in its camera space (x toward the image's right, y up, z toward the
/// camera): `R = Ry(yaw) * Rx(-pitch) * Rz(-roll)`, yaw applied last.
///
/// - `yaw` is positive when the face turns toward the image's right.
/// - `pitch` is positive when the face tilts up.
/// - `roll` is positive when the head tilts clockwise on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
}

/// The pose of a face from its mesh `points` (at least the 468 of the mesh)
/// in the pixels of the `width` x `height` upright image, z in pixels at the
/// scale of x, larger away from the camera, with the default `VERTICAL_FOV`.
pub fn head_pose(points: &[[f32; 3]], width: usize, height: usize) -> Option<Pose> {
    head_pose_fov(points, width, height, VERTICAL_FOV)
}

/// `head_pose` under a camera of vertical field of view `fov` degrees.
/// `None` on fewer than 468 points, an empty image, a non-finite point, a
/// face too compact to fit, a degenerate solve, or a roll beyond
/// `MAX_ROLL`.
pub fn head_pose_fov(points: &[[f32; 3]], width: usize, height: usize, fov: f64) -> Option<Pose> {
    if points.len() < MESH_POINTS || width == 0 || height == 0 {
        return None;
    }
    let (w, h) = (width as f64, height as f64);
    let screen: Vec<V3> = points[..MESH_POINTS]
        .iter()
        .map(|p| [p[0] as f64 / w, p[1] as f64 / h, p[2] as f64 / w])
        .collect();
    if screen.iter().flatten().any(|v| !v.is_finite()) || too_compact(&screen) {
        return None;
    }
    let height_at_near = 2.0 * NEAR * (fov.to_radians() / 2.0).tan();
    let width_at_near = w * height_at_near / h;
    let (left, bottom) = (-width_at_near / 2.0, -height_at_near / 2.0);
    // ProjectXY with the origin at the top-left corner.
    let projected: Vec<V3> = screen
        .iter()
        .map(|p| {
            [
                p[0] * width_at_near + left,
                (1.0 - p[1]) * height_at_near + bottom,
                p[2] * width_at_near,
            ]
        })
        .collect();
    let depth_offset = projected.iter().map(|p| p[2]).sum::<f64>() / MESH_POINTS as f64;
    let basis: Vec<V3> = BASIS.iter().map(|b| projected[b.0]).collect();
    let canonical: Vec<V3> = BASIS.iter().map(|b| b.2).collect();
    let weights: Vec<f64> = BASIS.iter().map(|b| b.1).collect();
    let fit = |targets: &[V3]| procrustes(&canonical, targets, &weights);
    // MoveAndRescaleZ, UnprojectXY and ChangeHandedness.
    let metric = |scale: f64| -> Vec<V3> {
        basis
            .iter()
            .map(|p| {
                let z = (p[2] - depth_offset + NEAR) / scale;
                [p[0] * z / NEAR, p[1] * z / NEAR, -z]
            })
            .collect()
    };
    let first: Vec<V3> = basis.iter().map(|p| [p[0], p[1], -p[2]]).collect();
    let first_scale = fit(&first)?.scale;
    let second_scale = fit(&metric(first_scale))?.scale;
    let r = fit(&metric(first_scale * second_scale))?.rotation;
    let pose = Pose {
        yaw: r[0][2].atan2(r[2][2]).to_degrees(),
        pitch: r[1][2].clamp(-1.0, 1.0).asin().to_degrees(),
        roll: -r[1][0].atan2(r[1][1]).to_degrees(),
    };
    ([pose.yaw, pose.pitch, pose.roll]
        .iter()
        .all(|v| v.is_finite())
        && pose.roll.abs() <= MAX_ROLL)
        .then_some(pose)
}

/// MediaPipe's `IsScreenLandmarkListTooCompact` on normalized points.
fn too_compact(points: &[V3]) -> bool {
    let n = points.len() as f64;
    let mx = points.iter().map(|p| p[0]).sum::<f64>() / n;
    let my = points.iter().map(|p| p[1]).sum::<f64>() / n;
    points
        .iter()
        .map(|p| (p[0] - mx).hypot(p[1] - my))
        .fold(0.0, f64::max)
        <= TOO_COMPACT
}

/// `target ≈ scale * rotation * source + translation`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Similarity {
    rotation: M3,
    scale: f64,
    translation: V3,
}

/// MediaPipe's weighted orthogonal Procrustes solve: the similarity that
/// best takes `sources` onto `targets` under `weights`, the rotation proper
/// (no reflection). `None` when the total weight, the design matrix, the
/// scale's denominator or the scale is at or below `EPS`.
fn procrustes(sources: &[V3], targets: &[V3], weights: &[f64]) -> Option<Similarity> {
    let total: f64 = weights.iter().sum();
    if total <= EPS {
        return None;
    }
    let sqrt_w: Vec<f64> = weights.iter().map(|w| w.sqrt()).collect();
    let scaled = |p: &V3, s: f64| p.map(|v| v * s);
    let ws: Vec<V3> = sources
        .iter()
        .zip(&sqrt_w)
        .map(|(p, s)| scaled(p, *s))
        .collect();
    let wt: Vec<V3> = targets
        .iter()
        .zip(&sqrt_w)
        .map(|(p, s)| scaled(p, *s))
        .collect();
    let mut center = [0.0; 3];
    for (p, w) in sources.iter().zip(weights) {
        for k in 0..3 {
            center[k] += p[k] * w / total;
        }
    }
    let cws: Vec<V3> = ws
        .iter()
        .zip(&sqrt_w)
        .map(|(p, s)| {
            [
                p[0] - center[0] * s,
                p[1] - center[1] * s,
                p[2] - center[2] * s,
            ]
        })
        .collect();
    let mut design = [[0.0; 3]; 3];
    for (t, c) in wt.iter().zip(&cws) {
        for (r, row) in design.iter_mut().enumerate() {
            for (k, v) in row.iter_mut().enumerate() {
                *v += t[r] * c[k];
            }
        }
    }
    if design.iter().flatten().map(|v| v * v).sum::<f64>().sqrt() <= EPS {
        return None;
    }
    let rotation = rotation_of(&design)?;
    let numerator: f64 = cws
        .iter()
        .zip(&wt)
        .map(|(c, t)| dot(&mul(&rotation, c), t))
        .sum();
    let denominator: f64 = cws.iter().zip(&ws).map(|(c, s)| dot(c, s)).sum();
    if denominator <= EPS || numerator / denominator <= EPS {
        return None;
    }
    let scale = numerator / denominator;
    let mut translation = [0.0; 3];
    for ((t, s), w) in wt.iter().zip(&ws).zip(&sqrt_w) {
        let rs = mul(&rotation, s);
        for k in 0..3 {
            translation[k] += (t[k] - scale * rs[k]) * w / total;
        }
    }
    Some(Similarity {
        rotation,
        scale,
        translation,
    })
}

/// The proper rotation `U V^T` of the SVD `a = U S V^T`, with the sign of
/// the column of the least singular value flipped when `det(U) det(V) < 0`,
/// as MediaPipe's `ComputeOptimalRotation` does. `V` and the singular values
/// come from the eigen-decomposition of `a^T a`, made proper; `U`'s first
/// two columns are `a v / |a v|` and its third their cross product, which
/// is that flip. `None` when `a` has rank below 2.
fn rotation_of(a: &M3) -> Option<M3> {
    let mut ata = [[0.0; 3]; 3];
    for (r, row) in ata.iter_mut().enumerate() {
        for (c, v) in row.iter_mut().enumerate() {
            *v = (0..3).map(|k| a[k][r] * a[k][c]).sum();
        }
    }
    let (_, mut v) = symmetric_eigen(&ata);
    if det(&v) < 0.0 {
        for row in v.iter_mut() {
            row[2] = -row[2];
        }
    }
    let col = |m: &M3, c: usize| [m[0][c], m[1][c], m[2][c]];
    let u1 = mul(a, &col(&v, 0));
    let n1 = dot(&u1, &u1).sqrt();
    if n1 <= EPS {
        return None;
    }
    let u1 = u1.map(|x| x / n1);
    let u2 = mul(a, &col(&v, 1));
    let along = dot(&u2, &u1);
    let u2 = [0, 1, 2].map(|k| u2[k] - along * u1[k]);
    let n2 = dot(&u2, &u2).sqrt();
    if n2 <= EPS * n1 {
        return None;
    }
    let u2 = u2.map(|x| x / n2);
    let u3 = cross(&u1, &u2);
    let u = [u1, u2, u3];
    let mut r = [[0.0; 3]; 3];
    for (i, row) in r.iter_mut().enumerate() {
        for (j, x) in row.iter_mut().enumerate() {
            *x = (0..3).map(|k| u[k][i] * v[j][k]).sum();
        }
    }
    Some(r)
}

/// The eigenvalues of a symmetric 3x3 matrix, largest first, and their unit
/// eigenvectors as the columns of the second matrix, by cyclic Jacobi
/// rotations.
fn symmetric_eigen(s: &M3) -> (V3, M3) {
    let mut a = *s;
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..50 {
        let off = a[0][1].abs() + a[0][2].abs() + a[1][2].abs();
        let diag = a[0][0].abs() + a[1][1].abs() + a[2][2].abs();
        if off <= f64::EPSILON * diag || off == 0.0 {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if a[p][q] == 0.0 {
                continue;
            }
            let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let c = 1.0 / (t * t + 1.0).sqrt();
            let s = t * c;
            for row in a.iter_mut().chain(v.iter_mut()) {
                let (kp, kq) = (row[p], row[q]);
                row[p] = c * kp - s * kq;
                row[q] = s * kp + c * kq;
            }
            let (rp, rq) = (a[p], a[q]);
            a[p] = [0, 1, 2].map(|k| c * rp[k] - s * rq[k]);
            a[q] = [0, 1, 2].map(|k| s * rp[k] + c * rq[k]);
        }
    }
    let mut order = [0, 1, 2];
    order.sort_by(|&i, &j| a[j][j].total_cmp(&a[i][i]));
    let values = order.map(|i| a[i][i]);
    let vectors = [0, 1, 2].map(|r| order.map(|i| v[r][i]));
    (values, vectors)
}

fn mul(m: &M3, p: &V3) -> V3 {
    m.map(|row| dot(&row, p))
}

fn dot(a: &V3, b: &V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: &V3, b: &V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn det(m: &M3) -> f64 {
    dot(&m[0], &cross(&m[1], &m[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matmul(a: &M3, b: &M3) -> M3 {
        [0, 1, 2].map(|i| [0, 1, 2].map(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
    }

    fn rx(deg: f64) -> M3 {
        let (s, c) = deg.to_radians().sin_cos();
        [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
    }

    fn ry(deg: f64) -> M3 {
        let (s, c) = deg.to_radians().sin_cos();
        [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]
    }

    fn rz(deg: f64) -> M3 {
        let (s, c) = deg.to_radians().sin_cos();
        [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    }

    /// The rotation of a `Pose` in the documented convention.
    fn rotation(yaw: f64, pitch: f64, roll: f64) -> M3 {
        matmul(&ry(yaw), &matmul(&rx(-pitch), &rz(-roll)))
    }

    fn close(a: &M3, b: &M3, tol: f64) -> bool {
        a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .all(|(x, y)| (x - y).abs() <= tol)
    }

    fn canonical() -> Vec<V3> {
        BASIS.iter().map(|b| b.2).collect()
    }

    fn weights() -> Vec<f64> {
        BASIS.iter().map(|b| b.1).collect()
    }

    /// The face mesh points of the canonical face posed at `pose` with its
    /// origin at `center` (camera space, cm), seen by the default camera on a
    /// `width` x `height` image: x and y projected through the frustum, z the
    /// depth below the origin's in pixels at the origin's scale. The points
    /// outside `BASIS` sit at the origin.
    fn render(pose: (f64, f64, f64), center: V3, width: usize, height: usize) -> Vec<[f32; 3]> {
        let r = rotation(pose.0, pose.1, pose.2);
        let (w, h) = (width as f64, height as f64);
        let height_at_near = 2.0 * NEAR * (VERTICAL_FOV.to_radians() / 2.0).tan();
        let width_at_near = w * height_at_near / h;
        let depth = -center[2];
        let to_pixels = |c: &V3| {
            let rc = mul(&r, c);
            let p = [0, 1, 2].map(|k| rc[k] + center[k]);
            let (x, y) = (p[0] * NEAR / -p[2], p[1] * NEAR / -p[2]);
            [
                ((x / width_at_near + 0.5) * w) as f32,
                ((0.5 - y / height_at_near) * h) as f32,
                ((-p[2] - depth) * NEAR / depth / width_at_near * w) as f32,
            ]
        };
        let mut points = vec![to_pixels(&[0.0; 3]); 478];
        for b in &BASIS {
            points[b.0] = to_pixels(&b.2);
        }
        points
    }

    fn assert_pose(p: Pose, yaw: f64, pitch: f64, roll: f64, tol: f64) {
        assert!(
            (p.yaw - yaw).abs() <= tol
                && (p.pitch - pitch).abs() <= tol
                && (p.roll - roll).abs() <= tol,
            "{p:?} against ({yaw}, {pitch}, {roll})"
        );
    }

    #[test]
    fn the_eigen_solve_finds_the_eigenpairs_largest_first() {
        let s = [[4.0, 1.0, 0.0], [1.0, 3.0, 0.0], [0.0, 0.0, 1.0]];
        let (values, vectors) = symmetric_eigen(&s);
        let r5 = 5f64.sqrt();
        let expected = [(7.0 + r5) / 2.0, (7.0 - r5) / 2.0, 1.0];
        for i in 0..3 {
            assert!((values[i] - expected[i]).abs() < 1e-12, "{values:?}");
            let v = [vectors[0][i], vectors[1][i], vectors[2][i]];
            let sv = mul(&s, &v);
            assert!((dot(&v, &v) - 1.0).abs() < 1e-12);
            for k in 0..3 {
                assert!(
                    (sv[k] - values[i] * v[k]).abs() < 1e-12,
                    "{i}: {sv:?} {v:?}"
                );
            }
        }
        let (values, _) = symmetric_eigen(&[[2.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 3.0]]);
        assert_eq!(values, [5.0, 3.0, 2.0]);
    }

    #[test]
    fn the_rotation_of_a_matrix_is_its_polar_factor_without_reflection() {
        let r = rotation(25.0, -40.0, 70.0);
        let stretched = matmul(&r, &[[3.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 0.5]]);
        assert!(close(&rotation_of(&stretched).unwrap(), &r, 1e-12));
        let rotated_back = matmul(&stretched, &rotation(-10.0, 5.0, 30.0));
        let expected = matmul(&r, &rotation(-10.0, 5.0, 30.0));
        assert!(close(
            &rotation_of(&rotated_back).unwrap(),
            &expected,
            1e-12
        ));
        // det(U) det(V) < 0: the least singular direction is flipped back.
        let reflected = matmul(&r, &[[3.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, -1.0]]);
        assert!(close(&rotation_of(&reflected).unwrap(), &r, 1e-12));
        let rank_one = [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 0.0]];
        assert_eq!(rotation_of(&rank_one), None);
        assert_eq!(rotation_of(&[[0.0; 3]; 3]), None);
    }

    #[test]
    fn procrustes_recovers_a_rotation_scale_and_translation() {
        let r = rotation(-30.0, 12.0, 5.0);
        let t = [1.5, -2.0, -40.0];
        let targets: Vec<V3> = canonical()
            .iter()
            .map(|p| {
                let q = mul(&r, p);
                [0, 1, 2].map(|k| 0.25 * q[k] + t[k])
            })
            .collect();
        let s = procrustes(&canonical(), &targets, &weights()).unwrap();
        assert!(close(&s.rotation, &r, 1e-9), "{s:?}");
        assert!((s.scale - 0.25).abs() < 1e-12, "{s:?}");
        for (got, want) in s.translation.iter().zip(t) {
            assert!((got - want).abs() < 1e-9, "{s:?}");
        }
    }

    #[test]
    fn procrustes_returns_a_proper_rotation_for_a_mirrored_target() {
        let r = rotation(20.0, 0.0, 0.0);
        let targets: Vec<V3> = canonical()
            .iter()
            .map(|p| mul(&r, &[p[0], p[1], -p[2]]))
            .collect();
        let s = procrustes(&canonical(), &targets, &weights()).unwrap();
        assert!((det(&s.rotation) - 1.0).abs() < 1e-12, "{s:?}");
        let rrt = matmul(
            &s.rotation,
            &[0, 1, 2].map(|i| [0, 1, 2].map(|j| s.rotation[j][i])),
        );
        assert!(close(
            &rrt,
            &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            1e-12
        ));
    }

    #[test]
    fn procrustes_fails_on_degenerate_input() {
        let c = canonical();
        assert_eq!(procrustes(&c, &c, &[0.0; 33]), None);
        assert_eq!(procrustes(&c, &vec![[1.0, 2.0, 3.0]; 33], &weights()), None);
        let mirrored_scale: Vec<V3> = c.iter().map(|p| p.map(|v| -v)).collect();
        let flat: Vec<V3> = c.iter().map(|p| [p[0], 0.0, 0.0]).collect();
        assert!(procrustes(&c, &mirrored_scale, &weights()).is_some());
        assert_eq!(procrustes(&c, &flat, &weights()), None);
    }

    #[test]
    fn an_unrotated_face_reads_zero_centered_or_not() {
        for center in [
            [0.0, 0.0, -60.0],
            [25.0, -15.0, -60.0],
            [-30.0, 18.0, -45.0],
        ] {
            let p = head_pose(&render((0.0, 0.0, 0.0), center, 1600, 1067), 1600, 1067).unwrap();
            assert_pose(p, 0.0, 0.0, 0.0, 0.5);
        }
    }

    #[test]
    fn a_posed_face_reads_its_angles() {
        for pose in [
            (30.0, 0.0, 0.0),
            (-30.0, 0.0, 0.0),
            (0.0, 20.0, 0.0),
            (0.0, -20.0, 0.0),
            (0.0, 0.0, 15.0),
            (0.0, 0.0, -15.0),
            (25.0, -10.0, 8.0),
            (-45.0, 15.0, -20.0),
        ] {
            for center in [[0.0, 0.0, -60.0], [20.0, 10.0, -70.0]] {
                let p = head_pose(&render(pose, center, 1600, 1067), 1600, 1067).unwrap();
                assert_pose(p, pose.0, pose.1, pose.2, 1.0);
            }
        }
    }

    /// The convention read off the image, not off `rotation`: the nose tip
    /// (4) against the midpoint of the outer eye corners (33, 263), and the
    /// image-right eye corner against the image-left one.
    #[test]
    fn the_signs_follow_the_image() {
        let at = |pose| render(pose, [0.0, 0.0, -60.0], 1600, 1067);
        let nose = |p: &[[f32; 3]]| {
            (
                p[4][0] - (p[33][0] + p[263][0]) / 2.0,
                p[4][1] - (p[33][1] + p[263][1]) / 2.0,
            )
        };
        let frontal = nose(&at((0.0, 0.0, 0.0)));

        let right = at((20.0, 0.0, 0.0));
        assert!(nose(&right).0 > frontal.0);
        assert!(head_pose(&right, 1600, 1067).unwrap().yaw > 10.0);

        let up = at((0.0, 20.0, 0.0));
        assert!(nose(&up).1 < frontal.1);
        assert!(head_pose(&up, 1600, 1067).unwrap().pitch > 10.0);

        let clockwise = at((0.0, 0.0, 20.0));
        assert!(clockwise[263][1] > clockwise[33][1]);
        assert!(head_pose(&clockwise, 1600, 1067).unwrap().roll > 10.0);
    }

    #[test]
    fn degenerate_input_has_no_pose() {
        let face = render((0.0, 0.0, 0.0), [0.0, 0.0, -60.0], 1600, 1067);
        assert_eq!(head_pose(&face[..467], 1600, 1067), None);
        assert_eq!(head_pose(&face, 0, 1067), None);
        assert_eq!(head_pose(&face, 1600, 0), None);
        let mut nan = face.clone();
        nan[100][2] = f32::NAN;
        assert_eq!(head_pose(&nan, 1600, 1067), None);
        let mut inf = face.clone();
        inf[4][0] = f32::INFINITY;
        assert_eq!(head_pose(&inf, 1600, 1067), None);
        assert_eq!(head_pose(&vec![[800.0, 500.0, 0.0]; 478], 1600, 1067), None);
        assert!(head_pose(&face, 1600, 1067).is_some());
    }

    #[test]
    fn an_upside_down_face_has_no_pose() {
        let at = |pose| render(pose, [0.0, 0.0, -60.0], 1600, 1067);
        assert!(head_pose(&at((0.0, 0.0, 80.0)), 1600, 1067).is_some());
        assert!(head_pose(&at((0.0, 0.0, -80.0)), 1600, 1067).is_some());
        assert_eq!(head_pose(&at((0.0, 0.0, 120.0)), 1600, 1067), None);
        assert_eq!(head_pose(&at((0.0, 0.0, -150.0)), 1600, 1067), None);
        assert_pose(
            head_pose(&at((110.0, 0.0, 0.0)), 1600, 1067).unwrap(),
            110.0,
            0.0,
            0.0,
            1.0,
        );
    }

    #[test]
    fn a_narrower_fov_reads_the_same_frontal_face() {
        let face = render((0.0, 0.0, 0.0), [0.0, 0.0, -60.0], 1600, 1067);
        assert_pose(
            head_pose_fov(&face, 1600, 1067, 10.0).unwrap(),
            0.0,
            0.0,
            0.0,
            0.5,
        );
    }
}
