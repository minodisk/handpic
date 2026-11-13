//! The MCP companion's server, the backend's `mcp::Mcp`, on a tokio runtime
//! of its own on one named thread, started only when `mcpEnabled` is on. A
//! tool call reaches the view as `Event::Mcp` through the view's channel,
//! and the view answers through `Server::reply`.

use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use futures::channel::mpsc::UnboundedSender;
use gpui::App;
use riffle_backend::{
    index::Index,
    mcp::{Bridge, MCP_PORT, Mcp},
};
use serde_json::Value;

use crate::scan::Event;

/// How long the quit handler waits for the server to stop: the backend's
/// own 2 s stop timeout and a little more.
const STOP_WAIT: Duration = Duration::from_millis(2500);

/// The running server, which the view replies through.
pub struct Server {
    mcp: Arc<Mcp>,
}

impl Server {
    /// Complete the tool call `id` with the view's answer.
    pub fn reply(&self, id: u64, reply: Result<Value, String>) {
        match reply {
            Ok(value) => self.mcp.reply(id, true, value),
            Err(message) => self.mcp.reply(id, false, Value::String(message)),
        }
    }
}

/// Start the server on `port` (else `MCP_PORT`) when `enabled`, and stop it
/// on quit. Off, nothing is started. A failed bind is logged by the backend
/// and the app runs on without a server.
pub fn start(
    enabled: bool,
    port: Option<u16>,
    events: UnboundedSender<Event>,
    index: Option<Arc<Mutex<Index>>>,
    cx: &mut App,
) -> Option<Server> {
    if !enabled {
        return None;
    }
    let bridge = Bridge::new(move |request| {
        events
            .unbounded_send(Event::Mcp(request))
            .map_err(|_| "Handpic's main window is not open".to_string())
    });
    let mcp = Arc::new(Mcp::new(bridge, index).with_port(port.unwrap_or(MCP_PORT)));
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            log::error!("mcp: failed to build the runtime: {e}");
            return None;
        }
    };
    let (stop, stopping) = tokio::sync::oneshot::channel::<()>();
    let (stopped, done) = mpsc::channel::<()>();
    let served = mcp.clone();
    let spawned = thread::Builder::new().name("mcp".into()).spawn(move || {
        runtime.block_on(async {
            let state = served.switch(true).await;
            log::info!("mcp: {state:?}");
            stopping.await.ok();
            served.stop().await;
            stopped.send(()).ok();
        });
    });
    if let Err(e) = spawned {
        log::error!("mcp: failed to spawn the server thread: {e}");
        return None;
    }
    let mut stop = Some(stop);
    App::on_app_quit(cx, move |_| {
        let started = Instant::now();
        if let Some(stop) = stop.take() {
            stop.send(()).ok();
        }
        let stopped = done.recv_timeout(STOP_WAIT).is_ok();
        log::info!(
            "quit: MCP server stopped={stopped} in {}ms",
            started.elapsed().as_millis()
        );
        async {}
    })
    .detach();
    Some(Server { mcp })
}
