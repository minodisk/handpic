//! The answering side of the MCP companion, a port of `companion.ts` over a
//! snapshot of the view. The native view has no compare and answers with no
//! selection yet, so the selection is the current file, and compare and a
//! selection of several photos are refused.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use riffle_backend::{
    index::{flag_name, parse_flag},
    ondemand::EyesJudgment,
};
use riffle_core::Flag;
use serde_json::{Map, Value, json};

use crate::{burst::BurstMember, filter::Facts, judgment::State};

const COMPARE_REFUSAL: &str = "compare is not supported by Handpic's native app yet";
const SELECTION_REFUSAL: &str =
    "selecting more than one photo is not supported by Handpic's native app yet";
/// `viewonly.ts`'s `VIEW_ONLY_REFUSAL`.
const VIEW_ONLY_REFUSAL: &str = "culling does not apply to a JPEG folder";
const FLAGS: [&str; 3] = ["none", "pick", "reject"];
/// The color labels the judgment keys set, as `State.label` holds them.
const LABELS: [&str; 7] = ["Red", "Orange", "Yellow", "Green", "Blue", "Pink", "Purple"];

/// What a request reads of the view.
pub struct View<'a> {
    /// The open folder, as the index keys it.
    pub folder: Option<&'a str>,
    /// The files that pass the filter, in strip order.
    pub files: &'a [PathBuf],
    pub index: usize,
    pub zoomed: bool,
    /// The sort key's setting name.
    pub sort: &'static str,
    pub filtered: bool,
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
}

/// What the view does before it answers with the new `get_view`.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Make the file at this index of `files` current.
    Show(usize),
    ToggleZoom,
    /// Record `changes` as one undo entry, as a judgment key does, and reply
    /// with `judged` instead of `get_view`.
    Judge {
        changes: Vec<Change>,
        judged: Value,
    },
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
    match kind {
        "get_view" => Ok(None),
        "show_photo" => visible(view, &args["path"]).map(|at| Some(Action::Show(at))),
        "select_photos" => select_photos(view, &args["paths"]).map(Some),
        "set_view" => set_view(view, &args["mode"]),
        "set_judgment" => set_judgment(view, args).map(Some),
        _ => Err(format!("unknown request: {kind}")),
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
    let (selected, current, burst) = match current {
        Some((path, current)) => {
            let burst = burst_of(view, &path);
            (vec![path], current, burst)
        }
        None => (Vec::new(), Value::Null, Vec::new()),
    };
    json!({
        "folder": view.folder,
        "count": view.files.len(),
        "current": current,
        "selected": selected,
        "mode": if view.zoomed { "zoom" } else { "normal" },
        "compare_active": null,
        "burst": burst,
        "sort": view.sort,
        "filtered": view.filtered,
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
    let mut seen = HashSet::new();
    let mut picked = Vec::new();
    for path in paths {
        let at = visible(view, path)?;
        if seen.insert(at) {
            picked.push(at);
        }
    }
    match picked[..] {
        [] => Err("paths must name at least one photo".into()),
        [at] => Ok(Action::Show(at)),
        _ => Err(SELECTION_REFUSAL.into()),
    }
}

fn set_view(view: &View, mode: &Value) -> Result<Option<Action>, String> {
    let mode = mode
        .as_str()
        .filter(|mode| ["normal", "zoom", "compare"].contains(mode))
        .ok_or("mode must be one of normal, zoom, compare")?;
    if mode != "normal" && view.files.is_empty() {
        return Err("no photo is shown in Handpic".into());
    }
    if mode == "compare" {
        return Err(COMPARE_REFUSAL.into());
    }
    Ok(((mode == "zoom") != view.zoomed).then_some(Action::ToggleZoom))
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
/// state, on the given paths or else the current file. Only the photos it
/// changes go into `changes`, each with its label known as a key's store
/// has it.
fn set_judgment(view: &View, args: &Value) -> Result<Action, String> {
    if view.view_only {
        return Err(VIEW_ONLY_REFUSAL.into());
    }
    let given = given(args)?;
    let paths = match args.get("paths") {
        None | Some(Value::Null) => vec![
            view.files
                .get(view.index)
                .ok_or("no photo is shown in Handpic")?
                .to_string_lossy()
                .into_owned(),
        ],
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
    use super::{Action, Change, View, answer, get_view};
    use crate::{burst::BurstMember, filter::Facts, judgment::State};
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

    fn view(files: &[PathBuf]) -> View<'_> {
        View {
            folder: Some("/d"),
            files,
            index: 0,
            zoomed: false,
            sort: "name",
            filtered: false,
            eyes: None,
            view_only: false,
            judgments: &NO_JUDGMENTS,
            seen: &NONE_SEEN,
            bursts: &NO_BURSTS,
            facts: &NO_FACTS,
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
            })
        );
    }

    #[test]
    fn reports_the_current_file_as_the_selection_and_no_burst() {
        let files = paths(&["/d/a.ARW", "/d/b.ARW"]);
        let view = View {
            index: 1,
            zoomed: true,
            sort: "capture",
            filtered: true,
            ..view(&files)
        };
        assert_eq!(
            get_view(&view),
            json!({
                "folder": "/d",
                "count": 2,
                "current": { "path": "/d/b.ARW", "position": 2 },
                "selected": ["/d/b.ARW"],
                "mode": "zoom",
                "compare_active": null,
                "burst": [],
                "sort": "capture",
                "filtered": true,
            })
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
    fn select_photos_shows_one_path_given_once_or_repeated() {
        let files = paths(&["/d/a", "/d/b", "/d/c"]);
        for given in [json!(["/d/c"]), json!(["/d/c", "/d/c"])] {
            assert_eq!(
                ask("select_photos", json!({ "paths": given }), &view(&files)),
                Ok(Some(Action::Show(2)))
            );
        }
    }

    #[test]
    fn select_photos_refuses_several_photos_until_the_view_has_a_selection() {
        let files = paths(&["/d/a", "/d/b"]);
        assert_eq!(
            ask(
                "select_photos",
                json!({ "paths": ["/d/a", "/d/b"] }),
                &view(&files)
            ),
            Err(
                "selecting more than one photo is not supported by Handpic's native app yet".into()
            )
        );
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
}
