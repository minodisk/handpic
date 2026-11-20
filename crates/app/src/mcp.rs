//! The MCP companion's server, the backend's `mcp::Mcp` with every tool
//! (`with_all_tools`, and `with_screenshot` in a build with the `screenshot`
//! feature) and the tree's home directory (`with_home`), on a tokio runtime
//! of its own on one named thread, spawned on the first switch on of
//! `mcpEnabled` and kept for the session, which runs each switch in order
//! and sends the state it returns as `Event::McpState`. A tool call reaches
//! the view as `Event::Mcp` through the view's channel, and the view answers
//! through `Control::reply`.

use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use futures::channel::mpsc::UnboundedSender;
use gpui_kit::App;
use handpic_engine::{
    index::Index,
    mcp::{Bridge, MCP_PORT, Mcp, McpState},
};
use serde_json::Value;

use crate::scan::Event;

/// How long the quit handler waits for the server to stop: the backend's
/// own 2 s stop timeout and a little more.
const STOP_WAIT: Duration = Duration::from_millis(2500);

enum Command {
    Switch(bool),
    Quit,
}

/// The server's thread, which the view switches and replies through.
struct Server {
    mcp: Arc<Mcp>,
    commands: tokio::sync::mpsc::UnboundedSender<Command>,
}

/// The server on `port`, `--mcp-port`'s else `MCP_PORT`, switched by the
/// `mcpEnabled` switch. No runtime and no thread until the first switch on.
pub struct Control {
    port: u16,
    events: UnboundedSender<Event>,
    index: Option<Arc<Mutex<Index>>>,
    server: Option<Server>,
}

impl Control {
    /// The control, the server started when `enabled`.
    pub fn new(
        enabled: bool,
        port: Option<u16>,
        events: UnboundedSender<Event>,
        index: Option<Arc<Mutex<Index>>>,
        cx: &mut App,
    ) -> Self {
        let mut control = Control {
            port: port.unwrap_or(MCP_PORT),
            events,
            index,
            server: None,
        };
        if enabled {
            control.switch(true, cx);
        }
        control
    }

    /// The port a switch on binds.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Start or stop the server; its state comes back as
    /// `Event::McpState`. A failed bind is logged by the backend and
    /// reported in that state, and the app runs on without a server.
    pub fn switch(&mut self, enabled: bool, cx: &mut App) {
        if self.server.is_none() {
            if !enabled {
                return;
            }
            match self.spawn(cx) {
                Ok(server) => self.server = Some(server),
                Err(e) => {
                    log::error!("mcp: {e}");
                    self.events
                        .unbounded_send(Event::McpState(McpState {
                            enabled: true,
                            port: self.port,
                            error: Some(e),
                        }))
                        .ok();
                    return;
                }
            }
        }
        if let Some(server) = &self.server {
            server.commands.send(Command::Switch(enabled)).ok();
        }
    }

    /// Complete the tool call `id` with the view's answer.
    pub fn reply(&self, id: u64, reply: Result<Value, String>) {
        let Some(server) = &self.server else { return };
        match reply {
            Ok(value) => server.mcp.reply(id, true, value),
            Err(message) => server.mcp.reply(id, false, Value::String(message)),
        }
    }

    /// The server's thread and runtime, stopped on quit.
    fn spawn(&self, cx: &mut App) -> Result<Server, String> {
        let events = self.events.clone();
        let bridge = Bridge::new(move |request| {
            events
                .unbounded_send(Event::Mcp(request))
                .map_err(|_| "Handpic's main window is not open".to_string())
        });
        let mcp = Mcp::new(bridge, self.index.clone())
            .with_all_tools()
            .with_home(dirs::home_dir())
            .with_port(self.port);
        #[cfg(feature = "screenshot")]
        let mcp = mcp.with_screenshot();
        let mcp = Arc::new(mcp);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("failed to build the runtime: {e}"))?;
        let (commands, mut received) = tokio::sync::mpsc::unbounded_channel();
        let (stopped, done) = mpsc::channel::<()>();
        let served = mcp.clone();
        let events = self.events.clone();
        thread::Builder::new()
            .name("mcp".into())
            .spawn(move || {
                runtime.block_on(async {
                    while let Some(Command::Switch(enabled)) = received.recv().await {
                        let state = served.switch(enabled).await;
                        log::info!("mcp: {state:?}");
                        events.unbounded_send(Event::McpState(state)).ok();
                    }
                    served.stop().await;
                    stopped.send(()).ok();
                });
            })
            .map_err(|e| format!("failed to spawn the server thread: {e}"))?;
        let quit = commands.clone();
        App::on_app_quit(cx, move |_| {
            let started = Instant::now();
            quit.send(Command::Quit).ok();
            let stopped = done.recv_timeout(STOP_WAIT).is_ok();
            log::info!(
                "quit: MCP server stopped={stopped} in {}ms",
                started.elapsed().as_millis()
            );
            async {}
        })
        .detach();
        Ok(Server { mcp, commands })
    }
}
