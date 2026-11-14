//! Burst grouping, a port of `burst.ts`: consecutive frames whose capture
//! times lie within a gap of each other. An impossible date (a month 00, a
//! day 32) is no capture time here, where `Date.UTC` rolls it over.

use std::collections::HashMap;

use crate::sort::{self, SortFacts, SortKey};

/// The largest gap between two frames of one burst. Leica files carry whole
/// seconds only, so anything below 1 s would split every Leica burst.
pub const BURST_GAP_MS: i64 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BurstMember {
    pub burst: usize,
    pub position: usize,
    pub size: usize,
}

/// What a strip cell needs to draw its part of a burst's band and its count
/// badge: whether the displayed cell before and after belong to the same
/// burst, and the file's position in the whole burst and the burst's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BurstMark {
    pub first: bool,
    pub last: bool,
    pub position: usize,
    pub size: usize,
}

fn digits(s: &str) -> Option<i64> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn capture_ms(facts: &SortFacts) -> Option<i64> {
    let time = facts.capture_time.as_deref()?;
    let (date, clock) = time.split_once(' ')?;
    let date: Vec<&str> = date.split(':').collect();
    let clock: Vec<&str> = clock.split(':').collect();
    let widths = [4, 2, 2, 2, 2, 2];
    let parts: Vec<&str> = date.into_iter().chain(clock).collect();
    if parts.len() != 6 || parts.iter().zip(widths).any(|(p, w)| p.len() != w) {
        return None;
    }
    let [year, month, day, hour, minute, second] = [0, 1, 2, 3, 4, 5].map(|at| digits(parts[at]));
    let (year, month, day) = (year?, month?, day?);
    let (hour, minute, second) = (hour?, minute?, second?);
    if !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    let subsec: String = facts
        .subsec
        .as_deref()
        .unwrap_or("")
        .chars()
        .take(3)
        .chain(std::iter::repeat('0'))
        .take(3)
        .collect();
    Some(seconds * 1000 + digits(&subsec).unwrap_or(0))
}

/// Every path's burst id (in capture order), its 0-based position within the
/// burst and the burst's size. A file without a usable capture time is a
/// burst of its own.
pub fn group_bursts(
    paths: &[String],
    lookup: impl Fn(&str) -> SortFacts,
    gap_ms: i64,
) -> HashMap<String, BurstMember> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut previous: Option<i64> = None;
    for path in sort::order_files(SortKey::Capture, paths, &lookup) {
        let ms = capture_ms(&lookup(&path));
        match (ms, previous, groups.last_mut()) {
            (Some(ms), Some(previous), Some(group)) if ms - previous <= gap_ms => {
                group.push(path);
            }
            _ => groups.push(vec![path]),
        }
        previous = ms;
    }
    let mut members = HashMap::new();
    for (burst, group) in groups.iter().enumerate() {
        for (position, path) in group.iter().enumerate() {
            members.insert(
                path.clone(),
                BurstMember {
                    burst,
                    position,
                    size: group.len(),
                },
            );
        }
    }
    members
}

/// Per displayed path, `None` unless it is in a burst of two or more.
pub fn burst_marks(
    files: &[String],
    members: &HashMap<String, BurstMember>,
) -> Vec<Option<BurstMark>> {
    let member_at = |at: Option<usize>| {
        at.and_then(|at| files.get(at))
            .and_then(|path| members.get(path))
            .filter(|member| member.size > 1)
    };
    (0..files.len())
        .map(|at| {
            let member = member_at(Some(at))?;
            Some(BurstMark {
                first: member_at(at.checked_sub(1)).map(|m| m.burst) != Some(member.burst),
                last: member_at(Some(at + 1)).map(|m| m.burst) != Some(member.burst),
                position: member.position,
                size: member.size,
            })
        })
        .collect()
}

/// A strip cell's count badge: the file's position in the whole burst and the
/// burst's size, empty outside a burst.
pub fn burst_badge(mark: Option<&BurstMark>) -> String {
    mark.map_or_else(String::new, |mark| {
        format!("{}/{}", mark.position + 1, mark.size)
    })
}

/// The index `burstNext` / `burstPrevious` move to over the displayed files'
/// burst ids (`direction` 1 or -1): the first displayed file of the next
/// burst, or the first of the current burst unless already there, else the
/// first of the previous one. Clamps at the ends.
pub fn burst_step(ids: &[i64], current: usize, direction: isize) -> usize {
    let start = |mut at: usize| {
        while at > 0 && ids[at - 1] == ids[at] {
            at -= 1;
        }
        at
    };
    if direction > 0 {
        let mut at = current;
        while at + 1 < ids.len() && ids[at + 1] == ids[current] {
            at += 1;
        }
        return if at + 1 < ids.len() { at + 1 } else { current };
    }
    let first = start(current);
    if first != current {
        return first;
    }
    if first > 0 { start(first - 1) } else { current }
}

/// The index `burstFrameNext` / `burstFramePrevious` move to over the
/// displayed files' burst ids: the neighboring displayed file when it is in
/// the same burst run, else `current`.
pub fn burst_frame_step(ids: &[i64], current: usize, direction: isize) -> usize {
    match current.checked_add_signed(direction) {
        Some(next) if next < ids.len() && ids[next] == ids[current] => next,
        _ => current,
    }
}

/// `burst.test.ts`.
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        BURST_GAP_MS, BurstMark, BurstMember, burst_badge, burst_frame_step, burst_marks,
        burst_step, group_bursts,
    };
    use crate::sort::SortFacts;

    fn facts(time: &str, subsec: Option<&str>) -> SortFacts {
        SortFacts {
            capture_time: Some(time.to_owned()),
            subsec: subsec.map(str::to_owned),
            rating: None,
        }
    }

    fn paths(ps: &[&str]) -> Vec<String> {
        ps.iter().map(|p| (*p).to_owned()).collect()
    }

    fn grouped(ps: &[&str], given: &[(&str, &str, Option<&str>)], gap_ms: i64) -> Vec<Vec<String>> {
        let given: HashMap<&str, SortFacts> = given
            .iter()
            .map(|(p, time, subsec)| (*p, facts(time, *subsec)))
            .collect();
        let members = group_bursts(
            &paths(ps),
            |p| given.get(p).cloned().unwrap_or_default(),
            gap_ms,
        );
        let mut groups: Vec<Vec<String>> = Vec::new();
        for (path, member) in &members {
            if groups.len() <= member.burst {
                groups.resize(member.burst + 1, Vec::new());
            }
            let group = &mut groups[member.burst];
            if group.len() <= member.position {
                group.resize(member.position + 1, String::new());
            }
            group[member.position] = path.clone();
        }
        groups
    }

    fn expect(groups: &[&[&str]]) -> Vec<Vec<String>> {
        groups.iter().map(|g| paths(g)).collect()
    }

    #[test]
    fn a_gap_exactly_at_the_threshold_stays_one_millisecond_over_splits() {
        let got = grouped(
            &["a", "b", "c"],
            &[
                ("a", "2026:09:19 10:00:00", Some("000")),
                ("b", "2026:09:19 10:00:01", Some("000")),
                ("c", "2026:09:19 10:00:02", Some("001")),
            ],
            1000,
        );
        assert_eq!(got, expect(&[&["a", "b"], &["c"]]));
    }

    #[test]
    fn a_run_across_second_minute_and_day_boundaries_stays_together() {
        let got = grouped(
            &["a", "b", "c", "d"],
            &[
                ("a", "2026:09:19 23:59:59", Some("5")),
                ("b", "2026:09:20 00:00:00", Some("1")),
                ("c", "2026:09:20 00:00:00", Some("9")),
                ("d", "2026:09:20 00:01:00", Some("0")),
            ],
            BURST_GAP_MS,
        );
        assert_eq!(got, expect(&[&["a", "b", "c"], &["d"]]));
    }

    #[test]
    fn subsec_orders_a_burst_whose_names_are_out_of_capture_order() {
        let got = grouped(
            &["x", "y", "z"],
            &[
                ("z", "2026:09:19 10:00:00", Some("5")),
                ("y", "2026:09:19 10:00:00", Some("122")),
                ("x", "2026:09:19 10:00:01", Some("4")),
            ],
            BURST_GAP_MS,
        );
        assert_eq!(got, expect(&[&["y", "z", "x"]]));
    }

    #[test]
    fn a_missing_subsec_is_0_ms_so_whole_seconds_stay_grouped() {
        let got = grouped(
            &["a", "b", "c", "d", "e"],
            &[
                ("a", "2026:09:19 10:00:00", None),
                ("b", "2026:09:19 10:00:01", None),
                ("c", "2026:09:19 10:00:02", None),
                ("d", "2026:09:19 10:00:02", Some("500")),
                ("e", "2026:09:19 10:00:04", None),
            ],
            BURST_GAP_MS,
        );
        assert_eq!(got, expect(&[&["a", "b", "c", "d"], &["e"]]));
    }

    #[test]
    fn files_without_a_capture_time_are_singletons_at_the_end() {
        let got = grouped(
            &["a", "b", "c", "d"],
            &[
                ("a", "2026:09:19 10:00:00", None),
                ("c", "2026:09:19 10:00:01", None),
                ("d", "garbage", None),
            ],
            BURST_GAP_MS,
        );
        assert_eq!(got, expect(&[&["a", "c"], &["d"], &["b"]]));
    }

    #[test]
    fn a_lone_file_is_a_burst_of_one() {
        let members = group_bursts(
            &paths(&["a"]),
            |_| facts("2026:09:19 10:00:00", None),
            BURST_GAP_MS,
        );
        assert_eq!(
            members.get("a"),
            Some(&BurstMember {
                burst: 0,
                position: 0,
                size: 1
            })
        );
    }

    /// Not in `burst.test.ts`: the deliberate difference from `Date.UTC`.
    #[test]
    fn an_impossible_date_is_no_capture_time() {
        let got = grouped(
            &["z1", "z2", "a", "b", "c", "d"],
            &[
                ("z1", "0000:00:00 00:00:00", None),
                ("z2", "0000:00:00 00:00:00", None),
                ("a", "2024:02:29 23:59:59", None),
                ("b", "2024:03:01 00:00:00", None),
                ("c", "2026:02:29 00:00:00", None),
                ("d", "2026:02:29 00:00:00", None),
            ],
            BURST_GAP_MS,
        );
        assert_eq!(
            got,
            expect(&[&["z1"], &["z2"], &["a", "b"], &["c"], &["d"]])
        );
    }

    fn member(burst: usize, position: usize, size: usize) -> BurstMember {
        BurstMember {
            burst,
            position,
            size,
        }
    }

    fn members(given: &[(&str, BurstMember)]) -> HashMap<String, BurstMember> {
        given.iter().map(|(p, m)| ((*p).to_owned(), *m)).collect()
    }

    fn mark(first: bool, last: bool, position: usize, size: usize) -> Option<BurstMark> {
        Some(BurstMark {
            first,
            last,
            position,
            size,
        })
    }

    fn marks_members() -> HashMap<String, BurstMember> {
        members(&[
            ("a", member(0, 0, 3)),
            ("b", member(0, 1, 3)),
            ("c", member(0, 2, 3)),
            ("d", member(1, 0, 1)),
            ("e", member(2, 0, 2)),
            ("f", member(2, 1, 2)),
        ])
    }

    #[test]
    fn bands_open_and_close_at_burst_edges_and_skip_singletons() {
        assert_eq!(
            burst_marks(&paths(&["a", "b", "c", "d", "e", "f"]), &marks_members()),
            [
                mark(true, false, 0, 3),
                mark(false, false, 1, 3),
                mark(false, true, 2, 3),
                None,
                mark(true, false, 0, 2),
                mark(false, true, 1, 2),
            ]
        );
    }

    #[test]
    fn members_separated_by_another_file_close_and_reopen_the_band() {
        assert_eq!(
            burst_marks(&paths(&["a", "e", "b"]), &marks_members()),
            [
                mark(true, true, 0, 3),
                mark(true, true, 0, 2),
                mark(true, true, 1, 3),
            ]
        );
    }

    fn badge_members() -> HashMap<String, BurstMember> {
        members(&[
            ("a", member(0, 0, 5)),
            ("b", member(0, 1, 5)),
            ("c", member(0, 2, 5)),
            ("d", member(0, 3, 5)),
            ("e", member(0, 4, 5)),
            ("f", member(1, 0, 1)),
        ])
    }

    fn badges(ps: &[&str]) -> Vec<String> {
        burst_marks(&paths(ps), &badge_members())
            .iter()
            .map(|m| burst_badge(m.as_ref()))
            .collect()
    }

    #[test]
    fn every_member_shows_its_position_and_the_bursts_size_the_first_included() {
        assert_eq!(
            badges(&["a", "b", "c", "d", "e"]),
            ["1/5", "2/5", "3/5", "4/5", "5/5"]
        );
    }

    #[test]
    fn a_split_or_partly_hidden_burst_keeps_its_whole_burst_numbering() {
        assert_eq!(badges(&["a", "c", "e"]), ["1/5", "3/5", "5/5"]);
        assert_eq!(badges(&["c", "f", "a"]), ["3/5", "", "1/5"]);
    }

    #[test]
    fn a_file_outside_a_burst_shows_nothing() {
        assert_eq!(burst_badge(None), "");
    }

    const STEP_IDS: [i64; 6] = [0, 0, 0, 1, 2, 2];

    #[test]
    fn next_moves_to_the_first_file_of_the_following_burst() {
        assert_eq!(burst_step(&STEP_IDS, 1, 1), 3);
        assert_eq!(burst_step(&STEP_IDS, 0, 1), 3);
    }

    #[test]
    fn a_singleton_is_its_own_burst() {
        assert_eq!(burst_step(&STEP_IDS, 3, 1), 4);
        assert_eq!(burst_step(&STEP_IDS, 3, -1), 0);
    }

    #[test]
    fn previous_goes_to_the_start_of_the_burst_then_to_the_previous_burst() {
        assert_eq!(burst_step(&STEP_IDS, 5, -1), 4);
        assert_eq!(burst_step(&STEP_IDS, 4, -1), 3);
        assert_eq!(burst_step(&STEP_IDS, 2, -1), 0);
    }

    #[test]
    fn clamps_at_either_end() {
        assert_eq!(burst_step(&STEP_IDS, 0, -1), 0);
        assert_eq!(burst_step(&STEP_IDS, 4, 1), 4);
        assert_eq!(burst_step(&STEP_IDS, 5, 1), 5);
        assert_eq!(burst_step(&[], 0, 1), 0);
    }

    #[test]
    fn works_over_non_adjacent_ids_left_by_a_filter() {
        let shown = [0, 3, 3, 7];
        assert_eq!(burst_step(&shown, 0, 1), 1);
        assert_eq!(burst_step(&shown, 1, 1), 3);
        assert_eq!(burst_step(&shown, 3, -1), 1);
        assert_eq!(burst_step(&shown, 2, -1), 1);
    }

    const FRAME_IDS: [i64; 5] = [0, 1, 1, 1, 2];

    #[test]
    fn steps_down_and_up_inside_a_burst() {
        assert_eq!(burst_frame_step(&FRAME_IDS, 1, 1), 2);
        assert_eq!(burst_frame_step(&FRAME_IDS, 2, 1), 3);
        assert_eq!(burst_frame_step(&FRAME_IDS, 3, -1), 2);
        assert_eq!(burst_frame_step(&FRAME_IDS, 2, -1), 1);
    }

    #[test]
    fn clamps_at_the_first_and_last_displayed_member() {
        assert_eq!(burst_frame_step(&FRAME_IDS, 1, -1), 1);
        assert_eq!(burst_frame_step(&FRAME_IDS, 3, 1), 3);
    }

    #[test]
    fn a_singleton_and_an_empty_list_are_no_ops() {
        assert_eq!(burst_frame_step(&FRAME_IDS, 0, 1), 0);
        assert_eq!(burst_frame_step(&FRAME_IDS, 0, -1), 0);
        assert_eq!(burst_frame_step(&FRAME_IDS, 4, 1), 4);
        assert_eq!(burst_frame_step(&FRAME_IDS, 4, -1), 4);
        assert_eq!(burst_frame_step(&[], 0, 1), 0);
        assert_eq!(burst_frame_step(&[], 0, -1), 0);
    }

    #[test]
    fn steps_only_within_a_run_of_equal_ids_left_by_a_filter_or_a_sort() {
        let shown = [0, 3, 3, 7];
        assert_eq!(burst_frame_step(&shown, 0, 1), 0);
        assert_eq!(burst_frame_step(&shown, 1, 1), 2);
        assert_eq!(burst_frame_step(&shown, 2, 1), 2);
        assert_eq!(burst_frame_step(&shown, 1, -1), 1);
        let split = [0, 1, 0];
        assert_eq!(burst_frame_step(&split, 0, 1), 0);
        assert_eq!(burst_frame_step(&split, 2, -1), 2);
    }
}
