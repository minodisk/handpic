//! The answering side of the MCP companion, a port of `companion.ts` over a
//! snapshot of the view, extended with the native app's own tools: the
//! moves, Select All, Reject Rest, undo and redo, each the `Key` its keymap
//! action runs, and the sort, the filter, the panes and the overlays, each
//! set to a given state rather than toggled. The selection is the strip's
//! (`selection::targets`). The native view has no compare and no ranking, so
//! compare and the `recommended` sort are refused, and so is every request
//! but `get_view` while the view drops keys (`Busy`).

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    hash::Hash,
    path::PathBuf,
    sync::Arc,
};

use riffle_backend::{
    index::{flag_name, parse_flag},
    ondemand::EyesJudgment,
};
use riffle_core::Flag;
use serde_json::{Map, Value, json};

use crate::{
    burst::BurstMember,
    exif::{GROUPS, Group, Groups},
    filter::{self, Facts, FilterState, OutputState},
    history::Top,
    judgment::State,
    menu::{self, Toggle},
    panels::Panels,
    selection::{self, Selection},
    sort::SortKey,
    viewonly,
};

const COMPARE_REFUSAL: &str = "compare is not supported by Handpic's native app yet";
const RECOMMENDED_REFUSAL: &str =
    "the recommended sort is not implemented in Handpic's native app yet";
const NOTHING_SHOWN: &str = "no photo is shown in Handpic";
/// `viewonly.ts`'s `VIEW_ONLY_REFUSAL`.
const VIEW_ONLY_REFUSAL: &str = "culling does not apply to a JPEG folder";
const FLAGS: [&str; 3] = ["none", "pick", "reject"];
/// The color labels the judgment keys set, as `State.label` holds them.
const LABELS: [&str; 7] = ["Red", "Orange", "Yellow", "Green", "Blue", "Pink", "Purple"];
/// The `Output` items' names.
const OUTPUTS: [(OutputState, &str); 2] =
    [(OutputState::Yes, "output"), (OutputState::No, "no_output")];
/// The EXIF sections' names.
const GROUP_NAMES: [(Group, &str); 6] = [
    (Group::Camera, "camera"),
    (Group::Lens, "lens"),
    (Group::Aperture, "aperture"),
    (Group::Shutter, "shutter"),
    (Group::Iso, "iso"),
    (Group::Focal, "focal"),
];

/// What a request reads of the view.
pub struct View<'a> {
    /// The open folder, as the index keys it.
    pub folder: Option<&'a str>,
    /// The files that pass the filter, in strip order.
    pub files: &'a [PathBuf],
    pub index: usize,
    pub zoomed: bool,
    /// The user's sort key, which a view-only folder sets aside.
    pub sort: SortKey,
    pub filter: &'a FilterState,
    /// The open folder gave an export answer, so the `Output` section applies.
    pub output_known: bool,
    /// The open folder's EXIF labels per group, the menu's EXIF sections.
    pub groups: &'a Groups,
    pub panes: Panels,
    pub overlays: Overlays,
    /// The current file's eyes: `None` not judged yet, `Some(None)` judged
    /// with the eyes unknown.
    pub eyes: Option<Option<Arc<EyesJudgment>>>,
    /// The open folder holds only JPEGs.
    pub view_only: bool,
    pub judgments: &'a HashMap<String, State>,
    /// The paths whose label the index has told the view.
    pub seen: &'a HashSet<String>,
    /// The open folder's bursts over every listed file.
    pub bursts: &'a HashMap<String, BurstMember>,
    /// The listed files' facts, for each burst frame's sharpness.
    pub facts: &'a HashMap<String, Facts>,
    pub selection: &'a Selection,
    /// Why the view drops keys now, if it does.
    pub busy: Option<Busy>,
    /// What an undo and a redo would step next.
    pub undo: Top,
    pub redo: Top,
    /// An undo or a redo of a run to the Trash is out.
    pub trash_stepping: bool,
}

/// Which of the `f`, `e` and `d` overlays are on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Overlays {
    pub focus: bool,
    pub peaking: bool,
    pub performance: bool,
}

/// The states in which `key_down` drops a key, which refuse every request
/// but `get_view`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Busy {
    Settings,
    Trash,
    FolderDialog,
    Rename,
}

impl Busy {
    fn refusal(self) -> &'static str {
        match self {
            Busy::Settings => "Handpic's settings are open; try again once the user closes them",
            Busy::Trash => {
                "a Move to Trash or Delete Permanently is under way in Handpic; try again once it ends"
            }
            Busy::FolderDialog => {
                "a folder operation's dialog is open in Handpic; try again once the user closes it"
            }
            Busy::Rename => {
                "a folder name is being edited in Handpic; try again once the user is done"
            }
        }
    }
}

/// What a keymap action and its tool both run, through the view's `perform`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Previous,
    Next,
    First,
    Last,
    BurstPrevious,
    BurstNext,
    BurstFramePrevious,
    BurstFrameNext,
    SelectAll,
    RejectRest,
    Undo,
    Redo,
}

impl Key {
    /// The keymap action's; `First` and `Last` are Home and End, which no
    /// action names.
    pub fn of_action(action: &str) -> Option<Self> {
        Some(match action {
            "previous" => Key::Previous,
            "next" => Key::Next,
            "burstPrevious" => Key::BurstPrevious,
            "burstNext" => Key::BurstNext,
            "burstFramePrevious" => Key::BurstFramePrevious,
            "burstFrameNext" => Key::BurstFrameNext,
            "selectAll" => Key::SelectAll,
            "rejectRest" => Key::RejectRest,
            "undo" => Key::Undo,
            "redo" => Key::Redo,
            _ => return None,
        })
    }
}

/// What the view does before it answers with the new `get_view`.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Make the file at this index of `files` current.
    Show(usize),
    /// Make the file at `at` of `files` current and select `paths`, the
    /// first one the anchor.
    Select {
        at: usize,
        paths: Vec<String>,
    },
    ToggleZoom,
    /// Apply these sort or filter menu clicks in order, as the menu does.
    Menu(Vec<menu::Action>),
    /// Show these panes, as their keys do.
    Panes(Panels),
    /// Flip the overlays marked true, as their keys do.
    Overlays(Overlays),
    /// Record `changes` as one undo entry, as a judgment key does, and reply
    /// with `judged` instead of `get_view`.
    Judge {
        changes: Vec<Change>,
        judged: Value,
    },
    Key(Key),
}

/// One file's judgment `set_judgment` changes.
#[derive(Debug, PartialEq)]
pub struct Change {
    pub path: String,
    pub before: State,
    pub after: State,
    pub label_known: bool,
}

/// The answer to `kind` over `view`: what the view has to do, if anything,
/// before it replies with `get_view`, or the refusal's message.
pub fn answer(kind: &str, args: &Value, view: &View) -> Result<Option<Action>, String> {
    if kind != "get_view"
        && let Some(busy) = view.busy
    {
        return Err(busy.refusal().into());
    }
    match kind {
        "get_view" => Ok(None),
        "show_photo" => visible(view, &args["path"]).map(|at| Some(Action::Show(at))),
        "select_photos" => select_photos(view, &args["paths"]).map(Some),
        "set_view" => set_view(view, &args["mode"]),
        "set_judgment" => set_judgment(view, args).map(Some),
        "navigate" => navigate(view, &args["to"]).map(|key| Some(Action::Key(key))),
        "select_all" => shown(view).map(|_| Some(Action::Key(Key::SelectAll))),
        "reject_rest" => reject_rest(view).map(|_| Some(Action::Key(Key::RejectRest))),
        "undo" => step(view, false).map(|key| Some(Action::Key(key))),
        "redo" => step(view, true).map(|key| Some(Action::Key(key))),
        "set_sort" => set_sort(view, &args["by"]),
        "set_filter" => set_filter(view, args),
        "set_panes" => set_panes(view, args),
        "set_overlays" => set_overlays(view, args),
        _ => Err(format!("unknown request: {kind}")),
    }
}

/// The selection's targets, the current file alone when one or none is
/// selected.
fn targets(view: &View) -> Vec<String> {
    let files: Vec<String> = view
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    selection::targets(view.selection, &files, view.index)
}

fn shown(view: &View) -> Result<(), String> {
    if view.files.is_empty() {
        return Err(NOTHING_SHOWN.into());
    }
    Ok(())
}

fn navigate(view: &View, to: &Value) -> Result<Key, String> {
    let key = match to.as_str() {
        Some("previous") => Key::Previous,
        Some("next") => Key::Next,
        Some("first") => Key::First,
        Some("last") => Key::Last,
        Some("burst_previous") => Key::BurstPrevious,
        Some("burst_next") => Key::BurstNext,
        Some("burst_frame_previous") => Key::BurstFramePrevious,
        Some("burst_frame_next") => Key::BurstFrameNext,
        _ => {
            return Err(
                "to must be one of previous, next, first, last, burst_previous, \
                burst_next, burst_frame_previous, burst_frame_next"
                    .into(),
            );
        }
    };
    shown(view)?;
    Ok(key)
}

/// `rejectRest` is the current file's, which must be in a burst of two or
/// more.
fn reject_rest(view: &View) -> Result<(), String> {
    if view.view_only {
        return Err(VIEW_ONLY_REFUSAL.into());
    }
    let current = view.files.get(view.index).ok_or(NOTHING_SHOWN)?;
    if view
        .bursts
        .get(current.to_string_lossy().as_ref())
        .is_none_or(|member| member.size < 2)
    {
        return Err("the current photo is not part of a burst".into());
    }
    Ok(())
}

/// The view's `step` rules, in its order: a run to the Trash first, then a
/// view-only folder, then an empty history.
fn step(view: &View, redo: bool) -> Result<Key, String> {
    let (verb, top) = if redo {
        ("redo", view.redo)
    } else {
        ("undo", view.undo)
    };
    if view.trash_stepping {
        return Err(
            "an undo or redo of a move to the Trash is under way in Handpic; try again once it ends"
                .into(),
        );
    }
    match top {
        Top::Trash => Err(format!(
            "the next {verb} is of a move to the Trash, which is not available over MCP yet"
        )),
        _ if view.view_only => Err(VIEW_ONLY_REFUSAL.into()),
        Top::Empty => Err(format!("nothing to {verb}")),
        Top::Judgments => Ok(if redo { Key::Redo } else { Key::Undo }),
    }
}

/// `companion.ts`'s `burstOf`: the current file's burst in capture order,
/// positions 1-based; empty when the file is a burst of its own. A frame the
/// filter hides is listed with `visible: false`.
fn burst_of(view: &View, path: &str) -> Vec<Value> {
    let Some(member) = view.bursts.get(path).filter(|m| m.size > 1) else {
        return Vec::new();
    };
    let mut frames: Vec<(&String, &BurstMember)> = view
        .bursts
        .iter()
        .filter(|(_, other)| other.burst == member.burst)
        .collect();
    frames.sort_by_key(|(_, other)| other.position);
    frames
        .into_iter()
        .map(|(other, frame)| {
            let state = view.judgments.get(other).cloned().unwrap_or_default();
            json!({
                "path": other,
                "position": frame.position + 1,
                "sharpness": view.facts.get(other).and_then(|f| f.sharpness),
                "rating": state.rating,
                "flag": flag_name(state.flag),
                "label": state.label,
                "visible": view.files.iter().any(|file| file.to_string_lossy() == other.as_str()),
            })
        })
        .collect()
}

pub fn get_view(view: &View) -> Value {
    let current = view.files.get(view.index).map(|path| {
        let mut current = Map::new();
        current.insert("path".into(), json!(path.to_string_lossy()));
        current.insert("position".into(), json!(view.index + 1));
        if let Some(eyes) = &view.eyes {
            current.insert(
                "eyes".into(),
                eyes.as_ref().map_or(Value::Null, |eyes| {
                    json!({
                        "state": eyes.state,
                        "probability": eyes.probability,
                        "pose": eyes.pose,
                    })
                }),
            );
        }
        (path.to_string_lossy().into_owned(), Value::Object(current))
    });
    let (current, burst) = match current {
        Some((path, current)) => {
            let burst = burst_of(view, &path);
            (current, burst)
        }
        None => (Value::Null, Vec::new()),
    };
    json!({
        "folder": view.folder,
        "count": view.files.len(),
        "current": current,
        "selected": targets(view),
        "mode": if view.zoomed { "zoom" } else { "normal" },
        "compare_active": null,
        "burst": burst,
        "sort": viewonly::sort_for(view.view_only, view.sort).setting(),
        "filtered": view.filter.is_active(),
        "filter": filter_of(view.filter),
        "filter_offers": {
            "output": view.output_known,
            "exif": exif_of(GROUPS.iter().map(|&g| (g, view.groups.labels(g)))),
        },
        "panes": { "left": view.panes.left, "right": view.panes.right, "strip": view.panes.strip },
        "overlays": {
            "focus": view.overlays.focus,
            "peaking": view.overlays.peaking,
            "performance": view.overlays.performance,
        },
    })
}

fn named<T: Copy + Eq + Hash>(items: &[(T, &'static str)], set: &HashSet<T>) -> Vec<&'static str> {
    items
        .iter()
        .filter(|(item, _)| set.contains(item))
        .map(|(_, name)| *name)
        .collect()
}

fn group_name(group: Group) -> &'static str {
    GROUP_NAMES
        .iter()
        .find(|(g, _)| *g == group)
        .map_or("", |(_, name)| *name)
}

/// Each group's labels by the group's name, a group without any left out.
fn exif_of<'s>(groups: impl Iterator<Item = (Group, &'s [String])>) -> Value {
    Value::Object(
        groups
            .filter(|(_, labels)| !labels.is_empty())
            .map(|(group, labels)| (group_name(group).to_owned(), json!(labels)))
            .collect(),
    )
}

/// The filter's checked items per section, as `set_filter` names them.
fn filter_of(filter: &FilterState) -> Value {
    let exif: Vec<(Group, Vec<String>)> = filter
        .exif
        .iter()
        .map(|(group, labels)| (*group, labels.iter().cloned().collect()))
        .collect();
    json!({
        "flags": named(&filter::FLAGS, &filter.flags),
        "stars": (0..=5).filter(|s| filter.stars.contains(s)).collect::<Vec<u8>>(),
        "labels": filter::LABELS
            .iter()
            .filter(|l| filter.labels.contains(**l))
            .collect::<Vec<_>>(),
        "orientations": named(&filter::ORIENTATIONS, &filter.orientations),
        "face_focus": named(&filter::CANDIDATES, &filter.candidates),
        "eyes": named(&filter::EYES, &filter.eyes),
        "output": named(&OUTPUTS, &filter.outputs),
        "exif": exif_of(exif.iter().map(|(g, labels)| (*g, labels.as_slice()))),
    })
}

/// The index in `files` of `path`, checked to be one the strip shows.
fn visible(view: &View, path: &Value) -> Result<usize, String> {
    let path = path.as_str().ok_or("path must be a string")?;
    view.files
        .iter()
        .position(|file| file.to_string_lossy() == path)
        .ok_or_else(|| {
            format!(
                "{path} is not shown in Handpic: not in the open folder, or hidden by the filter"
            )
        })
}

fn select_photos(view: &View, paths: &Value) -> Result<Action, String> {
    let paths = paths
        .as_array()
        .ok_or("paths must be an array of strings")?;
    let mut picked = Vec::new();
    for path in paths {
        let at = visible(view, path)?;
        if !picked.contains(&at) {
            picked.push(at);
        }
    }
    let at = *picked.first().ok_or("paths must name at least one photo")?;
    let paths = picked
        .iter()
        .map(|&at| view.files[at].to_string_lossy().into_owned())
        .collect();
    Ok(Action::Select { at, paths })
}

fn set_view(view: &View, mode: &Value) -> Result<Option<Action>, String> {
    let mode = mode
        .as_str()
        .filter(|mode| ["normal", "zoom", "compare"].contains(mode))
        .ok_or("mode must be one of normal, zoom, compare")?;
    if mode != "normal" && view.files.is_empty() {
        return Err(NOTHING_SHOWN.into());
    }
    if mode == "compare" {
        return Err(COMPARE_REFUSAL.into());
    }
    Ok(((mode == "zoom") != view.zoomed).then_some(Action::ToggleZoom))
}

/// The sort menu's pick; a JPEG-only folder, whose menu is disabled, keeps
/// capture order.
fn set_sort(view: &View, by: &Value) -> Result<Option<Action>, String> {
    let key = match by.as_str() {
        Some("name") => SortKey::Name,
        Some("capture") => SortKey::Capture,
        Some("rating") => SortKey::Rating,
        Some("recommended") => return Err(RECOMMENDED_REFUSAL.into()),
        _ => return Err("by must be one of name, capture, rating".into()),
    };
    if view.view_only {
        return Err("a JPEG folder stays in capture order; the sort does not apply to it".into());
    }
    Ok((key != view.sort).then(|| Action::Menu(vec![menu::Action::Sort(key)])))
}

/// A section's items given by name, each one of `items`.
fn items<T: Copy + Eq + Hash>(
    value: &Value,
    items: &[(T, &str)],
    field: &str,
) -> Result<HashSet<T>, String> {
    let refusal = || {
        let names: Vec<&str> = items.iter().map(|(_, name)| *name).collect();
        format!("{field} must be an array of {}", names.join(", "))
    };
    value
        .as_array()
        .ok_or_else(refusal)?
        .iter()
        .map(|item| {
            let name = item.as_str().ok_or_else(refusal)?;
            items
                .iter()
                .find(|(_, n)| *n == name)
                .map(|(item, _)| *item)
                .ok_or_else(refusal)
        })
        .collect()
}

/// A field `args` gives, `null` counting as left out.
fn given_field<'v>(args: &'v Value, name: &str) -> Option<&'v Value> {
    args.get(name).filter(|v| !v.is_null())
}

fn exif_section(
    view: &View,
    name: &str,
    labels: &Value,
) -> Result<(Group, BTreeSet<String>), String> {
    let group = GROUP_NAMES
        .iter()
        .find(|(_, n)| *n == name)
        .map(|(group, _)| *group)
        .ok_or_else(|| {
            let names: Vec<&str> = GROUP_NAMES.iter().map(|(_, n)| *n).collect();
            format!("exif sections must be among {}", names.join(", "))
        })?;
    let offered = view.groups.labels(group);
    let refusal =
        || format!("exif.{name} must be an array of the labels get_view's filter_offers lists");
    let checked = labels
        .as_array()
        .ok_or_else(refusal)?
        .iter()
        .map(|label| {
            label
                .as_str()
                .filter(|l| offered.iter().any(|o| o == l))
                .map(str::to_owned)
                .ok_or_else(refusal)
        })
        .collect::<Result<_, _>>()?;
    Ok((group, checked))
}

/// The filter `args` asks for over the one in force: `reset` unchecks every
/// item first, a section given replaces its checks, one left out keeps them.
fn wanted_filter(view: &View, args: &Value) -> Result<FilterState, String> {
    let reset = switch(args, "reset")?.unwrap_or(false);
    let mut wanted = if reset {
        FilterState::default()
    } else {
        view.filter.clone()
    };
    let mut given = reset;
    if let Some(v) = given_field(args, "flags") {
        wanted.flags = items(v, &filter::FLAGS, "flags")?;
        given = true;
    }
    if let Some(v) = given_field(args, "stars") {
        let refusal = "stars must be an array of integers from 0 to 5";
        wanted.stars = v
            .as_array()
            .ok_or(refusal)?
            .iter()
            .map(|n| {
                n.as_u64()
                    .filter(|n| *n <= 5)
                    .map(|n| n as u8)
                    .ok_or(refusal)
            })
            .collect::<Result<_, _>>()?;
        given = true;
    }
    if let Some(v) = given_field(args, "labels") {
        let labels: Vec<(&str, &str)> = filter::LABELS.iter().map(|l| (*l, *l)).collect();
        wanted.labels = items(v, &labels, "labels")?
            .into_iter()
            .map(str::to_owned)
            .collect();
        given = true;
    }
    if let Some(v) = given_field(args, "orientations") {
        wanted.orientations = items(v, &filter::ORIENTATIONS, "orientations")?;
        given = true;
    }
    if let Some(v) = given_field(args, "face_focus") {
        wanted.candidates = items(v, &filter::CANDIDATES, "face_focus")?;
        given = true;
    }
    if let Some(v) = given_field(args, "eyes") {
        wanted.eyes = items(v, &filter::EYES, "eyes")?;
        given = true;
    }
    if let Some(v) = given_field(args, "output") {
        let outputs = items(v, &OUTPUTS, "output")?;
        if !outputs.is_empty() && !view.output_known {
            return Err(
                "the open folder has no Output folder, so the output section does not apply".into(),
            );
        }
        wanted.outputs = outputs;
        given = true;
    }
    if let Some(v) = given_field(args, "exif") {
        let sections = v.as_object().ok_or("exif must be an object of sections")?;
        for (name, labels) in sections {
            let (group, checked) = exif_section(view, name, labels)?;
            if checked.is_empty() {
                wanted.exif.remove(&group);
            } else {
                wanted.exif.insert(group, checked);
            }
        }
        given = true;
    }
    if !given {
        return Err(
            "set reset or at least one of flags, stars, labels, orientations, \
            face_focus, eyes, output, exif"
                .into(),
        );
    }
    Ok(wanted)
}

/// The item clicks that turn the checks of `from` into those of `to`.
fn toggles(from: &FilterState, to: &FilterState) -> Vec<menu::Action> {
    fn differ<T: Copy + Eq + Hash>(
        items: impl IntoIterator<Item = T>,
        from: &HashSet<T>,
        to: &HashSet<T>,
        toggle: impl Fn(T) -> Toggle,
        out: &mut Vec<menu::Action>,
    ) {
        for item in items {
            if from.contains(&item) != to.contains(&item) {
                out.push(menu::Action::Toggle(toggle(item)));
            }
        }
    }
    let mut out = Vec::new();
    differ(
        filter::FLAGS.map(|(f, _)| f),
        &from.flags,
        &to.flags,
        Toggle::Flag,
        &mut out,
    );
    differ(0..=5, &from.stars, &to.stars, Toggle::Stars, &mut out);
    for label in filter::LABELS {
        if from.labels.contains(*label) != to.labels.contains(*label) {
            out.push(menu::Action::Toggle(Toggle::Label(label)));
        }
    }
    differ(
        filter::ORIENTATIONS.map(|(o, _)| o),
        &from.orientations,
        &to.orientations,
        Toggle::Orientation,
        &mut out,
    );
    differ(
        filter::CANDIDATES.map(|(c, _)| c),
        &from.candidates,
        &to.candidates,
        Toggle::Candidate,
        &mut out,
    );
    differ(
        filter::EYES.map(|(e, _)| e),
        &from.eyes,
        &to.eyes,
        Toggle::Eyes,
        &mut out,
    );
    differ(
        OUTPUTS.map(|(o, _)| o),
        &from.outputs,
        &to.outputs,
        Toggle::Output,
        &mut out,
    );
    let none = BTreeSet::new();
    for group in GROUPS {
        let was = from.exif.get(&group).unwrap_or(&none);
        let will = to.exif.get(&group).unwrap_or(&none);
        for label in was.symmetric_difference(will) {
            out.push(menu::Action::Toggle(Toggle::Exif(group, label.clone())));
        }
    }
    out
}

/// The menu clicks that check what `args` asks for, the menu's `Reset`
/// first when it asks for one.
fn set_filter(view: &View, args: &Value) -> Result<Option<Action>, String> {
    let wanted = wanted_filter(view, args)?;
    if wanted == *view.filter {
        return Ok(None);
    }
    let actions = if switch(args, "reset")? == Some(true) {
        let mut actions = vec![menu::Action::Reset];
        actions.extend(toggles(&FilterState::default(), &wanted));
        actions
    } else {
        toggles(view.filter, &wanted)
    };
    Ok(Some(Action::Menu(actions)))
}

/// A switch `args` gives, `None` when left out.
fn switch(args: &Value, name: &str) -> Result<Option<bool>, String> {
    given_field(args, name)
        .map(|on| {
            on.as_bool()
                .ok_or_else(|| format!("{name} must be true or false"))
        })
        .transpose()
}

fn set_panes(view: &View, args: &Value) -> Result<Option<Action>, String> {
    let (left, right, strip) = (
        switch(args, "left")?,
        switch(args, "right")?,
        switch(args, "strip")?,
    );
    if left.is_none() && right.is_none() && strip.is_none() {
        return Err("set at least one of left, right, strip".into());
    }
    let panes = Panels {
        left: left.unwrap_or(view.panes.left),
        right: right.unwrap_or(view.panes.right),
        strip: strip.unwrap_or(view.panes.strip),
    };
    Ok((panes != view.panes).then_some(Action::Panes(panes)))
}

/// The overlays to flip, those given that differ from the ones on.
fn set_overlays(view: &View, args: &Value) -> Result<Option<Action>, String> {
    let (focus, peaking, performance) = (
        switch(args, "focus")?,
        switch(args, "peaking")?,
        switch(args, "performance")?,
    );
    if focus.is_none() && peaking.is_none() && performance.is_none() {
        return Err("set at least one of focus, peaking, performance".into());
    }
    let on = view.overlays;
    let flips = Overlays {
        focus: focus.is_some_and(|f| f != on.focus),
        peaking: peaking.is_some_and(|p| p != on.peaking),
        performance: performance.is_some_and(|p| p != on.performance),
    };
    Ok((flips != Overlays::default()).then_some(Action::Overlays(flips)))
}

/// The fields `set_judgment` gives, `None` where it keeps each photo's own.
struct Given {
    /// `Some(None)` clears the stars.
    rating: Option<Option<u8>>,
    flag: Option<Flag>,
    /// `Some(None)` clears the label.
    label: Option<Option<String>>,
}

impl Given {
    fn over(&self, own: &State) -> State {
        State {
            rating: self.rating.unwrap_or(own.rating),
            flag: self.flag.unwrap_or(own.flag),
            label: self.label.clone().unwrap_or_else(|| own.label.clone()),
        }
    }
}

fn given(args: &Value) -> Result<Given, String> {
    let rating = match args.get("rating") {
        None => None,
        Some(rating) => Some(
            rating
                .as_u64()
                .filter(|r| *r <= 5)
                .map(|r| (r > 0).then_some(r as u8))
                .ok_or("rating must be an integer from 0 to 5")?,
        ),
    };
    let flag = match args.get("flag") {
        None => None,
        Some(flag) => Some(
            flag.as_str()
                .and_then(|f| parse_flag(f).ok())
                .ok_or_else(|| format!("flag must be one of {}", FLAGS.join(", ")))?,
        ),
    };
    let label = match args.get("label") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(label) => Some(Some(
            label
                .as_str()
                .filter(|l| LABELS.contains(l))
                .map(str::to_owned)
                .ok_or_else(|| format!("label must be one of {}, or null", LABELS.join(", ")))?,
        )),
    };
    if rating.is_none() && flag.is_none() && label.is_none() {
        return Err("set at least one of rating, flag, label".into());
    }
    Ok(Given {
        rating,
        flag,
        label,
    })
}

/// `companion.ts`'s `setJudgment`: the given fields over each photo's own
/// state, on the given paths or else the selection's targets, as a
/// judgment key's. Only the photos it
/// changes go into `changes`, each with its label known as a key's store
/// has it.
fn set_judgment(view: &View, args: &Value) -> Result<Action, String> {
    if view.view_only {
        return Err(VIEW_ONLY_REFUSAL.into());
    }
    let given = given(args)?;
    let paths = match args.get("paths") {
        None | Some(Value::Null) => {
            let targets = targets(view);
            if targets.is_empty() {
                return Err(NOTHING_SHOWN.into());
            }
            targets
        }
        Some(paths) => {
            let paths = paths
                .as_array()
                .ok_or("paths must be an array of strings")?;
            let mut picked: Vec<String> = Vec::new();
            for path in paths {
                let path = view.files[visible(view, path)?]
                    .to_string_lossy()
                    .into_owned();
                if !picked.contains(&path) {
                    picked.push(path);
                }
            }
            picked
        }
    };
    if paths.is_empty() {
        return Err("paths must name at least one photo".into());
    }
    let mut changes = Vec::new();
    let mut judged = Vec::new();
    for path in paths {
        let own = view.judgments.get(&path).cloned().unwrap_or_default();
        let after = given.over(&own);
        judged.push(json!({
            "path": path,
            "rating": after.rating,
            "flag": flag_name(after.flag),
            "label": after.label,
        }));
        if after != own {
            let label_known = view.seen.contains(&path) || own.label != after.label;
            changes.push(Change {
                path,
                before: own,
                after,
                label_known,
            });
        }
    }
    Ok(Action::Judge {
        changes,
        judged: Value::Array(judged),
    })
}

#[cfg(test)]
mod tests {
    use super::{Action, Busy, Change, Key, Overlays, View, answer, get_view};
    use crate::{
        burst::BurstMember,
        exif::{Group, Groups},
        filter::{EyesItem, Facts, FilterFlag, FilterState, OutputState},
        history::Top,
        judgment::State,
        menu::{self, Toggle},
        panels::Panels,
        selection::{Selection, selection_of},
        sort::SortKey,
    };
    use riffle_backend::{
        index::EyesPose,
        ondemand::{EyesJudgment, EyesMesh},
    };
    use riffle_core::Flag;
    use serde_json::{Value, json};
    use std::{
        collections::{HashMap, HashSet},
        path::PathBuf,
        sync::{Arc, LazyLock},
    };

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    static NO_JUDGMENTS: LazyLock<HashMap<String, State>> = LazyLock::new(HashMap::new);
    static NONE_SEEN: LazyLock<HashSet<String>> = LazyLock::new(HashSet::new);
    static NO_BURSTS: LazyLock<HashMap<String, BurstMember>> = LazyLock::new(HashMap::new);
    static NO_FACTS: LazyLock<HashMap<String, Facts>> = LazyLock::new(HashMap::new);
    static NO_SELECTION: LazyLock<Selection> = LazyLock::new(Selection::default);
    static NO_FILTER: LazyLock<FilterState> = LazyLock::new(FilterState::default);
    static NO_GROUPS: LazyLock<Groups> = LazyLock::new(Groups::default);

    fn view(files: &[PathBuf]) -> View<'_> {
        View {
            folder: Some("/d"),
            files,
            index: 0,
            zoomed: false,
            sort: SortKey::Name,
            filter: &NO_FILTER,
            output_known: false,
            groups: &NO_GROUPS,
            panes: Panels::default(),
            overlays: Overlays::default(),
            eyes: None,
            view_only: false,
            judgments: &NO_JUDGMENTS,
            seen: &NONE_SEEN,
            bursts: &NO_BURSTS,
            facts: &NO_FACTS,
            selection: &NO_SELECTION,
            busy: None,
            undo: Top::Empty,
            redo: Top::Empty,
            trash_stepping: false,
        }
    }

    fn ask(kind: &str, args: Value, view: &View) -> Result<Option<Action>, String> {
        answer(kind, &args, view)
    }

    #[test]
    fn reports_an_empty_view_with_no_folder_open() {
        let view = View {
            folder: None,
            ..view(&[])
        };
        assert_eq!(
            get_view(&view),
            json!({
                "folder": null,
                "count": 0,
                "current": null,
                "selected": [],
                "mode": "normal",
                "compare_active": null,
                "burst": [],
                "sort": "name",
                "filtered": false,
                "filter": {
                    "flags": [],
                    "stars": [],
                    "labels": [],
                    "orientations": [],
                    "face_focus": [],
                    "eyes": [],
                    "output": [],
                    "exif": {},
                },
                "filter_offers": { "output": false, "exif": {} },
                "panes": { "left": true, "right": true, "strip": true },
                "overlays": { "focus": false, "peaking": false, "performance": false },
            })
        );
    }

    #[test]
    fn reports_the_current_file_as_the_selection_and_no_burst() {
        let files = paths(&["/d/a.ARW", "/d/b.ARW"]);
        let filter = FilterState {
            flags: HashSet::from([FilterFlag::Picked]),
            ..FilterState::default()
        };
        let view = View {
            index: 1,
            zoomed: true,
            sort: SortKey::Capture,
            filter: &filter,
            ..view(&files)
        };
        let got = get_view(&view);
        assert_eq!(
            [
                "folder", "count", "current", "selected", "mode", "burst", "sort", "filtered"
            ]
            .map(|key| got[key].clone()),
            [
                json!("/d"),
                json!(2),
                json!({ "path": "/d/b.ARW", "position": 2 }),
                json!(["/d/b.ARW"]),
                json!("zoom"),
                json!([]),
                json!("capture"),
                json!(true),
            ]
        );
    }

    fn bursts(given: &[(&str, usize, usize, usize)]) -> HashMap<String, BurstMember> {
        given
            .iter()
            .map(|&(path, burst, position, size)| {
                (
                    path.to_owned(),
                    BurstMember {
                        burst,
                        position,
                        size,
                    },
                )
            })
            .collect()
    }

    fn sharp(score: f64) -> Facts {
        Facts {
            orientation: 1,
            capture_time: None,
            subsec: None,
            focus: None,
            exif: None,
            sharpness: Some(score),
        }
    }

    #[test]
    fn reports_a_single_file_with_no_burst() {
        let files = paths(&["/d/a.ARW"]);
        let bursts = bursts(&[("/d/a.ARW", 0, 0, 1)]);
        let view = View {
            bursts: &bursts,
            ..view(&files)
        };
        assert_eq!(get_view(&view)["burst"], json!([]));
    }

    #[test]
    fn lists_the_current_files_burst_in_capture_order_with_its_judgments() {
        let files = paths(&["/d/a", "/d/b", "/d/c", "/d/x"]);
        let bursts = bursts(&[
            ("/d/c", 0, 2, 3),
            ("/d/a", 0, 0, 3),
            ("/d/b", 0, 1, 3),
            ("/d/x", 1, 0, 1),
        ]);
        let facts = HashMap::from([
            ("/d/a".to_owned(), sharp(0.5)),
            ("/d/b".to_owned(), sharp(0.8)),
        ]);
        let judgments = HashMap::from([
            ("/d/b".to_owned(), state(Some(3), Flag::None, None)),
            ("/d/c".to_owned(), state(None, Flag::Reject, None)),
            ("/d/a".to_owned(), state(None, Flag::None, Some("Red"))),
        ]);
        let view = View {
            index: 1,
            bursts: &bursts,
            facts: &facts,
            judgments: &judgments,
            ..view(&files)
        };
        let state = get_view(&view);
        assert_eq!(state["current"], json!({ "path": "/d/b", "position": 2 }));
        assert_eq!(
            state["burst"],
            json!([
                {
                    "path": "/d/a",
                    "position": 1,
                    "sharpness": 0.5,
                    "rating": null,
                    "flag": "none",
                    "label": "Red",
                    "visible": true,
                },
                {
                    "path": "/d/b",
                    "position": 2,
                    "sharpness": 0.8,
                    "rating": 3,
                    "flag": "none",
                    "label": null,
                    "visible": true,
                },
                {
                    "path": "/d/c",
                    "position": 3,
                    "sharpness": null,
                    "rating": null,
                    "flag": "reject",
                    "label": null,
                    "visible": true,
                },
            ])
        );
    }

    #[test]
    fn marks_a_burst_frame_the_filter_hides_as_not_visible() {
        // The filter hides "/d/c", so it is missing from `files` even though
        // `bursts` still groups it with "/d/a" and "/d/b".
        let files = paths(&["/d/a", "/d/b"]);
        let bursts = bursts(&[("/d/a", 0, 0, 3), ("/d/b", 0, 1, 3), ("/d/c", 0, 2, 3)]);
        let view = View {
            bursts: &bursts,
            ..view(&files)
        };
        let visible: Vec<(Value, Value)> = get_view(&view)["burst"]
            .as_array()
            .unwrap()
            .iter()
            .map(|frame| (frame["path"].clone(), frame["visible"].clone()))
            .collect();
        assert_eq!(
            visible,
            [
                (json!("/d/a"), json!(true)),
                (json!("/d/b"), json!(true)),
                (json!("/d/c"), json!(false)),
            ]
        );
    }

    fn judged(pose: Option<EyesPose>) -> Option<Option<Arc<EyesJudgment>>> {
        Some(Some(Arc::new(EyesJudgment {
            state: "closed",
            probability: 0.9,
            ear: 0.1,
            pose,
            mesh: EyesMesh {
                width: 1616,
                height: 1080,
                points: vec![[1., 2.]],
            },
        })))
    }

    fn current_with(eyes: Option<Option<Arc<EyesJudgment>>>) -> Value {
        let files = paths(&["/d/a"]);
        get_view(&View {
            eyes,
            ..view(&files)
        })["current"]
            .clone()
    }

    #[test]
    fn reports_the_eyes_judgment_and_head_pose_without_the_mesh() {
        let pose = EyesPose {
            yaw: 40.,
            pitch: -5.,
            roll: 2.,
        };
        assert_eq!(
            current_with(judged(Some(pose))),
            json!({
                "path": "/d/a",
                "position": 1,
                "eyes": {
                    "state": "closed",
                    "probability": 0.9,
                    "pose": { "yaw": 40.0, "pitch": -5.0, "roll": 2.0 },
                },
            })
        );
    }

    #[test]
    fn reports_a_null_pose_when_the_fit_failed() {
        assert_eq!(current_with(judged(None))["eyes"]["pose"], Value::Null);
    }

    #[test]
    fn reports_null_eyes_for_a_file_judged_without_a_face() {
        let current = current_with(Some(None));
        assert_eq!(current["eyes"], Value::Null);
        assert!(current.as_object().unwrap().contains_key("eyes"));
    }

    #[test]
    fn leaves_the_eyes_key_out_until_the_file_has_been_judged() {
        let current = current_with(None);
        assert!(!current.as_object().unwrap().contains_key("eyes"));
    }

    #[test]
    fn get_view_changes_nothing() {
        assert_eq!(ask("get_view", json!({}), &view(&[])), Ok(None));
    }

    #[test]
    fn turns_an_unknown_request_into_an_error() {
        assert_eq!(
            ask("nope", json!({}), &view(&[])),
            Err("unknown request: nope".into())
        );
    }

    #[test]
    fn show_photo_makes_a_visible_path_current() {
        let files = paths(&["/d/a", "/d/b"]);
        assert_eq!(
            ask("show_photo", json!({ "path": "/d/b" }), &view(&files)),
            Ok(Some(Action::Show(1)))
        );
    }

    #[test]
    fn show_photo_refuses_a_path_the_strip_does_not_show() {
        let files = paths(&["/d/a"]);
        assert_eq!(
            ask("show_photo", json!({ "path": "/d/x" }), &view(&files)),
            Err(
                "/d/x is not shown in Handpic: not in the open folder, or hidden by the filter"
                    .into()
            )
        );
        assert_eq!(
            ask("show_photo", json!({}), &view(&files)),
            Err("path must be a string".into())
        );
    }

    #[test]
    fn select_photos_selects_one_path_given_once_or_repeated() {
        let files = paths(&["/d/a", "/d/b", "/d/c"]);
        for given in [json!(["/d/c"]), json!(["/d/c", "/d/c"])] {
            assert_eq!(
                ask("select_photos", json!({ "paths": given }), &view(&files)),
                Ok(Some(Action::Select {
                    at: 2,
                    paths: vec!["/d/c".into()]
                }))
            );
        }
    }

    #[test]
    fn select_photos_selects_several_photos_the_first_current() {
        let files = paths(&["/d/a", "/d/b", "/d/c"]);
        assert_eq!(
            ask(
                "select_photos",
                json!({ "paths": ["/d/c", "/d/a", "/d/c"] }),
                &view(&files)
            ),
            Ok(Some(Action::Select {
                at: 2,
                paths: vec!["/d/c".into(), "/d/a".into()]
            }))
        );
    }

    #[test]
    fn reports_the_selections_targets_in_strip_order() {
        let files = paths(&["/d/a", "/d/b", "/d/c"]);
        let selection = selection_of(&["/d/c".into(), "/d/a".into()]);
        let view = View {
            index: 2,
            selection: &selection,
            ..view(&files)
        };
        assert_eq!(get_view(&view)["selected"], json!(["/d/a", "/d/c"]));
    }

    #[test]
    fn navigate_answers_each_direction_with_its_key() {
        let files = paths(&["/d/a", "/d/b"]);
        let view = view(&files);
        for (to, key) in [
            ("previous", Key::Previous),
            ("next", Key::Next),
            ("first", Key::First),
            ("last", Key::Last),
            ("burst_previous", Key::BurstPrevious),
            ("burst_next", Key::BurstNext),
            ("burst_frame_previous", Key::BurstFramePrevious),
            ("burst_frame_next", Key::BurstFrameNext),
        ] {
            assert_eq!(
                ask("navigate", json!({ "to": to }), &view),
                Ok(Some(Action::Key(key)))
            );
        }
    }

    #[test]
    fn navigate_refuses_an_unknown_direction_and_an_empty_view() {
        let files = paths(&["/d/a"]);
        assert!(
            ask("navigate", json!({ "to": "up" }), &view(&files))
                .unwrap_err()
                .starts_with("to must be one of previous, next")
        );
        assert_eq!(
            ask("navigate", json!({ "to": "next" }), &view(&[])),
            Err("no photo is shown in Handpic".into())
        );
    }

    #[test]
    fn select_all_selects_unless_nothing_is_shown() {
        let files = paths(&["/d/a"]);
        assert_eq!(
            ask("select_all", json!({}), &view(&files)),
            Ok(Some(Action::Key(Key::SelectAll)))
        );
        assert_eq!(
            ask("select_all", json!({}), &view(&[])),
            Err("no photo is shown in Handpic".into())
        );
    }

    #[test]
    fn reject_rest_needs_a_burst_and_a_raw_folder() {
        let files = paths(&["/d/a", "/d/b", "/d/x"]);
        let bursts = bursts(&[("/d/a", 0, 0, 2), ("/d/b", 0, 1, 2), ("/d/x", 1, 0, 1)]);
        let in_burst = View {
            bursts: &bursts,
            ..view(&files)
        };
        assert_eq!(
            ask("reject_rest", json!({}), &in_burst),
            Ok(Some(Action::Key(Key::RejectRest)))
        );
        let alone = View {
            index: 2,
            bursts: &bursts,
            ..view(&files)
        };
        assert_eq!(
            ask("reject_rest", json!({}), &alone),
            Err("the current photo is not part of a burst".into())
        );
        let view_only = View {
            view_only: true,
            bursts: &bursts,
            ..view(&files)
        };
        assert_eq!(
            ask("reject_rest", json!({}), &view_only),
            Err("culling does not apply to a JPEG folder".into())
        );
        assert_eq!(
            ask("reject_rest", json!({}), &view(&[])),
            Err("no photo is shown in Handpic".into())
        );
    }

    #[test]
    fn undo_and_redo_step_a_judgment_batch() {
        let view = View {
            undo: Top::Judgments,
            redo: Top::Judgments,
            ..view(&[])
        };
        assert_eq!(
            ask("undo", json!({}), &view),
            Ok(Some(Action::Key(Key::Undo)))
        );
        assert_eq!(
            ask("redo", json!({}), &view),
            Ok(Some(Action::Key(Key::Redo)))
        );
    }

    #[test]
    fn undo_and_redo_refuse_a_trash_run_an_empty_history_and_a_view_only_folder() {
        let trash = View {
            undo: Top::Trash,
            redo: Top::Trash,
            ..view(&[])
        };
        for verb in ["undo", "redo"] {
            assert_eq!(
                ask(verb, json!({}), &trash),
                Err(format!(
                    "the next {verb} is of a move to the Trash, which is not available over MCP yet"
                ))
            );
            assert_eq!(
                ask(verb, json!({}), &view(&[])),
                Err(format!("nothing to {verb}"))
            );
        }
        let view_only = View {
            view_only: true,
            undo: Top::Judgments,
            ..view(&[])
        };
        assert_eq!(
            ask("undo", json!({}), &view_only),
            Err("culling does not apply to a JPEG folder".into())
        );
        let stepping = View {
            trash_stepping: true,
            undo: Top::Judgments,
            ..view(&[])
        };
        assert!(
            ask("undo", json!({}), &stepping)
                .unwrap_err()
                .starts_with("an undo or redo of a move to the Trash is under way")
        );
    }

    #[test]
    fn a_busy_view_refuses_every_request_but_get_view() {
        let files = paths(&["/d/a"]);
        for (busy, message) in [
            (
                Busy::Settings,
                "Handpic's settings are open; try again once the user closes them",
            ),
            (
                Busy::Trash,
                "a Move to Trash or Delete Permanently is under way in Handpic; try again once it ends",
            ),
            (
                Busy::FolderDialog,
                "a folder operation's dialog is open in Handpic; try again once the user closes it",
            ),
            (
                Busy::Rename,
                "a folder name is being edited in Handpic; try again once the user is done",
            ),
        ] {
            let view = View {
                busy: Some(busy),
                undo: Top::Judgments,
                ..view(&files)
            };
            assert_eq!(ask("get_view", json!({}), &view), Ok(None));
            for (kind, args) in [
                ("show_photo", json!({ "path": "/d/a" })),
                ("select_photos", json!({ "paths": ["/d/a"] })),
                ("set_view", json!({ "mode": "zoom" })),
                ("set_judgment", json!({ "rating": 1 })),
                ("navigate", json!({ "to": "next" })),
                ("select_all", json!({})),
                ("reject_rest", json!({})),
                ("undo", json!({})),
                ("redo", json!({})),
                ("set_sort", json!({ "by": "rating" })),
                ("set_filter", json!({ "reset": true })),
                ("set_panes", json!({ "left": false })),
                ("set_overlays", json!({ "focus": true })),
            ] {
                assert_eq!(ask(kind, args, &view), Err(message.into()), "{kind}");
            }
        }
    }

    #[test]
    fn key_of_action_names_the_keymap_actions_a_tool_shares() {
        for (action, key) in [
            ("previous", Key::Previous),
            ("next", Key::Next),
            ("burstPrevious", Key::BurstPrevious),
            ("burstNext", Key::BurstNext),
            ("burstFramePrevious", Key::BurstFramePrevious),
            ("burstFrameNext", Key::BurstFrameNext),
            ("selectAll", Key::SelectAll),
            ("rejectRest", Key::RejectRest),
            ("undo", Key::Undo),
            ("redo", Key::Redo),
        ] {
            assert_eq!(Key::of_action(action), Some(key));
        }
        for action in ["rate1", "zoom", "extendNext", "first"] {
            assert_eq!(Key::of_action(action), None);
        }
    }

    #[test]
    fn select_photos_refuses_an_empty_list_a_non_list_and_a_hidden_path() {
        let files = paths(&["/d/a", "/d/b"]);
        let view = view(&files);
        assert_eq!(
            ask("select_photos", json!({ "paths": [] }), &view),
            Err("paths must name at least one photo".into())
        );
        assert_eq!(
            ask("select_photos", json!({ "paths": "/d/a" }), &view),
            Err("paths must be an array of strings".into())
        );
        let hidden = ask("select_photos", json!({ "paths": ["/d/a", "/d/x"] }), &view);
        assert!(
            hidden
                .unwrap_err()
                .starts_with("/d/x is not shown in Handpic")
        );
    }

    #[test]
    fn set_view_toggles_zoom_only_when_the_mode_differs() {
        let files = paths(&["/d/a"]);
        let normal = view(&files);
        let zoomed = View {
            zoomed: true,
            ..view(&files)
        };
        let zoom = json!({ "mode": "zoom" });
        let fit = json!({ "mode": "normal" });
        assert_eq!(
            ask("set_view", zoom.clone(), &normal),
            Ok(Some(Action::ToggleZoom))
        );
        assert_eq!(ask("set_view", zoom, &zoomed), Ok(None));
        assert_eq!(
            ask("set_view", fit.clone(), &zoomed),
            Ok(Some(Action::ToggleZoom))
        );
        assert_eq!(ask("set_view", fit, &normal), Ok(None));
    }

    #[test]
    fn set_view_refuses_compare() {
        let files = paths(&["/d/a", "/d/b"]);
        assert_eq!(
            ask("set_view", json!({ "mode": "compare" }), &view(&files)),
            Err("compare is not supported by Handpic's native app yet".into())
        );
    }

    #[test]
    fn set_view_refuses_an_unknown_mode_and_zoom_with_nothing_shown() {
        let view = view(&[]);
        assert_eq!(
            ask("set_view", json!({ "mode": "grid" }), &view),
            Err("mode must be one of normal, zoom, compare".into())
        );
        assert_eq!(
            ask("set_view", json!({ "mode": "zoom" }), &view),
            Err("no photo is shown in Handpic".into())
        );
        assert_eq!(
            ask("set_view", json!({ "mode": "normal" }), &view),
            Ok(None)
        );
    }

    fn state(rating: Option<u8>, flag: Flag, label: Option<&str>) -> State {
        State {
            rating,
            flag,
            label: label.map(Into::into),
        }
    }

    fn judge(args: Value, view: &View) -> Result<(Vec<Change>, Value), String> {
        match answer("set_judgment", &args, view)? {
            Some(Action::Judge { changes, judged }) => Ok((changes, judged)),
            other => panic!("not a judgment: {other:?}"),
        }
    }

    #[test]
    fn set_judgment_changes_only_the_given_fields_and_reports_every_photo() {
        let files = paths(&["/d/a", "/d/b"]);
        let judgments = HashMap::from([
            ("/d/a".to_owned(), state(None, Flag::Pick, None)),
            ("/d/b".to_owned(), state(Some(3), Flag::None, Some("Red"))),
        ]);
        let seen = HashSet::from(["/d/a".to_owned(), "/d/b".to_owned()]);
        let view = View {
            judgments: &judgments,
            seen: &seen,
            ..view(&files)
        };
        let (changes, judged) =
            judge(json!({ "paths": ["/d/a", "/d/b"], "rating": 3 }), &view).unwrap();
        assert_eq!(
            judged,
            json!([
                { "path": "/d/a", "rating": 3, "flag": "pick", "label": null },
                { "path": "/d/b", "rating": 3, "flag": "none", "label": "Red" },
            ])
        );
        assert_eq!(
            changes,
            vec![Change {
                path: "/d/a".into(),
                before: state(None, Flag::Pick, None),
                after: state(Some(3), Flag::Pick, None),
                label_known: true,
            }]
        );
    }

    #[test]
    fn set_judgment_clears_with_a_zero_rating_and_a_null_label_on_the_current_file() {
        let files = paths(&["/d/a", "/d/b"]);
        let judgments =
            HashMap::from([("/d/b".to_owned(), state(Some(4), Flag::Pick, Some("Blue")))]);
        let view = View {
            index: 1,
            judgments: &judgments,
            ..view(&files)
        };
        let (changes, judged) = judge(
            json!({ "rating": 0, "flag": "reject", "label": null }),
            &view,
        )
        .unwrap();
        assert_eq!(
            judged,
            json!([{ "path": "/d/b", "rating": null, "flag": "reject", "label": null }])
        );
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].after, state(None, Flag::Reject, None));
        assert!(changes[0].label_known, "the call cleared the label itself");
    }

    #[test]
    fn set_judgment_keeps_an_unseen_photos_label_unknown_unless_it_sets_it() {
        let files = paths(&["/d/a"]);
        let (changes, _) = judge(
            json!({ "paths": ["/d/a", "/d/a"], "flag": "pick" }),
            &view(&files),
        )
        .unwrap();
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].label_known);
        let (changes, _) = judge(json!({ "label": "Green" }), &view(&files)).unwrap();
        assert!(changes[0].label_known);
    }

    #[test]
    fn set_judgment_changes_nothing_when_the_photo_already_holds_the_value() {
        let files = paths(&["/d/a"]);
        let judgments = HashMap::from([("/d/a".to_owned(), state(Some(2), Flag::None, None))]);
        let view = View {
            judgments: &judgments,
            ..view(&files)
        };
        let (changes, judged) = judge(json!({ "rating": 2 }), &view).unwrap();
        assert!(changes.is_empty());
        assert_eq!(
            judged,
            json!([{ "path": "/d/a", "rating": 2, "flag": "none", "label": null }])
        );
    }

    #[test]
    fn set_judgment_refuses_bad_fields_hidden_paths_and_an_empty_view() {
        let files = paths(&["/d/a"]);
        let shown = view(&files);
        assert!(
            judge(json!({ "paths": ["/d/hidden"], "rating": 1 }), &shown)
                .unwrap_err()
                .starts_with("/d/hidden is not shown in Handpic")
        );
        for (args, message) in [
            (
                json!({ "rating": 6 }),
                "rating must be an integer from 0 to 5",
            ),
            (
                json!({ "rating": 1.5 }),
                "rating must be an integer from 0 to 5",
            ),
            (
                json!({ "flag": "maybe" }),
                "flag must be one of none, pick, reject",
            ),
            (
                json!({ "label": "Teal" }),
                "label must be one of Red, Orange, Yellow, Green, Blue, Pink, Purple, or null",
            ),
            (json!({}), "set at least one of rating, flag, label"),
            (
                json!({ "paths": "/d/a", "rating": 1 }),
                "paths must be an array of strings",
            ),
            (
                json!({ "paths": [], "rating": 1 }),
                "paths must name at least one photo",
            ),
        ] {
            assert_eq!(judge(args, &shown).unwrap_err(), message);
        }
        assert_eq!(
            judge(json!({ "rating": 1 }), &view(&[])).unwrap_err(),
            "no photo is shown in Handpic"
        );
    }

    #[test]
    fn set_judgment_is_refused_in_a_view_only_folder() {
        let files = paths(&["/d/a.jpg"]);
        let view = View {
            view_only: true,
            ..view(&files)
        };
        for args in [
            json!({ "paths": ["/d/a.jpg"], "rating": 3 }),
            json!({ "flag": "pick" }),
        ] {
            assert_eq!(
                judge(args, &view).unwrap_err(),
                "culling does not apply to a JPEG folder"
            );
        }
    }

    #[test]
    fn set_judgment_without_paths_judges_the_selections_targets() {
        let files = paths(&["/d/a", "/d/b", "/d/c"]);
        let selection = selection_of(&["/d/c".into(), "/d/a".into()]);
        let view = View {
            index: 2,
            selection: &selection,
            ..view(&files)
        };
        let (changes, judged) = judge(json!({ "flag": "pick" }), &view).unwrap();
        assert_eq!(
            judged,
            json!([
                { "path": "/d/a", "rating": null, "flag": "pick", "label": null },
                { "path": "/d/c", "rating": null, "flag": "pick", "label": null },
            ])
        );
        assert_eq!(changes.len(), 2);
    }

    fn lens_groups() -> Groups {
        Groups(vec![(
            Group::Lens,
            vec!["FE 24mm".to_owned(), "FE 50mm".to_owned()],
        )])
    }

    #[test]
    fn reports_the_filters_checks_what_the_folder_offers_the_panes_and_the_overlays() {
        let groups = lens_groups();
        let filter = FilterState {
            flags: HashSet::from([FilterFlag::Rejected, FilterFlag::Picked]),
            stars: HashSet::from([5, 0]),
            labels: HashSet::from(["none".to_owned(), "red".to_owned()]),
            outputs: HashSet::from([OutputState::No]),
            exif: [(Group::Lens, ["FE 50mm".to_owned()].into())].into(),
            ..FilterState::default()
        };
        let view = View {
            filter: &filter,
            output_known: true,
            groups: &groups,
            panes: Panels {
                left: false,
                strip: true,
                right: true,
            },
            overlays: Overlays {
                peaking: true,
                ..Overlays::default()
            },
            ..view(&[])
        };
        let got = get_view(&view);
        assert_eq!(got["filtered"], json!(true));
        assert_eq!(
            got["filter"],
            json!({
                "flags": ["picked", "rejected"],
                "stars": [0, 5],
                "labels": ["red", "none"],
                "orientations": [],
                "face_focus": [],
                "eyes": [],
                "output": ["no_output"],
                "exif": { "lens": ["FE 50mm"] },
            })
        );
        assert_eq!(
            got["filter_offers"],
            json!({ "output": true, "exif": { "lens": ["FE 24mm", "FE 50mm"] } })
        );
        assert_eq!(
            got["panes"],
            json!({ "left": false, "right": true, "strip": true })
        );
        assert_eq!(
            got["overlays"],
            json!({ "focus": false, "peaking": true, "performance": false })
        );
    }

    #[test]
    fn set_sort_picks_a_menu_key_only_when_it_differs() {
        let view = view(&[]);
        assert_eq!(
            ask("set_sort", json!({ "by": "rating" }), &view),
            Ok(Some(Action::Menu(vec![menu::Action::Sort(
                SortKey::Rating
            )])))
        );
        assert_eq!(ask("set_sort", json!({ "by": "name" }), &view), Ok(None));
        assert_eq!(
            ask("set_sort", json!({ "by": "recommended" }), &view),
            Err("the recommended sort is not implemented in Handpic's native app yet".into())
        );
        assert_eq!(
            ask("set_sort", json!({ "by": "size" }), &view),
            Err("by must be one of name, capture, rating".into())
        );
    }

    #[test]
    fn set_sort_is_refused_in_a_view_only_folder_which_reports_capture_order() {
        let files = paths(&["/d/a.jpg"]);
        let view = View {
            view_only: true,
            sort: SortKey::Rating,
            ..view(&files)
        };
        assert_eq!(
            ask("set_sort", json!({ "by": "name" }), &view),
            Err("a JPEG folder stays in capture order; the sort does not apply to it".into())
        );
        assert_eq!(get_view(&view)["sort"], json!("capture"));
    }

    /// The filter `set_filter`'s clicks give over `view`'s.
    fn filtered(args: Value, view: &View) -> FilterState {
        let Ok(Some(Action::Menu(actions))) = ask("set_filter", args, view) else {
            panic!("set_filter gave no menu clicks");
        };
        let (mut filter, mut sort) = (view.filter.clone(), view.sort);
        for action in actions {
            menu::apply(&mut filter, &mut sort, action);
        }
        filter
    }

    #[test]
    fn set_filter_replaces_the_given_sections_and_keeps_the_others() {
        let groups = lens_groups();
        let filter = FilterState {
            flags: HashSet::from([FilterFlag::Rejected]),
            stars: HashSet::from([1]),
            ..FilterState::default()
        };
        let view = View {
            filter: &filter,
            groups: &groups,
            ..view(&[])
        };
        let got = filtered(
            json!({
                "flags": ["picked", "untagged"],
                "labels": ["green", "none"],
                "eyes": ["closed"],
                "exif": { "lens": ["FE 24mm"] },
            }),
            &view,
        );
        assert_eq!(
            got,
            FilterState {
                flags: HashSet::from([FilterFlag::Picked, FilterFlag::Untagged]),
                stars: HashSet::from([1]),
                labels: HashSet::from(["green".to_owned(), "none".to_owned()]),
                eyes: HashSet::from([EyesItem::Closed]),
                exif: [(Group::Lens, ["FE 24mm".to_owned()].into())].into(),
                ..FilterState::default()
            }
        );
    }

    #[test]
    fn set_filter_resets_through_the_menus_reset_then_checks_what_it_gives() {
        let filter = FilterState {
            flags: HashSet::from([FilterFlag::Rejected]),
            stars: HashSet::from([1]),
            ..FilterState::default()
        };
        let view = View {
            filter: &filter,
            ..view(&[])
        };
        let Ok(Some(Action::Menu(actions))) =
            ask("set_filter", json!({ "reset": true, "stars": [5] }), &view)
        else {
            panic!("set_filter gave no menu clicks");
        };
        assert_eq!(
            actions,
            [menu::Action::Reset, menu::Action::Toggle(Toggle::Stars(5))]
        );
        assert_eq!(
            filtered(json!({ "reset": true }), &view),
            FilterState::default()
        );
    }

    #[test]
    fn set_filter_changes_nothing_when_the_checks_already_match() {
        let filter = FilterState {
            stars: HashSet::from([4, 5]),
            ..FilterState::default()
        };
        let checked = View {
            filter: &filter,
            ..view(&[])
        };
        assert_eq!(
            ask("set_filter", json!({ "stars": [5, 4] }), &checked),
            Ok(None)
        );
        assert_eq!(
            ask("set_filter", json!({ "flags": [] }), &checked),
            Ok(None)
        );
        let unfiltered = view(&[]);
        assert_eq!(
            ask("set_filter", json!({ "reset": true }), &unfiltered),
            Ok(None)
        );
    }

    #[test]
    fn set_filter_takes_an_empty_exif_section_as_unchecking_it() {
        let groups = lens_groups();
        let filter = FilterState {
            exif: [(Group::Lens, ["FE 50mm".to_owned()].into())].into(),
            ..FilterState::default()
        };
        let view = View {
            filter: &filter,
            groups: &groups,
            ..view(&[])
        };
        assert_eq!(
            filtered(json!({ "exif": { "lens": [] } }), &view),
            FilterState::default()
        );
    }

    #[test]
    fn set_filter_refuses_what_the_menu_does_not_offer() {
        let groups = lens_groups();
        let view = View {
            groups: &groups,
            ..view(&[])
        };
        for (args, message) in [
            (
                json!({}),
                "set reset or at least one of flags, stars, labels, orientations, \
                face_focus, eyes, output, exif",
            ),
            (json!({ "reset": "yes" }), "reset must be true or false"),
            (
                json!({ "flags": ["starred"] }),
                "flags must be an array of picked, untagged, rejected",
            ),
            (
                json!({ "stars": [6] }),
                "stars must be an array of integers from 0 to 5",
            ),
            (
                json!({ "labels": ["Red"] }),
                "labels must be an array of red, orange, yellow, green, blue, pink, purple, none",
            ),
            (
                json!({ "face_focus": "good" }),
                "face_focus must be an array of good, neutral, low",
            ),
            (
                json!({ "output": ["output"] }),
                "the open folder has no Output folder, so the output section does not apply",
            ),
            (
                json!({ "exif": { "camera": ["ILCE-1"] } }),
                "exif.camera must be an array of the labels get_view's filter_offers lists",
            ),
            (
                json!({ "exif": { "lens": ["FE 85mm"] } }),
                "exif.lens must be an array of the labels get_view's filter_offers lists",
            ),
            (
                json!({ "exif": { "body": [] } }),
                "exif sections must be among camera, lens, aperture, shutter, iso, focal",
            ),
        ] {
            assert_eq!(
                ask("set_filter", args.clone(), &view),
                Err(message.into()),
                "{args}"
            );
        }
        assert_eq!(ask("set_filter", json!({ "output": [] }), &view), Ok(None));
        let known = View {
            output_known: true,
            ..self::view(&[])
        };
        assert_eq!(
            filtered(json!({ "output": ["output"] }), &known).outputs,
            HashSet::from([OutputState::Yes])
        );
    }

    #[test]
    fn set_panes_shows_the_given_panes_keeping_the_others() {
        let view = View {
            panes: Panels {
                left: true,
                strip: false,
                right: true,
            },
            ..view(&[])
        };
        assert_eq!(
            ask("set_panes", json!({ "left": false, "strip": true }), &view),
            Ok(Some(Action::Panes(Panels {
                left: false,
                strip: true,
                right: true,
            })))
        );
        assert_eq!(
            ask("set_panes", json!({ "left": true, "strip": false }), &view),
            Ok(None)
        );
        assert_eq!(
            ask("set_panes", json!({}), &view),
            Err("set at least one of left, right, strip".into())
        );
        assert_eq!(
            ask("set_panes", json!({ "right": 0 }), &view),
            Err("right must be true or false".into())
        );
    }

    #[test]
    fn set_overlays_flips_only_the_ones_that_differ() {
        let view = View {
            overlays: Overlays {
                focus: true,
                ..Overlays::default()
            },
            ..view(&[])
        };
        assert_eq!(
            ask(
                "set_overlays",
                json!({ "focus": true, "peaking": true, "performance": false }),
                &view
            ),
            Ok(Some(Action::Overlays(Overlays {
                peaking: true,
                ..Overlays::default()
            })))
        );
        assert_eq!(
            ask("set_overlays", json!({ "focus": false }), &view),
            Ok(Some(Action::Overlays(Overlays {
                focus: true,
                ..Overlays::default()
            })))
        );
        assert_eq!(
            ask("set_overlays", json!({ "focus": true }), &view),
            Ok(None)
        );
        assert_eq!(
            ask("set_overlays", json!({}), &view),
            Err("set at least one of focus, peaking, performance".into())
        );
    }
}
