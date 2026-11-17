//! The embedded MCP server: a Streamable HTTP endpoint on the loopback
//! interface that lets any MCP client follow and assist a culling session.
//! It is off by default and turned on by the `mcpEnabled` setting. The view
//! state and the judgments the tools read and drive are the app's, reached
//! through the `Bridge`'s send callback; the server's tasks run on the
//! caller's tokio runtime, so `Mcp::switch` and `Mcp::stop` must run inside
//! one. The tools a server lists are its host's: the seven companion tools by
//! default (the Tauri app), all of them after `Mcp::with_all_tools` (the
//! native app).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::JsonSchema;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::exif::{read_metadata, Metadata};
use crate::folders;
use crate::index::{self, Focus, Index};
use crate::ondemand::read_preview;

/// The fixed loopback port, outside the ranges development servers commonly
/// take, so the connection examples stay the same from launch to launch.
pub const MCP_PORT: u16 = 41917;

/// How long a tool call waits for the main window to answer.
const BRIDGE_TIMEOUT: Duration = Duration::from_secs(5);

/// How long `wait_for_idle` waits when no timeout is given, and the most it
/// waits, under the call timeouts MCP clients commonly set.
const IDLE_WAIT: Duration = Duration::from_secs(30);
const IDLE_WAIT_MAX: Duration = Duration::from_secs(55);

/// The long edge `get_preview` scales to when none is asked for, and the
/// range it clamps a request to (the top is the embedded preview's own).
const PREVIEW_LONG_EDGE: u32 = 1024;
const PREVIEW_LONG_EDGE_MIN: u32 = 256;
const PREVIEW_LONG_EDGE_MAX: u32 = 1616;

const PREVIEW_QUALITY: f32 = 75.0;

/// How long `stop` waits for open connections to close before it drops them.
const STOP_TIMEOUT: Duration = Duration::from_secs(2);

const INSTRUCTIONS: &str = "Handpic is a desktop app the user culls Sony ARW and Leica DNG \
photos with. This server connects you to the running app so you can assist while they cull: \
read what Handpic is showing (the current photo, the selection, its burst, the sharpness \
score, the AF point, the shooting settings and the current stars, pick / reject flag and \
color label), look at a small copy of a photo's preview, move the view, and record stars, \
picks / rejects and color labels the same way the user's keys do, so the app's undo and \
sidecar files stay consistent. Nothing here can move a file to the trash or delete it. \
Suggest judgments and explain them; write them only when the user asks you to.";

/// The native app's instructions, over the whole tool set.
const NATIVE_INSTRUCTIONS: &str = "Handpic is a desktop app the user culls RAW photos (Sony \
ARW, Canon CR3, Nikon NEF, Fujifilm RAF, OM System ORF, DNG) and JPEGs with. This server \
connects you to the running app so you can assist while they cull: read what Handpic is showing \
(the current photo, the selection, its burst, the sharpness score, the AF point, the shooting \
settings and the current stars, pick / reject flag and color label), look at a small copy of a \
photo's preview, move through the photos and the bursts, select photos, set the sort, the \
filter, the panes shown and the overlays, browse the folders, open one, expand or collapse the \
folder tree, reveal a folder or photo in the file manager, read what is under way (a listing, a \
scan, a move to the trash, a folder operation) and wait for it to end, read and change the settings, and record stars, picks / rejects and color labels and undo or redo them the same way the user's keys do, so the app's \
undo and sidecar files stay consistent. `trash_photos` moves photos to the trash or deletes \
them, and `rewrite_sidecars`, `delete_sidecars` and `sequence_timestamps` run the folder \
tree menu's operations on a folder, each only with the user's confirmation: unless the user \
has allowed MCP clients to run destructive operations in Handpic's settings, it opens \
Handpic's own dialog and the user confirms there; a permanent delete always waits for the \
user's click in that dialog. `cancel_operation` cancels what the user's Cancel would. Run \
them only when the user asks. \
A call that changes anything is refused while the user has Handpic's settings, a dialog or a \
folder rename open; try again once they are done. Suggest judgments and explain them; write \
them only when the user asks you to.";

type Reply = Result<Value, String>;

/// A request the main window answers through `Mcp::reply` with its `id`.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Request {
    pub id: u64,
    pub kind: &'static str,
    pub args: Value,
}

/// The request / reply bridge to the main window, which holds the view
/// state and the judgment path the tools read and drive.
pub struct Bridge {
    next: AtomicU64,
    pending: Mutex<HashMap<u64, oneshot::Sender<Reply>>>,
    send: Box<dyn Fn(Request) -> Result<(), String> + Send + Sync>,
}

impl Bridge {
    /// A bridge handing each request to `send`, whose error fails the call
    /// at once.
    pub fn new(send: impl Fn(Request) -> Result<(), String> + Send + Sync + 'static) -> Self {
        Bridge {
            next: AtomicU64::new(0),
            pending: Mutex::new(HashMap::new()),
            send: Box::new(send),
        }
    }

    /// Ask the main window for `kind` and wait for its reply.
    pub async fn call(&self, kind: &'static str, args: Value) -> Reply {
        self.call_within(kind, args, BRIDGE_TIMEOUT).await
    }

    async fn call_within(&self, kind: &'static str, args: Value, timeout: Duration) -> Reply {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, sender);
        if let Err(e) = (self.send)(Request { id, kind, args }) {
            self.pending.lock().unwrap().remove(&id);
            return Err(e);
        }
        let reply = tokio::time::timeout(timeout, receiver).await;
        self.pending.lock().unwrap().remove(&id);
        match reply {
            Ok(Ok(reply)) => reply,
            _ => Err(format!(
                "Handpic's main window did not answer within {timeout:?}"
            )),
        }
    }

    /// Complete the call `id`; `value` is the result, or the error message
    /// when `ok` is false. A reply after the timeout is dropped.
    pub fn reply(&self, id: u64, ok: bool, value: Value) {
        let Some(sender) = self.pending.lock().unwrap().remove(&id) else {
            return;
        };
        let reply = if ok {
            Ok(value)
        } else {
            Err(value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_string))
        };
        let _ = sender.send(reply);
    }
}

fn tool_result(reply: Reply) -> CallToolResult {
    match reply {
        Ok(value) => CallToolResult::structured(value),
        Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
    }
}

/// The photo `path` names, or the current one when it is `None`, checked
/// against the open folder of `view` (a `get_view` answer). The result is
/// the path the index keys the photo by: the folder resolved, the file name
/// kept.
fn resolve(view: &Value, path: Option<String>) -> Result<PathBuf, String> {
    let folder = view["folder"]
        .as_str()
        .ok_or_else(|| "no folder is open in Handpic".to_string())?;
    let path = match path {
        Some(path) => path,
        None => view["current"]["path"]
            .as_str()
            .ok_or_else(|| "no photo is shown in Handpic".to_string())?
            .to_string(),
    };
    let outside = || format!("{path} is not in the open folder {folder}");
    let file = Path::new(&path);
    let (Some(parent), Some(name)) = (file.parent(), file.file_name()) else {
        return Err(outside());
    };
    let parent = std::fs::canonicalize(parent).map_err(|_| outside())?;
    let folder_dir = std::fs::canonicalize(folder).map_err(|e| format!("{folder}: {e}"))?;
    if parent != folder_dir {
        return Err(outside());
    }
    Ok(parent.join(name))
}

fn long_edge(requested: Option<u32>) -> usize {
    requested
        .unwrap_or(PREVIEW_LONG_EDGE)
        .clamp(PREVIEW_LONG_EDGE_MIN, PREVIEW_LONG_EDGE_MAX) as usize
}

/// What `get_photo` answers: the index row's judgment and focus with the
/// shooting settings read from the file.
#[derive(serde::Serialize)]
struct Photo {
    path: String,
    rating: Option<i8>,
    flag: &'static str,
    label: Option<String>,
    sharpness: Option<f64>,
    focus: Option<Focus>,
    orientation: u16,
    capture_time: Option<String>,
    #[serde(flatten)]
    metadata: Metadata,
}

fn photo(index: &Mutex<Index>, path: &Path) -> Reply {
    let key = path.to_string_lossy();
    let row = index::lock(index)
        .entry(&key)?
        .ok_or_else(|| format!("{key} is not a photo Handpic has indexed"))?;
    let photo = Photo {
        path: row.path,
        rating: row.rating,
        flag: index::flag_name(row.flag),
        label: row.label,
        sharpness: row.sharpness,
        focus: row.focus,
        orientation: row.orientation,
        capture_time: row.capture_time,
        metadata: read_metadata(path)?,
    };
    serde_json::to_value(photo).map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct PhotoArgs {
    /// The photo's path as `get_view` reports it; the current photo when
    /// omitted.
    path: Option<String>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct PreviewArgs {
    /// The photo's path as `get_view` reports it; the current photo when
    /// omitted.
    path: Option<String>,
    /// The long edge in pixels, 256 to 1616; 1024 when omitted.
    long_edge: Option<u32>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ShowArgs {
    /// The photo's path as `get_view` reports it.
    path: String,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SelectArgs {
    /// The photos' paths as `get_view` reports them; the first becomes the
    /// current photo.
    paths: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum ViewMode {
    Normal,
    Zoom,
    Compare,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ViewArgs {
    /// `normal` for the fitted preview, `zoom` for the 1:1 focus check, or
    /// `compare` for side-by-side panes.
    mode: ViewMode,
}

/// The color labels `set_judgment` accepts: the ones the judgment keys set.
const LABELS: [&str; 7] = ["Red", "Orange", "Yellow", "Green", "Blue", "Pink", "Purple"];

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum PickFlag {
    None,
    Pick,
    Reject,
}

/// A present field, even `null`, as `Some`, so an omitted label (kept) and
/// a `null` one (cleared) differ.
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    serde::Deserialize::deserialize(d).map(Some)
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct JudgmentArgs {
    /// The photos' paths as `get_view` reports them; the selection when
    /// omitted, or the active pane in compare.
    paths: Option<Vec<String>>,
    /// Stars from 1 to 5, or 0 to clear them; kept when omitted.
    rating: Option<u8>,
    /// `pick`, `reject` or `none`; kept when omitted.
    flag: Option<PickFlag>,
    /// Red, Orange, Yellow, Green, Blue, Pink or Purple, or null to clear
    /// it; kept when omitted.
    #[serde(default, deserialize_with = "present")]
    label: Option<Option<String>>,
}

/// The `set_judgment` request for the main window, with only the fields
/// given, checked against what the judgment keys can set.
fn judgment(args: JudgmentArgs) -> Result<Value, String> {
    let mut request = serde_json::Map::new();
    if let Some(paths) = args.paths {
        request.insert("paths".into(), json!(paths));
    }
    if let Some(rating) = args.rating {
        if rating > 5 {
            return Err("rating must be an integer from 0 to 5".into());
        }
        request.insert("rating".into(), json!(rating));
    }
    if let Some(flag) = args.flag {
        request.insert("flag".into(), json!(flag));
    }
    if let Some(label) = args.label {
        if label.as_deref().is_some_and(|l| !LABELS.contains(&l)) {
            return Err(format!(
                "label must be one of {}, or null",
                LABELS.join(", ")
            ));
        }
        request.insert("label".into(), json!(label));
    }
    if !["rating", "flag", "label"]
        .iter()
        .any(|key| request.contains_key(*key))
    {
        return Err("set at least one of rating, flag, label".into());
    }
    Ok(Value::Object(request))
}

/// Where `navigate` moves the current photo, as the paging keys do.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(crate = "rmcp::schemars")]
pub enum Direction {
    Previous,
    Next,
    First,
    Last,
    BurstPrevious,
    BurstNext,
    BurstFramePrevious,
    BurstFrameNext,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct NavigateArgs {
    /// `previous` / `next` (wrapping around), `first` / `last`,
    /// `burst_previous` / `burst_next`, or `burst_frame_previous` /
    /// `burst_frame_next`.
    to: Direction,
}

/// The strip's order `set_sort` sets, as the sort menu's items name it.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum SortBy {
    Name,
    Capture,
    Rating,
    Recommended,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SortArgs {
    /// `name`, `capture` (capture time) or `rating`; `recommended` is not in
    /// the native app yet.
    by: SortBy,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum FlagItem {
    Picked,
    Untagged,
    Rejected,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum LabelItem {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Pink,
    Purple,
    None,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum OrientationItem {
    Portrait,
    Landscape,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum FaceFocusItem {
    Good,
    Neutral,
    Low,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum EyesItem {
    Open,
    Closed,
    Unknown,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(crate = "rmcp::schemars")]
pub enum OutputItem {
    Output,
    NoOutput,
}

/// The EXIF sections `set_filter` sets, each the labels to check among the
/// ones `get_view`'s `filter_offers.exif` lists.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct ExifArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    camera: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lens: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aperture: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shutter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iso: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focal: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct FilterArgs {
    /// Uncheck every item first, as the menu's Reset does.
    #[serde(skip_serializing_if = "Option::is_none")]
    reset: Option<bool>,
    /// The flags to check: `picked`, `untagged`, `rejected`.
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<FlagItem>>,
    /// The star counts to check, 0 (unrated) to 5.
    #[serde(skip_serializing_if = "Option::is_none")]
    stars: Option<Vec<u8>>,
    /// The color labels to check, lower-cased, or `none` for no label.
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Vec<LabelItem>>,
    /// `portrait` and / or `landscape`.
    #[serde(skip_serializing_if = "Option::is_none")]
    orientations: Option<Vec<OrientationItem>>,
    /// The `Face focus` items: `good`, `neutral`, `low`.
    #[serde(skip_serializing_if = "Option::is_none")]
    face_focus: Option<Vec<FaceFocusItem>>,
    /// The `Eyes` items: `open`, `closed`, `unknown`.
    #[serde(skip_serializing_if = "Option::is_none")]
    eyes: Option<Vec<EyesItem>>,
    /// `output` and / or `no_output`; only when the folder offers them.
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<Vec<OutputItem>>,
    /// The EXIF sections to set; a section left out keeps its checks.
    #[serde(skip_serializing_if = "Option::is_none")]
    exif: Option<ExifArgs>,
}

/// The `set_filter` request for the main window, with only the sections
/// given.
fn filter(args: FilterArgs) -> Result<Value, String> {
    if args
        .stars
        .as_ref()
        .is_some_and(|s| s.iter().any(|n| *n > 5))
    {
        return Err("stars must be integers from 0 to 5".into());
    }
    let request = serde_json::to_value(args).map_err(|e| e.to_string())?;
    if request.as_object().is_none_or(|o| o.is_empty()) {
        return Err(
            "set reset or at least one of flags, stars, labels, orientations, face_focus, \
            eyes, output, exif"
                .into(),
        );
    }
    Ok(request)
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct PanesArgs {
    /// Whether the left pane (the folder tree) is shown; kept when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    left: Option<bool>,
    /// Whether the right pane (the shooting settings) is shown; kept when
    /// omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    right: Option<bool>,
    /// Whether the thumbnail strip is shown; kept when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    strip: Option<bool>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct OverlaysArgs {
    /// The `f` overlay: the AF point, the face boxes and the eye judgment;
    /// kept when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<bool>,
    /// The `e` focus peaking; kept when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    peaking: Option<bool>,
    /// The `d` performance graph; kept when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    performance: Option<bool>,
}

/// The request of a tool whose fields are all optional switches, refused
/// when none is given.
fn switches(args: impl serde::Serialize, names: &str) -> Result<Value, String> {
    let request = serde_json::to_value(args).map_err(|e| e.to_string())?;
    if request.as_object().is_none_or(|o| o.is_empty()) {
        return Err(format!("set at least one of {names}"));
    }
    Ok(request)
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct FoldersArgs {
    /// An absolute folder path; the tree's roots when omitted.
    path: Option<String>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct FolderArgs {
    /// An absolute folder path.
    path: String,
}

/// What `tree_folder` does to a folder's row, as the tree's expander and
/// right-click items do.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(crate = "rmcp::schemars")]
pub enum TreeAction {
    Expand,
    Collapse,
    Refresh,
    ExpandAll,
    CollapseAll,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct TreeArgs {
    /// The folder's absolute path, a row the tree shows.
    path: String,
    /// `expand`, `collapse`, `refresh`, `expand_all` or `collapse_all`.
    action: TreeAction,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct RevealArgs {
    /// An absolute folder path, or a photo's path as `get_view` reports it.
    path: String,
}

/// What `trash_photos` does with the photos, as the strip menu's items do.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum TrashMode {
    Trash,
    Delete,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct TrashArgs {
    /// The photos' paths as `get_view` reports them, in the open folder and
    /// shown by the filter; the selection when omitted.
    paths: Option<Vec<String>>,
    /// `trash` (the default) moves them to the trash with their sidecars;
    /// `delete` deletes them permanently.
    mode: Option<TrashMode>,
    /// The `id` a preview returned, with `confirm: true`.
    id: Option<u64>,
    /// True to run the preview `id` names.
    confirm: Option<bool>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct FolderOpArgs {
    /// The folder's absolute path, for the call that starts the operation.
    path: Option<String>,
    /// The `id` of the preview in `get_status`' `pending`, with `confirm: true`.
    id: Option<u64>,
    /// True to run the preview `id` names.
    confirm: Option<bool>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct WaitArgs {
    /// How long to wait in milliseconds, at most 55000; 30000 when omitted.
    timeout_ms: Option<u64>,
}

/// `path` as an absolute path, refused when relative.
fn absolute(path: &str) -> Result<&Path, String> {
    let absolute = Path::new(path);
    if !absolute.is_absolute() {
        return Err(format!("{path} is not an absolute path"));
    }
    Ok(absolute)
}

/// `path`, checked to be an absolute path to a folder.
fn folder(path: &str) -> Result<&Path, String> {
    let dir = absolute(path)?;
    if !dir.is_dir() {
        return Err(format!("{path} is not a folder"));
    }
    Ok(dir)
}

/// `list_folders`: the roots the tree starts from (the home directory, then
/// the volumes), or one folder's photo count and visible subfolders, as the
/// tree lists them.
fn list_folders(home: Option<PathBuf>, path: Option<String>) -> Reply {
    let Some(path) = path else {
        return Ok(json!({ "roots": folders::roots(home) }));
    };
    let listed = folders::list(absolute(&path)?)?;
    Ok(json!({
        "path": path,
        "photo_count": listed.raw_count,
        "children": listed.children,
    }))
}

/// The `reveal` request: the path and whether it names a folder.
fn reveal(path: String) -> Reply {
    let kind = std::fs::metadata(absolute(&path)?)
        .map_err(|_| format!("{path} is neither a folder nor a file"))?;
    Ok(json!({ "path": path, "folder": kind.is_dir() }))
}

/// A sidecar format `set_settings` takes.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum FormatName {
    Xmp,
    Dop,
    Both,
}

/// The `xmp:Label` names `set_settings` sets, a color left out keeping its
/// own.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct LabelNamesArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    red: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    yellow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    green: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    purple: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(crate = "rmcp::schemars")]
pub enum ShortcutOp {
    Add,
    Remove,
    Reset,
    ResetAll,
}

/// One Keyboard Shortcuts page edit.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct ShortcutEdit {
    /// `add` a key to an action, `remove` one of its keys, `reset` an
    /// action to its default keys, or `reset_all`.
    op: ShortcutOp,
    /// The action, as `get_settings`' `shortcuts` names it; not for
    /// `reset_all`.
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<String>,
    /// The key for `add` and `remove`, as the keymap names it: lower-case,
    /// modifiers first in `ctrl+alt+shift+meta+` order, such as `k`,
    /// `shift+x`, `ctrl+z` or `arrowright`.
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum ThemeModeName {
    System,
    Light,
    Dark,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct ThemeArgs {
    /// `system` (following the OS), `light` or `dark`; kept when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<ThemeModeName>,
    /// The light theme, one of `get_settings`' `themes.light`; kept when
    /// omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    light: Option<String>,
    /// The dark theme, one of `get_settings`' `themes.dark`; kept when
    /// omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    dark: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(deny_unknown_fields)]
pub struct SettingsArgs {
    /// The sidecar files judgments are written to: `xmp` (Lightroom), `dop`
    /// (PhotoLab) or `both`.
    #[serde(skip_serializing_if = "Option::is_none")]
    sidecar_format: Option<FormatName>,
    /// The `xmp:Label` written for each color, only while the format writes
    /// XMP; a blank name is the English default.
    #[serde(skip_serializing_if = "Option::is_none")]
    label_names: Option<LabelNamesArgs>,
    /// A Lightroom language's code from `get_settings`' `label_languages`,
    /// whose preset names are put in force, instead of `label_names`.
    #[serde(skip_serializing_if = "Option::is_none")]
    label_language: Option<String>,
    /// Move to the next photo after a star, reject or pick.
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_advance: Option<bool>,
    /// Keyboard shortcut edits, applied in order.
    #[serde(skip_serializing_if = "Option::is_none")]
    shortcuts: Option<Vec<ShortcutEdit>>,
    /// The absolute folder imports are copied into.
    #[serde(skip_serializing_if = "Option::is_none")]
    import_root: Option<String>,
    /// The import's date folder pattern (chrono strftime date specifiers
    /// and `/`), such as `%Y/%F`.
    #[serde(skip_serializing_if = "Option::is_none")]
    import_pattern: Option<String>,
    /// The theme's mode and the light and dark themes.
    #[serde(skip_serializing_if = "Option::is_none")]
    theme: Option<ThemeArgs>,
}

/// The `set_settings` request: at least one setting, the import root an
/// absolute folder.
fn settings(args: SettingsArgs) -> Reply {
    if let Some(root) = &args.import_root {
        folder(root)?;
    }
    switches(
        args,
        "sidecar_format, label_names, label_language, auto_advance, shortcuts, import_root, \
        import_pattern, theme",
    )
}

/// `trash_photos`' request: the fields given.
fn trash(args: TrashArgs) -> Value {
    let mut request = serde_json::Map::new();
    if let Some(paths) = args.paths {
        request.insert("paths".into(), json!(paths));
    }
    if let Some(mode) = args.mode {
        request.insert("mode".into(), json!(mode));
    }
    if let Some(id) = args.id {
        request.insert("id".into(), json!(id));
    }
    if let Some(confirm) = args.confirm {
        request.insert("confirm".into(), json!(confirm));
    }
    Value::Object(request)
}

/// A folder operation's request: the fields given, the path checked to be
/// an absolute folder.
fn folder_op(args: FolderOpArgs) -> Reply {
    let mut request = serde_json::Map::new();
    if let Some(path) = args.path {
        folder(&path)?;
        request.insert("path".into(), json!(path));
    }
    if let Some(id) = args.id {
        request.insert("id".into(), json!(id));
    }
    if let Some(confirm) = args.confirm {
        request.insert("confirm".into(), json!(confirm));
    }
    Ok(Value::Object(request))
}

/// How long `wait_for_idle` waits for `timeout_ms`.
fn idle_wait(timeout_ms: Option<u64>) -> Duration {
    timeout_ms
        .map_or(IDLE_WAIT, Duration::from_millis)
        .min(IDLE_WAIT_MAX)
}

/// The MCP handler each client session gets.
#[derive(Clone)]
pub struct Companion {
    bridge: Arc<Bridge>,
    /// The index reader, `None` when the index cache is unavailable.
    index: Option<Arc<Mutex<Index>>>,
    /// The tools this host lists and answers.
    tools: ToolRouter<Companion>,
    instructions: &'static str,
    /// The home directory `list_folders` starts the roots from, as the
    /// host's tree does.
    home: Option<PathBuf>,
}

impl Companion {
    async fn target(&self, path: Option<String>) -> Result<PathBuf, String> {
        let view = self
            .bridge
            .call("get_view", Value::Object(Default::default()))
            .await?;
        resolve(&view, path)
    }

    /// The seven companion tools and the instructions over them.
    fn seven(bridge: Arc<Bridge>, index: Option<Arc<Mutex<Index>>>) -> Self {
        Companion {
            bridge,
            index,
            tools: Self::tool_router(),
            instructions: INSTRUCTIONS,
            home: None,
        }
    }

    /// The result of `call` once `check` passed on tokio's blocking pool,
    /// whose answer is the request.
    async fn checked_call(
        &self,
        kind: &'static str,
        check: impl FnOnce() -> Reply + Send + 'static,
    ) -> CallToolResult {
        let request = tokio::task::spawn_blocking(check)
            .await
            .map_err(|e| e.to_string())
            .and_then(|checked| checked);
        match request {
            Ok(request) => tool_result(self.bridge.call(kind, request).await),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
        }
    }

    async fn view_call(&self, kind: &'static str) -> CallToolResult {
        tool_result(
            self.bridge
                .call(kind, Value::Object(Default::default()))
                .await,
        )
    }

    async fn photo(&self, path: Option<String>) -> Reply {
        let path = self.target(path).await?;
        let index = self
            .index
            .clone()
            .ok_or_else(|| "no index cache available".to_string())?;
        tokio::task::spawn_blocking(move || photo(&index, &path))
            .await
            .map_err(|e| e.to_string())?
    }

    async fn preview(&self, args: PreviewArgs) -> Result<CallToolResult, String> {
        let path = self.target(args.path).await?;
        let long_edge = long_edge(args.long_edge);
        let name = path.to_string_lossy().into_owned();
        let (jpeg, width, height) = tokio::task::spawn_blocking(move || {
            let (orientation, preview) = read_preview(&path)?;
            riffle_core::decode::preview_jpeg(&preview, orientation, long_edge, PREVIEW_QUALITY)
                .map_err(|e| format!("{}: {e}", path.display()))
        })
        .await
        .map_err(|e| e.to_string())??;
        Ok(CallToolResult::success(vec![
            ContentBlock::image(
                base64::engine::general_purpose::STANDARD.encode(jpeg),
                "image/jpeg",
            ),
            ContentBlock::text(
                json!({ "path": name, "width": width, "height": height }).to_string(),
            ),
        ]))
    }
}

#[tool_router]
impl Companion {
    #[tool(
        description = "What Handpic is showing now: the open folder, the number of photos \
        visible after the filter, the current photo and its 1-based position, the selected \
        paths, the view mode (normal, zoom for the 1:1 focus check, or compare), the active \
        compare pane, the sort order, whether a filter is active, and the current photo's \
        burst (empty when it is not part of one) with each frame's sharpness score, stars \
        (null when unrated), pick / reject flag, color label, and whether the filter shows it \
        (a frame it hides cannot be shown or selected). The current photo carries `eyes`, the \
        judgment of the eyes of the face nearest the AF point (else the largest confident \
        face): `state` (open or closed), `probability` (the probability the eyes are closed, \
        0 to 1) and `pose` (the head pose in degrees: yaw positive when the face turns toward \
        the image's right, pitch positive up, roll positive clockwise on screen; null when it \
        could not be fitted). `eyes` is null when the photo has no face to judge (none found, \
        too small, or a failure), and absent until Handpic has judged the photo, which it does \
        a moment after showing it (never in a JPEG-only folder): call again shortly."
    )]
    async fn get_view(&self) -> CallToolResult {
        tool_result(
            self.bridge
                .call("get_view", Value::Object(Default::default()))
                .await,
        )
    }

    #[tool(
        description = "One photo's details: its stars (null when unrated), pick / reject flag \
        and color label, its sharpness score, the AF point and frame in sensor coordinates \
        with whether it was focused manually, its orientation and capture time, and its \
        shooting settings (camera, lens, aperture, shutter, ISO, focal length, exposure \
        bias, focus distance). The photo must be in the open folder; omit the path for the \
        current photo."
    )]
    async fn get_photo(&self, Parameters(args): Parameters<PhotoArgs>) -> CallToolResult {
        tool_result(self.photo(args.path).await)
    }

    #[tool(
        description = "A small upright JPEG of one photo, scaled from the camera's embedded \
        preview (never the RAW) to a long edge of at most `long_edge` pixels (256 to 1616, \
        1024 by default), followed by its path, width and height. The photo must be in the \
        open folder; omit the path for the current photo."
    )]
    async fn get_preview(&self, Parameters(args): Parameters<PreviewArgs>) -> CallToolResult {
        self.preview(args)
            .await
            .unwrap_or_else(|e| CallToolResult::error(vec![ContentBlock::text(e)]))
    }

    #[tool(
        description = "Show one photo in Handpic, as clicking it in the filmstrip does: it \
        becomes the current photo and the only selected one. The path must be one `get_view` \
        can report, in the open folder and not hidden by the filter. Returns the new view, \
        as `get_view` does."
    )]
    async fn show_photo(&self, Parameters(args): Parameters<ShowArgs>) -> CallToolResult {
        tool_result(
            self.bridge
                .call("show_photo", json!({ "path": args.path }))
                .await,
        )
    }

    #[tool(
        description = "Select one or more photos in Handpic's filmstrip; the first becomes the \
        current photo. Judgments and compare apply to the selection (compare shows its first \
        four). Every path must be in the open folder and not hidden by the filter. Returns \
        the new view, as `get_view` does."
    )]
    async fn select_photos(&self, Parameters(args): Parameters<SelectArgs>) -> CallToolResult {
        tool_result(
            self.bridge
                .call("select_photos", json!({ "paths": args.paths }))
                .await,
        )
    }

    #[tool(
        description = "Switch Handpic's view mode: `normal` for the fitted preview, `zoom` for \
        the 1:1 focus check of the current photo, or `compare` for the selected photos (2 to \
        4) side by side, or the current photo and its burst's first good frame (else its first \
        frame) when only one is selected. Compare fails when there are fewer than two photos \
        to compare. Returns the new view, as `get_view` does."
    )]
    async fn set_view(&self, Parameters(args): Parameters<ViewArgs>) -> CallToolResult {
        tool_result(
            self.bridge
                .call("set_view", json!({ "mode": args.mode }))
                .await,
        )
    }

    #[tool(
        description = "Record stars, a pick / reject flag and a color label in Handpic, exactly \
        as the user's judgment keys do: the change is shown at once, is one entry the user can \
        undo, and is written to each photo's sidecar. Omitted fields keep each photo's own \
        value; a rating of 0 clears the stars and a null label clears the label. Applies to \
        the given paths (in the open folder and not hidden by the filter), or to the \
        selection when omitted (the active pane in compare). Write only when the user asks. \
        Returns each photo's resulting stars (null when unrated), flag and label."
    )]
    async fn set_judgment(&self, Parameters(args): Parameters<JudgmentArgs>) -> CallToolResult {
        match judgment(args) {
            Ok(request) => tool_result(self.bridge.call("set_judgment", request).await),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
        }
    }
}

/// The tools only the native app lists.
#[tool_router(router = native_router)]
impl Companion {
    #[tool(
        description = "Move Handpic's current photo as the paging keys do: `previous` / `next` \
        (wrapping around), `first` / `last`, `burst_previous` / `burst_next` (the first frame of \
        the previous / next burst, or of the current one), `burst_frame_previous` / \
        `burst_frame_next` (the neighboring frame within the current burst). Only the photos \
        the filter shows count. The photo moved to becomes the only selected one. Returns the \
        new view, as `get_view` does."
    )]
    async fn navigate(&self, Parameters(args): Parameters<NavigateArgs>) -> CallToolResult {
        tool_result(self.bridge.call("navigate", json!({ "to": args.to })).await)
    }

    #[tool(
        description = "Select every photo the filter shows, as Select All does; the current \
        photo stays. Returns the new view, as `get_view` does."
    )]
    async fn select_all(&self) -> CallToolResult {
        self.view_call("select_all").await
    }

    #[tool(
        description = "Reject the other frames of the current photo's burst, the ones the \
        filter hides too, keeping their stars and labels, as one entry the user can undo, as \
        the Reject Rest key does. Write only when the user asks. Returns the new view, as \
        `get_view` does."
    )]
    async fn reject_rest(&self) -> CallToolResult {
        self.view_call("reject_rest").await
    }

    #[tool(
        description = "Undo the newest change in Handpic, as the undo key does, whether a key, \
        a menu or a tool made it: a change of stars, flags and labels (a change of one photo \
        makes it the current one; returns the new view, as `get_view` does), or a move to the \
        trash, whose files are restored (replies at once with `get_status`; the restore \
        finishes after it)."
    )]
    async fn undo(&self) -> CallToolResult {
        self.view_call("undo").await
    }

    #[tool(
        description = "Redo the change last undone in Handpic, as the redo key does: a change \
        of stars, flags and labels (a change of one photo makes it the current one; returns \
        the new view, as `get_view` does), or a move to the trash, which moves the restored \
        files to the trash again with no dialog, so it is refused unless the user allowed MCP \
        clients to run destructive operations (replies at once with `get_status`)."
    )]
    async fn redo(&self) -> CallToolResult {
        self.view_call("redo").await
    }

    #[tool(
        description = "Set the filmstrip's order, as the Sort menu does, keeping the current \
        photo: `name`, `capture` or `rating`. Remembered for the next launch. A JPEG-only \
        folder stays in capture order and refuses it. Returns the new view, as `get_view` \
        does (its `sort`)."
    )]
    async fn set_sort(&self, Parameters(args): Parameters<SortArgs>) -> CallToolResult {
        tool_result(self.bridge.call("set_sort", json!({ "by": args.by })).await)
    }

    #[tool(
        description = "Set which items of the Filter menu are checked. A section given \
        replaces its checks (an empty list unchecks it), a section left out keeps them, and \
        `reset` unchecks everything first. Within a section any checked item passes; across \
        sections all must. The output and EXIF sections take only what `get_view`'s \
        `filter_offers` lists for the open folder. The current photo stays when the filter \
        still shows it, else the nearest shown one becomes current. Returns the new view, as \
        `get_view` does (its `filter`)."
    )]
    async fn set_filter(&self, Parameters(args): Parameters<FilterArgs>) -> CallToolResult {
        match filter(args) {
            Ok(request) => tool_result(self.bridge.call("set_filter", request).await),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
        }
    }

    #[tool(
        description = "Show or hide Handpic's left pane (the folder tree), right pane (the \
        shooting settings) and thumbnail strip, as their keys do; an omitted one is kept. \
        Remembered for the next launch. Returns the new view, as `get_view` does (its \
        `panes`)."
    )]
    async fn set_panes(&self, Parameters(args): Parameters<PanesArgs>) -> CallToolResult {
        match switches(args, "left, right, strip") {
            Ok(request) => tool_result(self.bridge.call("set_panes", request).await),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
        }
    }

    #[tool(
        description = "Turn Handpic's overlays on or off, as their keys do: `focus` (the AF \
        point, face boxes and eye judgment over the photo), `peaking` (focus peaking) and \
        `performance` (the CPU, disk and latency graph); an omitted one is kept. Returns the \
        new view, as `get_view` does (its `overlays`)."
    )]
    async fn set_overlays(&self, Parameters(args): Parameters<OverlaysArgs>) -> CallToolResult {
        match switches(args, "focus, peaking, performance") {
            Ok(request) => tool_result(self.bridge.call("set_overlays", request).await),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
        }
    }

    #[tool(
        description = "The folders Handpic's folder tree starts from (the home folder, then \
        the volumes) when `path` is omitted; else that folder's photo count (its RAW files, or \
        its JPEGs when it holds no RAW) and its visible subfolders, as the tree lists them."
    )]
    async fn list_folders(&self, Parameters(args): Parameters<FoldersArgs>) -> CallToolResult {
        let home = self.home.clone();
        tool_result(
            tokio::task::spawn_blocking(move || list_folders(home, args.path))
                .await
                .map_err(|e| e.to_string())
                .and_then(|listed| listed),
        )
    }

    #[tool(
        description = "Open a folder in Handpic, as a click on its name in the folder tree \
        does. Replies at once, before the folder is listed, with `get_status`'s answer; call \
        `wait_for_idle`, then `get_view`. Opening the open folder changes nothing. While a move \
        to the trash or a folder operation runs, the open waits for its end (`opening`); while \
        that folder is being renamed it is refused."
    )]
    async fn open_folder(&self, Parameters(args): Parameters<FolderArgs>) -> CallToolResult {
        self.checked_call("open_folder", move || {
            folder(&args.path)?;
            Ok(json!({ "path": args.path }))
        })
        .await
    }

    #[tool(
        description = "Act on a folder's row in Handpic's folder tree, as its expander and \
        right-click items do: `expand` (listing it again) or `collapse`, `refresh`, and below a \
        root `expand_all` (the folder and every subfolder) or `collapse_all` (every subfolder, \
        the folder itself kept). The folder must be a row the tree shows: expand its parents \
        first. Returns its path as the tree spells it and whether it is expanded; listings \
        finish after the reply."
    )]
    async fn tree_folder(&self, Parameters(args): Parameters<TreeArgs>) -> CallToolResult {
        tool_result(
            self.bridge
                .call(
                    "tree_folder",
                    json!({ "path": args.path, "action": args.action }),
                )
                .await,
        )
    }

    #[tool(
        description = "Show a folder, or a photo of the open folder that the filter shows, in \
        the OS file manager, as the right-click menus' reveal item does; a root folder opens."
    )]
    async fn reveal(&self, Parameters(args): Parameters<RevealArgs>) -> CallToolResult {
        self.checked_call("reveal", move || reveal(args.path)).await
    }

    #[tool(
        description = "What Handpic's status row and pane notes tell the user: `idle` (nothing \
        under way), the open `folder`, a folder `opening` once a run ends, whether a `listing` \
        or a `scan` pass (`scanning` or `analyzing`, with done / total) is under way, whether a \
        run on the open folder's files is `running`, the status line's text, the pane's \
        `notes` and `errors`, the `operation` under way, else the last one to end (its \
        `name`, such as `Move to Trash`, `Undo Move to Trash` or `Delete Sidecars`, its \
        `state`: `previewing`, `awaiting_confirmation` in Handpic's dialog (whose content is \
        in `dialog`), `held_for_scan`, `running` with `done` / `total`, `finished` or `failed`, \
        and the `message` it ended with), and the `pending` preview a call with its `id` and \
        `confirm: true` runs. A held operation dropped before it ran (by a folder open, a later \
        request or a dialog open at the scan's end) is `failed` with a `dropped: ...` message \
        and did nothing."
    )]
    async fn get_status(&self) -> CallToolResult {
        self.view_call("get_status").await
    }

    #[tool(
        description = "Move photos to the trash, or with `mode: delete` delete them for good, \
        as the filmstrip's Move to Trash and Delete Permanently items do, each with its \
        sidecars. Applies to the given paths or to the selection. A move is confirmed: while \
        the user allows MCP clients to run destructive operations (`get_settings`' \
        `mcp_allow_destructive`), the call returns a preview (the dialog's text, the paths and \
        an `id`) and a second call with that `id` and `confirm: true` runs it; otherwise the \
        call opens Handpic's dialog and the user confirms there. A delete always opens the \
        dialog and runs only on the user's click. A run replies at once; follow it with \
        `get_status` or `wait_for_idle`. A move can be undone with `undo`. Use only when the \
        user asks."
    )]
    async fn trash_photos(&self, Parameters(args): Parameters<TrashArgs>) -> CallToolResult {
        tool_result(self.bridge.call("trash_photos", trash(args)).await)
    }

    #[tool(
        description = "Rewrite a folder's sidecars of the current format from Handpic's index, \
        as the folder tree's Rewrite Sidecars from Index does. Confirmed like a folder \
        operation: see `delete_sidecars`. Use only when the user asks."
    )]
    async fn rewrite_sidecars(&self, Parameters(args): Parameters<FolderOpArgs>) -> CallToolResult {
        self.checked_call("rewrite_sidecars", move || folder_op(args))
            .await
    }

    #[tool(
        description = "Move a folder's sidecars of the current format to the trash, as the \
        folder tree's Delete Sidecars does (`undo` restores them). Called with `path`, it \
        starts the operation and replies at once with `get_status`: while the user allows MCP \
        clients to run destructive operations (`get_settings`' `mcp_allow_destructive`), \
        Handpic reads the preview its dialog would show, and once `wait_for_idle` returns \
        `get_status`' `pending` holds it with an `id`, which a second call with that `id` and \
        `confirm: true` runs; otherwise Handpic's dialog asks the user, its content in \
        `get_status`' `operation.dialog`. Waits for a scan under way before it reads the \
        preview; a run is refused while a listing or scan is under way. Use only when the user \
        asks."
    )]
    async fn delete_sidecars(&self, Parameters(args): Parameters<FolderOpArgs>) -> CallToolResult {
        self.checked_call("delete_sidecars", move || folder_op(args))
            .await
    }

    #[tool(
        description = "Write copies of a folder's JPEGs with unique capture-time seconds into \
        its `-sequenced` sibling folder, as the folder tree's Sequence JPEG Timestamps does, \
        rebuilding that folder. Confirmed like a folder operation: see `delete_sidecars`; \
        `cancel_operation` stops the run. Use only when the user asks."
    )]
    async fn sequence_timestamps(
        &self,
        Parameters(args): Parameters<FolderOpArgs>,
    ) -> CallToolResult {
        self.checked_call("sequence_timestamps", move || folder_op(args))
            .await
    }

    #[tool(
        description = "Cancel what the user's Cancel would: close Handpic's dialog asking about \
        a move to the trash or a folder operation, or stop a Sequence JPEG Timestamps run; \
        else forget the preview in `get_status`' `pending`. Refused, naming it, when the \
        operation under way cannot be canceled. Returns `get_status`."
    )]
    async fn cancel_operation(&self) -> CallToolResult {
        self.view_call("cancel_operation").await
    }

    #[tool(
        description = "Wait until nothing is under way in Handpic (no listing, scan or run), \
        or until `timeout_ms` passes (30000 by default, at most 55000), and return \
        `get_status`'s answer; its `idle` is false after a timeout."
    )]
    async fn wait_for_idle(&self, Parameters(args): Parameters<WaitArgs>) -> CallToolResult {
        let wait = idle_wait(args.timeout_ms);
        tool_result(
            self.bridge
                .call_within(
                    "wait_for_idle",
                    json!({ "timeout_ms": wait.as_millis() as u64 }),
                    wait + BRIDGE_TIMEOUT,
                )
                .await,
        )
    }

    #[tool(
        description = "What Handpic's settings show: the `sidecar_format` (with the \
        `sidecar_formats`), the `label_names` written as `xmp:Label` per color and the \
        `label_language` whose Lightroom preset they are (null when none; the presets in \
        `label_languages`), `auto_advance`, the keyboard `shortcuts` (each action, its label \
        and its keys), the `import_root` (null while not set) and `import_pattern`, the \
        `theme` (mode, light and dark theme; the choices in `themes`), and \
        `mcp_allow_destructive`, which only the user can change."
    )]
    async fn get_settings(&self) -> CallToolResult {
        self.view_call("get_settings").await
    }

    #[tool(
        description = "Change Handpic's settings as its settings window does, with its rules: \
        a field left out is kept, and the whole call is refused when one field is. A sidecar \
        format switch writes the pending judgments and re-reads the open folder; the label \
        names apply only while the format writes XMP; a shortcut edit refuses a key bound to \
        another action, one of the app's own keys or an action's last key; an invalid import \
        pattern is refused; the theme applies at once. Remembered for the next launch. \
        `mcp_allow_destructive` cannot be set here. Refused while a folder operation runs. \
        Returns the settings, as `get_settings` does."
    )]
    async fn set_settings(&self, Parameters(args): Parameters<SettingsArgs>) -> CallToolResult {
        self.checked_call("set_settings", move || settings(args))
            .await
    }
}

#[tool_handler(router = self.tools)]
impl ServerHandler for Companion {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("handpic", env!("CARGO_PKG_VERSION")))
            .with_instructions(self.instructions)
    }
}

struct Running {
    port: u16,
    token: CancellationToken,
    task: JoinHandle<()>,
}

#[derive(Default)]
struct Status {
    enabled: bool,
    running: Option<Running>,
    error: Option<String>,
}

/// The server's on/off setting, the running server if any, and the last
/// bind error (a tokio mutex, since a switch holds it across the bind), and
/// the handler every session clones.
pub struct Mcp {
    status: tokio::sync::Mutex<Status>,
    companion: Companion,
    /// The port a switch binds, `MCP_PORT` unless a test run gives another.
    port: u16,
}

impl Mcp {
    pub fn new(bridge: Bridge, index: Option<Arc<Mutex<Index>>>) -> Self {
        Mcp {
            status: Default::default(),
            companion: Companion::seven(Arc::new(bridge), index),
            port: MCP_PORT,
        }
    }

    /// The server listening on `port` instead of `MCP_PORT`.
    pub fn with_port(self, port: u16) -> Self {
        Mcp { port, ..self }
    }

    /// The server listing every tool, with the instructions over them: the
    /// native app's. The Tauri app keeps the seven.
    pub fn with_all_tools(self) -> Self {
        let companion = Companion {
            tools: Companion::tool_router() + Companion::native_router(),
            instructions: NATIVE_INSTRUCTIONS,
            ..self.companion
        };
        Mcp { companion, ..self }
    }

    /// The home directory `list_folders` starts the roots from, the one the
    /// host's tree is given.
    pub fn with_home(self, home: Option<PathBuf>) -> Self {
        let companion = Companion {
            home,
            ..self.companion
        };
        Mcp { companion, ..self }
    }

    /// Complete a bridge call with the main window's answer.
    pub fn reply(&self, id: u64, ok: bool, value: Value) {
        self.companion.bridge.reply(id, ok, value);
    }
}

/// Whether the server is on, its port and its last bind error.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct McpState {
    enabled: bool,
    port: u16,
    error: Option<String>,
}

impl Status {
    fn state(&self, port: u16) -> McpState {
        McpState {
            enabled: self.enabled,
            port: self.running.as_ref().map_or(port, |r| r.port),
            error: self.error.clone(),
        }
    }
}

/// The `/mcp` route. Host validation keeps its loopback-only default, and
/// any request carrying an `Origin` header, which only a web page sends, is
/// refused.
fn router(token: &CancellationToken, companion: Companion) -> axum::Router {
    let service: StreamableHttpService<Companion, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(companion.clone()),
        Default::default(),
        StreamableHttpServerConfig::default()
            .with_cancellation_token(token.child_token())
            .enforce_origin_validation(),
    );
    axum::Router::new().nest_service("/mcp", service)
}

/// Serve `/mcp` on `listener` until the returned token is cancelled.
fn serve(listener: TcpListener, companion: Companion) -> Result<Running, String> {
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let token = CancellationToken::new();
    let app = router(&token, companion);
    let shutdown = token.clone();
    let task = tokio::spawn(async move {
        let served = axum::serve(listener, app)
            .with_graceful_shutdown(async move { shutdown.cancelled_owned().await })
            .await;
        if let Err(e) = served {
            log::error!("the MCP server stopped: {e}");
        }
    });
    Ok(Running { port, token, task })
}

async fn bind(port: u16, companion: Companion) -> Result<Running, String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("could not listen on 127.0.0.1:{port}: {e}"))?;
    serve(listener, companion)
}

async fn shut(running: Running) {
    running.token.cancel();
    let mut task = running.task;
    if tokio::time::timeout(STOP_TIMEOUT, &mut task).await.is_err() {
        task.abort();
    }
}

/// Start the server on `port` unless it is running, and return its port.
async fn start(status: &mut Status, companion: Companion, port: u16) -> Result<u16, String> {
    if let Some(running) = &status.running {
        return Ok(running.port);
    }
    let running = bind(port, companion).await?;
    let port = running.port;
    status.running = Some(running);
    Ok(port)
}

impl Mcp {
    /// Stop the server if it is running, waiting briefly for open
    /// connections.
    pub async fn stop(&self) {
        let running = self.status.lock().await.running.take();
        if let Some(running) = running {
            shut(running).await;
        }
    }

    /// The current state, as the settings modal shows it.
    pub async fn state(&self) -> McpState {
        self.status.lock().await.state(self.port)
    }

    /// Turn the server on or off and remember a bind error. A failed bind is
    /// logged, not returned: the setting stays on, so the next launch tries
    /// again.
    pub async fn switch(&self, enabled: bool) -> McpState {
        let mut status = self.status.lock().await;
        status.enabled = enabled;
        status.error = None;
        if enabled {
            if let Err(e) = start(&mut status, self.companion.clone(), self.port).await {
                log::error!("failed to start the MCP server: {e}");
                status.error = Some(e);
            }
        } else if let Some(running) = status.running.take() {
            shut(running).await;
        }
        status.state(self.port)
    }
}

/// The stored `mcpEnabled` value; missing or non-boolean means off.
pub fn mcp_enabled_setting(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use tower::ServiceExt;

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;

    fn silent() -> Companion {
        Companion::seven(Arc::new(Bridge::new(|_| Ok(()))), None)
    }

    fn initialize(origin: Option<&str>) -> Request<Body> {
        let mut request = Request::post("/mcp")
            .header(header::HOST, format!("127.0.0.1:{MCP_PORT}"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream");
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        request.body(Body::from(INITIALIZE)).unwrap()
    }

    #[test]
    fn a_request_without_an_origin_initializes() {
        block_on(async {
            let token = CancellationToken::new();
            let response = router(&token, silent())
                .oneshot(initialize(None))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            token.cancel();
        });
    }

    #[test]
    fn a_request_with_an_origin_is_refused() {
        block_on(async {
            let token = CancellationToken::new();
            let response = router(&token, silent())
                .oneshot(initialize(Some("http://evil.example")))
                .await
                .unwrap();
            assert!(response.status().is_client_error(), "{}", response.status());
            token.cancel();
        });
    }

    #[test]
    fn serve_binds_the_port_and_stop_frees_it() {
        block_on(async {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let mcp = Mcp::new(Bridge::new(|_| Ok(())), None);
            mcp.status.lock().await.running = Some(serve(listener, silent()).unwrap());
            assert!(tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok());
            assert!(bind(port, silent()).await.is_err());

            mcp.stop().await;
            assert!(mcp.status.lock().await.running.is_none());
            assert!(tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_err());
            let rebound = bind(port, silent()).await.unwrap();
            shut(rebound).await;
        });
    }

    #[test]
    fn start_returns_the_running_port_without_binding_again() {
        block_on(async {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let mut status = Status {
                running: Some(serve(listener, silent()).unwrap()),
                ..Status::default()
            };

            let started = start(&mut status, silent(), MCP_PORT).await.unwrap();
            assert_eq!(started, port);

            let running = status.running.take().unwrap();
            shut(running).await;
        });
    }

    #[test]
    fn a_switch_binds_the_given_port_and_reports_it() {
        block_on(async {
            let free = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let port = free.local_addr().unwrap().port();
            drop(free);
            let mcp = Mcp::new(Bridge::new(|_| Ok(())), None).with_port(port);
            assert_eq!(
                mcp.switch(true).await,
                McpState {
                    enabled: true,
                    port,
                    error: None
                }
            );
            assert!(tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok());
            mcp.stop().await;
            assert_eq!(mcp.state().await.port, port);
        });
    }

    /// POST `body` to `/mcp` on the loopback `port` over a fresh connection
    /// and return the whole response, headers and body.
    async fn post(port: u16, session: Option<&str>, body: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let session = session.map_or_else(String::new, |id| format!("Mcp-Session-Id: {id}\r\n"));
        let request = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
             Accept: application/json, text/event-stream\r\n{session}Content-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response))
            .await
            .expect("the MCP server did not close the response")
            .unwrap();
        String::from_utf8(response).unwrap()
    }

    #[test]
    fn a_client_over_loopback_initializes_and_lists_the_tools() {
        block_on(async {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let mcp = Mcp::new(Bridge::new(|_| Ok(())), None);
            mcp.status.lock().await.running = Some(serve(listener, silent()).unwrap());

            let initialized = post(port, None, INITIALIZE).await;
            assert!(initialized.starts_with("HTTP/1.1 200"), "{initialized}");
            assert!(initialized.contains(r#""name":"handpic""#), "{initialized}");
            let session = initialized
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("mcp-session-id")
                        .then(|| value.trim().to_string())
                })
                .expect("no Mcp-Session-Id header");

            let notified = post(
                port,
                Some(&session),
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            )
            .await;
            assert!(notified.starts_with("HTTP/1.1 202"), "{notified}");

            let listed = post(
                port,
                Some(&session),
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            )
            .await;
            assert!(listed.starts_with("HTTP/1.1 200"), "{listed}");
            for tool in [
                "get_photo",
                "get_preview",
                "get_view",
                "select_photos",
                "set_judgment",
                "set_view",
                "show_photo",
            ] {
                assert!(listed.contains(&format!(r#""name":"{tool}""#)), "{listed}");
            }

            mcp.stop().await;
        });
    }

    #[test]
    fn mcp_enabled_setting_is_off_unless_a_stored_true() {
        assert!(!mcp_enabled_setting(None));
        assert!(!mcp_enabled_setting(Some(&json!("true"))));
        assert!(!mcp_enabled_setting(Some(&json!(false))));
        assert!(mcp_enabled_setting(Some(&json!(true))));
    }

    #[test]
    fn get_info_names_handpic_and_enables_tools() {
        let info = silent().get_info();
        assert_eq!(info.server_info.name, "handpic");
        assert!(info.capabilities.tools.is_some());
        assert!(info.instructions.is_some());
    }

    fn names(companion: &Companion) -> Vec<String> {
        companion
            .tools
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect()
    }

    #[test]
    fn the_tools_are_exactly_the_seven_companion_tools() {
        let mcp = Mcp::new(Bridge::new(|_| Ok(())), None).with_port(1);
        assert_eq!(
            names(&mcp.companion),
            [
                "get_photo",
                "get_preview",
                "get_view",
                "select_photos",
                "set_judgment",
                "set_view",
                "show_photo"
            ]
        );
        assert_eq!(
            mcp.companion.get_info().instructions.as_deref(),
            Some(INSTRUCTIONS)
        );
    }

    #[test]
    fn all_tools_lists_the_native_apps_full_set() {
        let mcp = Mcp::new(Bridge::new(|_| Ok(())), None)
            .with_all_tools()
            .with_port(1);
        assert_eq!(
            names(&mcp.companion),
            [
                "cancel_operation",
                "delete_sidecars",
                "get_photo",
                "get_preview",
                "get_settings",
                "get_status",
                "get_view",
                "list_folders",
                "navigate",
                "open_folder",
                "redo",
                "reject_rest",
                "reveal",
                "rewrite_sidecars",
                "select_all",
                "select_photos",
                "sequence_timestamps",
                "set_filter",
                "set_judgment",
                "set_overlays",
                "set_panes",
                "set_settings",
                "set_sort",
                "set_view",
                "show_photo",
                "trash_photos",
                "tree_folder",
                "undo",
                "wait_for_idle"
            ]
        );
        assert_eq!(mcp.port, 1);
        assert_eq!(
            mcp.companion.get_info().instructions.as_deref(),
            Some(NATIVE_INSTRUCTIONS)
        );
    }

    #[test]
    fn navigate_forwards_the_direction_by_its_snake_case_name() {
        let args: NavigateArgs =
            serde_json::from_value(json!({ "to": "burst_frame_next" })).unwrap();
        assert_eq!(
            json!({ "to": args.to }),
            json!({ "to": "burst_frame_next" })
        );
        assert!(serde_json::from_value::<NavigateArgs>(json!({ "to": "up" })).is_err());
    }

    #[test]
    fn set_sort_takes_the_menus_keys_by_name() {
        let args: SortArgs = serde_json::from_value(json!({ "by": "capture" })).unwrap();
        assert_eq!(json!({ "by": args.by }), json!({ "by": "capture" }));
        assert!(serde_json::from_value::<SortArgs>(json!({ "by": "size" })).is_err());
    }

    fn filter_args(value: Value) -> Result<Value, String> {
        filter(serde_json::from_value(value).map_err(|e| e.to_string())?)
    }

    #[test]
    fn filter_forwards_only_the_given_sections() {
        assert_eq!(
            filter_args(json!({
                "flags": ["picked"],
                "output": ["no_output"],
                "exif": { "lens": [] },
            })),
            Ok(json!({ "flags": ["picked"], "output": ["no_output"], "exif": { "lens": [] } }))
        );
        assert_eq!(
            filter_args(json!({ "reset": true })),
            Ok(json!({ "reset": true }))
        );
    }

    #[test]
    fn filter_refuses_an_empty_call_and_stars_out_of_range() {
        assert!(filter_args(json!({})).unwrap_err().starts_with("set reset"));
        assert_eq!(
            filter_args(json!({ "stars": [6] })),
            Err("stars must be integers from 0 to 5".into())
        );
        assert!(filter_args(json!({ "labels": ["teal"] })).is_err());
    }

    #[test]
    fn filter_refuses_a_misspelled_field_or_exif_section() {
        assert!(filter_args(json!({ "exif": { "body": [] } })).is_err());
        assert!(filter_args(json!({ "flag": ["picked"], "stars": [5] })).is_err());
        assert!(serde_json::from_value::<PanesArgs>(json!({ "leftt": true })).is_err());
        assert!(serde_json::from_value::<OverlaysArgs>(json!({ "nope": true })).is_err());
    }

    #[test]
    fn switches_forward_the_given_ones_and_refuse_none() {
        let panes: PanesArgs = serde_json::from_value(json!({ "left": false })).unwrap();
        assert_eq!(
            switches(panes, "left, right, strip"),
            Ok(json!({ "left": false }))
        );
        let none: OverlaysArgs = serde_json::from_value(json!({})).unwrap();
        assert_eq!(
            switches(none, "focus, peaking, performance"),
            Err("set at least one of focus, peaking, performance".into())
        );
    }

    #[test]
    fn list_folders_answers_the_roots_or_one_folders_count_and_subfolders() {
        let dir = temp_dir("folders");
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::create_dir_all(dir.join("A")).unwrap();
        std::fs::write(dir.join("x.ARW"), b"").unwrap();
        let home = dir.join("b");
        let roots = list_folders(Some(home.clone()), None).unwrap();
        assert_eq!(roots["roots"][0]["path"], json!(home.to_string_lossy()));
        let path = dir.to_string_lossy().into_owned();
        let listed = list_folders(None, Some(path.clone())).unwrap();
        assert_eq!(listed["path"], json!(path));
        assert_eq!(listed["photo_count"], json!(1));
        let names: Vec<&str> = listed["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|child| child["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["A", "b"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_folder_path_must_be_absolute_and_a_folder() {
        let dir = temp_dir("folder");
        let file = dir.join("x.ARW");
        std::fs::write(&file, b"").unwrap();
        assert_eq!(
            list_folders(None, Some("photos".into())),
            Err("photos is not an absolute path".into())
        );
        assert_eq!(
            folder("photos"),
            Err("photos is not an absolute path".into())
        );
        let file_path = file.to_string_lossy().into_owned();
        assert_eq!(
            folder(&file_path),
            Err(format!("{file_path} is not a folder"))
        );
        assert_eq!(folder(&dir.to_string_lossy()), Ok(dir.as_path()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reveal_tells_a_folder_from_a_file_and_refuses_a_missing_path() {
        let dir = temp_dir("reveal");
        let file = dir.join("x.ARW");
        std::fs::write(&file, b"").unwrap();
        let path = dir.to_string_lossy().into_owned();
        assert_eq!(
            reveal(path.clone()),
            Ok(json!({ "path": path, "folder": true }))
        );
        let file_path = file.to_string_lossy().into_owned();
        assert_eq!(
            reveal(file_path.clone()),
            Ok(json!({ "path": file_path, "folder": false }))
        );
        let missing = dir.join("gone").to_string_lossy().into_owned();
        assert_eq!(
            reveal(missing.clone()),
            Err(format!("{missing} is neither a folder nor a file"))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn settings_args(value: Value) -> Result<Value, String> {
        serde_json::from_value::<SettingsArgs>(value)
            .map_err(|e| e.to_string())
            .and_then(settings)
    }

    #[test]
    fn settings_forwards_only_the_given_fields() {
        let dir = temp_dir("settings-root");
        let root = dir.to_string_lossy().into_owned();
        assert_eq!(
            settings_args(json!({
                "sidecar_format": "both",
                "label_names": { "red": "Rot" },
                "shortcuts": [
                    { "op": "add", "action": "pick", "key": "k" },
                    { "op": "reset_all" }
                ],
                "import_root": root,
                "theme": { "mode": "system", "dark": "Ayu Dark" }
            })),
            Ok(json!({
                "sidecar_format": "both",
                "label_names": { "red": "Rot" },
                "shortcuts": [
                    { "op": "add", "action": "pick", "key": "k" },
                    { "op": "reset_all" }
                ],
                "import_root": root,
                "theme": { "mode": "system", "dark": "Ayu Dark" }
            }))
        );
        assert_eq!(
            settings_args(json!({ "auto_advance": false })),
            Ok(json!({ "auto_advance": false }))
        );
    }

    #[test]
    fn settings_refuses_an_empty_call_a_bad_root_and_the_destructive_switch() {
        assert!(settings_args(json!({}))
            .unwrap_err()
            .starts_with("set at least one of"));
        assert!(settings_args(json!({ "import_root": "relative" }))
            .unwrap_err()
            .contains("is not an absolute path"));
        let file = temp_dir("settings-file").join("a.txt");
        std::fs::write(&file, b"x").unwrap();
        assert!(
            settings_args(json!({ "import_root": file.to_string_lossy() }))
                .unwrap_err()
                .contains("is not a folder")
        );
        assert!(settings_args(json!({ "mcp_allow_destructive": true }))
            .unwrap_err()
            .contains("unknown field"));
        assert!(settings_args(json!({ "sidecar_format": "tiff" })).is_err());
        assert!(settings_args(json!({ "theme": { "mode": "dim" } })).is_err());
    }

    #[test]
    fn tree_folder_takes_the_actions_by_their_snake_case_names() {
        let args: TreeArgs =
            serde_json::from_value(json!({ "path": "/d", "action": "expand_all" })).unwrap();
        assert_eq!(json!(args.action), json!("expand_all"));
        assert!(
            serde_json::from_value::<TreeArgs>(json!({ "path": "/d", "action": "open" })).is_err()
        );
        assert!(serde_json::from_value::<FolderArgs>(json!({ "folder": "/d" })).is_err());
    }

    #[test]
    fn trash_forwards_only_the_given_fields_and_refuses_an_unknown_mode() {
        let args: TrashArgs = serde_json::from_value(json!({})).unwrap();
        assert_eq!(trash(args), json!({}));
        let args: TrashArgs = serde_json::from_value(json!({
            "paths": ["/d/a.ARW"],
            "mode": "delete",
        }))
        .unwrap();
        assert_eq!(
            trash(args),
            json!({ "paths": ["/d/a.ARW"], "mode": "delete" })
        );
        let args: TrashArgs = serde_json::from_value(json!({ "id": 3, "confirm": true })).unwrap();
        assert_eq!(trash(args), json!({ "id": 3, "confirm": true }));
        assert!(serde_json::from_value::<TrashArgs>(json!({ "mode": "shred" })).is_err());
        assert!(serde_json::from_value::<TrashArgs>(json!({ "force": true })).is_err());
    }

    #[test]
    fn folder_op_forwards_the_given_fields_and_refuses_a_path_that_is_no_folder() {
        let args: FolderOpArgs =
            serde_json::from_value(json!({ "id": 2, "confirm": true })).unwrap();
        assert_eq!(folder_op(args), Ok(json!({ "id": 2, "confirm": true })));
        let dir = std::env::temp_dir();
        let path = dir.to_string_lossy().into_owned();
        let args: FolderOpArgs = serde_json::from_value(json!({ "path": path })).unwrap();
        assert_eq!(folder_op(args), Ok(json!({ "path": path })));
        let args: FolderOpArgs = serde_json::from_value(json!({ "path": "relative" })).unwrap();
        assert_eq!(
            folder_op(args),
            Err("relative is not an absolute path".into())
        );
        assert!(serde_json::from_value::<FolderOpArgs>(json!({ "force": true })).is_err());
    }

    #[test]
    fn idle_wait_defaults_and_caps_the_timeout() {
        assert_eq!(idle_wait(None), IDLE_WAIT);
        assert_eq!(idle_wait(Some(200)), Duration::from_millis(200));
        assert_eq!(idle_wait(Some(600_000)), IDLE_WAIT_MAX);
    }

    #[test]
    fn a_call_resolves_with_the_reply() {
        block_on(async {
            let sent = Arc::new(Mutex::new(Vec::new()));
            let log = sent.clone();
            let bridge = Arc::new(Bridge::new(move |request| {
                log.lock().unwrap().push(request);
                Ok(())
            }));
            let replier = bridge.clone();
            let requests = sent.clone();
            let answer = tokio::spawn(async move {
                loop {
                    let id = requests
                        .lock()
                        .unwrap()
                        .first()
                        .map(|r: &super::Request| r.id);
                    if let Some(id) = id {
                        replier.reply(id, true, serde_json::json!({ "count": 3 }));
                        return;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            });

            let reply = bridge.call("get_view", Value::Null).await;
            answer.await.unwrap();
            assert_eq!(reply, Ok(serde_json::json!({ "count": 3 })));
            assert_eq!(sent.lock().unwrap()[0].kind, "get_view");
            assert!(bridge.pending.lock().unwrap().is_empty());
        });
    }

    #[test]
    fn an_error_reply_carries_its_message() {
        block_on(async {
            let bridge = Arc::new(Bridge::new(|_| Ok(())));
            let replier = bridge.clone();
            tokio::spawn(async move {
                while replier.pending.lock().unwrap().is_empty() {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                replier.reply(0, false, Value::String("no folder".into()));
            });
            assert_eq!(
                bridge.call("get_view", Value::Null).await,
                Err("no folder".to_string())
            );
        });
    }

    #[test]
    fn a_call_times_out_when_nothing_replies() {
        block_on(async {
            let bridge = Bridge::new(|_| Ok(()));
            let reply = bridge
                .call_within("get_view", Value::Null, Duration::from_millis(20))
                .await;
            assert_eq!(
                reply,
                Err("Handpic's main window did not answer within 20ms".to_string())
            );
            assert!(bridge.pending.lock().unwrap().is_empty());
            bridge.reply(0, true, Value::Null);
        });
    }

    #[test]
    fn a_call_fails_at_once_when_the_request_cannot_be_sent() {
        block_on(async {
            let bridge = Bridge::new(|_| Err("Handpic's main window is not open".to_string()));
            assert_eq!(
                bridge.call("get_view", Value::Null).await,
                Err("Handpic's main window is not open".to_string())
            );
            assert!(bridge.pending.lock().unwrap().is_empty());
        });
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-mcp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn view(folder: &Path, current: Option<&Path>) -> Value {
        serde_json::json!({
            "folder": folder.to_string_lossy(),
            "current": current.map(|p| serde_json::json!({ "path": p.to_string_lossy(), "position": 1 })),
        })
    }

    #[test]
    fn resolve_defaults_to_the_current_photo() {
        let dir = temp_dir("current");
        let folder = std::fs::canonicalize(&dir).unwrap();
        let current = folder.join("a.ARW");
        assert_eq!(resolve(&view(&dir, Some(&current)), None), Ok(current));
        assert_eq!(
            resolve(&view(&dir, None), None),
            Err("no photo is shown in Handpic".to_string())
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_accepts_a_path_in_the_open_folder() {
        let dir = temp_dir("inside");
        let path = dir.join("b.ARW");
        let resolved = resolve(&view(&dir, None), Some(path.to_string_lossy().into_owned()));
        assert_eq!(
            resolved,
            Ok(std::fs::canonicalize(&dir).unwrap().join("b.ARW"))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_refuses_a_path_outside_the_open_folder() {
        let dir = temp_dir("outside");
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let view = view(&dir, None);
        for path in [
            sub.join("a.ARW"),
            dir.join("missing").join("a.ARW"),
            PathBuf::from("a.ARW"),
            PathBuf::from("/"),
        ] {
            let path = path.to_string_lossy().into_owned();
            let refused = resolve(&view, Some(path.clone())).unwrap_err();
            assert!(refused.starts_with(&path), "{refused}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_needs_an_open_folder() {
        assert_eq!(
            resolve(
                &serde_json::json!({ "folder": null }),
                Some("/a.ARW".into())
            ),
            Err("no folder is open in Handpic".to_string())
        );
    }

    #[test]
    fn photo_refuses_a_path_the_index_does_not_know() {
        let dir = temp_dir("unknown");
        let index = Mutex::new(Index::open(&dir.join("index.db")).unwrap());
        let path = dir.join("a.ARW");
        assert_eq!(
            photo(&index, &path),
            Err(format!(
                "{} is not a photo Handpic has indexed",
                path.display()
            ))
        );
        drop(index);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn long_edge_defaults_and_clamps() {
        assert_eq!(long_edge(None), 1024);
        assert_eq!(long_edge(Some(800)), 800);
        assert_eq!(long_edge(Some(10)), 256);
        assert_eq!(long_edge(Some(5000)), 1616);
    }

    fn judgment_args(value: Value) -> Result<Value, String> {
        judgment(serde_json::from_value(value).unwrap())
    }

    #[test]
    fn judgment_forwards_only_the_given_fields() {
        assert_eq!(
            judgment_args(json!({ "rating": 3 })),
            Ok(json!({ "rating": 3 }))
        );
        assert_eq!(
            judgment_args(json!({ "paths": ["/d/a"], "flag": "reject", "label": null })),
            Ok(json!({ "paths": ["/d/a"], "flag": "reject", "label": null }))
        );
        assert_eq!(
            judgment_args(json!({ "rating": 0, "label": "Purple" })),
            Ok(json!({ "rating": 0, "label": "Purple" }))
        );
    }

    #[test]
    fn judgment_refuses_what_the_keys_cannot_set() {
        assert_eq!(
            judgment_args(json!({ "rating": 6 })),
            Err("rating must be an integer from 0 to 5".to_string())
        );
        assert_eq!(
            judgment_args(json!({ "label": "Teal" })),
            Err(
                "label must be one of Red, Orange, Yellow, Green, Blue, Pink, Purple, or null"
                    .to_string()
            )
        );
        assert_eq!(
            judgment_args(json!({ "paths": ["/d/a"] })),
            Err("set at least one of rating, flag, label".to_string())
        );
        assert!(serde_json::from_value::<JudgmentArgs>(json!({ "flag": "maybe" })).is_err());
    }
}
