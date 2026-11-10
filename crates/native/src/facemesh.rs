//! The face parts the eye judgment was taken on, as the `f` overlay draws
//! them. A port of the Tauri frontend's `facemesh.ts`.

/* MediaPipe Face Mesh `FACEMESH_CONTOURS`, `FACEMESH_NOSE` and
 * `FACEMESH_IRISES`, from mediapipe/python/solutions/face_mesh_connections.py
 * at commit 212f110c65818db7f2d08c42a9539ecb2b6c0e1d
 * (https://github.com/google-ai-edge/mediapipe), Apache License 2.0,
 * Copyright 2021 The MediaPipe Authors. Modification: each set's directed
 * pairs reduced to undirected edges, each as (lower, higher) index, sorted;
 * the contours (`FACEMESH_LIPS`, `FACEMESH_LEFT_EYE`,
 * `FACEMESH_LEFT_EYEBROW`, `FACEMESH_RIGHT_EYE`, `FACEMESH_RIGHT_EYEBROW`,
 * `FACEMESH_FACE_OVAL`, 124 edges) merged with the nose (25 edges) into one
 * table of 149 edges; the irises (`FACEMESH_LEFT_IRIS` +
 * `FACEMESH_RIGHT_IRIS`) kept as 8 edges. Full notice in
 * `crates/core/models/LICENSE-mediapipe`. */
/// The two point indices of each edge, all below 468.
pub const FACE_OUTLINE_EDGES: [[usize; 2]; 149] = [
    [0, 37],
    [0, 267],
    [1, 4],
    [1, 19],
    [2, 94],
    [2, 97],
    [2, 326],
    [4, 5],
    [4, 45],
    [4, 275],
    [5, 195],
    [6, 168],
    [6, 197],
    [7, 33],
    [7, 163],
    [10, 109],
    [10, 338],
    [13, 82],
    [13, 312],
    [14, 87],
    [14, 317],
    [17, 84],
    [17, 314],
    [19, 94],
    [21, 54],
    [21, 162],
    [33, 246],
    [37, 39],
    [39, 40],
    [40, 185],
    [45, 220],
    [46, 53],
    [48, 64],
    [48, 115],
    [52, 53],
    [52, 65],
    [54, 103],
    [55, 65],
    [58, 132],
    [58, 172],
    [61, 146],
    [61, 185],
    [63, 70],
    [63, 105],
    [64, 98],
    [66, 105],
    [66, 107],
    [67, 103],
    [67, 109],
    [78, 95],
    [78, 191],
    [80, 81],
    [80, 191],
    [81, 82],
    [84, 181],
    [87, 178],
    [88, 95],
    [88, 178],
    [91, 146],
    [91, 181],
    [93, 132],
    [93, 234],
    [97, 98],
    [115, 220],
    [127, 162],
    [127, 234],
    [133, 155],
    [133, 173],
    [136, 150],
    [136, 172],
    [144, 145],
    [144, 163],
    [145, 153],
    [148, 152],
    [148, 176],
    [149, 150],
    [149, 176],
    [152, 377],
    [153, 154],
    [154, 155],
    [157, 158],
    [157, 173],
    [158, 159],
    [159, 160],
    [160, 161],
    [161, 246],
    [195, 197],
    [249, 263],
    [249, 390],
    [251, 284],
    [251, 389],
    [263, 466],
    [267, 269],
    [269, 270],
    [270, 409],
    [275, 440],
    [276, 283],
    [278, 294],
    [278, 344],
    [282, 283],
    [282, 295],
    [284, 332],
    [285, 295],
    [288, 361],
    [288, 397],
    [291, 375],
    [291, 409],
    [293, 300],
    [293, 334],
    [294, 327],
    [296, 334],
    [296, 336],
    [297, 332],
    [297, 338],
    [308, 324],
    [308, 415],
    [310, 311],
    [310, 415],
    [311, 312],
    [314, 405],
    [317, 402],
    [318, 324],
    [318, 402],
    [321, 375],
    [321, 405],
    [323, 361],
    [323, 454],
    [326, 327],
    [344, 440],
    [356, 389],
    [356, 454],
    [362, 382],
    [362, 398],
    [365, 379],
    [365, 397],
    [373, 374],
    [373, 390],
    [374, 380],
    [377, 400],
    [378, 379],
    [378, 400],
    [380, 381],
    [381, 382],
    [384, 385],
    [384, 398],
    [385, 386],
    [386, 387],
    [387, 388],
    [388, 466],
];

/// The two rings around the iris centers.
pub const FACE_IRIS_EDGES: [[usize; 2]; 8] = [
    [469, 470],
    [469, 472],
    [470, 471],
    [471, 472],
    [474, 475],
    [474, 477],
    [475, 476],
    [476, 477],
];

/// The iris centers, in no edge, so they are drawn as dots.
pub const FACE_IRIS_CENTERS: [usize; 2] = [468, 473];

/// What to draw for a judgment in `state`: the parts outline always, the
/// irises only for open eyes, since the model places iris points on a closed
/// eye too. The edges, then the dots.
pub fn mesh_edges(state: &str) -> (Vec<[usize; 2]>, &'static [usize]) {
    if state == "closed" {
        return (FACE_OUTLINE_EDGES.to_vec(), &[]);
    }
    let edges = FACE_OUTLINE_EDGES
        .iter()
        .chain(&FACE_IRIS_EDGES)
        .copied()
        .collect();
    (edges, &FACE_IRIS_CENTERS)
}

/// The points scaled onto the unrotated preview drawn `draw_width` x
/// `draw_height` and centered on the origin, the same rule as `face_marks`.
pub fn mesh_points(
    points: &[[f32; 2]],
    preview_width: f32,
    preview_height: f32,
    draw_width: f32,
    draw_height: f32,
) -> Vec<(f32, f32)> {
    let sx = draw_width / preview_width;
    let sy = draw_height / preview_height;
    points
        .iter()
        .map(|[x, y]| (x * sx - draw_width / 2., y * sy - draw_height / 2.))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn assert_well_formed(edges: &[[usize; 2]], low: usize, high: usize) {
        let mut seen = HashSet::new();
        for &[a, b] in edges {
            assert!((low..high).contains(&a) && (low..high).contains(&b));
            assert_ne!(a, b);
            assert!(seen.insert((a.min(b), a.max(b))));
        }
    }

    #[test]
    fn the_outline_holds_149_contour_and_nose_edges_over_the_first_468_points() {
        assert_eq!(FACE_OUTLINE_EDGES.len(), 149);
        assert_well_formed(&FACE_OUTLINE_EDGES, 0, 468);
    }

    #[test]
    fn the_irises_hold_8_ring_edges_over_the_iris_points() {
        assert_eq!(FACE_IRIS_EDGES.len(), 8);
        assert_well_formed(&FACE_IRIS_EDGES, 468, 478);
    }

    #[test]
    fn the_iris_centers_468_and_473_are_in_no_edge() {
        assert_eq!(FACE_IRIS_CENTERS, [468, 473]);
        for center in FACE_IRIS_CENTERS {
            assert!(FACE_IRIS_EDGES.iter().all(|e| !e.contains(&center)));
        }
    }

    #[test]
    fn open_eyes_get_the_outline_the_iris_rings_and_the_iris_dots() {
        let (edges, dots) = mesh_edges("open");
        assert_eq!(
            edges,
            [FACE_OUTLINE_EDGES.as_slice(), &FACE_IRIS_EDGES].concat()
        );
        assert_eq!(dots, [468, 473]);
    }

    #[test]
    fn closed_eyes_get_the_outline_only_no_iris_and_no_dot() {
        let (edges, dots) = mesh_edges("closed");
        assert_eq!(edges, FACE_OUTLINE_EDGES);
        assert!(dots.is_empty());
    }

    #[test]
    fn a_point_at_the_previews_origin_sits_at_the_drawn_images_corner() {
        assert_eq!(
            mesh_points(&[[0., 0.]], 1600., 1080., 1600., 1080.),
            [(-800., -540.)]
        );
    }

    #[test]
    fn points_scale_with_the_drawn_size_not_the_bitmaps() {
        assert_eq!(
            mesh_points(&[[800., 540.], [400., 270.]], 1600., 1080., 800., 540.),
            [(0., 0.), (-200., -135.)]
        );
    }
}
