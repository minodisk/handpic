//! The answering side of the MCP companion, a port of `companion.ts` over a
//! snapshot of the view. The native view has no selection, no compare and no
//! bursts yet, so the selection is the current file, `burst` is always empty,
//! and compare, a selection of several photos and `set_judgment` are refused.

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use riffle_backend::ondemand::EyesJudgment;
use serde_json::{Map, Value, json};

const COMPARE_REFUSAL: &str = "compare is not supported by Handpic's native app yet";
const SELECTION_REFUSAL: &str =
    "selecting more than one photo is not supported by Handpic's native app yet";
const JUDGMENT_REFUSAL: &str = "set_judgment is not supported by Handpic's native app yet";

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
}

/// What the view does before it answers with the new `get_view`.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Make the file at this index of `files` current.
    Show(usize),
    ToggleZoom,
}

/// The answer to `kind` over `view`: what the view has to do, if anything,
/// before it replies with `get_view`, or the refusal's message.
pub fn answer(kind: &str, args: &Value, view: &View) -> Result<Option<Action>, String> {
    match kind {
        "get_view" => Ok(None),
        "show_photo" => visible(view, &args["path"]).map(|at| Some(Action::Show(at))),
        "select_photos" => select_photos(view, &args["paths"]).map(Some),
        "set_view" => set_view(view, &args["mode"]),
        "set_judgment" => Err(JUDGMENT_REFUSAL.into()),
        _ => Err(format!("unknown request: {kind}")),
    }
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
    let (selected, current) = match current {
        Some((path, current)) => (vec![path], current),
        None => (Vec::new(), Value::Null),
    };
    json!({
        "folder": view.folder,
        "count": view.files.len(),
        "current": current,
        "selected": selected,
        "mode": if view.zoomed { "zoom" } else { "normal" },
        "compare_active": null,
        "burst": [],
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

#[cfg(test)]
mod tests {
    use super::{Action, View, answer, get_view};
    use riffle_backend::{
        index::EyesPose,
        ondemand::{EyesJudgment, EyesMesh},
    };
    use serde_json::{Value, json};
    use std::{path::PathBuf, sync::Arc};

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    fn view(files: &[PathBuf]) -> View<'_> {
        View {
            folder: Some("/d"),
            files,
            index: 0,
            zoomed: false,
            sort: "name",
            filtered: false,
            eyes: None,
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

    #[test]
    fn set_judgment_is_refused_for_now() {
        let files = paths(&["/d/a"]);
        assert_eq!(
            ask("set_judgment", json!({ "rating": 3 }), &view(&files)),
            Err("set_judgment is not supported by Handpic's native app yet".into())
        );
    }
}
