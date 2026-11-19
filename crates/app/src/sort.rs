//! The strip's order, a port of `sort.ts`. `Name` compares base names by
//! `natord`, the native order since port Step 2, not the Tauri plain compare.

use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Capture,
    Rating,
}

impl SortKey {
    /// The stored `sortOrder`, by the Tauri `parse_sort_order` rule.
    pub fn from_setting(value: Option<&str>) -> Self {
        match value {
            Some("capture") => Self::Capture,
            Some("rating") => Self::Rating,
            // The ranking's order is port Step 9's; until then a stored
            // `recommended` is capture order, the Tauri `sortInForce` while
            // the ranking is off.
            Some("recommended") => Self::Capture,
            _ => Self::Name,
        }
    }

    pub fn setting(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Capture => "capture",
            Self::Rating => "rating",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SortFacts {
    pub capture_time: Option<String>,
    pub subsec: Option<String>,
    /// `-1` for a reject, which sorts after the unrated whatever its stars.
    pub rating: Option<i8>,
}

fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn compare_subsec(a: &str, b: &str) -> Ordering {
    fn padded(s: &str, width: usize) -> impl Iterator<Item = u8> + '_ {
        s.bytes().chain(std::iter::repeat(b'0')).take(width)
    }
    let width = a.len().max(b.len());
    padded(a, width).cmp(padded(b, width))
}

fn compare_capture(a: &SortFacts, b: &SortFacts) -> Ordering {
    match (&a.capture_time, &b.capture_time) {
        (Some(ta), Some(tb)) => ta.cmp(tb).then_with(|| {
            compare_subsec(
                a.subsec.as_deref().unwrap_or(""),
                b.subsec.as_deref().unwrap_or(""),
            )
        }),
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
    }
}

fn rating_rank(rating: Option<i8>) -> i8 {
    match rating {
        None | Some(0) => 6,
        Some(r) if r < 0 => 7,
        Some(r) => 6 - r,
    }
}

pub fn order_files(
    key: SortKey,
    paths: &[String],
    lookup: impl Fn(&str) -> SortFacts,
) -> Vec<String> {
    let mut rows: Vec<(&String, SortFacts)> = paths.iter().map(|p| (p, lookup(p))).collect();
    rows.sort_by(|(pa, a), (pb, b)| {
        let by_key = match key {
            SortKey::Name => Ordering::Equal,
            SortKey::Capture => compare_capture(a, b),
            SortKey::Rating => rating_rank(a.rating).cmp(&rating_rank(b.rating)),
        };
        by_key.then_with(|| natord::compare(base_name(pa), base_name(pb)))
    });
    rows.into_iter().map(|(path, _)| path.clone()).collect()
}

/// `sort.test.ts`, less its two `recommended` cases (port Step 9's).
#[cfg(test)]
mod tests {
    use super::{SortFacts, SortKey, order_files};
    use std::collections::HashMap;

    fn paths(ps: &[&str]) -> Vec<String> {
        ps.iter().map(|p| (*p).to_owned()).collect()
    }

    fn time(t: &str) -> SortFacts {
        SortFacts {
            capture_time: Some(t.to_owned()),
            ..SortFacts::default()
        }
    }

    fn timed(t: &str, subsec: &str) -> SortFacts {
        SortFacts {
            subsec: Some(subsec.to_owned()),
            ..time(t)
        }
    }

    fn rated(rating: i8) -> SortFacts {
        SortFacts {
            rating: Some(rating),
            ..SortFacts::default()
        }
    }

    fn order(key: SortKey, ps: &[&str], facts: &[(&str, SortFacts)]) -> Vec<String> {
        let facts: HashMap<&str, SortFacts> = facts.iter().cloned().collect();
        order_files(key, &paths(ps), |p| {
            facts.get(p).cloned().unwrap_or_default()
        })
    }

    #[test]
    fn capture_walks_a_counter_rollover_in_time_order() {
        let got = order(
            SortKey::Capture,
            &["/f/DSC00001.ARW", "/f/DSC09998.ARW", "/f/DSC09999.ARW"],
            &[
                ("/f/DSC09998.ARW", time("2026:09:19 10:00:00")),
                ("/f/DSC09999.ARW", time("2026:09:19 10:00:01")),
                ("/f/DSC00001.ARW", time("2026:09:19 10:00:02")),
            ],
        );
        assert_eq!(
            got,
            paths(&["/f/DSC09998.ARW", "/f/DSC09999.ARW", "/f/DSC00001.ARW"])
        );
    }

    #[test]
    fn capture_interleaves_two_bodies_by_time() {
        let got = order(
            SortKey::Capture,
            &["/f/A001.ARW", "/f/A002.ARW", "/f/L001.DNG", "/f/L002.DNG"],
            &[
                ("/f/A001.ARW", time("2026:09:19 10:00:00")),
                ("/f/A002.ARW", time("2026:09:19 10:00:02")),
                ("/f/L001.DNG", time("2026:09:19 10:00:01")),
                ("/f/L002.DNG", time("2026:09:19 10:00:03")),
            ],
        );
        assert_eq!(
            got,
            paths(&["/f/A001.ARW", "/f/L001.DNG", "/f/A002.ARW", "/f/L002.DNG"])
        );
    }

    #[test]
    fn capture_splits_an_equal_second_by_subsec_as_a_fraction() {
        let got = order(
            SortKey::Capture,
            &["/f/a.ARW", "/f/b.ARW", "/f/c.ARW"],
            &[
                ("/f/a.ARW", timed("2026:09:19 10:00:00", "5")),
                ("/f/b.ARW", timed("2026:09:19 10:00:00", "122")),
                ("/f/c.ARW", time("2026:09:19 10:00:00")),
            ],
        );
        assert_eq!(got, paths(&["/f/c.ARW", "/f/b.ARW", "/f/a.ARW"]));
    }

    #[test]
    fn capture_puts_files_without_a_time_last_in_name_order() {
        let got = order(
            SortKey::Capture,
            &["/f/a.ARW", "/f/b.ARW", "/f/c.ARW", "/f/d.ARW"],
            &[
                ("/f/c.ARW", time("2026:09:19 10:00:01")),
                ("/f/d.ARW", time("2026:09:19 10:00:00")),
            ],
        );
        assert_eq!(
            got,
            paths(&["/f/d.ARW", "/f/c.ARW", "/f/a.ARW", "/f/b.ARW"])
        );
    }

    #[test]
    fn rating_orders_stars_descending_unrated_then_rejected_ties_by_name() {
        let got = order(
            SortKey::Rating,
            &[
                "/f/a.ARW", "/f/b.ARW", "/f/c.ARW", "/f/d.ARW", "/f/e.ARW", "/f/f.ARW",
            ],
            &[
                ("/f/a.ARW", rated(-1)),
                ("/f/b.ARW", rated(1)),
                ("/f/d.ARW", rated(5)),
                ("/f/e.ARW", rated(3)),
                ("/f/f.ARW", rated(5)),
            ],
        );
        assert_eq!(
            got,
            paths(&[
                "/f/d.ARW", "/f/f.ARW", "/f/e.ARW", "/f/b.ARW", "/f/c.ARW", "/f/a.ARW",
            ])
        );
    }

    #[test]
    fn name_keeps_an_already_name_sorted_list_as_is() {
        let ps = [
            "/f/DSC00001.ARW",
            "/f/DSC09998.ARW",
            "/f/DSC09999.ARW",
            "/f/L1000001.DNG",
        ];
        assert_eq!(order(SortKey::Name, &ps, &[]), paths(&ps));
    }

    #[test]
    fn does_not_mutate_its_input() {
        let ps = paths(&["/f/b.ARW", "/f/a.ARW"]);
        let copy = ps.clone();
        assert_eq!(
            order_files(SortKey::Name, &ps, |_| SortFacts::default()),
            paths(&["/f/a.ARW", "/f/b.ARW"])
        );
        assert_eq!(ps, copy);
    }

    /// Adjusted to natord: the Tauri plain compare puts `DSC10` before `DSC9`.
    #[test]
    fn name_is_natord_not_the_plain_compare() {
        let got = order(
            SortKey::Name,
            &[r"\\?\D:\f\DSC10.ARW", r"\\?\D:\f\DSC9.ARW", "/g/DSC100.ARW"],
            &[],
        );
        assert_eq!(
            got,
            paths(&[r"\\?\D:\f\DSC9.ARW", r"\\?\D:\f\DSC10.ARW", "/g/DSC100.ARW"])
        );
    }

    #[test]
    fn the_stored_sort_order_follows_the_tauri_rule() {
        assert_eq!(SortKey::from_setting(Some("capture")), SortKey::Capture);
        assert_eq!(SortKey::from_setting(Some("rating")), SortKey::Rating);
        assert_eq!(SortKey::from_setting(Some("name")), SortKey::Name);
        assert_eq!(SortKey::from_setting(Some("recommended")), SortKey::Capture);
        assert_eq!(SortKey::from_setting(Some("size")), SortKey::Name);
        assert_eq!(SortKey::from_setting(None), SortKey::Name);
    }
}
