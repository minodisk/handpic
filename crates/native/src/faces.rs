//! The faces the `f` overlay draws, detected on demand per file and kept for
//! the session. A port of the Tauri frontend's `faces.ts` (`FaceCache`).

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use riffle_backend::ondemand::FacesResponse;

/// What a file whose detection failed is cached as, so it is not retried on
/// every draw.
pub fn no_faces() -> FacesResponse {
    FacesResponse {
        width: 0,
        height: 0,
        faces: Vec::new(),
    }
}

#[derive(Default)]
pub struct FaceCache {
    found: HashMap<String, Arc<FacesResponse>>,
    in_flight: HashSet<String>,
    generation: u64,
}

impl FaceCache {
    pub fn get(&self, path: &str) -> Option<&Arc<FacesResponse>> {
        self.found.get(path)
    }

    /// The token to settle the detection with, or `None` when `path` is
    /// cached or already being detected, so a path is never detected twice.
    pub fn request(&mut self, path: &str) -> Option<u64> {
        if self.found.contains_key(path) || self.in_flight.contains(path) {
            return None;
        }
        self.in_flight.insert(path.to_owned());
        Some(self.generation)
    }

    /// Whether the answer was kept: one issued before the last `clear` is
    /// dropped, since the file may have changed since. It is kept by path,
    /// so an answer for a file paged away from still saves a detection when
    /// it is shown again.
    pub fn settle(&mut self, path: String, token: u64, faces: FacesResponse) -> bool {
        if token != self.generation {
            return false;
        }
        self.in_flight.remove(&path);
        self.found.insert(path, Arc::new(faces));
        true
    }

    pub fn clear(&mut self) {
        self.found.clear();
        self.in_flight.clear();
        self.generation += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use riffle_backend::ondemand::{FaceBox, FacePoint};

    fn found() -> FacesResponse {
        let p = |x: f32, y: f32| FacePoint { x, y };
        FacesResponse {
            width: 1616,
            height: 1080,
            faces: vec![FaceBox {
                x: 10.,
                y: 20.,
                width: 30.,
                height: 40.,
                score: 0.9,
                left_eye: p(18., 30.),
                right_eye: p(32., 30.),
                nose: p(25., 38.),
                mouth_left: p(19., 48.),
                mouth_right: p(31., 48.),
            }],
        }
    }

    fn faces_of(cache: &FaceCache, path: &str) -> Option<usize> {
        cache.get(path).map(|f| f.faces.len())
    }

    #[test]
    fn requests_an_uncached_path_once_while_it_is_in_flight() {
        let mut cache = FaceCache::default();
        assert!(cache.request("a").is_some());
        assert!(cache.request("a").is_none());
        assert!(cache.request("b").is_some());
    }

    #[test]
    fn keeps_an_answer_and_never_requests_its_path_again() {
        let mut cache = FaceCache::default();
        let token = cache.request("a").unwrap();
        assert!(cache.settle("a".into(), token, found()));
        assert_eq!(faces_of(&cache, "a"), Some(1));
        assert!(cache.request("a").is_none());
    }

    #[test]
    fn a_failure_cached_as_no_faces_is_not_retried() {
        let mut cache = FaceCache::default();
        let token = cache.request("a").unwrap();
        cache.settle("a".into(), token, no_faces());
        assert_eq!(faces_of(&cache, "a"), Some(0));
        assert!(cache.request("a").is_none());
    }

    #[test]
    fn drops_an_answer_issued_before_a_clear_and_keeps_the_new_request_in_flight() {
        let mut cache = FaceCache::default();
        let stale = cache.request("a").unwrap();
        cache.clear();
        let fresh = cache.request("a").unwrap();
        assert!(!cache.settle("a".into(), stale, found()));
        assert_eq!(faces_of(&cache, "a"), None);
        assert!(cache.request("a").is_none());
        assert!(cache.settle("a".into(), fresh, found()));
        assert_eq!(faces_of(&cache, "a"), Some(1));
    }

    #[test]
    fn clear_forgets_what_was_found() {
        let mut cache = FaceCache::default();
        let token = cache.request("a").unwrap();
        cache.settle("a".into(), token, found());
        cache.clear();
        assert_eq!(faces_of(&cache, "a"), None);
        assert!(cache.request("a").is_some());
    }
}
