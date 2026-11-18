//! The answering side of the MCP companion, a port of `companion.ts` over a
//! snapshot of the view, extended with the native app's own tools: the
//! moves, Select All, Reject Rest, undo and redo, each the `Key` its keymap
//! action runs, and the sort, the filter, the panes and the overlays, each
//! set to a given state rather than toggled, and the folder tree's opens,
//! expansions and reveals. `get_status` reports what the status row and the
//! pane's notes tell the user (`Status`), which `wait_for_idle` waits on.
//! `get_settings` reports what the settings modal shows, and `set_settings`
//! turns a call into the modal's own `settingsview::Change`s, by its rules,
//! never `mcpAllowDestructive`, which only the user grants. `trash_photos`
//! moves photos to the Trash or deletes them: a destructive operation runs
//! on a preview (`Op`, kept by the view as the `Pending` its id names) and a
//! confirming call while `mcpAllowDestructive` is on, else opens the app's
//! own dialog, and Delete Permanently always opens it; `Status::operation`
//! reports it until it ends. `rewrite_sidecars`, `delete_sidecars` and
//! `sequence_timestamps` start the folder tree menu's flows (`FolderOp`):
//! with the app's dialog while `mcpAllowDestructive` is off, else reading
//! the dialog's content into a `Pending` (`Status::pending`) that a
//! confirming call runs; `cancel_operation` presses a dialog's Cancel, stops
//! a sequencing or import run or forgets the `Pending`. `import_folder` and
//! `rename_by_capture_date` start their flows the same way, the import's
//! `Pending` naming the plan its run copies. `rename_folder` previews a
//! folder rename into a `Pending` while `mcpAllowDestructive` is on, else
//! opens the tree's inline rename on the folder with the new name filled in,
//! which the user confirms with Enter. `clear_cache` is the settings' Cache
//! page's `Clear Cache`, confirmed the same way. The
//! selection is the strip's (`selection::targets`). Every request but the
//! reads (`READS`) is refused while the view drops keys (`Busy`). The
//! coverage table (`coverage`, compiled for its tests: `KEYMAP`,
//! `context_coverage`, `menu_coverage`, `folder_op_coverage`,
//! `page_coverage` and `OTHER`) maps
//! every user-facing action to its tool, to a tool refused until the native
//! app has the feature (`not_in_native`: compare, the recommended sort, the
//! Genre filter, file rename, the ranking and the AI switch), or to the
//! reason it has none.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    hash::Hash,
    path::PathBuf,
    sync::Arc,
};

use gpui_kit::component::ThemeMode;
use handpic_backend::shortcuts::Keymap;
use handpic_backend::{
    import::{self as importer, ImportSettings},
    index::{flag_name, parse_flag},
    ondemand::EyesJudgment,
};
use handpic_core::{Flag, i18n, xmp::LabelNames};
use serde_json::{Map, Value, json};

use crate::{
    burst::BurstMember,
    cache, context, datefolder,
    dialog::{Button, Dialog},
    exif::{GROUPS, Group, Groups},
    filter::{self, Facts, FilterState, OutputState},
    history::Top,
    import,
    judgment::State,
    keys,
    menu::{self, Toggle},
    panels::Panels,
    rename,
    selection::{self, Selection},
    sequence,
    settings::Settings,
    settingsmodal::{self, Edit, FORMATS, LABEL_COLORS, MODES},
    settingsview::Change as SettingChange,
    sidecars::{self, Kind},
    sort::SortKey,
    theme,
    trash::{self, Mode, Target},
    tree::{self, IGNORE_CASE, Relation, Row},
    viewonly,
};

/// The features of the Tauri app the native app does not have yet, each
/// with its tool, which refuses with `not_in_native` (D6).
const COMPARE: &str = "compare";
const RECOMMENDED: &str = "the recommended sort";
const GENRE: &str = "the Genre filter";
const FILE_RENAME: &str = "file rename";
const RANKING: &str = "the personal ranking";
const AI_SWITCH: &str = "the AI switch";
/// The tools whose whole call is a feature not in the native app yet.
const UNPORTED_TOOLS: [(&str, &str); 2] =
    [("rename_file", FILE_RENAME), ("refit_ranking", RANKING)];
const CLEAR_RUNNING: &str = "a Clear Cache is under way in Handpic; try again once it ends";
const NOTHING_SHOWN: &str = "no photo is shown in Handpic";
/// `viewonly.ts`'s `VIEW_ONLY_REFUSAL`.
const VIEW_ONLY_REFUSAL: &str = "culling does not apply to a JPEG folder";

/// The one refusal of a feature the native app does not have yet.
pub fn not_in_native(feature: &str) -> String {
    format!("{feature} is not implemented in Handpic's native app yet")
}

/// The coverage table: how an MCP client reaches every user-facing action
/// of the native app. Only its tests read it, which keep it complete.
#[cfg(test)]
pub mod coverage {
    use super::{AI_SWITCH, COMPARE, FILE_RENAME, FolderOp, GENRE, RANKING, RECOMMENDED};
    use crate::{
        context,
        menu::{self, Toggle},
        settingsmodal::Tab,
    };

    /// How an MCP client reaches one user-facing action of the native app.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Coverage {
        /// The tool that performs it.
        Tool(&'static str),
        /// The tool that refuses it with `not_in_native(feature)` until the
        /// native app has the feature (D6), then performs it.
        NotImplemented {
            tool: &'static str,
            feature: &'static str,
        },
        /// No tool, for this reason.
        Excluded(&'static str),
    }

    const CLIPBOARD: &str = "a client has the paths already, and a tool would write the user's \
         clipboard unseen";
    const MCP_GRANT: &str = "the user's to grant, on the MCP page of the settings";

    /// Every keymap action (`shortcuts.rs`'s `DEFAULTS`), in its order.
    pub const KEYMAP: &[(&str, Coverage)] = &[
        ("previous", Coverage::Tool("navigate")),
        ("next", Coverage::Tool("navigate")),
        ("burstPrevious", Coverage::Tool("navigate")),
        ("burstNext", Coverage::Tool("navigate")),
        ("burstFramePrevious", Coverage::Tool("navigate")),
        ("burstFrameNext", Coverage::Tool("navigate")),
        ("extendPrevious", Coverage::Tool("select_photos")),
        ("extendNext", Coverage::Tool("select_photos")),
        ("selectAll", Coverage::Tool("select_all")),
        ("open", Coverage::Tool("open_folder")),
        ("undo", Coverage::Tool("undo")),
        ("redo", Coverage::Tool("redo")),
        ("toggleLeft", Coverage::Tool("set_panes")),
        ("toggleRight", Coverage::Tool("set_panes")),
        ("toggleStrip", Coverage::Tool("set_panes")),
        ("toggleSides", Coverage::Tool("set_panes")),
        ("focus", Coverage::Tool("set_overlays")),
        ("zoom", Coverage::Tool("set_view")),
        ("grayscale", Coverage::Tool("set_overlays")),
        (
            "compare",
            Coverage::NotImplemented {
                tool: "set_view",
                feature: COMPARE,
            },
        ),
        ("performance", Coverage::Tool("set_overlays")),
        ("peaking", Coverage::Tool("set_overlays")),
        ("rate1", Coverage::Tool("set_judgment")),
        ("rate2", Coverage::Tool("set_judgment")),
        ("rate3", Coverage::Tool("set_judgment")),
        ("rate4", Coverage::Tool("set_judgment")),
        ("rate5", Coverage::Tool("set_judgment")),
        ("reject", Coverage::Tool("set_judgment")),
        ("rejectRest", Coverage::Tool("reject_rest")),
        ("pick", Coverage::Tool("set_judgment")),
        ("unflag", Coverage::Tool("set_judgment")),
        ("clear", Coverage::Tool("set_judgment")),
        ("red", Coverage::Tool("set_judgment")),
        ("orange", Coverage::Tool("set_judgment")),
        ("yellow", Coverage::Tool("set_judgment")),
        ("green", Coverage::Tool("set_judgment")),
        ("blue", Coverage::Tool("set_judgment")),
        ("pink", Coverage::Tool("set_judgment")),
        ("purple", Coverage::Tool("set_judgment")),
        ("clearlabel", Coverage::Tool("set_judgment")),
        ("clearall", Coverage::Tool("set_judgment")),
    ];

    /// Each right-click menu item.
    pub fn context_coverage(action: context::Action) -> Coverage {
        use context::Action as A;
        match action {
            A::RevealFolder | A::RevealFile => Coverage::Tool("reveal"),
            A::CopyPath | A::CopyFolderName | A::CopyFileName => Coverage::Excluded(CLIPBOARD),
            A::RefreshFolder | A::ExpandAll | A::CollapseAll => Coverage::Tool("tree_folder"),
            A::RenameFolder => Coverage::Tool("rename_folder"),
            A::RenameByDate => Coverage::Tool("rename_by_capture_date"),
            A::RewriteSidecars => Coverage::Tool("rewrite_sidecars"),
            A::DeleteSidecars => Coverage::Tool("delete_sidecars"),
            A::SequenceTimestamps => Coverage::Tool("sequence_timestamps"),
            A::ImportFolder => Coverage::Tool("import_folder"),
            A::SelectAll => Coverage::Tool("select_all"),
            A::Judge(_) => Coverage::Tool("set_judgment"),
            A::TrashFiles | A::DeleteFiles => Coverage::Tool("trash_photos"),
        }
    }

    /// Each Sort and Filter menu item.
    pub fn menu_coverage(action: &menu::Action) -> Coverage {
        match action {
            menu::Action::Toggle(
                Toggle::Flag(_)
                | Toggle::Stars(_)
                | Toggle::Label(_)
                | Toggle::Orientation(_)
                | Toggle::Candidate(_)
                | Toggle::Eyes(_)
                | Toggle::Output(_)
                | Toggle::Exif(..),
            )
            | menu::Action::Reset => Coverage::Tool("set_filter"),
            menu::Action::Sort(_) => Coverage::Tool("set_sort"),
        }
    }

    /// Each folder operation of the tree's menu.
    pub fn folder_op_coverage(op: &FolderOp) -> Coverage {
        Coverage::Tool(op.tool())
    }

    /// Each page of the settings modal.
    pub fn page_coverage(tab: Tab) -> Coverage {
        match tab {
            Tab::Sidecar | Tab::Culling | Tab::Shortcuts | Tab::Import | Tab::Appearance => {
                Coverage::Tool("set_settings")
            }
            Tab::Cache => Coverage::Tool("clear_cache"),
            Tab::Mcp => Coverage::Excluded(MCP_GRANT),
        }
    }

    /// The actions outside the keymap, the menus and the settings pages, and
    /// the features the native app does not have yet.
    pub const OTHER: &[(&str, Coverage)] = &[
        ("a click on a strip cell", Coverage::Tool("show_photo")),
        (
            "a Ctrl / Shift click on a strip cell",
            Coverage::Tool("select_photos"),
        ),
        ("Home / End", Coverage::Tool("navigate")),
        (
            "a click on a tree row or its expander",
            Coverage::Tool("open_folder"),
        ),
        (
            "the tree's Rename\u{2026} field",
            Coverage::Tool("rename_folder"),
        ),
        ("a dialog's Cancel", Coverage::Tool("cancel_operation")),
        (
            "the recommended sort",
            Coverage::NotImplemented {
                tool: "set_sort",
                feature: RECOMMENDED,
            },
        ),
        (
            "the Genre filter",
            Coverage::NotImplemented {
                tool: "set_filter",
                feature: GENRE,
            },
        ),
        (
            "a photo's rename",
            Coverage::NotImplemented {
                tool: "rename_file",
                feature: FILE_RENAME,
            },
        ),
        (
            "the ranking's refit",
            Coverage::NotImplemented {
                tool: "refit_ranking",
                feature: RANKING,
            },
        ),
        (
            "the AI switch",
            Coverage::NotImplemented {
                tool: "set_settings",
                feature: AI_SWITCH,
            },
        ),
        (
            "quit (q)",
            Coverage::Excluded(
                "a client would end the app the user is working in, and its own connection",
            ),
        ),
        (
            "opening the Sort, Filter and right-click menus",
            Coverage::Excluded("a menu is a way to an action, and each item has its tool"),
        ),
        (
            "opening the settings modal",
            Coverage::Excluded("get_settings and set_settings read and change what it shows"),
        ),
        (
            "the tree's keyboard cursor and type-ahead",
            Coverage::Excluded("a way to a row, which the folder tools name by path"),
        ),
        (
            "strip scrolling",
            Coverage::Excluded("pointer-only geometry: show_photo brings a photo into view"),
        ),
        (
            "the pane widths and the strip height",
            Coverage::Excluded("pointer-only geometry, set by a drag"),
        ),
        (
            "window geometry",
            Coverage::Excluded("pointer-only geometry, the window manager's"),
        ),
        (
            "the OS folder pickers",
            Coverage::Excluded("replaced by the path arguments of open_folder and set_settings"),
        ),
        (
            "drag and drop",
            Coverage::Excluded("replaced by open_folder's path"),
        ),
        (
            "mcpEnabled",
            Coverage::Excluded(
                "the user's switch on the MCP page, and a client would cut its own connection",
            ),
        ),
        ("mcpAllowDestructive", Coverage::Excluded(MCP_GRANT)),
    ];
}
const FLAGS: [&str; 3] = ["none", "pick", "reject"];
/// The requests that change nothing, answered while the view drops keys.
const READS: [&str; 4] = ["get_view", "get_status", "wait_for_idle", "get_settings"];
/// Answered while the view drops keys too: it closes the dialog that makes
/// it drop them.
const CANCEL: &str = "cancel_operation";
const ALLOW_DESTRUCTIVE_REFUSAL: &str = "mcp_allow_destructive is the user's to change, on the \
     MCP page of Handpic's settings; a client cannot set it";
/// `key_down`'s refusal of the settings while a folder operation runs, whose
/// run reads the format and the import settings.
const FOLDER_OP_REFUSAL: &str =
    "a folder operation is under way in Handpic; try again once it ends";
/// A client's confirmed run or undo would replace the user's held one.
const HELD_REFUSAL: &str =
    "an operation is held in Handpic for the scan's end; try again once it has run";
const NO_FOLDER: &str = "no folder is open in Handpic";
/// A run to the Trash, a delete or a redo out, with no dialog to refuse by.
const TRASH_RUNNING: &str =
    "a Move to Trash or Delete Permanently is under way in Handpic; try again once it ends";
/// Why a destructive operation will not run on a client's word alone.
const DESTRUCTIVE_OFF: &str = "mcp_allow_destructive is off, so only the user confirms this, in \
     Handpic's dialog: call without confirm to open it";
const DELETE_NEEDS_CLICK: &str = "Delete Permanently runs only on the user's click in Handpic's \
     dialog: call without confirm to open it";
/// Why a folder rename will not run on a client's word alone.
const RENAME_OFF: &str = "mcp_allow_destructive is off, so only the user confirms this, in the \
     folder tree: call without confirm to fill in its rename field";
const REDO_TRASH_OFF: &str = "the next redo moves files to the Trash again without a dialog, \
     which needs mcp_allow_destructive, on the MCP page of Handpic's settings; only the user \
     can turn it on";
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
    /// The folder tree's drawn rows.
    pub tree: &'a [Row],
    /// The folder the tree shows open, in its spelling.
    pub tree_open: Option<&'a str>,
    /// Nothing is under way (`Status::idle`).
    pub idle: bool,
    pub settings: &'a Settings,
    /// A folder operation is under way (`FolderOps::busy`), which keeps the
    /// settings modal from opening.
    pub folder_op: bool,
    /// A run to the Trash, a delete or a redo of a run is out.
    pub trashing: bool,
    /// The idle gate holds a confirmed operation for the scan's end, which a
    /// later request would replace.
    pub held: bool,
    /// The destructive operation a preview returned the id of.
    pub pending: Option<&'a Pending>,
    /// What `cancel_operation` cancels under way (a dialog asking, a
    /// sequencing or import run), else the name of the operation under way
    /// that cannot be canceled.
    pub cancel: Option<Result<Cancel, &'static str>>,
    /// An import is under way, which refuses a folder rename.
    pub importing: bool,
    /// A Clear Cache is held, asked or running (`ClearFlow::busy`).
    pub clearing: bool,
}

/// A destructive operation a tool previews, then runs on a confirming call
/// or through the app's dialog.
// One `Op` at a time is kept or passed by value, so its size does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Trash(Target),
    /// A folder operation, with its dialog once its preview is read.
    Folder(FolderOp, Option<Dialog>),
    /// The tree's `Rename…` of the folder `path` (the tree's spelling when
    /// it draws it) to `name`.
    Rename {
        path: String,
        name: String,
    },
    /// The settings' `Clear Cache`.
    ClearCache,
}

/// A folder operation of the tree's menu, on a folder by the caller's
/// spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderOp {
    Sidecars(sidecars::Target),
    Sequence(String),
    /// `Import…` of `dir`, with the id of the plan its preview read, which
    /// its run copies.
    Import {
        dir: String,
        plan: Option<u64>,
    },
    Date(String),
}

impl FolderOp {
    /// The operation `tool` runs on `dir`.
    fn of(tool: &str, dir: String) -> Self {
        match tool {
            "rewrite_sidecars" => FolderOp::Sidecars(sidecars::Target {
                kind: Kind::Rewrite,
                dir,
            }),
            "delete_sidecars" => FolderOp::Sidecars(sidecars::Target {
                kind: Kind::Delete,
                dir,
            }),
            "import_folder" => FolderOp::Import { dir, plan: None },
            "rename_by_capture_date" => FolderOp::Date(dir),
            _ => FolderOp::Sequence(dir),
        }
    }

    /// Its name, as the gate, the dialog and `get_status` give it.
    pub fn label(&self) -> &'static str {
        match self {
            FolderOp::Sidecars(target) => target.kind.label(),
            FolderOp::Sequence(_) => sequence::LABEL,
            FolderOp::Import { .. } => import::LABEL,
            FolderOp::Date(_) => datefolder::LABEL,
        }
    }

    pub fn dir(&self) -> &str {
        match self {
            FolderOp::Sidecars(target) => &target.dir,
            FolderOp::Sequence(dir) | FolderOp::Import { dir, .. } | FolderOp::Date(dir) => dir,
        }
    }

    /// The tree menu's item that starts it.
    pub fn action(&self) -> context::Action {
        match self {
            FolderOp::Sidecars(target) if target.kind == Kind::Rewrite => {
                context::Action::RewriteSidecars
            }
            FolderOp::Sidecars(_) => context::Action::DeleteSidecars,
            FolderOp::Sequence(_) => context::Action::SequenceTimestamps,
            FolderOp::Import { .. } => context::Action::ImportFolder,
            FolderOp::Date(_) => context::Action::RenameByDate,
        }
    }

    fn tool(&self) -> &'static str {
        match self {
            FolderOp::Sidecars(target) if target.kind == Kind::Rewrite => "rewrite_sidecars",
            FolderOp::Sidecars(_) => "delete_sidecars",
            FolderOp::Sequence(_) => "sequence_timestamps",
            FolderOp::Import { .. } => "import_folder",
            FolderOp::Date(_) => "rename_by_capture_date",
        }
    }
}

/// What a folder operation's dialog shows, top to bottom, with its run
/// button's label and whether it is enabled.
pub fn dialog_content(dialog: &Dialog) -> Map<String, Value> {
    let content = &dialog.content;
    let mut shown = Map::new();
    shown.insert("title".into(), json!(content.title));
    shown.insert("lines".into(), json!(content.lines));
    shown.insert(
        "rows".into(),
        json!(
            content
                .rows
                .iter()
                .map(|row| format!("{}{}", row.text, row.accent))
                .collect::<Vec<_>>()
        ),
    );
    shown.insert(
        "lists".into(),
        json!(
            content
                .lists
                .iter()
                .filter(|(_, items)| !items.is_empty())
                .map(|(heading, items)| json!({ "heading": heading, "items": items }))
                .collect::<Vec<_>>()
        ),
    );
    shown.insert("notes".into(), json!(content.notes));
    shown.insert("errors".into(), json!(content.errors));
    shown.insert("reason".into(), json!(content.reason));
    shown.insert("run".into(), json!(dialog.run_label));
    shown.insert("run_enabled".into(), json!(dialog.enabled(Button::Run)));
    shown
}

/// What `cancel_operation` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cancel {
    /// The trash dialog's Cancel.
    TrashDialog,
    /// The folder dialog's Cancel, or the Cancel of a sequencing or import
    /// run, with its dialog or without one.
    Folder,
    /// Forget the `Pending`.
    Pending,
    /// The Clear Cache dialog's Cancel, or the drop of a held clear.
    Clear,
}

impl Op {
    /// The tool that runs it.
    fn tool(&self) -> &'static str {
        match self {
            Op::Trash(_) => "trash_photos",
            Op::Folder(op, _) => op.tool(),
            Op::Rename { .. } => "rename_folder",
            Op::ClearCache => "clear_cache",
        }
    }

    /// What its dialog says, and what it acts on.
    fn content(&self) -> Map<String, Value> {
        match self {
            Op::Trash(target) => {
                let text = trash::dialog_text(target.mode, target.paths.len());
                let mut content = Map::new();
                content.insert("operation".into(), json!(target.mode.label()));
                content.insert("title".into(), json!(text.title));
                content.insert("text".into(), json!(text.line));
                content.insert("run".into(), json!(text.run));
                content.insert("folder".into(), json!(target.dir));
                content.insert("paths".into(), json!(target.paths));
                content
            }
            Op::Folder(op, dialog) => {
                let mut content = dialog.as_ref().map(dialog_content).unwrap_or_default();
                content.insert("operation".into(), json!(op.label()));
                content.insert("folder".into(), json!(op.dir()));
                content
            }
            Op::Rename { path, name } => {
                let mut content = Map::new();
                content.insert("operation".into(), json!(rename::LABEL));
                content.insert("folder".into(), json!(path));
                content.insert("name".into(), json!(name));
                content
            }
            Op::ClearCache => {
                let mut content = dialog_content(&cache::dialog());
                content.insert("operation".into(), json!(cache::CLEAR_LABEL));
                content
            }
        }
    }
}

/// The operation a preview returned `id` for, which a call naming that id
/// with `confirm` runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Pending {
    pub id: u64,
    pub op: Op,
}

/// A preview's reply: the dialog's content and the id to confirm it with.
pub fn previewed(pending: &Pending) -> Value {
    let mut reply = pending.op.content();
    reply.insert("id".into(), json!(pending.id));
    reply.insert(
        "next".into(),
        json!(format!(
            "call {} with this id and confirm: true to run it",
            pending.op.tool()
        )),
    );
    Value::Object(reply)
}

/// The reply once the app's dialog asks the user.
pub fn awaiting(op: &Op) -> Value {
    let mut reply = op.content();
    reply.insert("awaiting_confirmation".into(), json!(true));
    reply.insert(
        "next".into(),
        json!(if matches!(op, Op::Rename { .. }) {
            "the folder tree's rename field holds the new name: the user confirms it with Enter \
             or cancels with Escape; follow it with get_status or wait_for_idle"
        } else {
            "the user confirms or cancels in Handpic's dialog; follow it with get_status or \
             wait_for_idle"
        }),
    );
    Value::Object(reply)
}

/// The reply once a folder operation's flow started, `dialog` when the
/// app's dialog asks the user, else its preview is read into a `Pending`.
pub fn started(status: &Status, dialog: bool) -> Value {
    let mut reply = get_status(status);
    reply["next"] = json!(if dialog {
        "Handpic reads the preview, then its dialog asks the user, who confirms or cancels \
         there; get_status shows the dialog in operation.dialog, and wait_for_idle returns \
         once the user answers"
    } else {
        "Handpic reads the preview; once wait_for_idle returns, get_status's pending holds it \
         and the id to confirm it with"
    });
    reply
}

/// Where an operation `get_status` reports stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpState {
    /// Its preview is being read.
    Previewing,
    /// Its dialog asks the user.
    Awaiting,
    /// Confirmed, held by the idle gate for the listing or scan under way.
    Held,
    Running,
    /// Came back; `message` is the status line's summary.
    Finished,
    /// Came back with an error; `message` says it.
    Failed,
}

impl OpState {
    fn name(self) -> &'static str {
        match self {
            OpState::Previewing => "previewing",
            OpState::Awaiting => "awaiting_confirmation",
            OpState::Held => "held_for_scan",
            OpState::Running => "running",
            OpState::Finished => "finished",
            OpState::Failed => "failed",
        }
    }

    pub fn under_way(self) -> bool {
        matches!(
            self,
            OpState::Previewing | OpState::Awaiting | OpState::Held | OpState::Running
        )
    }
}

/// The operation under way, else the last one to come back.
#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    /// Its label, as the dialog and the pane name it (`Move to Trash`,
    /// `Undo Move to Trash`, ...).
    pub name: &'static str,
    pub state: OpState,
    /// `(done, total)` while a run reports its progress.
    pub progress: Option<(usize, usize)>,
    pub message: Option<String>,
    /// The folder dialog asking the user (`dialog_content`).
    pub dialog: Option<Map<String, Value>>,
}

/// What `get_status` reads of the view: what the status row and the pane's
/// notes tell the user.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// The open folder, as `get_view` reports it.
    pub folder: Option<String>,
    /// A folder whose open waits for the run under way.
    pub opening: Option<String>,
    /// The open folder is being listed, or listed again.
    pub listing: bool,
    /// The scan's running pass, as the status line names it, with its
    /// `(done, total)`.
    pub scan: Option<(&'static str, usize, usize)>,
    /// A run on the open folder's files is out (the idle gate's in-flight
    /// count).
    pub running: bool,
    /// The status line's text.
    pub line: String,
    /// The pane's notes, then its error lines.
    pub notes: Vec<String>,
    pub errors: Vec<String>,
    /// The operation under way, else the last one to come back.
    pub operation: Option<Operation>,
    /// The preview a confirming call runs (`previewed`).
    pub pending: Option<Value>,
}

impl Status {
    /// Nothing is under way: no listing, no scan pass, no run, no open
    /// waiting for one, no operation asking or held.
    pub fn idle(&self) -> bool {
        !self.listing
            && self.scan.is_none()
            && !self.running
            && self.opening.is_none()
            && !self
                .operation
                .as_ref()
                .is_some_and(|op| op.state.under_way())
    }
}

pub fn get_status(status: &Status) -> Value {
    json!({
        "idle": status.idle(),
        "folder": status.folder,
        "opening": status.opening,
        "listing": status.listing,
        "scan": status.scan.map(|(pass, done, total)| {
            json!({ "pass": pass, "done": done, "total": total })
        }),
        "running": status.running,
        "status": status.line,
        "notes": status.notes,
        "errors": status.errors,
        "operation": status.operation.as_ref().map(|op| json!({
            "name": op.name,
            "state": op.state.name(),
            "done": op.progress.map(|(done, _)| done),
            "total": op.progress.map(|(_, total)| total),
            "message": op.message,
            "dialog": op.dialog,
        })),
        "pending": status.pending,
    })
}

/// Which of the `f`, `e` and `d` overlays and the `g` grayscale are on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Overlays {
    pub focus: bool,
    pub peaking: bool,
    pub performance: bool,
    pub grayscale: bool,
}

/// The states in which `key_down` drops a key, which refuse every request
/// but `get_view`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Busy {
    Settings,
    Trash,
    FolderDialog,
    Rename,
    CacheDialog,
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
            Busy::CacheDialog => {
                "Handpic's Clear Cache dialog is open; try again once the user answers it"
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
    /// Open this folder as a click on its name in the tree does, and reply
    /// with `get_status`.
    Open(String),
    /// Act on the tree's row of this folder, by the tree's spelling.
    Tree {
        path: String,
        op: TreeOp,
    },
    /// Reveal this folder, or this file of the open folder, in the OS file
    /// manager.
    Reveal {
        path: String,
        folder: bool,
    },
    /// Reply with `get_status` once the view is idle, or after this many
    /// milliseconds.
    Wait(u64),
    /// Apply these settings changes in order, as the settings modal's, and
    /// reply with `get_settings`.
    Settings(Vec<SettingChange>),
    /// Open the app's dialog asking about this operation, and reply with
    /// `awaiting`.
    Ask(Op),
    /// Keep this operation as the `Pending` a new id names, and reply with
    /// `previewed`.
    Preview(Op),
    /// Run this confirmed operation, forgetting the `Pending`, and reply
    /// with `get_status`.
    Run(Op),
    /// Undo (or redo) the run to the Trash on top of the history, as the
    /// key does, and reply with `get_status`.
    StepTrash(Key),
    /// Start this folder operation's flow as the tree menu's item does, with
    /// the app's dialog when `dialog`, else reading its preview into a
    /// `Pending`, and reply with `started`.
    Folder {
        op: FolderOp,
        dialog: bool,
    },
    /// Cancel this, and reply with `get_status`.
    Cancel(Cancel),
}

/// What `tree_folder` does: the expander, `Space` on a row, and the tree
/// menu's items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeOp {
    /// Expand, listing the folder again, or collapse, as `tree_toggle` does.
    Toggle,
    Refresh,
    ExpandAll,
    CollapseAll,
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
    if let Some((_, feature)) = UNPORTED_TOOLS.iter().find(|(tool, _)| *tool == kind) {
        return Err(not_in_native(feature));
    }
    if !READS.contains(&kind)
        && kind != CANCEL
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
        "undo" => step(view, false).map(Some),
        "redo" => step(view, true).map(Some),
        "set_sort" => set_sort(view, &args["by"]),
        "set_filter" => set_filter(view, args),
        "set_panes" => set_panes(view, args),
        "set_overlays" => set_overlays(view, args),
        "get_status" => Ok(None),
        "wait_for_idle" => wait_for_idle(view, &args["timeout_ms"]),
        "open_folder" => open_folder(view, &args["path"]),
        "tree_folder" => tree_folder(view, args),
        "reveal" => reveal(view, args).map(Some),
        "get_settings" => Ok(None),
        "set_settings" => set_settings(view, args),
        "trash_photos" => trash_photos(view, args).map(Some),
        "rewrite_sidecars" | "delete_sidecars" | "sequence_timestamps" => {
            folder_operation(view, args, kind).map(Some)
        }
        "import_folder" | "rename_by_capture_date" => folder_operation(view, args, kind).map(Some),
        "rename_folder" => rename_folder(view, args).map(Some),
        "clear_cache" => clear_cache(view, args).map(Some),
        CANCEL => cancel_operation(view).map(Some),
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
/// view-only folder, then an empty history. A run's redo moves files to the
/// Trash with no dialog, so it needs `mcpAllowDestructive`.
fn step(view: &View, redo: bool) -> Result<Action, String> {
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
    let key = if redo { Key::Redo } else { Key::Undo };
    match top {
        Top::Trash if redo && !view.settings.mcp_allow_destructive => Err(REDO_TRASH_OFF.into()),
        Top::Trash if view.held => Err(HELD_REFUSAL.into()),
        Top::Trash => Ok(Action::StepTrash(key)),
        _ if view.view_only => Err(VIEW_ONLY_REFUSAL.into()),
        Top::Empty => Err(format!("nothing to {verb}")),
        Top::Judgments => Ok(Action::Key(key)),
    }
}

/// The photos `paths` names, each one the strip shows, in the order given,
/// or else the selection's targets.
fn photos(view: &View, paths: Option<&Value>) -> Result<Vec<String>, String> {
    let Some(paths) = paths else {
        let targets = targets(view);
        if targets.is_empty() {
            return Err(NOTHING_SHOWN.into());
        }
        return Ok(targets);
    };
    let mut picked: Vec<String> = Vec::new();
    for path in paths
        .as_array()
        .ok_or("paths must be an array of strings")?
    {
        let path = view.files[visible(view, path)?]
            .to_string_lossy()
            .into_owned();
        if !picked.contains(&path) {
            picked.push(path);
        }
    }
    if picked.is_empty() {
        return Err("paths must name at least one photo".into());
    }
    Ok(picked)
}

/// The strip menu's `Move to Trash…` / `Delete Permanently…` on the given
/// photos or the selection's targets. Without `confirm`, a move previews
/// while `mcpAllowDestructive` is on and otherwise, like any delete, opens
/// the app's dialog; with it, the move a preview's `id` names runs, its
/// photos still shown. A view-only folder trashes too, as its menu offers.
fn trash_photos(view: &View, args: &Value) -> Result<Action, String> {
    let mode = match given_field(args, "mode").map(Value::as_str) {
        None | Some(Some("trash")) => Mode::Trash,
        Some(Some("delete")) => Mode::Delete,
        Some(_) => return Err("mode must be trash or delete".into()),
    };
    let confirm = switch(args, "confirm")?.unwrap_or(false);
    let id = given_field(args, "id")
        .map(|id| {
            id.as_u64()
                .ok_or("id must be the number a preview returned")
        })
        .transpose()?;
    if view.trashing {
        return Err(TRASH_RUNNING.into());
    }
    if view.folder_op {
        return Err(FOLDER_OP_REFUSAL.into());
    }
    if confirm && view.held {
        return Err(HELD_REFUSAL.into());
    }
    let folder = view.folder.ok_or(NO_FOLDER)?;
    if !confirm {
        if id.is_some() {
            return Err("an id goes with confirm: true".into());
        }
        let op = Op::Trash(Target {
            dir: folder.to_owned(),
            paths: photos(view, given_field(args, "paths"))?,
            mode,
        });
        return Ok(
            if mode == Mode::Trash && view.settings.mcp_allow_destructive {
                Action::Preview(op)
            } else {
                Action::Ask(op)
            },
        );
    }
    if mode == Mode::Delete {
        return Err(DELETE_NEEDS_CLICK.into());
    }
    if !view.settings.mcp_allow_destructive {
        return Err(DESTRUCTIVE_OFF.into());
    }
    if given_field(args, "paths").is_some() {
        return Err("confirm runs the photos its preview named; give only the id".into());
    }
    let id = id.ok_or("confirm: true needs the id a preview returned")?;
    let Some(Pending {
        op: Op::Trash(target),
        ..
    }) = view.pending.filter(|pending| pending.id == id)
    else {
        return Err(format!(
            "no preview of trash_photos has id {id}; preview again"
        ));
    };
    let shown = |path: &String| {
        view.files
            .iter()
            .any(|file| file.to_string_lossy() == path.as_str())
    };
    if target.dir != folder || !target.paths.iter().all(shown) {
        return Err(
            "the previewed photos are no longer all shown in Handpic; preview again".into(),
        );
    }
    Ok(Action::Run(Op::Trash(target.clone())))
}

/// `rewrite_sidecars`, `delete_sidecars` and `sequence_timestamps`, the
/// folder tree menu's items on the folder `path`, one folder flow at a
/// time. Without `confirm`, the flow starts: with the app's dialog while
/// `mcpAllowDestructive` is off, else reading the dialog's content into a
/// `Pending`; with it, the operation a preview's `id` names runs, refused
/// when its dialog would have nothing to run.
fn folder_operation(view: &View, args: &Value, tool: &str) -> Result<Action, String> {
    let confirm = switch(args, "confirm")?.unwrap_or(false);
    let id = given_field(args, "id")
        .map(|id| {
            id.as_u64()
                .ok_or("id must be the number a preview returned")
        })
        .transpose()?;
    if view.trashing {
        return Err(TRASH_RUNNING.into());
    }
    if view.folder_op {
        return Err(FOLDER_OP_REFUSAL.into());
    }
    if !confirm {
        if id.is_some() {
            return Err("an id goes with confirm: true".into());
        }
        let path = given_field(args, "path").ok_or("path must be the folder's absolute path")?;
        let mut op = FolderOp::of(tool, folder_path(path)?);
        match &mut op {
            // Its rename renames the tree's row, by the tree's spelling.
            FolderOp::Date(dir) => {
                if let Some(row) = tree_row(view, dir) {
                    *dir = row.path.clone();
                }
            }
            FolderOp::Import { .. } => {
                let settings = ImportSettings {
                    root: view.settings.import_root.clone(),
                    pattern: view.settings.import_pattern.clone(),
                };
                if let Some(reason) = importer::refusal(&settings) {
                    return Err(format!(
                        "{reason}; set_settings' import_root and import_pattern set them"
                    ));
                }
            }
            _ => {}
        }
        // A sidecar start goes through the gate, which drops what it holds.
        if view.held && matches!(op, FolderOp::Sidecars(_)) {
            return Err(HELD_REFUSAL.into());
        }
        return Ok(Action::Folder {
            op,
            dialog: !view.settings.mcp_allow_destructive,
        });
    }
    if !view.settings.mcp_allow_destructive {
        return Err(DESTRUCTIVE_OFF.into());
    }
    if given_field(args, "path").is_some() {
        return Err("confirm runs the folder its preview named; give only the id".into());
    }
    let id = id.ok_or("confirm: true needs the id a preview returned")?;
    let Some(Pending {
        op: Op::Folder(op, Some(dialog)),
        ..
    }) = view
        .pending
        .filter(|pending| pending.id == id && pending.op.tool() == tool)
    else {
        return Err(format!("no preview of {tool} has id {id}; preview again"));
    };
    if !dialog.enabled(Button::Run) {
        return Err(dialog
            .content
            .reason
            .clone()
            .or_else(|| dialog.content.notes.first().cloned())
            .unwrap_or_else(|| "its preview has nothing to run".into()));
    }
    Ok(Action::Run(Op::Folder(op.clone(), Some(dialog.clone()))))
}

/// `rename_folder`, the tree's `Rename…` of the folder `path` to `name`,
/// whose name the backend checked (`rename::folder_target`), refused while
/// an import runs. Without `confirm`, it previews while
/// `mcpAllowDestructive` is on, else opens the tree's inline rename on the
/// folder's row with `name` filled in; with it, the rename a preview's `id`
/// names goes through the gate as a confirmed edit does.
fn rename_folder(view: &View, args: &Value) -> Result<Action, String> {
    let confirm = switch(args, "confirm")?.unwrap_or(false);
    let id = given_field(args, "id")
        .map(|id| {
            id.as_u64()
                .ok_or("id must be the number a preview returned")
        })
        .transpose()?;
    if view.importing {
        return Err(rename::RENAME_REFUSED.into());
    }
    if !confirm {
        if id.is_some() {
            return Err("an id goes with confirm: true".into());
        }
        let path = folder_path(
            given_field(args, "path").ok_or("path must be the folder's absolute path")?,
        )?;
        let typed = given_field(args, "name")
            .and_then(Value::as_str)
            .ok_or("name must be the folder's new name")?;
        let row = tree_row(view, &path);
        let current = row.map_or_else(
            || {
                std::path::Path::new(&path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            },
            |row| row.name.clone(),
        );
        let name = rename::confirm_name(&current, typed)
            .ok_or_else(|| format!("{path} is already named {}", typed.trim()))?;
        if !view.settings.mcp_allow_destructive && row.is_none() {
            return Err(format!(
                "{path} is not a folder the tree shows, where Handpic asks the user; expand its \
                 parent first"
            ));
        }
        let op = Op::Rename {
            path: row.map_or(path, |row| row.path.clone()),
            name,
        };
        return Ok(if view.settings.mcp_allow_destructive {
            Action::Preview(op)
        } else {
            Action::Ask(op)
        });
    }
    if !view.settings.mcp_allow_destructive {
        return Err(RENAME_OFF.into());
    }
    // A rename goes through the gate, which drops what it holds.
    if view.held {
        return Err(HELD_REFUSAL.into());
    }
    if given_field(args, "path").is_some() || given_field(args, "name").is_some() {
        return Err("confirm renames the folder its preview named; give only the id".into());
    }
    let id = id.ok_or("confirm: true needs the id a preview returned")?;
    match view.pending {
        Some(Pending {
            id: pending,
            op: op @ Op::Rename { .. },
        }) if *pending == id => Ok(Action::Run(op.clone())),
        _ => Err(format!(
            "no preview of rename_folder has id {id}; preview again"
        )),
    }
}

/// `clear_cache`, the settings' `Clear Cache`, one at a time and not while
/// a folder operation runs, whose flow keeps the settings shut. Without
/// `confirm`, it previews while `mcpAllowDestructive` is on and otherwise
/// opens the app's confirmation; with it, the clear a preview's `id` names
/// runs, or waits for the listing or scan under way, without a dialog.
fn clear_cache(view: &View, args: &Value) -> Result<Action, String> {
    let confirm = switch(args, "confirm")?.unwrap_or(false);
    let id = given_field(args, "id")
        .map(|id| {
            id.as_u64()
                .ok_or("id must be the number a preview returned")
        })
        .transpose()?;
    if view.clearing {
        return Err(CLEAR_RUNNING.into());
    }
    if view.folder_op {
        return Err(FOLDER_OP_REFUSAL.into());
    }
    if !confirm {
        if id.is_some() {
            return Err("an id goes with confirm: true".into());
        }
        return Ok(if view.settings.mcp_allow_destructive {
            Action::Preview(Op::ClearCache)
        } else {
            Action::Ask(Op::ClearCache)
        });
    }
    if !view.settings.mcp_allow_destructive {
        return Err(DESTRUCTIVE_OFF.into());
    }
    let id = id.ok_or("confirm: true needs the id a preview returned")?;
    match view.pending {
        Some(Pending {
            id: pending,
            op: Op::ClearCache,
        }) if *pending == id => Ok(Action::Run(Op::ClearCache)),
        _ => Err(format!(
            "no preview of clear_cache has id {id}; preview again"
        )),
    }
}

/// `cancel_operation`: what the user's Cancel would cancel now, else the
/// `Pending`; refused naming an operation under way that cannot be
/// canceled, or with nothing to cancel.
fn cancel_operation(view: &View) -> Result<Action, String> {
    match view.cancel {
        Some(Ok(cancel)) => Ok(Action::Cancel(cancel)),
        _ if view.pending.is_some() => Ok(Action::Cancel(Cancel::Pending)),
        Some(Err(name)) => Err(format!(
            "{name} is under way and cannot be canceled; wait_for_idle waits for its end"
        )),
        None => Err("nothing to cancel in Handpic".into()),
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
            "grayscale": view.overlays.grayscale,
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
        return Err(not_in_native(COMPARE));
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
        Some("recommended") => return Err(not_in_native(RECOMMENDED)),
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
    if given_field(args, "genres").is_some() {
        return Err(not_in_native(GENRE));
    }
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
    let (focus, peaking, performance, grayscale) = (
        switch(args, "focus")?,
        switch(args, "peaking")?,
        switch(args, "performance")?,
        switch(args, "grayscale")?,
    );
    if focus.is_none() && peaking.is_none() && performance.is_none() && grayscale.is_none() {
        return Err("set at least one of focus, peaking, performance, grayscale".into());
    }
    let on = view.overlays;
    let flips = Overlays {
        focus: focus.is_some_and(|f| f != on.focus),
        peaking: peaking.is_some_and(|p| p != on.peaking),
        performance: performance.is_some_and(|p| p != on.performance),
        grayscale: grayscale.is_some_and(|g| g != on.grayscale),
    };
    Ok((flips != Overlays::default()).then_some(Action::Overlays(flips)))
}

/// No wait when the view is idle already.
fn wait_for_idle(view: &View, timeout_ms: &Value) -> Result<Option<Action>, String> {
    let timeout_ms = timeout_ms
        .as_u64()
        .ok_or("timeout_ms must be a number of milliseconds")?;
    Ok((!view.idle).then_some(Action::Wait(timeout_ms)))
}

/// A folder path without the Windows verbatim prefix, as the tree spells
/// folders.
fn folder_path(path: &Value) -> Result<String, String> {
    path.as_str()
        .map(context::shown_path)
        .ok_or_else(|| "path must be a string".into())
}

/// The folder the tree shows open is not opened again, as a click on its
/// row only expands or collapses it.
fn open_folder(view: &View, path: &Value) -> Result<Option<Action>, String> {
    let path = folder_path(path)?;
    let open = view
        .tree_open
        .is_some_and(|open| tree::relation(open, &path, IGNORE_CASE) == Some(Relation::Same));
    Ok((!open).then_some(Action::Open(path)))
}

/// The tree's row of `path`, compared as the tree compares paths.
fn tree_row<'v>(view: &'v View, path: &str) -> Option<&'v Row> {
    view.tree
        .iter()
        .find(|row| tree::relation(&row.path, path, IGNORE_CASE) == Some(Relation::Same))
}

/// The row's action, nothing when it is already expanded or collapsed as
/// asked; `Expand All` and `Collapse All` are the menu's below a root only.
fn tree_folder(view: &View, args: &Value) -> Result<Option<Action>, String> {
    let wanted = folder_path(&args["path"])?;
    let action = args["action"].as_str().unwrap_or_default();
    let row = tree_row(view, &wanted).ok_or_else(|| {
        format!("{wanted} is not a folder the tree shows; expand its parent first")
    })?;
    let op = match action {
        "expand" | "collapse" => {
            if row.expanded == (action == "expand") {
                return Ok(None);
            }
            TreeOp::Toggle
        }
        "refresh" => TreeOp::Refresh,
        "expand_all" | "collapse_all" if row.depth == 0 => {
            return Err(format!(
                "{action} does not apply to a root, as the tree's menu offers it only below one"
            ));
        }
        "expand_all" => TreeOp::ExpandAll,
        "collapse_all" => TreeOp::CollapseAll,
        _ => {
            return Err(
                "action must be one of expand, collapse, refresh, expand_all, collapse_all".into(),
            );
        }
    };
    Ok(Some(Action::Tree {
        path: row.path.clone(),
        op,
    }))
}

/// `tree_folder`'s reply, also when it changed nothing: the row of `path`
/// by the tree's spelling and whether it is expanded.
pub fn tree_reply(view: &View, path: &Value) -> Value {
    let path = folder_path(path).unwrap_or_default();
    let row = tree_row(view, &path);
    json!({
        "path": row.map_or(path.as_str(), |row| row.path.as_str()),
        "expanded": row.is_some_and(|row| row.expanded),
    })
}

/// A folder anywhere, or a file the strip shows, as the strip's menu
/// reveals only those.
fn reveal(view: &View, args: &Value) -> Result<Action, String> {
    let folder = args["folder"]
        .as_bool()
        .ok_or("folder must be true or false")?;
    let path = if folder {
        folder_path(&args["path"])?
    } else {
        view.files[visible(view, &args["path"])?]
            .to_string_lossy()
            .into_owned()
    };
    Ok(Action::Reveal { path, folder })
}

/// What the settings modal shows: the sidecar format, the label names with
/// the Lightroom language whose preset they are, auto-advance, the keymap,
/// the import root and pattern, the theme and the themes to choose from, and
/// `mcpAllowDestructive`.
pub fn get_settings(settings: &Settings) -> Value {
    let names: Map<String, Value> = LABEL_COLORS
        .iter()
        .zip(settingsmodal::label_texts(&settings.names))
        .map(|(color, name)| (color.to_lowercase(), name.into()))
        .collect();
    let theme = settings.theme;
    json!({
        "sidecar_format": settings.format.setting(),
        "sidecar_formats": FORMATS.iter().map(|(format, _)| format.setting()).collect::<Vec<_>>(),
        "label_names": names,
        "label_language": settingsmodal::preset_of(&settings.names)
            .map(|at| i18n::presets()[at].code.clone()),
        "label_languages": i18n::presets()
            .iter()
            .map(|preset| json!({ "code": preset.code, "name": preset.name }))
            .collect::<Vec<_>>(),
        "auto_advance": settings.auto_advance,
        "shortcuts": settings
            .keymap
            .bindings()
            .iter()
            .map(|b| json!({
                "action": b.action,
                "label": settingsmodal::shortcut_label(b.action),
                "keys": b.keys,
            }))
            .collect::<Vec<_>>(),
        "import_root": (!settings.import_root.is_empty()).then_some(&settings.import_root),
        "import_pattern": settings.import_pattern,
        "theme": {
            "mode": theme.mode.setting(),
            "light": theme.light,
            "dark": theme.dark,
        },
        "themes": {
            "light": theme::names(ThemeMode::Light).collect::<Vec<_>>(),
            "dark": theme::names(ThemeMode::Dark).collect::<Vec<_>>(),
        },
        "mcp_allow_destructive": settings.mcp_allow_destructive,
    })
}

/// A string field `args` gives, `None` when left out.
fn text<'v>(args: &'v Value, name: &str) -> Result<Option<&'v str>, String> {
    given_field(args, name)
        .map(|v| v.as_str().ok_or_else(|| format!("{name} must be a string")))
        .transpose()
}

/// The label names `label_names` or `label_language` asks for, the colors
/// left out keeping theirs, normalized as a commit of the fields is.
fn wanted_names(settings: &Settings, args: &Value) -> Result<Option<LabelNames>, String> {
    let language = text(args, "label_language")?;
    let Some(names) = given_field(args, "label_names") else {
        return language
            .map(|code| {
                i18n::presets()
                    .iter()
                    .find(|preset| preset.code == code)
                    .map(|preset| preset.names.clone())
                    .ok_or_else(|| format!("{code:?} is not one of get_settings' label_languages"))
            })
            .transpose();
    };
    if language.is_some() {
        return Err("set label_names or label_language, not both".into());
    }
    let names = names
        .as_object()
        .ok_or("label_names must be an object of color names")?;
    let colors: Vec<String> = LABEL_COLORS.iter().map(|c| c.to_lowercase()).collect();
    if let Some(other) = names.keys().find(|key| !colors.contains(key)) {
        return Err(format!(
            "{other:?} is not one of the label colors: red, yellow, green, blue, purple"
        ));
    }
    let current = settingsmodal::label_texts(&settings.names);
    let texts = colors
        .iter()
        .zip(current)
        .map(|(color, current)| match names.get(color) {
            None => Ok(current),
            Some(name) => name
                .as_str()
                .ok_or_else(|| format!("label_names.{color} must be a string")),
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Some(settingsmodal::label_names_of(std::array::from_fn(
        |at| texts[at],
    ))))
}

/// `key` is spelled as `keys::key_name` names a keystroke: the modifiers in
/// their order, then one key, which a key press can match.
fn named_key(key: &str) -> bool {
    let stroke = keys::keystroke_of(key);
    keys::key_name(&stroke).as_deref() == Some(key)
        && !stroke.key.is_empty()
        && (stroke.key == "+" || !stroke.key.contains('+'))
}

/// The keymap after `shortcuts`' edits in order, each refused as the
/// Keyboard Shortcuts page refuses it.
fn edited_keymap(settings: &Settings, shortcuts: &Value) -> Result<Keymap, String> {
    let edits = shortcuts
        .as_array()
        .ok_or("shortcuts must be a list of edits")?;
    let mut keymap = settings.keymap.clone();
    let bindings = keymap.bindings();
    for edit in edits {
        let op = text(edit, "op")?.ok_or("each shortcut edit needs its op")?;
        let action = text(edit, "action")?
            .map(|action| {
                bindings
                    .iter()
                    .find(|b| b.action == action)
                    .map(|b| b.action)
                    .ok_or_else(|| format!("unknown action {action:?}"))
            })
            .transpose()?;
        let key = text(edit, "key")?;
        let edit = match (op, action, key) {
            ("reset_all", None, None) => Edit::ResetAll,
            ("reset", Some(action), None) => Edit::Reset(action),
            ("remove", Some(action), Some(key)) => Edit::Remove(action, key.to_owned()),
            ("add", Some(action), Some(key)) => {
                if !named_key(key) {
                    return Err(format!(
                        "{key:?} is not a key as Handpic names it, such as k, shift+x, ctrl+z \
                         or arrowright"
                    ));
                }
                Edit::Add(action, key.to_owned())
            }
            ("add" | "remove", ..) => {
                return Err(format!("a shortcut {op} needs an action and a key"));
            }
            ("reset", ..) => return Err("a shortcut reset needs an action and no key".into()),
            ("reset_all", ..) => return Err("a shortcut reset_all takes no action or key".into()),
            _ => {
                return Err(format!(
                    "{op:?} is not a shortcut op: add, remove, reset or reset_all"
                ));
            }
        };
        settingsmodal::edit(&mut keymap, edit)?;
    }
    Ok(keymap)
}

/// The theme setting `theme`'s fields ask for over `current`, each through
/// the Appearance page's rule.
fn wanted_theme(current: theme::Setting, args: &Value) -> Result<theme::Setting, String> {
    if !args.is_object() {
        return Err("theme must be an object of mode, light and dark".into());
    }
    let mut setting = current;
    if let Some(mode) = text(args, "mode")? {
        let (chosen, _) = MODES
            .iter()
            .find(|(m, _)| m.setting() == mode)
            .ok_or_else(|| format!("{mode:?} is not a theme mode: system, light or dark"))?;
        setting = settingsmodal::mode_change(setting, *chosen).unwrap_or(setting);
    }
    for (field, mode) in [("light", ThemeMode::Light), ("dark", ThemeMode::Dark)] {
        let Some(name) = text(args, field)? else {
            continue;
        };
        if !theme::names(mode).any(|n| n == name) {
            return Err(format!(
                "{name:?} is not one of get_settings' themes.{field}"
            ));
        }
        setting = settingsmodal::theme_change(setting, mode, name).unwrap_or(setting);
    }
    Ok(setting)
}

/// The settings modal's changes a `set_settings` call asks for, by the
/// modal's rules; the whole call is refused when one field is. A call that
/// changes nothing answers no action.
fn set_settings(view: &View, args: &Value) -> Result<Option<Action>, String> {
    if args.get("mcp_allow_destructive").is_some() {
        return Err(ALLOW_DESTRUCTIVE_REFUSAL.into());
    }
    if args.get("ai_enabled").is_some() {
        return Err(not_in_native(AI_SWITCH));
    }
    if view.folder_op {
        return Err(FOLDER_OP_REFUSAL.into());
    }
    let settings = view.settings;
    let mut changes = Vec::new();
    let mut format = settings.format;
    if let Some(name) = text(args, "sidecar_format")? {
        let (chosen, _) = FORMATS
            .iter()
            .find(|(f, _)| f.setting() == name)
            .ok_or_else(|| format!("{name:?} is not a sidecar format: xmp, dop or both"))?;
        if let Some(chosen) = settingsmodal::format_change(format, *chosen) {
            format = chosen;
            changes.push(SettingChange::SidecarFormat(chosen));
        }
    }
    if let Some(names) = wanted_names(settings, args)? {
        if !settingsmodal::writes_xmp(format) {
            return Err(
                "the label names apply only while the sidecar format writes XMP (xmp or both)"
                    .into(),
            );
        }
        if let Some(names) = settingsmodal::label_change(&settings.names, names) {
            changes.push(SettingChange::LabelNames(names));
        }
    }
    if let Some(on) = switch(args, "auto_advance")?
        && on != settings.auto_advance
    {
        changes.push(SettingChange::AutoAdvance(on));
    }
    if let Some(shortcuts) = given_field(args, "shortcuts") {
        let keymap = edited_keymap(settings, shortcuts)?;
        if keymap != settings.keymap {
            changes.push(SettingChange::Keymap(keymap));
        }
    }
    if let Some(root) = text(args, "import_root")?
        && root != settings.import_root
    {
        changes.push(SettingChange::ImportRoot(root.to_owned()));
    }
    if let Some(pattern) = text(args, "import_pattern")?
        && let Some(pattern) = settingsmodal::pattern_change(&settings.import_pattern, pattern)?
    {
        changes.push(SettingChange::ImportPattern(pattern));
    }
    if let Some(wanted) = given_field(args, "theme") {
        let setting = wanted_theme(settings.theme, wanted)?;
        if setting != settings.theme {
            changes.push(SettingChange::Theme(setting));
        }
    }
    Ok((!changes.is_empty()).then_some(Action::Settings(changes)))
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
    use super::{
        Action, Busy, Cancel, Change, FolderOp, Key, Op, OpState, Operation, Overlays, Pending,
        Status, TreeOp, View, answer, awaiting,
        coverage::{
            Coverage, KEYMAP, OTHER, context_coverage, folder_op_coverage, menu_coverage,
            page_coverage,
        },
        get_settings, get_status, get_view, not_in_native, previewed, started, tree_reply,
    };
    use crate::{
        burst::BurstMember,
        context,
        dialog::{Content, Dialog, Style},
        exif::{Group, Groups},
        filter::{EyesItem, Facts, FilterFlag, FilterState, OutputState},
        history::Top,
        judgment::State,
        menu::{self, Toggle},
        panels::Panels,
        selection::{Selection, selection_of},
        settings::{self, Settings},
        settingsmodal::{Edit, TABS, edit},
        settingsview::Change as SettingChange,
        sort::SortKey,
        theme::{self, Mode, NEUTRAL},
        trash::{self, Target},
        tree::Row,
    };
    use handpic_backend::{
        index::EyesPose,
        ondemand::{EyesJudgment, EyesMesh},
    };
    use handpic_backend::{shortcuts::Keymap, sidecar::SidecarFormat};
    use handpic_core::{Flag, i18n, xmp::LabelNames};
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
    static DEFAULT_SETTINGS: LazyLock<Settings> = LazyLock::new(settings::defaults);
    static ALLOW_DESTRUCTIVE: LazyLock<Settings> = LazyLock::new(|| Settings {
        mcp_allow_destructive: true,
        ..settings::defaults()
    });

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
            tree: &[],
            tree_open: None,
            idle: true,
            settings: &DEFAULT_SETTINGS,
            folder_op: false,
            trashing: false,
            held: false,
            pending: None,
            cancel: None,
            importing: false,
            clearing: false,
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
                "overlays": {
                    "focus": false,
                    "peaking": false,
                    "performance": false,
                    "grayscale": false,
                },
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
    fn undo_steps_a_trash_run_and_redo_only_while_destructive_operations_are_allowed() {
        let trash = View {
            undo: Top::Trash,
            redo: Top::Trash,
            view_only: true,
            ..view(&[])
        };
        assert_eq!(
            ask("undo", json!({}), &trash),
            Ok(Some(Action::StepTrash(Key::Undo)))
        );
        assert!(
            ask("redo", json!({}), &trash)
                .unwrap_err()
                .starts_with("the next redo moves files to the Trash again without a dialog")
        );
        let allowed = View {
            settings: &ALLOW_DESTRUCTIVE,
            ..trash
        };
        assert_eq!(
            ask("redo", json!({}), &allowed),
            Ok(Some(Action::StepTrash(Key::Redo)))
        );
    }

    #[test]
    fn undo_and_redo_refuse_an_empty_history_and_a_view_only_folder() {
        for verb in ["undo", "redo"] {
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
            (
                Busy::CacheDialog,
                "Handpic's Clear Cache dialog is open; try again once the user answers it",
            ),
        ] {
            let view = View {
                busy: Some(busy),
                undo: Top::Judgments,
                ..view(&files)
            };
            assert_eq!(ask("get_view", json!({}), &view), Ok(None));
            assert_eq!(ask("get_status", json!({}), &view), Ok(None));
            assert_eq!(
                ask("wait_for_idle", json!({ "timeout_ms": 10 }), &view),
                Ok(None)
            );
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
                ("open_folder", json!({ "path": "/e" })),
                ("tree_folder", json!({ "path": "/d", "action": "refresh" })),
                ("reveal", json!({ "path": "/d", "folder": true })),
                ("trash_photos", json!({})),
                ("clear_cache", json!({})),
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
            Err("compare is not implemented in Handpic's native app yet".into())
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
            json!({ "focus": false, "peaking": true, "performance": false, "grayscale": false })
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
            Err("set at least one of focus, peaking, performance, grayscale".into())
        );
    }

    #[test]
    fn set_overlays_latches_the_grayscale_only_when_it_differs() {
        let gray = View {
            overlays: Overlays {
                grayscale: true,
                ..Overlays::default()
            },
            ..view(&[])
        };
        let flip = Ok(Some(Action::Overlays(Overlays {
            grayscale: true,
            ..Overlays::default()
        })));
        assert_eq!(
            ask("set_overlays", json!({ "grayscale": true }), &view(&[])),
            flip
        );
        assert_eq!(
            ask("set_overlays", json!({ "grayscale": true }), &gray),
            Ok(None)
        );
        assert_eq!(
            ask("set_overlays", json!({ "grayscale": false }), &gray),
            flip
        );
        assert_eq!(get_view(&gray)["overlays"]["grayscale"], json!(true));
    }

    /// Every tool a table entry names, and the feature each not-implemented
    /// one is refused for.
    fn covered() -> Vec<Coverage> {
        let mut all: Vec<Coverage> = KEYMAP.iter().chain(OTHER).map(|(_, c)| *c).collect();
        all.extend(
            [
                context::Action::RevealFolder,
                context::Action::CopyPath,
                context::Action::CopyFolderName,
                context::Action::RefreshFolder,
                context::Action::ExpandAll,
                context::Action::CollapseAll,
                context::Action::RenameFolder,
                context::Action::RenameByDate,
                context::Action::RewriteSidecars,
                context::Action::DeleteSidecars,
                context::Action::SequenceTimestamps,
                context::Action::ImportFolder,
                context::Action::SelectAll,
                context::Action::RevealFile,
                context::Action::CopyFileName,
                context::Action::Judge("pick"),
                context::Action::TrashFiles,
                context::Action::DeleteFiles,
            ]
            .map(context_coverage),
        );
        all.extend(
            [
                menu::Action::Toggle(Toggle::Stars(5)),
                menu::Action::Reset,
                menu::Action::Sort(SortKey::Rating),
            ]
            .iter()
            .map(menu_coverage),
        );
        all.extend(
            [
                FolderOp::Sequence("/d".into()),
                FolderOp::Import {
                    dir: "/d".into(),
                    plan: None,
                },
                FolderOp::Date("/d".into()),
                FolderOp::Sidecars(crate::sidecars::Target {
                    kind: crate::sidecars::Kind::Delete,
                    dir: "/d".into(),
                }),
            ]
            .iter()
            .map(folder_op_coverage),
        );
        all.extend(TABS.map(page_coverage));
        all
    }

    #[test]
    fn the_coverage_table_names_every_keymap_action_once() {
        let keymap: Vec<&str> = Keymap::defaults()
            .bindings()
            .into_iter()
            .map(|b| b.action)
            .collect();
        let table: Vec<&str> = KEYMAP.iter().map(|(action, _)| *action).collect();
        assert_eq!(table, keymap);
    }

    #[test]
    fn every_covered_tool_is_one_the_companion_answers() {
        let files = paths(&["/d/a"]);
        let view = view(&files);
        for coverage in covered() {
            let tool = match coverage {
                Coverage::Tool(tool) | Coverage::NotImplemented { tool, .. } => tool,
                Coverage::Excluded(reason) => {
                    assert!(!reason.is_empty());
                    continue;
                }
            };
            if let Err(e) = ask(tool, json!({}), &view) {
                assert_ne!(e, format!("unknown request: {tool}"));
            }
        }
    }

    #[test]
    fn every_feature_not_in_the_native_app_is_refused_with_the_shared_message() {
        let files = paths(&["/d/a", "/d/b"]);
        let view = view(&files);
        let calls = [
            ("compare", "set_view", json!({ "mode": "compare" })),
            (
                "the recommended sort",
                "set_sort",
                json!({ "by": "recommended" }),
            ),
            (
                "the Genre filter",
                "set_filter",
                json!({ "genres": ["Landscape"] }),
            ),
            (
                "file rename",
                "rename_file",
                json!({ "path": "/d/a", "name": "b" }),
            ),
            ("the personal ranking", "refit_ranking", json!({})),
            (
                "the AI switch",
                "set_settings",
                json!({ "ai_enabled": true }),
            ),
        ];
        let mut listed: Vec<(&str, &str)> = covered()
            .into_iter()
            .filter_map(|c| match c {
                Coverage::NotImplemented { tool, feature } => Some((feature, tool)),
                _ => None,
            })
            .collect();
        listed.sort();
        listed.dedup();
        let mut called: Vec<(&str, &str)> = calls.iter().map(|(f, t, _)| (*f, *t)).collect();
        called.sort();
        assert_eq!(listed, called);
        for (feature, tool, args) in calls {
            let refusal = format!("{feature} is not implemented in Handpic's native app yet");
            assert_eq!(not_in_native(feature), refusal);
            assert_eq!(ask(tool, args, &view), Err(refusal), "{tool}");
        }
        // A whole tool not in the native app is refused even while busy.
        let busy = View {
            busy: Some(Busy::Settings),
            ..view
        };
        assert_eq!(
            ask("refit_ranking", json!({}), &busy),
            Err(not_in_native("the personal ranking"))
        );
    }

    #[test]
    fn clear_cache_previews_or_asks_by_the_grant_and_runs_on_the_previewed_id() {
        assert_eq!(
            ask("clear_cache", json!({}), &view(&[])),
            Ok(Some(Action::Ask(Op::ClearCache)))
        );
        let allowed = View {
            folder: None,
            settings: &ALLOW_DESTRUCTIVE,
            ..view(&[])
        };
        assert_eq!(
            ask("clear_cache", json!({}), &allowed),
            Ok(Some(Action::Preview(Op::ClearCache)))
        );
        let pending = Pending {
            id: 4,
            op: Op::ClearCache,
        };
        let previewed_view = View {
            pending: Some(&pending),
            ..allowed
        };
        assert_eq!(
            ask(
                "clear_cache",
                json!({ "id": 4, "confirm": true }),
                &previewed_view
            ),
            Ok(Some(Action::Run(Op::ClearCache)))
        );
        assert_eq!(
            ask(
                "clear_cache",
                json!({ "id": 5, "confirm": true }),
                &previewed_view
            ),
            Err("no preview of clear_cache has id 5; preview again".into())
        );
        assert_eq!(
            ask("clear_cache", json!({ "id": 4 }), &previewed_view),
            Err("an id goes with confirm: true".into())
        );
        assert_eq!(
            ask("clear_cache", json!({ "confirm": true }), &previewed_view),
            Err("confirm: true needs the id a preview returned".into())
        );
        let off = View {
            pending: Some(&pending),
            ..view(&[])
        };
        assert!(
            ask("clear_cache", json!({ "id": 4, "confirm": true }), &off)
                .unwrap_err()
                .starts_with("mcp_allow_destructive is off")
        );
        let clearing = View {
            clearing: true,
            ..view(&[])
        };
        assert_eq!(
            ask("clear_cache", json!({}), &clearing),
            Err("a Clear Cache is under way in Handpic; try again once it ends".into())
        );
        let folder_op = View {
            folder_op: true,
            ..view(&[])
        };
        assert_eq!(
            ask("clear_cache", json!({}), &folder_op),
            Err("a folder operation is under way in Handpic; try again once it ends".into())
        );
        let preview = previewed(&pending);
        assert_eq!(preview["operation"], json!("Clear Cache"));
        assert_eq!(preview["title"], json!("Clear the index cache?"));
        assert_eq!(preview["run"], json!("Clear"));
        assert_eq!(preview["id"], json!(4));
    }

    #[test]
    fn cancel_operation_cancels_a_clear() {
        let view = View {
            cancel: Some(Ok(Cancel::Clear)),
            busy: Some(Busy::CacheDialog),
            ..view(&[])
        };
        assert_eq!(
            ask("cancel_operation", json!({}), &view),
            Ok(Some(Action::Cancel(Cancel::Clear)))
        );
    }

    fn row(path: &str, depth: usize, expanded: bool) -> Row {
        Row {
            depth,
            path: path.to_owned(),
            name: path.rsplit('/').next().unwrap_or(path).to_owned(),
            expanded,
            expandable: true,
            raw_count: None,
            failed: None,
        }
    }

    #[test]
    fn get_status_reports_the_scan_pass_the_line_and_the_notes() {
        let status = Status {
            folder: Some("/d".into()),
            scan: Some(("analyzing", 4, 9)),
            line: "a.ARW  analyzing 4/9".into(),
            notes: vec!["Move to Trash: waiting for the scan to finish".into()],
            errors: vec!["a.ARW: denied".into()],
            ..Status::default()
        };
        assert_eq!(
            get_status(&status),
            json!({
                "idle": false,
                "folder": "/d",
                "opening": null,
                "listing": false,
                "scan": { "pass": "analyzing", "done": 4, "total": 9 },
                "running": false,
                "status": "a.ARW  analyzing 4/9",
                "notes": ["Move to Trash: waiting for the scan to finish"],
                "errors": ["a.ARW: denied"],
                "operation": null,
                "pending": null,
            })
        );
    }

    #[test]
    fn get_status_reports_the_operation_and_waits_while_it_asks_is_held_or_runs() {
        let operation = |state, progress, message: Option<&str>| Status {
            operation: Some(Operation {
                name: "Move to Trash",
                state,
                progress,
                message: message.map(str::to_owned),
                dialog: None,
            }),
            ..Status::default()
        };
        let running = operation(OpState::Running, Some((2, 5)), None);
        assert_eq!(
            get_status(&running)["operation"],
            json!({
                "name": "Move to Trash",
                "state": "running",
                "done": 2,
                "total": 5,
                "message": null,
                "dialog": null,
            })
        );
        assert!(!running.idle());
        for (state, name) in [
            (OpState::Previewing, "previewing"),
            (OpState::Awaiting, "awaiting_confirmation"),
            (OpState::Held, "held_for_scan"),
        ] {
            let status = operation(state, None, None);
            assert_eq!(get_status(&status)["operation"]["state"], json!(name));
            assert!(!status.idle());
        }
        for (state, name) in [(OpState::Finished, "finished"), (OpState::Failed, "failed")] {
            let status = operation(state, None, Some("Moved 2 files to the Trash"));
            assert_eq!(get_status(&status)["operation"]["state"], json!(name));
            assert_eq!(
                get_status(&status)["operation"]["message"],
                json!("Moved 2 files to the Trash")
            );
            assert!(status.idle());
        }
    }

    fn target(paths: &[&str], mode: trash::Mode) -> Target {
        Target {
            dir: "/d".into(),
            paths: paths.iter().map(|p| (*p).to_owned()).collect(),
            mode,
        }
    }

    #[test]
    fn trash_photos_opens_the_dialog_while_destructive_operations_are_not_allowed() {
        let files = paths(&["/d/a", "/d/b", "/d/c"]);
        let view = view(&files);
        assert_eq!(
            ask(
                "trash_photos",
                json!({ "paths": ["/d/c", "/d/a", "/d/c"] }),
                &view
            ),
            Ok(Some(Action::Ask(Op::Trash(target(
                &["/d/c", "/d/a"],
                trash::Mode::Trash
            )))))
        );
        assert_eq!(
            ask("trash_photos", json!({}), &view),
            Ok(Some(Action::Ask(Op::Trash(target(
                &["/d/a"],
                trash::Mode::Trash
            )))))
        );
    }

    #[test]
    fn trash_photos_without_paths_takes_the_selections_targets() {
        let files = paths(&["/d/a", "/d/b", "/d/c"]);
        let selection = selection_of(&["/d/c".to_owned(), "/d/a".to_owned()]);
        let view = View {
            selection: &selection,
            ..view(&files)
        };
        assert_eq!(
            ask("trash_photos", json!({ "mode": "delete" }), &view),
            Ok(Some(Action::Ask(Op::Trash(target(
                &["/d/a", "/d/c"],
                trash::Mode::Delete
            )))))
        );
    }

    #[test]
    fn trash_photos_previews_a_move_while_allowed_but_always_asks_about_a_delete() {
        let files = paths(&["/d/a"]);
        let view = View {
            settings: &ALLOW_DESTRUCTIVE,
            ..view(&files)
        };
        assert_eq!(
            ask("trash_photos", json!({ "mode": "trash" }), &view),
            Ok(Some(Action::Preview(Op::Trash(target(
                &["/d/a"],
                trash::Mode::Trash
            )))))
        );
        assert_eq!(
            ask("trash_photos", json!({ "mode": "delete" }), &view),
            Ok(Some(Action::Ask(Op::Trash(target(
                &["/d/a"],
                trash::Mode::Delete
            )))))
        );
    }

    #[test]
    fn trash_photos_refuses_paths_outside_the_open_folder_or_hidden_by_the_filter() {
        let files = paths(&["/d/a"]);
        let view = view(&files);
        for path in ["/e/a", "/d/hidden"] {
            assert_eq!(
                ask("trash_photos", json!({ "paths": [path] }), &view),
                Err(format!(
                    "{path} is not shown in Handpic: not in the open folder, or hidden by the filter"
                ))
            );
        }
        assert_eq!(
            ask("trash_photos", json!({ "paths": [] }), &view),
            Err("paths must name at least one photo".into())
        );
        let empty = super::tests::view(&[]);
        assert_eq!(
            ask("trash_photos", json!({}), &empty),
            Err("no photo is shown in Handpic".into())
        );
        let closed = View {
            folder: None,
            ..empty
        };
        assert_eq!(
            ask("trash_photos", json!({}), &closed),
            Err("no folder is open in Handpic".into())
        );
        assert_eq!(
            ask("trash_photos", json!({ "mode": "shred" }), &view),
            Err("mode must be trash or delete".into())
        );
    }

    #[test]
    fn trash_photos_trashes_in_a_view_only_folder_as_its_menu_offers() {
        let files = paths(&["/d/a.jpg"]);
        let view = View {
            view_only: true,
            ..view(&files)
        };
        assert_eq!(
            ask("trash_photos", json!({}), &view),
            Ok(Some(Action::Ask(Op::Trash(target(
                &["/d/a.jpg"],
                trash::Mode::Trash
            )))))
        );
    }

    #[test]
    fn trash_photos_is_refused_while_a_run_or_a_folder_operation_is_out() {
        let files = paths(&["/d/a"]);
        let running = View {
            trashing: true,
            ..view(&files)
        };
        assert_eq!(
            ask("trash_photos", json!({}), &running),
            Err(
                "a Move to Trash or Delete Permanently is under way in Handpic; try again once it ends"
                    .into()
            )
        );
        let folder_op = View {
            folder_op: true,
            ..view(&files)
        };
        assert_eq!(
            ask("trash_photos", json!({}), &folder_op),
            Err("a folder operation is under way in Handpic; try again once it ends".into())
        );
    }

    #[test]
    fn a_held_operation_is_not_replaced_by_a_confirmed_run_or_a_trash_step() {
        let files = paths(&["/d/a"]);
        let held = View {
            held: true,
            ..view(&files)
        };
        let refusal = Err(super::HELD_REFUSAL.to_owned());
        assert_eq!(
            ask("trash_photos", json!({"confirm": true, "id": 1}), &held),
            refusal
        );
        // A preview or a dialog replaces nothing.
        assert!(ask("trash_photos", json!({}), &held).is_ok());
        let trash_top = View {
            held: true,
            undo: Top::Trash,
            redo: Top::Trash,
            settings: &ALLOW_DESTRUCTIVE,
            ..view(&files)
        };
        assert_eq!(ask("undo", json!({}), &trash_top), refusal);
        assert_eq!(ask("redo", json!({}), &trash_top), refusal);
        // A sidecar start goes through the gate and would drop it; a
        // sequence start does not.
        for tool in ["rewrite_sidecars", "delete_sidecars"] {
            assert_eq!(ask(tool, json!({"path": "/d"}), &held), refusal);
        }
        assert!(ask("sequence_timestamps", json!({"path": "/d"}), &held).is_ok());
    }

    #[test]
    fn trash_photos_runs_the_preview_its_id_names_on_confirm() {
        let files = paths(&["/d/a", "/d/b"]);
        let pending = Pending {
            id: 4,
            op: Op::Trash(target(&["/d/b"], trash::Mode::Trash)),
        };
        let view = View {
            settings: &ALLOW_DESTRUCTIVE,
            pending: Some(&pending),
            ..view(&files)
        };
        assert_eq!(
            ask("trash_photos", json!({ "id": 4, "confirm": true }), &view),
            Ok(Some(Action::Run(Op::Trash(target(
                &["/d/b"],
                trash::Mode::Trash
            )))))
        );
        assert_eq!(
            ask("trash_photos", json!({ "id": 3, "confirm": true }), &view),
            Err("no preview of trash_photos has id 3; preview again".into())
        );
        assert_eq!(
            ask("trash_photos", json!({ "confirm": true }), &view),
            Err("confirm: true needs the id a preview returned".into())
        );
        assert_eq!(
            ask("trash_photos", json!({ "id": 4 }), &view),
            Err("an id goes with confirm: true".into())
        );
        assert_eq!(
            ask(
                "trash_photos",
                json!({ "id": 4, "confirm": true, "paths": ["/d/a"] }),
                &view
            ),
            Err("confirm runs the photos its preview named; give only the id".into())
        );
    }

    #[test]
    fn trash_photos_refuses_a_confirm_without_permission_a_delete_or_a_stale_preview() {
        let files = paths(&["/d/a"]);
        let pending = Pending {
            id: 1,
            op: Op::Trash(target(&["/d/a"], trash::Mode::Trash)),
        };
        let off = View {
            pending: Some(&pending),
            ..view(&files)
        };
        assert!(
            ask("trash_photos", json!({ "id": 1, "confirm": true }), &off)
                .unwrap_err()
                .starts_with("mcp_allow_destructive is off")
        );
        let on = || View {
            settings: &ALLOW_DESTRUCTIVE,
            pending: Some(&pending),
            ..view(&files)
        };
        assert!(
            ask(
                "trash_photos",
                json!({ "id": 1, "confirm": true, "mode": "delete" }),
                &on()
            )
            .unwrap_err()
            .starts_with("Delete Permanently runs only on the user's click")
        );
        let hidden = paths(&["/d/b"]);
        let stale = View {
            files: &hidden,
            ..on()
        };
        assert_eq!(
            ask("trash_photos", json!({ "id": 1, "confirm": true }), &stale),
            Err("the previewed photos are no longer all shown in Handpic; preview again".into())
        );
        let elsewhere = View {
            folder: Some("/e"),
            ..on()
        };
        assert_eq!(
            ask(
                "trash_photos",
                json!({ "id": 1, "confirm": true }),
                &elsewhere
            ),
            Err("the previewed photos are no longer all shown in Handpic; preview again".into())
        );
    }

    #[test]
    fn a_preview_names_its_id_and_the_dialogs_text_and_awaiting_says_the_user_confirms() {
        let pending = Pending {
            id: 7,
            op: Op::Trash(target(&["/d/a", "/d/b"], trash::Mode::Trash)),
        };
        assert_eq!(
            previewed(&pending),
            json!({
                "id": 7,
                "operation": "Move to Trash",
                "title": "Move to Trash",
                "text": "Move 2 files to the Trash?",
                "run": "Move to Trash",
                "folder": "/d",
                "paths": ["/d/a", "/d/b"],
                "next": "call trash_photos with this id and confirm: true to run it",
            })
        );
        let reply = awaiting(&Op::Trash(target(&["/d/a"], trash::Mode::Delete)));
        assert_eq!(reply["awaiting_confirmation"], json!(true));
        assert_eq!(reply["operation"], json!("Delete Permanently"));
        assert_eq!(reply["paths"], json!(["/d/a"]));
        assert!(
            reply["next"]
                .as_str()
                .unwrap()
                .starts_with("the user confirms")
        );
    }

    #[test]
    fn idle_is_no_listing_no_scan_no_run_and_no_open_waiting() {
        assert!(Status::default().idle());
        for status in [
            Status {
                listing: true,
                ..Status::default()
            },
            Status {
                scan: Some(("scanning", 0, 3)),
                ..Status::default()
            },
            Status {
                running: true,
                ..Status::default()
            },
            Status {
                opening: Some("/e".into()),
                ..Status::default()
            },
        ] {
            assert!(!status.idle(), "{status:?}");
            assert_eq!(get_status(&status)["idle"], json!(false));
        }
    }

    #[test]
    fn wait_for_idle_waits_only_while_something_is_under_way() {
        let idle = view(&[]);
        assert_eq!(
            ask("wait_for_idle", json!({ "timeout_ms": 500 }), &idle),
            Ok(None)
        );
        let busy = View {
            idle: false,
            ..view(&[])
        };
        assert_eq!(
            ask("wait_for_idle", json!({ "timeout_ms": 500 }), &busy),
            Ok(Some(Action::Wait(500)))
        );
        assert_eq!(
            ask("wait_for_idle", json!({}), &busy),
            Err("timeout_ms must be a number of milliseconds".into())
        );
    }

    #[test]
    fn open_folder_opens_another_folder_without_the_verbatim_prefix() {
        let view = View {
            tree_open: Some("C:\\d"),
            ..view(&[])
        };
        assert_eq!(
            ask("open_folder", json!({ "path": "\\\\?\\C:\\e" }), &view),
            Ok(Some(Action::Open("C:\\e".into())))
        );
        assert_eq!(
            ask("open_folder", json!({ "path": 3 }), &view),
            Err("path must be a string".into())
        );
    }

    #[test]
    fn open_folder_of_the_open_folder_changes_nothing() {
        let view = View {
            tree_open: Some("/photos/day"),
            ..view(&[])
        };
        assert_eq!(
            ask("open_folder", json!({ "path": "/photos/day/" }), &view),
            Ok(None)
        );
    }

    #[test]
    fn tree_folder_expands_or_collapses_only_a_row_not_already_so() {
        let rows = [row("/r", 0, true), row("/r/a", 1, false)];
        let view = View {
            tree: &rows,
            ..view(&[])
        };
        assert_eq!(
            ask(
                "tree_folder",
                json!({ "path": "/r/a", "action": "expand" }),
                &view
            ),
            Ok(Some(Action::Tree {
                path: "/r/a".into(),
                op: TreeOp::Toggle
            }))
        );
        assert_eq!(
            ask(
                "tree_folder",
                json!({ "path": "/r/a", "action": "collapse" }),
                &view
            ),
            Ok(None)
        );
        assert_eq!(
            ask(
                "tree_folder",
                json!({ "path": "/r", "action": "expand" }),
                &view
            ),
            Ok(None)
        );
        assert_eq!(
            ask(
                "tree_folder",
                json!({ "path": "/r", "action": "collapse" }),
                &view
            ),
            Ok(Some(Action::Tree {
                path: "/r".into(),
                op: TreeOp::Toggle
            }))
        );
    }

    #[test]
    fn tree_folder_runs_the_menus_items_on_the_rows_spelling() {
        let rows = [row("C:\\", 0, true), row("C:\\Photos", 1, false)];
        let view = View {
            tree: &rows,
            ..view(&[])
        };
        for (action, op) in [
            ("refresh", TreeOp::Refresh),
            ("expand_all", TreeOp::ExpandAll),
            ("collapse_all", TreeOp::CollapseAll),
        ] {
            assert_eq!(
                ask(
                    "tree_folder",
                    json!({ "path": "C:/Photos/", "action": action }),
                    &view
                ),
                Ok(Some(Action::Tree {
                    path: "C:\\Photos".into(),
                    op
                })),
                "{action}"
            );
        }
        assert_eq!(
            ask(
                "tree_folder",
                json!({ "path": "C:\\", "action": "refresh" }),
                &view
            ),
            Ok(Some(Action::Tree {
                path: "C:\\".into(),
                op: TreeOp::Refresh
            }))
        );
    }

    #[test]
    fn tree_folder_refuses_a_root_for_expand_all_a_hidden_row_and_an_unknown_action() {
        let rows = [row("/r", 0, false)];
        let view = View {
            tree: &rows,
            ..view(&[])
        };
        assert_eq!(
            ask(
                "tree_folder",
                json!({ "path": "/r", "action": "expand_all" }),
                &view
            ),
            Err(
                "expand_all does not apply to a root, as the tree's menu offers it only below one"
                    .into()
            )
        );
        assert_eq!(
            ask(
                "tree_folder",
                json!({ "path": "/r/a", "action": "expand" }),
                &view
            ),
            Err("/r/a is not a folder the tree shows; expand its parent first".into())
        );
        assert_eq!(
            ask(
                "tree_folder",
                json!({ "path": "/r", "action": "open" }),
                &view
            ),
            Err("action must be one of expand, collapse, refresh, expand_all, collapse_all".into())
        );
    }

    #[test]
    fn tree_reply_names_the_row_and_whether_it_is_expanded() {
        let rows = [row("/r", 0, true)];
        let view = View {
            tree: &rows,
            ..view(&[])
        };
        assert_eq!(
            tree_reply(&view, &json!("/r/")),
            json!({ "path": "/r", "expanded": true })
        );
        assert_eq!(
            tree_reply(&view, &json!("/gone")),
            json!({ "path": "/gone", "expanded": false })
        );
    }

    #[test]
    fn reveal_takes_any_folder_but_only_a_file_the_strip_shows() {
        let files = paths(&["/d/a.ARW"]);
        let view = view(&files);
        assert_eq!(
            ask("reveal", json!({ "path": "/e", "folder": true }), &view),
            Ok(Some(Action::Reveal {
                path: "/e".into(),
                folder: true
            }))
        );
        assert_eq!(
            ask(
                "reveal",
                json!({ "path": "/d/a.ARW", "folder": false }),
                &view
            ),
            Ok(Some(Action::Reveal {
                path: "/d/a.ARW".into(),
                folder: false
            }))
        );
        assert_eq!(
            ask(
                "reveal",
                json!({ "path": "/d/b.ARW", "folder": false }),
                &view
            ),
            Err(
                "/d/b.ARW is not shown in Handpic: not in the open folder, or hidden by the filter"
                    .into()
            )
        );
    }

    fn set(args: Value, settings: &Settings) -> Result<Option<Action>, String> {
        let files = paths(&["/d/a.ARW"]);
        let view = View {
            settings,
            ..view(&files)
        };
        answer("set_settings", &args, &view)
    }

    fn changes(args: Value, settings: &Settings) -> Vec<SettingChange> {
        match set(args, settings) {
            Ok(Some(Action::Settings(changes))) => changes,
            other => panic!("not a settings change: {other:?}"),
        }
    }

    #[test]
    fn get_settings_reports_what_the_settings_modal_shows() {
        let settings = Settings {
            import_pattern: "%Y/%m".into(),
            mcp_allow_destructive: true,
            ..settings::defaults()
        };
        let reply = get_settings(&settings);
        assert_eq!(reply["sidecar_format"], "xmp");
        assert_eq!(reply["sidecar_formats"], json!(["xmp", "dop", "both"]));
        assert_eq!(
            reply["label_names"],
            json!({
                "red": "Red", "yellow": "Yellow", "green": "Green", "blue": "Blue",
                "purple": "Purple"
            })
        );
        assert_eq!(reply["label_language"], "en");
        assert_eq!(
            reply["label_languages"][0],
            json!({ "code": "en", "name": i18n::presets()[0].name })
        );
        assert_eq!(reply["auto_advance"], true);
        let pick = reply["shortcuts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["action"] == "pick")
            .unwrap();
        assert_eq!(
            pick,
            &json!({ "action": "pick", "label": "Pick", "keys": ["p"] })
        );
        assert_eq!(reply["import_root"], Value::Null);
        assert_eq!(reply["import_pattern"], "%Y/%m");
        assert_eq!(
            reply["theme"],
            json!({ "mode": "dark", "light": NEUTRAL, "dark": NEUTRAL })
        );
        assert_eq!(reply["themes"]["light"][0], NEUTRAL);
        assert!(
            reply["themes"]["dark"]
                .as_array()
                .unwrap()
                .contains(&json!("Ayu Dark"))
        );
        assert_eq!(reply["mcp_allow_destructive"], true);
    }

    #[test]
    fn get_settings_names_no_language_for_edited_names() {
        let settings = Settings {
            names: LabelNames {
                red: "Custom".into(),
                ..LabelNames::default()
            },
            import_root: "/photos".into(),
            ..settings::defaults()
        };
        let reply = get_settings(&settings);
        assert_eq!(reply["label_language"], Value::Null);
        assert_eq!(reply["label_names"]["red"], "Custom");
        assert_eq!(reply["import_root"], "/photos");
    }

    #[test]
    fn get_settings_is_answered_while_the_settings_are_open_and_set_settings_is_not() {
        let files = paths(&["/d/a.ARW"]);
        let open = View {
            busy: Some(Busy::Settings),
            ..view(&files)
        };
        assert_eq!(ask("get_settings", json!({}), &open), Ok(None));
        assert_eq!(
            ask("set_settings", json!({ "auto_advance": false }), &open),
            Err("Handpic's settings are open; try again once the user closes them".into())
        );
    }

    #[test]
    fn set_settings_refuses_the_destructive_switch_and_a_running_folder_operation() {
        let defaults = settings::defaults();
        assert!(
            set(json!({ "mcp_allow_destructive": true }), &defaults)
                .unwrap_err()
                .contains("the user's to change")
        );
        let files = paths(&["/d/a.ARW"]);
        let running = View {
            folder_op: true,
            ..view(&files)
        };
        assert_eq!(
            ask("set_settings", json!({ "auto_advance": false }), &running),
            Err("a folder operation is under way in Handpic; try again once it ends".into())
        );
    }

    #[test]
    fn set_settings_gives_the_modals_changes_in_order_and_none_for_the_values_in_force() {
        let defaults = settings::defaults();
        assert_eq!(
            set(
                json!({
                    "sidecar_format": "xmp",
                    "auto_advance": true,
                    "import_pattern": "%Y/%F",
                    "import_root": "",
                    "theme": { "mode": "dark", "light": NEUTRAL },
                    "label_language": "en",
                    "shortcuts": [{ "op": "reset", "action": "pick" }]
                }),
                &defaults
            ),
            Ok(None)
        );
        let theme = theme::Setting {
            mode: Mode::System,
            dark: "Ayu Dark",
            ..theme::Setting::default()
        };
        assert_eq!(
            changes(
                json!({
                    "theme": { "mode": "system", "dark": "Ayu Dark" },
                    "import_pattern": "%Y/%m",
                    "import_root": "/photos",
                    "auto_advance": false,
                    "label_names": { "red": " Rot " },
                    "sidecar_format": "both"
                }),
                &defaults
            ),
            [
                SettingChange::SidecarFormat(SidecarFormat::Both),
                SettingChange::LabelNames(LabelNames {
                    red: "Rot".into(),
                    ..LabelNames::default()
                }),
                SettingChange::AutoAdvance(false),
                SettingChange::ImportRoot("/photos".into()),
                SettingChange::ImportPattern("%Y/%m".into()),
                SettingChange::Theme(theme),
            ]
        );
    }

    #[test]
    fn set_settings_takes_a_languages_preset_or_names_and_only_while_writing_xmp() {
        let defaults = settings::defaults();
        let ja = i18n::preset("ja").unwrap();
        assert_eq!(
            changes(json!({ "label_language": "ja" }), &defaults),
            [SettingChange::LabelNames(ja.names.clone())]
        );
        assert!(
            set(json!({ "label_language": "xx" }), &defaults)
                .unwrap_err()
                .contains("label_languages")
        );
        assert_eq!(
            set(
                json!({ "label_language": "ja", "label_names": { "red": "Rot" } }),
                &defaults
            ),
            Err("set label_names or label_language, not both".into())
        );
        assert!(
            set(json!({ "label_names": { "orange": "Orange" } }), &defaults)
                .unwrap_err()
                .contains("is not one of the label colors")
        );
        assert_eq!(
            set(
                json!({ "sidecar_format": "dop", "label_names": { "red": "Rot" } }),
                &defaults
            ),
            Err(
                "the label names apply only while the sidecar format writes XMP (xmp or both)"
                    .into()
            )
        );
        let dop = Settings {
            format: SidecarFormat::Dop,
            ..settings::defaults()
        };
        assert!(set(json!({ "label_language": "ja" }), &dop).is_err());
        assert_eq!(
            changes(
                json!({ "sidecar_format": "xmp", "label_language": "ja" }),
                &dop
            ),
            [
                SettingChange::SidecarFormat(SidecarFormat::Xmp),
                SettingChange::LabelNames(ja.names.clone()),
            ]
        );
    }

    #[test]
    fn set_settings_edits_the_keymap_as_the_shortcuts_page_does() {
        let defaults = settings::defaults();
        let mut expected = Keymap::defaults();
        edit(&mut expected, Edit::Add("pick", "k".into())).unwrap();
        edit(&mut expected, Edit::Remove("pick", "p".into())).unwrap();
        assert_eq!(
            changes(
                json!({ "shortcuts": [
                    { "op": "add", "action": "pick", "key": "k" },
                    { "op": "remove", "action": "pick", "key": "p" }
                ] }),
                &defaults
            ),
            [SettingChange::Keymap(expected.clone())]
        );
        let edited = Settings {
            keymap: expected,
            ..settings::defaults()
        };
        assert_eq!(
            changes(json!({ "shortcuts": [{ "op": "reset_all" }] }), &edited),
            [SettingChange::Keymap(Keymap::defaults())]
        );
        for (edits, refusal) in [
            (
                json!([{ "op": "add", "action": "pick", "key": "q" }]),
                "\"q\" is taken by this app",
            ),
            (
                json!([{ "op": "add", "action": "pick", "key": "x" }]),
                "\"x\" is bound to reject",
            ),
            (
                json!([{ "op": "remove", "action": "pick", "key": "p" }]),
                "\"p\" is the only key for pick; use Reset",
            ),
            (
                json!([{ "op": "add", "action": "nothing", "key": "k" }]),
                "unknown action \"nothing\"",
            ),
            (
                json!([{ "op": "add", "action": "pick" }]),
                "a shortcut add needs an action and a key",
            ),
            (
                json!([{ "op": "reset_all", "action": "pick" }]),
                "a shortcut reset_all takes no action or key",
            ),
        ] {
            assert_eq!(
                set(json!({ "shortcuts": edits }), &defaults),
                Err(refusal.to_owned()),
                "{edits}"
            );
        }
        for key in ["K", "Ctrl+K", "shift", "alt+ctrl+k", "", "ctrl+"] {
            assert!(
                set(
                    json!({ "shortcuts": [{ "op": "add", "action": "pick", "key": key }] }),
                    &defaults
                )
                .unwrap_err()
                .contains("is not a key as Handpic names it"),
                "{key}"
            );
        }
        assert!(
            set(
                json!({ "auto_advance": false, "shortcuts": [
                    { "op": "add", "action": "pick", "key": "k" },
                    { "op": "add", "action": "pick", "key": "x" }
                ] }),
                &defaults
            )
            .is_err()
        );
    }

    #[test]
    fn set_settings_refuses_an_invalid_pattern_and_a_theme_not_of_its_mode() {
        let defaults = settings::defaults();
        assert!(set(json!({ "import_pattern": "%H" }), &defaults).is_err());
        assert_eq!(
            set(json!({ "theme": { "light": "Ayu Dark" } }), &defaults),
            Err("\"Ayu Dark\" is not one of get_settings' themes.light".into())
        );
        assert!(set(json!({ "theme": { "mode": "dim" } }), &defaults).is_err());
        assert!(set(json!({ "sidecar_format": "tiff" }), &defaults).is_err());
        assert_eq!(
            set(json!({ "auto_advance": "no" }), &defaults),
            Err("auto_advance must be true or false".into())
        );
    }

    fn rewrite(dir: &str) -> FolderOp {
        FolderOp::Sidecars(crate::sidecars::Target {
            kind: crate::sidecars::Kind::Rewrite,
            dir: dir.into(),
        })
    }

    fn folder_dialog(run_enabled: bool) -> Dialog {
        Dialog::new(
            Content {
                title: "Rewrite Sidecars from Index".into(),
                rows: vec!["2 files with a judgment".to_owned().into()],
                notes: vec!["Rewrite the XMP sidecars of 2 files in d?".into()],
                reason: (!run_enabled).then(|| "nothing to write".into()),
                ..Content::default()
            },
            "Rewrite",
            Style::Primary,
            run_enabled,
        )
    }

    #[test]
    fn a_folder_operation_opens_the_dialog_unless_destructive_operations_are_allowed() {
        let asking = view(&[]);
        for (tool, op) in [
            ("rewrite_sidecars", rewrite("/e")),
            (
                "delete_sidecars",
                FolderOp::Sidecars(crate::sidecars::Target {
                    kind: crate::sidecars::Kind::Delete,
                    dir: "/e".into(),
                }),
            ),
            ("sequence_timestamps", FolderOp::Sequence("/e".into())),
        ] {
            assert_eq!(
                ask(tool, json!({ "path": "/e" }), &asking),
                Ok(Some(Action::Folder {
                    op: op.clone(),
                    dialog: true,
                }))
            );
            let allowed = View {
                settings: &ALLOW_DESTRUCTIVE,
                ..view(&[])
            };
            assert_eq!(
                ask(tool, json!({ "path": "/e" }), &allowed),
                Ok(Some(Action::Folder { op, dialog: false }))
            );
        }
        assert_eq!(rewrite("/e").action(), context::Action::RewriteSidecars);
        assert_eq!(
            FolderOp::Sequence("/e".into()).label(),
            "Sequence JPEG Timestamps"
        );
    }

    #[test]
    fn a_folder_operation_is_refused_while_another_or_a_trash_run_is_under_way() {
        let busy = View {
            folder_op: true,
            ..view(&[])
        };
        assert_eq!(
            ask("rewrite_sidecars", json!({ "path": "/e" }), &busy),
            Err("a folder operation is under way in Handpic; try again once it ends".into())
        );
        let trashing = View {
            trashing: true,
            ..view(&[])
        };
        assert!(
            ask("sequence_timestamps", json!({ "path": "/e" }), &trashing)
                .unwrap_err()
                .starts_with("a Move to Trash or Delete Permanently is under way")
        );
        assert_eq!(
            ask("delete_sidecars", json!({}), &view(&[])),
            Err("path must be the folder's absolute path".into())
        );
        assert_eq!(
            ask(
                "delete_sidecars",
                json!({ "path": "/e", "id": 1 }),
                &view(&[])
            ),
            Err("an id goes with confirm: true".into())
        );
    }

    #[test]
    fn a_confirm_runs_the_folder_operation_its_preview_named() {
        let pending = Pending {
            id: 4,
            op: Op::Folder(rewrite("/e"), Some(folder_dialog(true))),
        };
        let allowed = View {
            settings: &ALLOW_DESTRUCTIVE,
            pending: Some(&pending),
            ..view(&[])
        };
        assert_eq!(
            ask(
                "rewrite_sidecars",
                json!({ "id": 4, "confirm": true }),
                &allowed
            ),
            Ok(Some(Action::Run(pending.op.clone())))
        );
        assert_eq!(
            ask(
                "delete_sidecars",
                json!({ "id": 4, "confirm": true }),
                &allowed
            ),
            Err("no preview of delete_sidecars has id 4; preview again".into())
        );
        assert_eq!(
            ask(
                "rewrite_sidecars",
                json!({ "id": 5, "confirm": true }),
                &allowed
            ),
            Err("no preview of rewrite_sidecars has id 5; preview again".into())
        );
        assert_eq!(
            ask(
                "rewrite_sidecars",
                json!({ "path": "/e", "id": 4, "confirm": true }),
                &allowed
            ),
            Err("confirm runs the folder its preview named; give only the id".into())
        );
        assert_eq!(
            ask("rewrite_sidecars", json!({ "confirm": true }), &allowed),
            Err("confirm: true needs the id a preview returned".into())
        );
        let off = View {
            pending: Some(&pending),
            ..view(&[])
        };
        assert!(
            ask(
                "rewrite_sidecars",
                json!({ "id": 4, "confirm": true }),
                &off
            )
            .unwrap_err()
            .starts_with("mcp_allow_destructive is off")
        );
        let nothing = Pending {
            id: 4,
            op: Op::Folder(rewrite("/e"), Some(folder_dialog(false))),
        };
        let empty = View {
            settings: &ALLOW_DESTRUCTIVE,
            pending: Some(&nothing),
            ..view(&[])
        };
        assert_eq!(
            ask(
                "rewrite_sidecars",
                json!({ "id": 4, "confirm": true }),
                &empty
            ),
            Err("nothing to write".into())
        );
    }

    #[test]
    fn a_folder_preview_reports_the_dialogs_content_and_its_id() {
        let pending = Pending {
            id: 2,
            op: Op::Folder(rewrite("/e"), Some(folder_dialog(true))),
        };
        assert_eq!(
            previewed(&pending),
            json!({
                "operation": "Rewrite Sidecars from Index",
                "folder": "/e",
                "title": "Rewrite Sidecars from Index",
                "lines": [],
                "rows": ["2 files with a judgment"],
                "lists": [],
                "notes": ["Rewrite the XMP sidecars of 2 files in d?"],
                "errors": [],
                "reason": null,
                "run": "Rewrite",
                "run_enabled": true,
                "id": 2,
                "next": "call rewrite_sidecars with this id and confirm: true to run it",
            })
        );
        let status = Status {
            pending: Some(previewed(&pending)),
            ..Status::default()
        };
        assert!(status.idle());
        assert_eq!(started(&status, false)["pending"]["id"], json!(2));
        assert!(
            started(&status, true)["next"]
                .as_str()
                .unwrap()
                .contains("its dialog asks the user")
        );
    }

    #[test]
    fn cancel_operation_cancels_a_dialog_or_a_run_else_the_pending_and_says_what_it_cannot() {
        for cancel in [Cancel::TrashDialog, Cancel::Folder] {
            let view = View {
                busy: Some(Busy::FolderDialog),
                cancel: Some(Ok(cancel)),
                ..view(&[])
            };
            assert_eq!(
                ask("cancel_operation", json!({}), &view),
                Ok(Some(Action::Cancel(cancel)))
            );
        }
        let pending = Pending {
            id: 1,
            op: Op::Folder(rewrite("/e"), Some(folder_dialog(true))),
        };
        let previewed = View {
            pending: Some(&pending),
            cancel: Some(Err("Delete Sidecars")),
            ..view(&[])
        };
        assert_eq!(
            ask("cancel_operation", json!({}), &previewed),
            Ok(Some(Action::Cancel(Cancel::Pending)))
        );
        let running = View {
            cancel: Some(Err("Rewrite Sidecars from Index")),
            ..view(&[])
        };
        assert_eq!(
            ask("cancel_operation", json!({}), &running),
            Err(
                "Rewrite Sidecars from Index is under way and cannot be canceled; \
                 wait_for_idle waits for its end"
                    .into()
            )
        );
        assert_eq!(
            ask("cancel_operation", json!({}), &view(&[])),
            Err("nothing to cancel in Handpic".into())
        );
    }

    static IMPORTING: LazyLock<Settings> = LazyLock::new(|| Settings {
        import_root: "/photos".into(),
        ..settings::defaults()
    });
    static IMPORTING_ALLOWED: LazyLock<Settings> = LazyLock::new(|| Settings {
        import_root: "/photos".into(),
        mcp_allow_destructive: true,
        ..settings::defaults()
    });

    fn import(plan: Option<u64>) -> FolderOp {
        FolderOp::Import {
            dir: "/e".into(),
            plan,
        }
    }

    #[test]
    fn an_import_starts_its_flow_and_is_refused_without_a_destination() {
        let set = View {
            settings: &IMPORTING,
            ..view(&[])
        };
        assert_eq!(
            ask("import_folder", json!({ "path": "/e" }), &set),
            Ok(Some(Action::Folder {
                op: import(None),
                dialog: true,
            }))
        );
        let allowed = View {
            settings: &IMPORTING_ALLOWED,
            ..view(&[])
        };
        assert_eq!(
            ask("import_folder", json!({ "path": "/e" }), &allowed),
            Ok(Some(Action::Folder {
                op: import(None),
                dialog: false,
            }))
        );
        assert_eq!(
            ask("import_folder", json!({ "path": "/e" }), &view(&[])),
            Err(
                "the destination folder is not set (Settings > Import); set_settings' \
                 import_root and import_pattern set them"
                    .into()
            )
        );
        assert_eq!(import(None).action(), context::Action::ImportFolder);
        assert_eq!(import(None).label(), "Import");
    }

    #[test]
    fn a_confirmed_import_runs_the_plan_its_preview_named() {
        let pending = Pending {
            id: 3,
            op: Op::Folder(import(Some(7)), Some(folder_dialog(true))),
        };
        let allowed = View {
            settings: &IMPORTING_ALLOWED,
            pending: Some(&pending),
            ..view(&[])
        };
        assert_eq!(
            ask(
                "import_folder",
                json!({ "id": 3, "confirm": true }),
                &allowed
            ),
            Ok(Some(Action::Run(pending.op.clone())))
        );
        assert_eq!(
            ask(
                "rename_by_capture_date",
                json!({ "id": 3, "confirm": true }),
                &allowed
            ),
            Err("no preview of rename_by_capture_date has id 3; preview again".into())
        );
        let nothing = Pending {
            id: 3,
            op: Op::Folder(
                import(None),
                Some(Dialog::new(
                    Content {
                        title: "Import".into(),
                        notes: vec!["No photos were found under the folder.".into()],
                        ..Content::default()
                    },
                    "Run",
                    Style::Primary,
                    false,
                )),
            ),
        };
        let empty = View {
            pending: Some(&nothing),
            ..allowed
        };
        assert_eq!(
            ask("import_folder", json!({ "id": 3, "confirm": true }), &empty),
            Err("No photos were found under the folder.".into())
        );
    }

    #[test]
    fn rename_by_capture_date_names_the_folder_by_the_trees_spelling() {
        let rows = [row("/E", 0, true)];
        let drawn = View {
            tree: &rows,
            ..view(&[])
        };
        let expected = if crate::tree::IGNORE_CASE { "/E" } else { "/e" };
        assert_eq!(
            ask("rename_by_capture_date", json!({ "path": "/e" }), &drawn),
            Ok(Some(Action::Folder {
                op: FolderOp::Date(expected.into()),
                dialog: true,
            }))
        );
        assert_eq!(
            FolderOp::Date("/e".into()).action(),
            context::Action::RenameByDate
        );
    }

    #[test]
    fn rename_folder_opens_the_trees_rename_field_unless_destructive_operations_are_allowed() {
        let rows = [row("/d/a", 1, false)];
        let drawn = View {
            tree: &rows,
            ..view(&[])
        };
        let op = Op::Rename {
            path: "/d/a".into(),
            name: "b".into(),
        };
        assert_eq!(
            ask(
                "rename_folder",
                json!({ "path": "/d/a", "name": " b " }),
                &drawn
            ),
            Ok(Some(Action::Ask(op.clone())))
        );
        let allowed = View {
            settings: &ALLOW_DESTRUCTIVE,
            ..view(&[])
        };
        assert_eq!(
            ask(
                "rename_folder",
                json!({ "path": "/d/a", "name": "b" }),
                &allowed
            ),
            Ok(Some(Action::Preview(op.clone())))
        );
        assert_eq!(
            ask(
                "rename_folder",
                json!({ "path": "/d/a", "name": "b" }),
                &view(&[])
            ),
            Err(
                "/d/a is not a folder the tree shows, where Handpic asks the user; expand its \
                 parent first"
                    .into()
            )
        );
        assert_eq!(
            ask(
                "rename_folder",
                json!({ "path": "/d/a", "name": "a" }),
                &drawn
            ),
            Err("/d/a is already named a".into())
        );
        assert!(
            awaiting(&op)["next"]
                .as_str()
                .unwrap()
                .contains("rename field")
        );
        assert_eq!(
            previewed(&Pending { id: 5, op })["next"],
            "call rename_folder with this id and confirm: true to run it"
        );
    }

    #[test]
    fn a_confirmed_rename_runs_the_rename_its_preview_named() {
        let pending = Pending {
            id: 5,
            op: Op::Rename {
                path: "/d/a".into(),
                name: "b".into(),
            },
        };
        let allowed = View {
            settings: &ALLOW_DESTRUCTIVE,
            pending: Some(&pending),
            ..view(&[])
        };
        assert_eq!(
            ask(
                "rename_folder",
                json!({ "id": 5, "confirm": true }),
                &allowed
            ),
            Ok(Some(Action::Run(pending.op.clone())))
        );
        assert_eq!(
            ask(
                "rename_folder",
                json!({ "id": 6, "confirm": true }),
                &allowed
            ),
            Err("no preview of rename_folder has id 6; preview again".into())
        );
        assert_eq!(
            ask(
                "rename_folder",
                json!({ "id": 5, "confirm": true, "name": "c" }),
                &allowed
            ),
            Err("confirm renames the folder its preview named; give only the id".into())
        );
        let held = View {
            held: true,
            ..allowed
        };
        assert_eq!(
            ask("rename_folder", json!({ "id": 5, "confirm": true }), &held),
            Err(
                "an operation is held in Handpic for the scan's end; try again once it has run"
                    .into()
            )
        );
        let off = View {
            pending: Some(&pending),
            ..view(&[])
        };
        assert!(
            ask("rename_folder", json!({ "id": 5, "confirm": true }), &off)
                .unwrap_err()
                .starts_with("mcp_allow_destructive is off")
        );
        let importing = View {
            importing: true,
            ..view(&[])
        };
        assert_eq!(
            ask(
                "rename_folder",
                json!({ "path": "/d/a", "name": "b" }),
                &importing
            ),
            Err("Rename\u{2026} is unavailable while an import runs".into())
        );
    }
}
