//! Whether the eyes of the shown file's AF face are closed, judged on demand
//! per file and kept for the session, for the `f` overlay's face mesh. A
//! port of the Tauri frontend's `eyes.ts` (`EyesCache`).

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use riffle_backend::ondemand::EyesJudgment;

/// What `request` hands out: the id the backend's `current` check
/// supersedes older requests by, and the generation the answer is settled
/// against.
#[derive(Debug, Clone, Copy)]
pub struct EyesTicket {
    pub id: u64,
    generation: u64,
}

#[derive(Default)]
pub struct EyesCache {
    /// `None` is a file whose eyes are unknown (no face, a face too small, a
    /// failure), cached so it is not judged again.
    judged: HashMap<String, Option<Arc<EyesJudgment>>>,
    in_flight: bool,
    generation: u64,
    last_id: u64,
    /// The latest ticket's id, `0` after a `clear`, read by the running
    /// judgment's `current` closure on its own thread.
    latest: Arc<AtomicU64>,
}

impl EyesCache {
    pub fn get(&self, path: &str) -> Option<&Option<Arc<EyesJudgment>>> {
        self.judged.get(path)
    }

    /// The ticket to judge `path` with, or `None` when it is cached or any
    /// judgment is in flight: at most one runs at a time, so paging with a
    /// key held never queues one per file passed. The caller asks again for
    /// the file that is current once the running one settles.
    pub fn request(&mut self, path: &str) -> Option<EyesTicket> {
        if self.judged.contains_key(path) || self.in_flight {
            return None;
        }
        self.in_flight = true;
        self.last_id += 1;
        self.latest.store(self.last_id, Ordering::Relaxed);
        Some(EyesTicket {
            id: self.last_id,
            generation: self.generation,
        })
    }

    /// Whether `ticket` is still the latest request: false once a newer one
    /// is issued or the cache is cleared, which stops the judgment at its
    /// next stage as superseded.
    pub fn current(&self, ticket: EyesTicket) -> impl Fn() -> bool + Send + 'static {
        let latest = self.latest.clone();
        move || latest.load(Ordering::Relaxed) == ticket.id
    }

    /// Whether the answer was kept: one issued before the last `clear` is
    /// dropped, since the file may have changed since, and leaves a judgment
    /// started after the clear in flight. It is kept by path, so an answer
    /// for a file paged away from still saves a judgment when it is shown
    /// again. `None` (a judgment the backend stopped as superseded) frees the
    /// slot without caching anything; `Some(None)` caches unknown eyes.
    pub fn settle(
        &mut self,
        path: String,
        ticket: EyesTicket,
        eyes: Option<Option<EyesJudgment>>,
    ) -> bool {
        if ticket.generation != self.generation {
            return false;
        }
        self.in_flight = false;
        let Some(eyes) = eyes else {
            return false;
        };
        self.judged.insert(path, eyes.map(Arc::new));
        true
    }

    pub fn clear(&mut self) {
        self.judged.clear();
        self.in_flight = false;
        self.generation += 1;
        self.latest.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use riffle_backend::ondemand::EyesMesh;

    fn judgment(state: &'static str) -> EyesJudgment {
        EyesJudgment {
            state,
            probability: if state == "closed" { 0.81 } else { 0.07 },
            ear: if state == "closed" { 0.08 } else { 0.3 },
            pose: None,
            mesh: EyesMesh {
                width: 1600,
                height: 1080,
                points: vec![[800., 540.]],
            },
        }
    }

    fn state_of(cache: &EyesCache, path: &str) -> Option<Option<&'static str>> {
        cache
            .get(path)
            .map(|eyes| eyes.as_ref().map(|eyes| eyes.state))
    }

    #[test]
    fn judges_an_uncached_path_once_while_it_is_in_flight() {
        let mut cache = EyesCache::default();
        assert!(cache.request("a").is_some());
        assert!(cache.request("a").is_none());
    }

    #[test]
    fn runs_one_judgment_at_a_time_and_asks_again_once_it_settles() {
        let mut cache = EyesCache::default();
        let a = cache.request("a").unwrap();
        assert!(cache.request("b").is_none());
        assert!(cache.request("c").is_none());
        assert!(cache.settle("a".into(), a, Some(Some(judgment("open")))));
        let c = cache.request("c").unwrap();
        assert!(c.id > a.id);
    }

    #[test]
    fn keeps_an_answer_for_a_path_paged_away_from_and_never_judges_it_again() {
        let mut cache = EyesCache::default();
        let a = cache.request("a").unwrap();
        assert!(cache.settle("a".into(), a, Some(Some(judgment("closed")))));
        assert_eq!(state_of(&cache, "a"), Some(Some("closed")));
        assert!(cache.request("a").is_none());
    }

    #[test]
    fn caches_unknown_eyes_so_they_are_not_judged_again() {
        let mut cache = EyesCache::default();
        let a = cache.request("a").unwrap();
        assert!(cache.settle("a".into(), a, Some(None)));
        assert_eq!(state_of(&cache, "a"), Some(None));
        assert!(cache.request("a").is_none());
    }

    #[test]
    fn a_superseded_judgment_frees_the_slot_without_caching() {
        let mut cache = EyesCache::default();
        let a = cache.request("a").unwrap();
        assert!(!cache.settle("a".into(), a, None));
        assert_eq!(state_of(&cache, "a"), None);
        assert!(cache.request("a").is_some());
    }

    #[test]
    fn drops_an_answer_issued_before_a_clear_and_keeps_the_new_request_in_flight() {
        let mut cache = EyesCache::default();
        let stale = cache.request("a").unwrap();
        cache.clear();
        let fresh = cache.request("b").unwrap();
        assert!(fresh.id > stale.id);
        assert!(!cache.settle("a".into(), stale, Some(Some(judgment("closed")))));
        assert_eq!(state_of(&cache, "a"), None);
        assert!(cache.request("a").is_none());
        assert!(cache.settle("b".into(), fresh, Some(Some(judgment("open")))));
        assert_eq!(state_of(&cache, "b"), Some(Some("open")));
    }

    #[test]
    fn clear_forgets_what_was_judged() {
        let mut cache = EyesCache::default();
        let a = cache.request("a").unwrap();
        cache.settle("a".into(), a, Some(Some(judgment("closed"))));
        cache.clear();
        assert_eq!(state_of(&cache, "a"), None);
        assert!(cache.request("a").is_some());
    }

    #[test]
    fn a_request_across_a_clear_supersedes_the_older_one() {
        let mut cache = EyesCache::default();
        let older = cache.request("a").unwrap();
        let older_current = cache.current(older);
        assert!(older_current());
        cache.clear();
        assert!(!older_current());
        let newer = cache.request("b").unwrap();
        assert!(!older_current());
        assert!(cache.current(newer)());
    }
}
