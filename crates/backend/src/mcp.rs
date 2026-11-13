//! The embedded MCP server: a Streamable HTTP endpoint on the loopback
//! interface that lets any MCP client follow and assist a culling session.
//! It is off by default and turned on by the `mcpEnabled` setting. The view
//! state and the judgments the tools read and drive are the app's, reached
//! through the `Bridge`'s send callback; the server's tasks run on the
//! caller's tokio runtime, so `Mcp::switch` and `Mcp::stop` must run inside
//! one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
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
use crate::index::{self, Focus, Index};
use crate::ondemand::read_preview;

/// The fixed loopback port, outside the ranges development servers commonly
/// take, so the connection examples stay the same from launch to launch.
pub const MCP_PORT: u16 = 41917;

/// How long a tool call waits for the main window to answer.
const BRIDGE_TIMEOUT: Duration = Duration::from_secs(5);

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

/// The MCP handler each client session gets.
#[derive(Clone)]
pub struct Companion {
    bridge: Arc<Bridge>,
    /// The index reader, `None` when the index cache is unavailable.
    index: Option<Arc<Mutex<Index>>>,
}

impl Companion {
    async fn target(&self, path: Option<String>) -> Result<PathBuf, String> {
        let view = self
            .bridge
            .call("get_view", Value::Object(Default::default()))
            .await?;
        resolve(&view, path)
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

#[tool_handler]
impl ServerHandler for Companion {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("handpic", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
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
            companion: Companion {
                bridge: Arc::new(bridge),
                index,
            },
            port: MCP_PORT,
        }
    }

    /// The server listening on `port` instead of `MCP_PORT`.
    pub fn with_port(self, port: u16) -> Self {
        Mcp { port, ..self }
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
        Companion {
            bridge: Arc::new(Bridge::new(|_| Ok(()))),
            index: None,
        }
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

    #[test]
    fn the_tools_are_exactly_the_seven_companion_tools() {
        let names: Vec<_> = Companion::tool_router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect();
        assert_eq!(
            names,
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
