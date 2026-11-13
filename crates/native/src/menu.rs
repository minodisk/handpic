//! The strip bar's sort and filter menus, a port of `index.html`'s
//! `#sort-menu` and `#filter-menu`: their rows in the HTML's order and
//! labels, each item's checked state, what a click on it does, and which menu
//! is open. The `Face focus` `?` help is left out (the user's decision), the
//! `Recommended` sort and the `Genre` section are port Step 9's. The EXIF
//! sections come last, one per group the folder has (`exif::groups_of`), an
//! item keyed by its group and label rather than its row, as the groups can
//! be rebuilt between the frame that drew the menu and the click.

use crate::{
    exif::{GROUPS, Group, Groups},
    filter::{AfEye, EyesItem, FilterFlag, FilterState, Orientation, OutputState},
    mark::MarkState,
    sort::SortKey,
    strip::Tint,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    Sort,
    Filter,
}

/// One filter item's section and value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Toggle {
    Flag(FilterFlag),
    Stars(u8),
    /// The lower-cased color, or `none`.
    Label(&'static str),
    Orientation(Orientation),
    Candidate(AfEye),
    Eyes(EyesItem),
    Output(OutputState),
    Exif(Group, String),
}

/// What a click on an item does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Toggle(Toggle),
    Reset,
    Sort(SortKey),
}

/// What an item's text is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    Text(&'static str),
    /// A folder's EXIF label.
    Owned(String),
    /// `on` lit stars, then `5 - on` dim ones, as the HTML's `.on` / `.off`.
    Stars(u8),
}

/// The swatch before an item's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lead {
    None,
    /// The pick or reject dot, or the muted one of `Untagged`.
    Flag(Option<FilterFlag>),
    /// A label's dot; `None` is the hollow dot of `No label`.
    Label(Option<Tint>),
    /// A `Face focus` item's icon in the focus mark's color; `Neutral` keeps
    /// its slot empty.
    Face(Option<MarkState>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub label: Label,
    pub lead: Lead,
    pub checked: bool,
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Item(Item),
    Separator,
    Heading(&'static str),
}

const SORTS: [(SortKey, &str); 3] = [
    (SortKey::Capture, "Capture time"),
    (SortKey::Name, "File name"),
    (SortKey::Rating, "Rating"),
];

const LABELS: [(&str, &str); 8] = [
    ("red", "Red"),
    ("orange", "Orange"),
    ("yellow", "Yellow"),
    ("green", "Green"),
    ("blue", "Blue"),
    ("pink", "Pink"),
    ("purple", "Purple"),
    ("none", "No label"),
];

/// The sort menu's items, `in_force` checked.
pub fn sort_rows(in_force: SortKey) -> Vec<Row> {
    SORTS
        .iter()
        .map(|&(key, text)| {
            Row::Item(Item {
                label: Label::Text(text),
                lead: Lead::None,
                checked: key == in_force,
                action: Action::Sort(key),
            })
        })
        .collect()
}

/// The filter menu's rows, each item checked from `filter`; the `Output`
/// section only while the folder gave an export answer, then the folder's
/// EXIF `groups`.
pub fn filter_rows(filter: &FilterState, output_known: bool, groups: &Groups) -> Vec<Row> {
    let item = |label, lead, toggle: Toggle| {
        Row::Item(Item {
            label,
            lead,
            checked: is_checked(filter, &toggle),
            action: Action::Toggle(toggle),
        })
    };
    let mut rows = vec![
        Row::Item(Item {
            label: Label::Text("Reset"),
            lead: Lead::None,
            checked: false,
            action: Action::Reset,
        }),
        Row::Separator,
    ];
    for (flag, text) in [
        (FilterFlag::Picked, "Picked"),
        (FilterFlag::Untagged, "Untagged"),
        (FilterFlag::Rejected, "Rejected"),
    ] {
        let dot = (flag != FilterFlag::Untagged).then_some(flag);
        rows.push(item(Label::Text(text), Lead::Flag(dot), Toggle::Flag(flag)));
    }
    rows.push(Row::Separator);
    for stars in 0..=5 {
        rows.push(item(Label::Stars(stars), Lead::None, Toggle::Stars(stars)));
    }
    rows.push(Row::Separator);
    for (key, text) in LABELS {
        let tint = (key != "none").then(|| Tint::of(key));
        rows.push(item(
            Label::Text(text),
            Lead::Label(tint),
            Toggle::Label(key),
        ));
    }
    rows.push(Row::Separator);
    for (orientation, text) in [
        (Orientation::Portrait, "Portrait"),
        (Orientation::Landscape, "Landscape"),
    ] {
        rows.push(item(
            Label::Text(text),
            Lead::None,
            Toggle::Orientation(orientation),
        ));
    }
    rows.push(Row::Separator);
    rows.push(Row::Heading("Face focus"));
    for (state, text) in [
        (MarkState::Good, "Good"),
        (MarkState::Neutral, "Neutral"),
        (MarkState::Low, "Low"),
    ] {
        let icon = (state != MarkState::Neutral).then_some(state);
        rows.push(item(
            Label::Text(text),
            Lead::Face(icon),
            Toggle::Candidate(state),
        ));
    }
    rows.push(Row::Separator);
    rows.push(Row::Heading("Eyes"));
    for (eyes, text) in [
        (EyesItem::Open, "Open"),
        (EyesItem::Closed, "Closed"),
        (EyesItem::Unknown, "Unknown"),
    ] {
        rows.push(item(Label::Text(text), Lead::None, Toggle::Eyes(eyes)));
    }
    if output_known {
        rows.push(Row::Separator);
        rows.push(Row::Heading("Output"));
        for (output, text) in [(OutputState::Yes, "Output"), (OutputState::No, "No output")] {
            rows.push(item(Label::Text(text), Lead::None, Toggle::Output(output)));
        }
    }
    for group in GROUPS {
        let labels = groups.labels(group);
        if labels.is_empty() {
            continue;
        }
        rows.push(Row::Separator);
        rows.push(Row::Heading(group.heading()));
        for label in labels {
            rows.push(item(
                Label::Owned(label.clone()),
                Lead::None,
                Toggle::Exif(group, label.clone()),
            ));
        }
    }
    rows
}

fn is_checked(filter: &FilterState, toggle: &Toggle) -> bool {
    match toggle {
        Toggle::Flag(flag) => filter.flags.contains(flag),
        Toggle::Stars(stars) => filter.stars.contains(stars),
        Toggle::Label(label) => filter.labels.contains(*label),
        Toggle::Orientation(o) => filter.orientations.contains(o),
        Toggle::Candidate(state) => filter.candidates.contains(state),
        Toggle::Eyes(eyes) => filter.eyes.contains(eyes),
        Toggle::Output(output) => filter.outputs.contains(output),
        Toggle::Exif(group, label) => filter.exif.get(group).is_some_and(|s| s.contains(label)),
    }
}

fn flip<T: Eq + std::hash::Hash>(set: &mut std::collections::HashSet<T>, value: T) {
    if !set.remove(&value) {
        set.insert(value);
    }
}

/// Apply a click on an item: a toggle flips its item, `Reset` clears every
/// section, a sort pick sets the key. Nothing is written.
pub fn apply(filter: &mut FilterState, sort: &mut SortKey, action: Action) {
    match action {
        Action::Toggle(toggle) => match toggle {
            Toggle::Flag(flag) => flip(&mut filter.flags, flag),
            Toggle::Stars(stars) => flip(&mut filter.stars, stars),
            Toggle::Label(label) => flip(&mut filter.labels, label.to_owned()),
            Toggle::Orientation(o) => flip(&mut filter.orientations, o),
            Toggle::Candidate(state) => flip(&mut filter.candidates, state),
            Toggle::Eyes(eyes) => flip(&mut filter.eyes, eyes),
            Toggle::Output(output) => flip(&mut filter.outputs, output),
            Toggle::Exif(group, label) => filter.toggle_exif(group, &label),
        },
        Action::Reset => *filter = FilterState::default(),
        Action::Sort(key) => *sort = key,
    }
}

/// Which menu is open. A press outside the open menu closes it, as the
/// Tauri `mousedown` listener; the press that closed a menu from its own
/// button (outside the popover, so it closes first) does not open it again,
/// as the Tauri toggle sits inside the menu's `#sort` / `#filter` box.
#[derive(Debug, Default)]
pub struct Menus {
    open: Option<Which>,
    /// The menu the last press outside closed, and where that press was.
    dismissed: Option<(Which, (f32, f32))>,
}

impl Menus {
    pub fn open(&self) -> Option<Which> {
        self.open
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// A press on `which`'s button at `at`: open it, unless that same press
    /// just closed it.
    pub fn toggle(&mut self, which: Which, at: (f32, f32)) {
        let closed_by_this_press = self.dismissed.take() == Some((which, at));
        self.open = if closed_by_this_press || self.open == Some(which) {
            None
        } else {
            Some(which)
        };
    }

    /// A press at `at` outside the open menu's popover.
    pub fn dismiss(&mut self, at: (f32, f32)) {
        if let Some(which) = self.open.take() {
            self.dismissed = Some((which, at));
        }
    }

    /// Escape, or the sort menu going away in a view-only folder. Whether a
    /// menu was open.
    pub fn close(&mut self) -> bool {
        self.dismissed = None;
        self.open.take().is_some()
    }

    /// After an item's action: a sort pick closes the menu, a filter item
    /// keeps it open so several can be checked in one go.
    pub fn acted(&mut self, action: &Action) {
        if matches!(action, Action::Sort(_)) {
            self.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(rows: &[Row]) -> Vec<Item> {
        rows.iter()
            .filter_map(|row| match row {
                Row::Item(item) => Some(item.clone()),
                _ => None,
            })
            .collect()
    }

    fn texts(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Item(Item {
                    label: Label::Text(text),
                    ..
                }) => (*text).to_owned(),
                Row::Item(Item {
                    label: Label::Owned(text),
                    ..
                }) => text.clone(),
                Row::Item(Item {
                    label: Label::Stars(on),
                    ..
                }) => format!("stars {on}"),
                Row::Separator => "---".to_owned(),
                Row::Heading(text) => format!("# {text}"),
            })
            .collect()
    }

    fn groups() -> Groups {
        Groups(vec![
            (Group::Camera, vec!["ILCE-1".into(), "ILCE-7RM5".into()]),
            (Group::Iso, vec!["ISO 100".into()]),
            (Group::Focal, vec!["<24 mm".into(), ">200 mm".into()]),
        ])
    }

    fn checked(rows: &[Row]) -> Vec<Action> {
        items(rows)
            .into_iter()
            .filter(|i| i.checked)
            .map(|i| i.action)
            .collect()
    }

    #[test]
    fn the_sort_menu_lists_the_html_items_in_order() {
        let rows = sort_rows(SortKey::Name);
        assert_eq!(texts(&rows), ["Capture time", "File name", "Rating"]);
        assert_eq!(
            items(&rows)
                .into_iter()
                .map(|i| i.action)
                .collect::<Vec<_>>(),
            [
                Action::Sort(SortKey::Capture),
                Action::Sort(SortKey::Name),
                Action::Sort(SortKey::Rating)
            ]
        );
    }

    #[test]
    fn the_sort_in_force_is_the_one_checked() {
        for key in [SortKey::Name, SortKey::Capture, SortKey::Rating] {
            assert_eq!(checked(&sort_rows(key)), [Action::Sort(key)]);
        }
    }

    #[test]
    fn the_filter_menu_lists_the_html_rows_in_order() {
        let rows = filter_rows(&FilterState::default(), true, &Groups::default());
        assert_eq!(
            texts(&rows),
            [
                "Reset",
                "---",
                "Picked",
                "Untagged",
                "Rejected",
                "---",
                "stars 0",
                "stars 1",
                "stars 2",
                "stars 3",
                "stars 4",
                "stars 5",
                "---",
                "Red",
                "Orange",
                "Yellow",
                "Green",
                "Blue",
                "Pink",
                "Purple",
                "No label",
                "---",
                "Portrait",
                "Landscape",
                "---",
                "# Face focus",
                "Good",
                "Neutral",
                "Low",
                "---",
                "# Eyes",
                "Open",
                "Closed",
                "Unknown",
                "---",
                "# Output",
                "Output",
                "No output",
            ]
        );
    }

    #[test]
    fn the_output_section_is_hidden_without_an_answer() {
        let rows = filter_rows(&FilterState::default(), false, &Groups::default());
        assert_eq!(texts(&rows).last().map(String::as_str), Some("Unknown"));
        assert!(
            !items(&rows)
                .iter()
                .any(|i| matches!(i.action, Action::Toggle(Toggle::Output(_))))
        );
    }

    #[test]
    fn the_leads_follow_the_css() {
        let rows = filter_rows(&FilterState::default(), true, &Groups::default());
        let lead = |action| {
            items(&rows)
                .into_iter()
                .find(|i| i.action == action)
                .map(|i| i.lead)
                .unwrap()
        };
        let toggle = |t| Action::Toggle(t);
        assert_eq!(lead(Action::Reset), Lead::None);
        assert_eq!(
            lead(toggle(Toggle::Flag(FilterFlag::Picked))),
            Lead::Flag(Some(FilterFlag::Picked))
        );
        assert_eq!(
            lead(toggle(Toggle::Flag(FilterFlag::Untagged))),
            Lead::Flag(None)
        );
        assert_eq!(
            lead(toggle(Toggle::Flag(FilterFlag::Rejected))),
            Lead::Flag(Some(FilterFlag::Rejected))
        );
        assert_eq!(
            lead(toggle(Toggle::Label("red"))),
            Lead::Label(Some(Tint::Red))
        );
        assert_eq!(
            lead(toggle(Toggle::Label("purple"))),
            Lead::Label(Some(Tint::Purple))
        );
        assert_eq!(lead(toggle(Toggle::Label("none"))), Lead::Label(None));
        assert_eq!(
            lead(toggle(Toggle::Candidate(MarkState::Good))),
            Lead::Face(Some(MarkState::Good))
        );
        assert_eq!(
            lead(toggle(Toggle::Candidate(MarkState::Neutral))),
            Lead::Face(None)
        );
        assert_eq!(
            lead(toggle(Toggle::Candidate(MarkState::Low))),
            Lead::Face(Some(MarkState::Low))
        );
        assert_eq!(lead(toggle(Toggle::Stars(3))), Lead::None);
    }

    #[test]
    fn no_item_is_checked_on_an_empty_filter() {
        assert!(
            checked(&filter_rows(
                &FilterState::default(),
                true,
                &Groups::default()
            ))
            .is_empty()
        );
    }

    #[test]
    fn each_section_checks_its_items_from_the_state() {
        let mut filter = FilterState::default();
        filter.flags.insert(FilterFlag::Picked);
        filter.stars.insert(0);
        filter.stars.insert(4);
        filter.labels.insert("red".into());
        filter.labels.insert("none".into());
        filter.orientations.insert(Orientation::Portrait);
        filter.candidates.insert(MarkState::Low);
        filter.eyes.insert(EyesItem::Unknown);
        filter.outputs.insert(OutputState::No);
        assert_eq!(
            checked(&filter_rows(&filter, true, &Groups::default())),
            [
                Toggle::Flag(FilterFlag::Picked),
                Toggle::Stars(0),
                Toggle::Stars(4),
                Toggle::Label("red"),
                Toggle::Label("none"),
                Toggle::Orientation(Orientation::Portrait),
                Toggle::Candidate(MarkState::Low),
                Toggle::Eyes(EyesItem::Unknown),
                Toggle::Output(OutputState::No),
            ]
            .map(Action::Toggle)
        );
    }

    #[test]
    fn a_foreign_label_checks_no_item() {
        let mut filter = FilterState::default();
        filter.labels.insert("Approved".into());
        assert!(checked(&filter_rows(&filter, true, &Groups::default())).is_empty());
    }

    #[test]
    fn every_toggle_flips_its_item_and_leaves_the_sort() {
        for item in items(&filter_rows(&FilterState::default(), true, &groups())) {
            let Action::Toggle(toggle) = item.action.clone() else {
                continue;
            };
            let (mut filter, mut sort) = (FilterState::default(), SortKey::Rating);
            apply(&mut filter, &mut sort, item.action.clone());
            assert!(is_checked(&filter, &toggle), "{toggle:?} on");
            assert!(filter.is_active());
            apply(&mut filter, &mut sort, item.action);
            assert!(!is_checked(&filter, &toggle), "{toggle:?} off");
            assert_eq!(filter, FilterState::default());
            assert_eq!(sort, SortKey::Rating);
        }
    }

    #[test]
    fn a_label_toggle_keys_the_lower_cased_color() {
        let (mut filter, mut sort) = (FilterState::default(), SortKey::Name);
        apply(
            &mut filter,
            &mut sort,
            Action::Toggle(Toggle::Label("none")),
        );
        assert!(filter.labels.contains("none"));
    }

    #[test]
    fn reset_clears_every_section() {
        let mut filter = FilterState::default();
        filter.flags.insert(FilterFlag::Rejected);
        filter.stars.insert(5);
        filter.labels.insert("blue".into());
        filter.orientations.insert(Orientation::Landscape);
        filter.candidates.insert(MarkState::Good);
        filter.eyes.insert(EyesItem::Closed);
        filter.outputs.insert(OutputState::Yes);
        filter.toggle_exif(Group::Camera, "ILCE-1");
        let mut sort = SortKey::Capture;
        apply(&mut filter, &mut sort, Action::Reset);
        assert!(!filter.is_active());
        assert!(filter.exif.is_empty());
        assert_eq!(sort, SortKey::Capture);
    }

    #[test]
    fn a_sort_pick_sets_the_key_and_leaves_the_filter() {
        let mut filter = FilterState::default();
        filter.flags.insert(FilterFlag::Picked);
        let before = filter.clone();
        let mut sort = SortKey::Name;
        apply(&mut filter, &mut sort, Action::Sort(SortKey::Rating));
        assert_eq!(sort, SortKey::Rating);
        assert_eq!(filter, before);
    }

    #[test]
    fn nothing_is_open_at_first() {
        let menus = Menus::default();
        assert!(!menus.is_open());
        assert_eq!(menus.open(), None);
    }

    #[test]
    fn a_button_opens_and_closes_its_menu() {
        let mut menus = Menus::default();
        menus.toggle(Which::Filter, (10., 10.));
        assert_eq!(menus.open(), Some(Which::Filter));
        menus.toggle(Which::Filter, (11., 10.));
        assert_eq!(menus.open(), None);
        menus.toggle(Which::Filter, (11., 10.));
        assert_eq!(menus.open(), Some(Which::Filter));
    }

    #[test]
    fn the_press_on_the_button_that_closed_its_menu_does_not_reopen_it() {
        let mut menus = Menus::default();
        menus.toggle(Which::Filter, (10., 10.));
        // The popover's press-outside runs first, then the button's.
        menus.dismiss((12., 10.));
        menus.toggle(Which::Filter, (12., 10.));
        assert_eq!(menus.open(), None);
        // The next press on it opens it again.
        menus.toggle(Which::Filter, (12., 10.));
        assert_eq!(menus.open(), Some(Which::Filter));
    }

    #[test]
    fn a_non_left_dismiss_leaves_no_record_to_swallow_the_next_toggle() {
        let mut menus = Menus::default();
        menus.toggle(Which::Filter, (10., 10.));
        // A right press outside closes without recording.
        assert!(menus.close());
        menus.toggle(Which::Filter, (10., 10.));
        assert_eq!(menus.open(), Some(Which::Filter));
    }

    #[test]
    fn the_other_button_closes_one_menu_and_opens_its_own() {
        let mut menus = Menus::default();
        menus.toggle(Which::Sort, (10., 10.));
        menus.dismiss((40., 10.));
        menus.toggle(Which::Filter, (40., 10.));
        assert_eq!(menus.open(), Some(Which::Filter));
    }

    #[test]
    fn a_press_elsewhere_closes_and_a_later_press_on_the_button_opens() {
        let mut menus = Menus::default();
        menus.toggle(Which::Filter, (10., 10.));
        menus.dismiss((300., 200.));
        assert_eq!(menus.open(), None);
        menus.toggle(Which::Filter, (10., 10.));
        assert_eq!(menus.open(), Some(Which::Filter));
    }

    #[test]
    fn a_press_outside_with_nothing_open_does_nothing() {
        let mut menus = Menus::default();
        menus.dismiss((10., 10.));
        menus.toggle(Which::Sort, (10., 10.));
        assert_eq!(menus.open(), Some(Which::Sort));
    }

    #[test]
    fn close_says_whether_a_menu_was_open() {
        let mut menus = Menus::default();
        assert!(!menus.close());
        menus.toggle(Which::Sort, (0., 0.));
        assert!(menus.close());
        assert!(!menus.is_open());
    }

    #[test]
    fn a_sort_pick_closes_the_menu() {
        let mut menus = Menus::default();
        menus.toggle(Which::Sort, (0., 0.));
        menus.acted(&Action::Sort(SortKey::Capture));
        assert!(!menus.is_open());
    }

    #[test]
    fn a_filter_toggle_and_reset_keep_the_menu_open() {
        let mut menus = Menus::default();
        menus.toggle(Which::Filter, (0., 0.));
        menus.acted(&Action::Toggle(Toggle::Flag(FilterFlag::Picked)));
        assert_eq!(menus.open(), Some(Which::Filter));
        menus.acted(&Action::Reset);
        assert_eq!(menus.open(), Some(Which::Filter));
    }

    #[test]
    fn the_exif_sections_come_last_with_their_headings() {
        let rows = filter_rows(&FilterState::default(), true, &groups());
        let all = texts(&rows);
        let output = all.iter().position(|t| t == "No output").unwrap();
        assert_eq!(
            all[output + 1..],
            [
                "---",
                "# Camera",
                "ILCE-1",
                "ILCE-7RM5",
                "---",
                "# ISO",
                "ISO 100",
                "---",
                "# Focal length",
                "<24 mm",
                ">200 mm",
            ]
        );
        let without_output = texts(&filter_rows(&FilterState::default(), false, &groups()));
        let eyes = without_output.iter().position(|t| t == "Unknown").unwrap();
        assert_eq!(without_output[eyes + 1..], all[output + 1..]);
    }

    #[test]
    fn every_exif_heading_follows_the_tauri_names() {
        let groups = Groups(
            GROUPS
                .iter()
                .map(|g| (*g, vec![format!("{g:?}")]))
                .collect(),
        );
        let headings: Vec<String> = texts(&filter_rows(&FilterState::default(), true, &groups))
            .into_iter()
            .filter_map(|t| t.strip_prefix("# ").map(str::to_owned))
            .skip(3)
            .collect();
        assert_eq!(
            headings,
            [
                "Camera",
                "Lens",
                "Aperture",
                "Shutter speed",
                "ISO",
                "Focal length"
            ]
        );
    }

    #[test]
    fn an_exif_item_is_checked_from_its_group_and_label() {
        let mut filter = FilterState::default();
        filter.toggle_exif(Group::Camera, "ILCE-7RM5");
        filter.toggle_exif(Group::Iso, "ILCE-7RM5");
        assert_eq!(
            checked(&filter_rows(&filter, true, &groups())),
            [Action::Toggle(Toggle::Exif(
                Group::Camera,
                "ILCE-7RM5".into()
            ))]
        );
    }

    #[test]
    fn an_exif_toggle_flips_its_label_and_lights_the_filter() {
        let (mut filter, mut sort) = (FilterState::default(), SortKey::Name);
        let action = Action::Toggle(Toggle::Exif(Group::Focal, ">200 mm".into()));
        apply(&mut filter, &mut sort, action.clone());
        assert!(filter.is_active());
        assert!(filter.exif[&Group::Focal].contains(">200 mm"));
        apply(&mut filter, &mut sort, action);
        assert!(!filter.is_active());
        assert_eq!(filter, FilterState::default());
    }
}
