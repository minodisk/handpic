//! The main view on GPUI's test platform: `Ui`, which opens it as `main`
//! does over a temporary index and settings store, and the UI tests.

use std::{
    fs,
    path::Path,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use gpui_kit::{Entity, TestAppContext, VisualTestContext};
use tempfile::TempDir;

use crate::{Handpic, args::Args, open_index, open_main, settings};

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

    pub fn dir(&self) -> &Path {
        self.dir.as_ref().unwrap().path()
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
            assert!(started.elapsed() < WAIT, "timed out waiting for {what}");
            thread::sleep(Duration::from_millis(10));
        }
    }
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
