//! The main view on GPUI's test platform: `Ui`, which opens it as `main`
//! does over a temporary index and settings store, and the UI tests.

use std::{
    fs,
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

use gpui_kit::{Context, Entity, TestAppContext, VisualTestContext, px};
use handpic_core::{Flag, scan::Priority};
use handpic_engine::store::Store;
use tempfile::TempDir;

use crate::{
    Handpic,
    args::Args,
    entries_of, file_name,
    filter::Orientation,
    fitted_name, menu, open_index, open_main,
    panewidths::PaneWidths,
    scan::{Priorities, Running},
    settings,
};

/// How long `wait_until` waits on the app's own threads before failing.
const WAIT: Duration = Duration::from_secs(10);

/// Under every quit handler's drain timeout, so a quit that waits one out
/// fails the test.
const QUIT_MAX: Duration = Duration::from_secs(1);

/// The `log` records of every test in the process, each stamped with the
/// time since the first `Ui` opened, which a timed-out `wait_until` prints
/// for its own folder: no logger is installed under `cargo test` otherwise.
struct TestLog {
    started: Instant,
    lines: Mutex<Vec<String>>,
}

impl log::Log for TestLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let line = format!(
                "{:>8.3}s [{}][{}] {}",
                self.started.elapsed().as_secs_f64(),
                record.target(),
                record.level(),
                record.args()
            );
            self.lines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(line);
        }
    }

    fn flush(&self) {}
}

static LOG: OnceLock<TestLog> = OnceLock::new();

fn install_log() {
    let log = LOG.get_or_init(|| TestLog {
        started: Instant::now(),
        lines: Mutex::new(Vec::new()),
    });
    if log::set_logger(log).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

/// The logged lines that name `dir`, a temporary folder's unique name.
fn log_lines(dir: &str) -> String {
    LOG.get().map_or_else(String::new, |log| {
        log.lines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|line| line.contains(dir))
            .map(|line| format!("\n  {line}"))
            .collect()
    })
}

pub struct Ui {
    pub cx: VisualTestContext,
    /// Taken at the drop, so the view goes with the app.
    view: Option<Entity<Handpic>>,
    /// The view's scan, cancelled when the `Ui` drops while panicking.
    scan: Running,
    /// Taken at the drop, which removes it once the app's threads let go.
    dir: Option<TempDir>,
}

impl Ui {
    /// The main window over a new index and settings store in a temporary
    /// folder, with no folder open.
    pub fn open() -> Self {
        Self::open_with(Args::default())
    }

    /// `open`, under the command line's `args`.
    pub fn open_with(args: Args) -> Self {
        install_log();
        let dir = tempfile::tempdir().unwrap();
        let cx = TestAppContext::single();
        // The view's scan, watchers and writers are threads of their own, so
        // the scheduler may park while they work.
        cx.executor().allow_parking();
        let index = open_index(Some(dir.path().join("index.sqlite")));
        let settings = settings::load(Some(dir.path().join("settings.sqlite")), None);
        let (window, view) = cx
            .update(|cx| open_main(args, index, settings, None, cx))
            .unwrap();
        let mut cx = VisualTestContext::from_window(window, &cx);
        // Pass 2 at the app's lowest priority starves past `WAIT` behind the
        // other tests running in parallel, on every OS.
        let scan = view.update(&mut cx, |view, _| {
            view.scan_priorities = Priorities {
                extract: Priority::Normal,
                faces: Priority::Normal,
                allfaces: Priority::Normal,
            };
            view.scan.clone()
        });
        cx.run_until_parked();
        Self {
            cx,
            view: Some(view),
            scan,
            dir: Some(dir),
        }
    }

    pub fn read<R>(&self, read: impl FnOnce(&Handpic) -> R) -> R {
        self.view
            .as_ref()
            .unwrap()
            .read_with(&self.cx, |view, _| read(view))
    }

    pub fn update<R>(
        &mut self,
        update: impl FnOnce(&mut Handpic, &mut Context<Handpic>) -> R,
    ) -> R {
        let view = self.view.clone().unwrap();
        let result = view.update(&mut self.cx, update);
        self.cx.run_until_parked();
        result
    }

    pub fn dir(&self) -> &Path {
        self.dir.as_ref().unwrap().path()
    }

    /// Writes `files` (name, Orientation tag) as generated DNGs into a
    /// `photos` folder, opens it as a click in the tree does and waits for the
    /// listing, both scan passes and the current file's preview.
    pub fn open_folder(&mut self, files: &[(&str, u16)]) -> PathBuf {
        let folder = self.dir().join("photos");
        fs::create_dir(&folder).unwrap();
        for (name, orientation) in files {
            fs::write(folder.join(name), dng(*orientation)).unwrap();
        }
        let path = folder.to_string_lossy().into_owned();
        self.update(|view, cx| view.open_from_tree(path, cx).detach());
        self.wait_idle("the folder's listing, scan and preview", files.len());
        folder
    }

    /// Waits until `listed` files are listed, the view is idle (no listing,
    /// scan or run) and the current file's preview is on screen.
    pub fn wait_idle(&mut self, what: &str, listed: usize) {
        self.wait_until(what, |view| {
            view.dir.is_some()
                && view.all.len() == listed
                && view.status_of().idle()
                && view.screen.as_ref().map(|s| &s.path) == view.current_path().as_ref()
        });
    }

    /// The current file's name.
    pub fn current(&self) -> String {
        self.read(|view| file_name(&view.files[view.pager.current()]))
    }

    /// The shown files' names, in the strip's order.
    pub fn names(&self) -> Vec<String> {
        self.read(|view| view.files.iter().map(|p| file_name(p)).collect())
    }

    /// Runs the test platform's work until `done` holds for the view,
    /// letting the app's own threads, which the platform does not run, catch
    /// up in real time; fails the test after `WAIT`.
    pub fn wait_until(&mut self, what: &str, done: impl Fn(&Handpic) -> bool) {
        let started = Instant::now();
        loop {
            self.cx.run_until_parked();
            if self.read(&done) {
                return;
            }
            if started.elapsed() >= WAIT {
                let state = self.read(|view| {
                    let s = view.status_of();
                    format!(
                        "dir {:?}, listed {}, listing {}, scan {:?}, running {}, opening {:?}, screen {:?}, current {:?}",
                        view.dir,
                        view.all.len(),
                        s.listing,
                        s.scan,
                        s.running,
                        s.opening,
                        view.screen.as_ref().map(|s| &s.path),
                        view.current_path(),
                    )
                });
                let name = self
                    .dir()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                panic!(
                    "timed out waiting for {what}: {state}; log:{}",
                    log_lines(&name)
                );
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

/// A baseline JPEG of `width` x `height`, a gradient.
fn jpeg(width: usize, height: usize) -> Vec<u8> {
    let rgb: Vec<u8> = (0..width * height)
        .flat_map(|i| [(i % width * 4) as u8, (i / width * 4) as u8, 128])
        .collect();
    let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    c.set_size(width, height);
    let mut c = c.start_compress(Vec::new()).unwrap();
    c.write_scanlines(&rgb).unwrap();
    c.finish().unwrap()
}

/// The smallest DNG the reader takes: a little-endian TIFF whose IFD0 holds
/// the Orientation tag and an embedded preview JPEG, and no raw data.
fn dng(orientation: u16) -> Vec<u8> {
    let preview = jpeg(64, 48);
    let entries: [(u16, u16, u32); 3] = [
        (0x0112, 3, u32::from(orientation)),
        (0x0201, 4, 8 + 2 + 3 * 12 + 4),
        (0x0202, 4, preview.len() as u32),
    ];
    let mut buf = b"II".to_vec();
    buf.extend_from_slice(&42u16.to_le_bytes());
    buf.extend_from_slice(&8u32.to_le_bytes());
    buf.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, kind, value) in entries {
        buf.extend_from_slice(&tag.to_le_bytes());
        buf.extend_from_slice(&kind.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&value.to_le_bytes());
    }
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&preview);
    buf
}

/// Removes `dir`, retrying while the app's threads, which end only once
/// their senders are dropped, still hold its SQLite files open.
fn remove_within(dir: &Path, wait: Duration) -> bool {
    let started = Instant::now();
    loop {
        if fs::remove_dir_all(dir).is_ok() || !dir.exists() {
            return true;
        }
        if started.elapsed() > wait {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

impl Drop for Ui {
    /// Quits as the app does, running the quit handlers' drains, then
    /// removes the temporary folder.
    fn drop(&mut self) {
        self.view.take();
        if thread::panicking() {
            self.scan.cancel();
            return;
        }
        let started = Instant::now();
        self.cx.cx.quit();
        let quit = started.elapsed();
        assert!(quit < QUIT_MAX, "the quit handlers took {quit:?}");
        let dir = self.dir.take().unwrap().keep();
        assert!(
            remove_within(&dir, WAIT),
            "{} is still held after the quit",
            dir.display()
        );
    }
}

#[test]
fn opens_with_no_folder() {
    let mut ui = Ui::open();
    ui.wait_until("the index eviction", |view| view.eviction.is_finished());
    ui.read(|view| {
        assert!(view.index.is_ok());
        assert!(view.dir.is_none());
        assert!(view.files.is_empty());
        assert_eq!(view.status.as_ref(), "no folder");
    });
    let quads = ui.cx.update(|window, _| window.painted_quads());
    assert!(!quads.is_empty());
    let dir = ui.dir().to_path_buf();
    drop(ui);
    assert!(!dir.exists());
}

#[test]
fn opening_a_folder_lists_its_files() {
    let mut ui = Ui::open();
    ui.open_folder(&[("c.dng", 1), ("a.dng", 1), ("b.dng", 1)]);
    assert_eq!(ui.names(), ["a.dng", "b.dng", "c.dng"]);
    assert_eq!(ui.current(), "a.dng");
    ui.read(|view| {
        assert!(!view.view_only);
        assert_eq!(view.status().as_ref(), "");
        assert_eq!(view.failed, 0);
        let cells = view.strip_cells();
        let names: Vec<&str> = cells.iter().map(|c| c.name.as_ref()).collect();
        assert_eq!(names, ["a.dng", "b.dng", "c.dng"]);
        let current: Vec<bool> = cells.iter().map(|c| c.current).collect();
        assert_eq!(current, [true, false, false]);
    });
}

#[test]
fn the_paging_keys_move_through_the_keymap_and_wrap() {
    let mut ui = Ui::open();
    ui.open_folder(&[("a.dng", 1), ("b.dng", 1), ("c.dng", 1)]);
    ui.cx.simulate_keystrokes("right");
    assert_eq!(ui.current(), "b.dng");
    ui.cx.simulate_keystrokes("right right");
    assert_eq!(ui.current(), "a.dng");
    ui.cx.simulate_keystrokes("left");
    assert_eq!(ui.current(), "c.dng");
    let current = ui.read(|view| {
        view.strip_cells()
            .iter()
            .map(|c| c.current)
            .collect::<Vec<_>>()
    });
    assert_eq!(current, [false, false, true]);
    ui.wait_idle("the last file's preview", 3);
}

#[test]
fn a_judgment_key_judges_the_current_file_in_the_view_the_index_and_the_strip() {
    let mut ui = Ui::open();
    let folder = ui.open_folder(&[("a.dng", 1), ("b.dng", 1)]);
    ui.cx.simulate_keystrokes("3");
    let (rating, marks, dir) = ui.read(|view| {
        let path = view.files[0].to_string_lossy().into_owned();
        (
            view.judgments.get(&path).and_then(|s| s.rating),
            view.strip_cells()[0].marks,
            view.dir.clone().unwrap(),
        )
    });
    assert_eq!(rating, Some(3));
    assert_eq!((marks.stars, marks.flag), (3, Flag::None));
    ui.wait_until("the rating's index row", |view| {
        let reader = view.reader.as_ref().unwrap();
        entries_of(reader, &dir).is_some_and(|(judgments, _, _)| {
            judgments
                .iter()
                .any(|(path, state)| path.ends_with("a.dng") && state.rating == Some(3))
        })
    });
    ui.wait_until("the rating's sidecar", |_| folder.join("a.xmp").exists());
    ui.wait_idle("the current file's preview", 2);
}

#[test]
fn a_filter_choice_narrows_the_files_and_lands_on_the_anchor() {
    let mut ui = Ui::open();
    ui.open_folder(&[("a.dng", 1), ("b.dng", 6), ("c.dng", 1), ("d.dng", 8)]);
    ui.cx.simulate_keystrokes("right");
    assert_eq!(ui.current(), "b.dng");
    ui.update(|view, cx| {
        view.apply_menu(
            vec![menu::Action::Toggle(menu::Toggle::Orientation(
                Orientation::Landscape,
            ))],
            cx,
        )
    });
    assert_eq!(ui.names(), ["a.dng", "c.dng"]);
    assert_eq!(ui.current(), "c.dng");
    let cells = ui.read(|view| view.strip_cells().len());
    assert_eq!(cells, 2);
    ui.wait_idle("the anchor's preview", 4);
}

#[test]
fn the_settings_modal_drops_the_paging_keys_until_escape_closes_it() {
    let mut ui = Ui::open();
    ui.open_folder(&[("a.dng", 1), ("b.dng", 1)]);
    let open = if cfg!(target_os = "macos") {
        "cmd-,"
    } else {
        "ctrl-,"
    };
    let settings = ui.read(|view| view.settings_view.clone());
    let is_open = |ui: &Ui| settings.read_with(&ui.cx, |settings, _| settings.is_open());
    let before = ui.cx.update(|window, _| window.painted_quads().len());
    ui.cx.simulate_keystrokes(open);
    assert!(is_open(&ui));
    let after = ui.cx.update(|window, _| window.painted_quads().len());
    assert!(
        after > before,
        "the modal drew nothing: {before} -> {after} quads"
    );
    ui.cx.simulate_keystrokes("right");
    assert_eq!(ui.current(), "a.dng");
    ui.cx.simulate_keystrokes("escape");
    assert!(!is_open(&ui));
    ui.cx.simulate_keystrokes("right");
    assert_eq!(ui.current(), "b.dng");
    ui.wait_idle("the next file's preview", 2);
}

#[test]
fn the_settings_layer_lies_under_the_title_bar_and_fills_the_rest() {
    let mut ui = Ui::open();
    ui.open_folder(&[("a.dng", 1)]);
    let open = if cfg!(target_os = "macos") {
        "cmd-,"
    } else {
        "ctrl-,"
    };
    let settings = ui.read(|view| view.settings_view.clone());
    ui.cx.simulate_keystrokes(open);
    assert!(settings.read_with(&ui.cx, |settings, _| settings.is_open()));
    let bounds = ui
        .cx
        .debug_bounds("settings-box")
        .expect("the settings box is not drawn");
    let (viewport, pad) = ui.cx.update(|window, _| {
        (
            window.viewport_size(),
            gpui_kit::component::window_paddings(window),
        )
    });
    assert_eq!(bounds.origin.x, pad.left);
    assert_eq!(
        bounds.origin.y,
        pad.top + gpui_kit::component::TITLE_BAR_HEIGHT
    );
    assert_eq!(bounds.right(), viewport.width - pad.right);
    assert_eq!(bounds.bottom(), viewport.height - pad.bottom);
    ui.cx.simulate_click(
        gpui_kit::point(viewport.width / 2., pad.top + px(10.)),
        gpui_kit::Modifiers::default(),
    );
    assert!(settings.read_with(&ui.cx, |settings, _| settings.is_open()));
}

#[test]
fn the_no_folder_message_wraps_inside_a_narrow_preview_and_still_opens_the_picker() {
    // The test text system gives every glyph the same synthetic width, which
    // puts this message between the narrow preview's 208 px content box and
    // the default window's preview, so it is one line only when wide.
    let mut ui = Ui::open();
    let wide = ui
        .cx
        .debug_bounds("empty-text")
        .expect("the no-folder message is not drawn");
    ui.update(|view, _| {
        view.pane_widths = PaneWidths {
            left: 480.,
            right: 480.,
        }
    });
    let height = ui.cx.update(|window, _| window.viewport_size().height);
    ui.cx.simulate_resize(gpui_kit::size(px(1200.), height));
    ui.cx.run_until_parked();
    let pad = ui
        .cx
        .update(|window, _| gpui_kit::component::window_paddings(window));
    let bounds = ui
        .cx
        .debug_bounds("empty-text")
        .expect("the no-folder message is not drawn");
    assert!(bounds.origin.x >= pad.left + px(480. + 16.), "{bounds:?}");
    assert!(bounds.right() <= pad.left + px(720. - 16.), "{bounds:?}");
    assert!(bounds.size.height > wide.size.height, "{bounds:?} {wide:?}");
    assert!(!ui.cx.did_prompt_for_paths());
    ui.cx
        .simulate_click(bounds.center(), gpui_kit::Modifiers::default());
    assert!(ui.cx.did_prompt_for_paths());
}

#[test]
fn the_settings_key_opens_the_settings_from_the_tree_but_not_from_a_rename() {
    let mut ui = Ui::open();
    let folder = ui.open_folder(&[("a.dng", 1), ("b.dng", 1)]);
    let open = if cfg!(target_os = "macos") {
        "cmd-,"
    } else {
        "ctrl-,"
    };
    let settings = ui.read(|view| view.settings_view.clone());
    let is_open = |ui: &Ui| settings.read_with(&ui.cx, |settings, _| settings.is_open());
    ui.update(|view, cx| view.set_tree_keys(true, cx));
    ui.cx.simulate_keystrokes(open);
    assert!(is_open(&ui));
    ui.cx.simulate_keystrokes("escape");
    assert!(!is_open(&ui));
    ui.read(|view| assert!(view.tree_keys));

    let path = folder.to_string_lossy().into_owned();
    let view = ui.view.clone().unwrap();
    view.update_in(&mut ui.cx, |view, window, cx| {
        if !view.tree_rows.iter().any(|row| row.path == path) {
            view.tree_rows.push(crate::tree::Row {
                depth: 0,
                path: path.clone(),
                name: "photos".into(),
                expanded: false,
                expandable: false,
                raw_count: None,
                failed: None,
            });
        }
        view.start_rename(path, None, window, cx);
        assert!(view.rename_owns_keys());
    });
    ui.cx.run_until_parked();
    ui.cx.simulate_keystrokes(open);
    assert!(!is_open(&ui));
}

/// Flips the MCP page's server switch as a click does.
fn flip_mcp(ui: &mut Ui) {
    let settings = ui.read(|view| view.settings_view.clone());
    settings.update(&mut ui.cx, |view, cx| view.flip_mcp_enabled(cx));
    ui.cx.run_until_parked();
}

/// The stored `mcpEnabled`, once the view's writes are in.
fn stored_mcp_enabled(ui: &Ui) -> Option<serde_json::Value> {
    assert!(ui.read(|view| view.settings.writes.drain(settings::DRAIN_TIMEOUT)));
    let store = Store::open(&ui.dir().join("settings.sqlite"), Default::default).unwrap();
    store.values().unwrap().get("mcpEnabled").cloned()
}

#[test]
fn the_mcp_switch_starts_and_stops_the_server_and_is_stored() {
    let mut ui = Ui::open_with(Args {
        mcp_port: Some(0),
        ..Args::default()
    });
    assert!(ui.read(|view| view.mcp_state.is_none()));
    flip_mcp(&mut ui);
    ui.wait_until("the server's start", |view| {
        view.mcp_state
            .as_ref()
            .is_some_and(|state| state.enabled && state.error.is_none())
    });
    let port = ui.read(|view| view.mcp_state.as_ref().unwrap().port);
    assert_ne!(port, 0);
    assert!(TcpStream::connect(("127.0.0.1", port)).is_ok());
    assert_eq!(stored_mcp_enabled(&ui), Some(true.into()));
    flip_mcp(&mut ui);
    ui.wait_until("the server's stop", |view| {
        view.mcp_state.as_ref().is_some_and(|state| !state.enabled)
    });
    assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
    assert_eq!(stored_mcp_enabled(&ui), Some(false.into()));
}

#[test]
fn a_port_in_use_leaves_the_switch_on_with_the_bind_error() {
    let held = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = held.local_addr().unwrap().port();
    let mut ui = Ui::open_with(Args {
        mcp_port: Some(port),
        ..Args::default()
    });
    flip_mcp(&mut ui);
    ui.wait_until("the failed start", |view| {
        view.mcp_state
            .as_ref()
            .is_some_and(|state| state.error.is_some())
    });
    ui.read(|view| {
        assert!(view.settings.mcp_enabled);
        let state = view.mcp_state.as_ref().unwrap();
        assert!(state.enabled);
        assert!(
            crate::settingsmodal::mcp_status(true, Some(state))
                .starts_with("The server could not start: could not listen on 127.0.0.1:")
        );
    });
    assert_eq!(stored_mcp_enabled(&ui), Some(true.into()));
    drop(held);
}

#[test]
fn a_folder_rename_in_flight_starts_no_prefetch() {
    let mut ui = Ui::open();
    let folder = ui.open_folder(&[("a.dng", 1), ("b.dng", 1), ("c.dng", 1)]);
    let started = ui.update(|view, cx| {
        view.clear_ring(cx);
        view.request_prefetch(cx);
        view.prefetch.as_ref().unwrap().still_wanted()
    });
    assert!(started, "an emptied ring starts no read with no rename");
    ui.wait_until("the prefetch's end", |view| {
        !view.prefetch.as_ref().unwrap().still_wanted()
    });
    let path = folder.to_string_lossy().into_owned();
    let started = ui.update(|view, cx| {
        view.clear_ring(cx);
        view.request_rename(path, "renamed".into(), cx);
        assert!(view.renaming());
        view.request_prefetch(cx);
        view.prefetch.as_ref().unwrap().still_wanted()
    });
    assert!(!started);
    let renamed = ui.dir().join("renamed");
    ui.wait_until("the rename's reopen", |view| {
        view.dir
            .as_ref()
            .is_some_and(|dir| dir.ends_with("renamed"))
    });
    ui.wait_idle("the renamed folder's preview", 3);
    assert!(renamed.exists());
}

#[test]
fn a_rename_held_over_the_scan_runs_at_its_end_and_the_ring_fills_again() {
    let mut ui = Ui::open();
    let folder = ui.dir().join("photos");
    fs::create_dir(&folder).unwrap();
    for name in ["a.dng", "b.dng", "c.dng"] {
        fs::write(folder.join(name), dng(1)).unwrap();
    }
    let path = folder.to_string_lossy().into_owned();
    let held = ui.update(|view, cx| {
        view.open_from_tree(path.clone(), cx).detach();
        view.request_rename(path, "renamed".into(), cx);
        view.gate.waiting()
    });
    assert_eq!(held, Some(crate::rename::LABEL));
    let renamed = ui.dir().join("renamed");
    ui.wait_until("the held rename's reopen", |view| {
        !view.renaming()
            && view
                .dir
                .as_ref()
                .is_some_and(|dir| dir.ends_with("renamed"))
    });
    assert!(renamed.exists());
    assert!(!folder.exists());
    ui.read(|view| assert_eq!(view.step_note, None));
    ui.wait_idle("the renamed folder's preview", 3);
    assert_eq!(ui.current(), "a.dng");
    ui.wait_until("the ring's neighbour", |view| {
        let next = view.files[1].to_string_lossy();
        view.prefetch.as_ref().unwrap().holds(&next)
    });
}

#[test]
fn a_refused_folder_rename_fills_the_ring_again() {
    let mut ui = Ui::open();
    let folder = ui.open_folder(&[("a.dng", 1), ("b.dng", 1), ("c.dng", 1)]);
    fs::create_dir(ui.dir().join("taken")).unwrap();
    let path = folder.to_string_lossy().into_owned();
    let started = ui.update(|view, cx| {
        view.clear_ring(cx);
        view.request_rename(path, "taken".into(), cx);
        assert!(view.renaming());
        view.request_prefetch(cx);
        view.prefetch.as_ref().unwrap().still_wanted()
    });
    assert!(!started);
    ui.wait_until("the refused rename's end", |view| !view.renaming());
    ui.read(|view| assert!(view.step_note.is_some()));
    ui.wait_until("the ring's neighbour", |view| {
        let next = view.files[1].to_string_lossy();
        view.prefetch.as_ref().unwrap().holds(&next)
    });
}

#[test]
fn a_rename_of_another_folder_does_not_hold_the_prefetch() {
    let mut ui = Ui::open();
    ui.open_folder(&[("a.dng", 1), ("b.dng", 1), ("c.dng", 1)]);
    let other = ui.dir().join("other");
    fs::create_dir(&other).unwrap();
    let path = other.to_string_lossy().into_owned();
    ui.update(|view, cx| {
        view.request_rename(path, "elsewhere".into(), cx);
        assert!(!view.renaming());
    });
}

#[test]
fn a_strip_name_wider_than_its_cell_is_shortened_in_the_middle() {
    let mut ui = Ui::open();
    let (long, short) = ui.cx.update(|window, cx| {
        let width = px(146.);
        (
            fitted_name(&"DSC-RX100M2_DSC00416.ARW".into(), width, window, cx),
            fitted_name(&"DSC00416.ARW".into(), width, window, cx),
        )
    });
    assert!(long.starts_with("DSC-RX100M2_"), "{long}");
    assert!(long.ends_with(".ARW"), "{long}");
    assert_eq!(long.matches('\u{2026}').count(), 1, "{long}");
    assert!(
        long.chars().count() < "DSC-RX100M2_DSC00416.ARW".len(),
        "{long}"
    );
    assert_eq!(short.as_ref(), "DSC00416.ARW");
}

/// A Clear Cache an MCP client confirmed during a re-listing, whose end, the
/// idle point, passes while the dialog `ui` shows is up: the clear stays held.
pub(crate) fn hold_a_clear_past_the_idle_point(ui: &mut Ui) {
    ui.update(|view, cx| {
        view.resync.open();
        view.relist(cx);
        view.confirmed_clear(cx);
        assert!(view.clear.waiting());
    });
    ui.wait_until("the re-listing's end", |view| !view.clear_busy());
    ui.read(|view| assert!(view.clear.waiting(), "drained under a dialog"));
}

#[test]
fn canceling_the_trash_dialog_drains_a_held_clear() {
    let mut ui = Ui::open();
    ui.open_folder(&[("a.dng", 1), ("b.dng", 1)]);
    ui.update(|view, cx| {
        let paths = [view.files[0].to_string_lossy().into_owned()];
        view.confirm_trash(&paths, crate::trash::Mode::Trash, cx);
        assert!(view.trash.busy());
    });
    hold_a_clear_past_the_idle_point(&mut ui);
    let running = ui.update(|view, cx| {
        view.dismiss_trash(cx);
        view.clear.running()
    });
    assert!(running);
    ui.wait_until("the clear's end", |view| !view.clear.running());
}
