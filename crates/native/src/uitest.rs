//! The main view on GPUI's test platform: `Ui`, which opens it as `main`
//! does over a temporary index and settings store, and the UI tests.

use std::{
    fs,
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use gpui_kit::{Context, Entity, TestAppContext, VisualTestContext};
use handpic_core::Flag;
use tempfile::TempDir;

use crate::{
    Handpic, args::Args, entries_of, file_name, filter::Orientation, menu, open_index, open_main,
    settings,
};

/// How long `wait_until` waits on the app's own threads before failing.
const WAIT: Duration = Duration::from_secs(10);

/// Under every quit handler's drain timeout, so a quit that waits one out
/// fails the test.
const QUIT_MAX: Duration = Duration::from_secs(1);

pub struct Ui {
    pub cx: VisualTestContext,
    /// Taken at the drop, so the view goes with the app.
    view: Option<Entity<Handpic>>,
    /// Taken at the drop, which removes it once the app's threads let go.
    dir: Option<TempDir>,
}

impl Ui {
    /// The main window over a new index and settings store in a temporary
    /// folder, with no folder open.
    pub fn open() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let cx = TestAppContext::single();
        // The view's scan, watchers and writers are threads of their own, so
        // the scheduler may park while they work.
        cx.executor().allow_parking();
        let index = open_index(Some(dir.path().join("index.sqlite")));
        let settings = settings::load(Some(dir.path().join("settings.sqlite")), None);
        let (window, view) = cx
            .update(|cx| open_main(Args::default(), index, settings, None, cx))
            .unwrap();
        let cx = VisualTestContext::from_window(window, &cx);
        cx.run_until_parked();
        Self {
            cx,
            view: Some(view),
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
                panic!("timed out waiting for {what}: {state}");
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
    ui.wait_until("the index eviction", |view| {
        view.eviction.as_ref().is_none_or(JoinHandle::is_finished)
    });
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
