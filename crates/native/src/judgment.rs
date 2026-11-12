//! One file's judgment as the view keeps it: the stars, the pick / reject
//! flag and the color label; what a judgment key makes of it, a port of the
//! Tauri `judge` and `advance.ts`; and `store`, the row-then-writer order the
//! key's background task follows.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use futures::channel::oneshot;

use riffle_backend::{
    index::{self, Index, IndexedFile},
    sidecar::{SidecarFormat, Writer},
};
use riffle_core::{Flag, xmp::LabelNames};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// `1..=5`; unrated and `0` are `None`.
    pub rating: Option<u8>,
    pub flag: Flag,
    pub label: Option<String>,
}

impl From<&IndexedFile> for State {
    fn from(file: &IndexedFile) -> Self {
        Self {
            rating: file.rating.filter(|r| (1..=5).contains(r)).map(|r| r as u8),
            flag: file.flag,
            label: file.label.clone(),
        }
    }
}

/// A judgment key's command, named by the keymap's action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Rate(u8),
    Reject,
    Pick,
    Unflag,
    ClearRating,
    /// The label as stored, `Red` ... `Purple`.
    Label(&'static str),
    ClearLabel,
    ClearAll,
}

impl Command {
    pub fn from_action(action: &str) -> Option<Self> {
        Some(match action {
            "rate1" => Self::Rate(1),
            "rate2" => Self::Rate(2),
            "rate3" => Self::Rate(3),
            "rate4" => Self::Rate(4),
            "rate5" => Self::Rate(5),
            "reject" => Self::Reject,
            "pick" => Self::Pick,
            "unflag" => Self::Unflag,
            "clear" => Self::ClearRating,
            "red" => Self::Label("Red"),
            "orange" => Self::Label("Orange"),
            "yellow" => Self::Label("Yellow"),
            "green" => Self::Label("Green"),
            "blue" => Self::Label("Blue"),
            "pink" => Self::Label("Pink"),
            "purple" => Self::Label("Purple"),
            "clearlabel" => Self::ClearLabel,
            "clearall" => Self::ClearAll,
            _ => return None,
        })
    }
}

/// `own` after `cmd`, its value decided from `focused`: reject and pick are
/// sticky and keep the stars, clearing the stars keeps the flag, a color
/// toggles against the focused file's label, and `ClearAll` clears all three.
pub fn apply(focused: &State, own: &State, cmd: Command) -> State {
    let mut after = own.clone();
    match cmd {
        Command::Rate(stars) => after.rating = Some(stars),
        Command::Reject => after.flag = Flag::Reject,
        Command::Pick => after.flag = Flag::Pick,
        Command::Unflag => after.flag = Flag::None,
        Command::ClearRating => after.rating = None,
        Command::Label(name) => {
            after.label = (focused.label.as_deref() != Some(name)).then(|| name.to_owned());
        }
        Command::ClearLabel => after.label = None,
        Command::ClearAll => after = State::default(),
    }
    after
}

pub fn changed(own: &State, after: &State) -> bool {
    own != after
}

/// Whether the judgment goes out: it changed the file, or it is `ClearAll`
/// on a file whose label the view has not been told, which clears whatever
/// label the sidecar holds.
pub fn writes(seen: bool, own: &State, after: &State, cmd: Command) -> bool {
    changed(own, after) || (cmd == Command::ClearAll && !seen)
}

/// Whether the writer may take the label as given: the file's entry was
/// seen, the command set the label itself, or it is `ClearAll`. Otherwise
/// the writer keeps the sidecar's own label.
pub fn label_known(seen: bool, own: &State, after: &State, cmd: Command) -> bool {
    seen || cmd == Command::ClearAll || own.label != after.label
}

/// Whether Auto-advance moves on after `cmd`: the stars, reject, pick and
/// unflag; a color toggles and the clears are corrections, so they stay.
pub fn advances_after(cmd: Command) -> bool {
    matches!(
        cmd,
        Command::Rate(_) | Command::Reject | Command::Pick | Command::Unflag
    )
}

/// The `ratings` row, then the writer, as the Tauri `set_rating` orders
/// them, so a crash between the two leaves the row dirty for the next open.
#[allow(clippy::too_many_arguments)]
pub fn store(
    index: &Arc<Mutex<Index>>,
    writer: &Writer,
    dir: &str,
    path: &str,
    state: &State,
    label_known: bool,
    format: SidecarFormat,
    names: LabelNames,
) -> Result<(), String> {
    let rating = state.rating.map(|r| r as i8);
    index::lock(index).set_rating(
        dir,
        path,
        rating,
        state.flag,
        state.label.as_deref(),
        label_known,
    )?;
    writer.set(
        PathBuf::from(path),
        rating,
        state.flag,
        state.label.clone(),
        label_known,
        format,
        names,
    )
}

/// What the store thread runs: one key's `store`, or a barrier that
/// answers once every job before it has run.
enum Job {
    Store(Box<Store>),
    Barrier(mpsc::Sender<()>),
}

/// One key's `store`, with where its outcome goes.
struct Store {
    index: Arc<Mutex<Index>>,
    writer: Arc<Writer>,
    dir: String,
    path: String,
    state: State,
    label_known: bool,
    format: SidecarFormat,
    names: LabelNames,
    reply: oneshot::Sender<Result<(), String>>,
}

/// The stores in the order the keys were pressed: each carries the file's
/// whole state, so the last one to run decides the row and the sidecar, and
/// a pool of tasks would let an earlier key win. One thread runs them.
#[derive(Clone)]
pub struct Queue {
    jobs: mpsc::Sender<Job>,
}

impl Queue {
    pub fn new() -> Self {
        let (jobs, queued) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name("judgment-store".into())
            .spawn(move || {
                for job in queued {
                    let job = match job {
                        Job::Store(job) => job,
                        Job::Barrier(done) => {
                            done.send(()).ok();
                            continue;
                        }
                    };
                    let stored = store(
                        &job.index,
                        &job.writer,
                        &job.dir,
                        &job.path,
                        &job.state,
                        job.label_known,
                        job.format,
                        job.names,
                    );
                    job.reply.send(stored).ok();
                }
            })
            .expect("spawn the judgment store thread");
        Self { jobs }
    }

    /// Queues a `store`; the receiver gets its outcome.
    #[allow(clippy::too_many_arguments)]
    pub fn submit(
        &self,
        index: Arc<Mutex<Index>>,
        writer: Arc<Writer>,
        dir: String,
        path: String,
        state: State,
        label_known: bool,
        format: SidecarFormat,
        names: LabelNames,
    ) -> oneshot::Receiver<Result<(), String>> {
        let (reply, outcome) = oneshot::channel();
        let job = Store {
            index,
            writer,
            dir,
            path,
            state,
            label_known,
            format,
            names,
            reply,
        };
        if let Err(mpsc::SendError(Job::Store(job))) = self.jobs.send(Job::Store(Box::new(job))) {
            job.reply
                .send(Err("the judgment store thread is not running".into()))
                .ok();
        }
        outcome
    }

    /// Waits until every store queued so far has run, at most `timeout`;
    /// the quit hook calls it before the writer's flush, since a store
    /// reaches the writer only after its row. `false` when the wait timed
    /// out or the thread is gone.
    pub fn drain(&self, timeout: Duration) -> bool {
        let (done, reached) = mpsc::channel();
        if self.jobs.send(Job::Barrier(done)).is_err() {
            return false;
        }
        reached.recv_timeout(timeout).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Command, Queue, State, advances_after, apply, changed, label_known, store, writes,
    };
    use riffle_backend::{
        index::{Index, IndexedFile},
        sidecar::{DEBOUNCE, DRAIN_TIMEOUT, SidecarFormat, Writer, label_names_setting},
    };
    use riffle_core::{Flag, xmp::LabelNames};
    use serde_json::json;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        thread,
        time::Instant,
    };

    fn file(rating: Option<i8>, flag: Flag, label: Option<&str>) -> IndexedFile {
        IndexedFile {
            path: "D:/a/DSC1.ARW".into(),
            orientation: 1,
            capture_time: None,
            subsec: None,
            focus: None,
            has_thumb: true,
            rating,
            flag,
            label: label.map(Into::into),
            has_sidecar: true,
            sharpness: None,
            exif: None,
            error: None,
        }
    }

    fn state(rating: Option<u8>, flag: Flag, label: Option<&str>) -> State {
        State {
            rating,
            flag,
            label: label.map(Into::into),
        }
    }

    #[test]
    fn takes_the_stars_flag_and_label() {
        assert_eq!(
            State::from(&file(Some(3), Flag::Pick, Some("red"))),
            state(Some(3), Flag::Pick, Some("red"))
        );
    }

    #[test]
    fn zero_and_out_of_range_stars_are_unrated() {
        for rating in [None, Some(0), Some(-1), Some(6)] {
            assert_eq!(State::from(&file(rating, Flag::None, None)).rating, None);
        }
    }

    #[test]
    fn names_the_keymap_actions() {
        assert_eq!(Command::from_action("rate3"), Some(Command::Rate(3)));
        assert_eq!(Command::from_action("clear"), Some(Command::ClearRating));
        assert_eq!(
            Command::from_action("purple"),
            Some(Command::Label("Purple"))
        );
        assert_eq!(Command::from_action("clearall"), Some(Command::ClearAll));
        for action in ["rejectRest", "next", "zoom", "undo", "rate6"] {
            assert_eq!(Command::from_action(action), None);
        }
    }

    #[test]
    fn a_label_toggle_follows_the_focused_file() {
        let red = state(Some(2), Flag::None, Some("Red"));
        let none = state(Some(5), Flag::Pick, None);
        let cmd = Command::Label("Red");
        assert_eq!(apply(&none, &red, cmd).label.as_deref(), Some("Red"));
        assert_eq!(apply(&red, &none, cmd).label, None);
        assert_eq!(apply(&red, &red, cmd).label, None);
        let blue = state(None, Flag::None, Some("Blue"));
        assert_eq!(apply(&blue, &blue, cmd).label.as_deref(), Some("Red"));
    }

    #[test]
    fn a_star_command_keeps_the_label_and_flag() {
        let own = state(Some(1), Flag::Reject, Some("Blue"));
        assert_eq!(
            apply(&own, &own, Command::Rate(3)),
            state(Some(3), Flag::Reject, Some("Blue"))
        );
    }

    #[test]
    fn reject_and_pick_are_sticky_and_keep_the_stars() {
        let picked = state(Some(4), Flag::Pick, None);
        assert_eq!(
            apply(&picked, &picked, Command::Reject),
            state(Some(4), Flag::Reject, None)
        );
        assert_eq!(apply(&picked, &picked, Command::Pick), picked);
        assert_eq!(
            apply(&picked, &picked, Command::Unflag),
            state(Some(4), Flag::None, None)
        );
    }

    #[test]
    fn clearing_the_stars_keeps_the_flag_and_clear_all_clears_all() {
        let own = state(Some(4), Flag::Pick, Some("Red"));
        assert_eq!(
            apply(&own, &own, Command::ClearRating),
            state(None, Flag::Pick, Some("Red"))
        );
        assert_eq!(
            apply(&own, &own, Command::ClearLabel),
            state(Some(4), Flag::Pick, None)
        );
        assert_eq!(apply(&own, &own, Command::ClearAll), State::default());
    }

    #[test]
    fn a_file_already_at_the_value_is_skipped_unless_clear_all_on_an_unseen_file() {
        let two = state(Some(2), Flag::None, None);
        let after = apply(&two, &two, Command::Rate(2));
        assert!(!changed(&two, &after));
        assert!(!writes(false, &two, &after, Command::Rate(2)));
        assert!(changed(&two, &apply(&two, &two, Command::Rate(3))));
        let blank = State::default();
        let after = apply(&blank, &blank, Command::ClearAll);
        assert!(!writes(true, &blank, &after, Command::ClearAll));
        assert!(writes(false, &blank, &after, Command::ClearAll));
    }

    #[test]
    fn the_label_is_known_once_seen_or_set_by_the_command_or_cleared_by_clear_all() {
        let own = State::default();
        let rated = apply(&own, &own, Command::Rate(3));
        assert!(label_known(true, &own, &rated, Command::Rate(3)));
        assert!(!label_known(false, &own, &rated, Command::Rate(3)));
        let red = apply(&own, &own, Command::Label("Red"));
        assert!(label_known(false, &own, &red, Command::Label("Red")));
        let cleared = apply(&own, &own, Command::ClearAll);
        assert!(label_known(false, &own, &cleared, Command::ClearAll));
        let unlabeled = apply(&own, &own, Command::ClearLabel);
        assert!(!label_known(false, &own, &unlabeled, Command::ClearLabel));
    }

    #[test]
    fn advances_after_stars_reject_pick_and_unflag_only() {
        for cmd in [
            Command::Rate(1),
            Command::Rate(5),
            Command::Reject,
            Command::Pick,
            Command::Unflag,
        ] {
            assert!(advances_after(cmd), "{cmd:?}");
        }
        for cmd in [
            Command::Label("Red"),
            Command::ClearLabel,
            Command::ClearRating,
            Command::ClearAll,
        ] {
            assert!(!advances_after(cmd), "{cmd:?}");
        }
    }

    /// A folder with one empty `DSC1.ARW`, a fresh index and a writer, as the
    /// key's background task finds them.
    struct Folder {
        _temp: tempfile::TempDir,
        dir: String,
        arw: String,
        index: Arc<Mutex<Index>>,
        writer: Writer,
    }

    impl Folder {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let folder = temp.path().join("photos");
            fs::create_dir(&folder).unwrap();
            let arw = folder.join("DSC1.ARW");
            fs::write(&arw, b"").unwrap();
            let index = Arc::new(Mutex::new(
                Index::open(&temp.path().join("index.sqlite")).unwrap(),
            ));
            let writer = Writer::spawn(index.clone(), |path, message| {
                panic!("sidecar write failed: {} {message}", path.display())
            });
            Self {
                dir: folder.to_string_lossy().into_owned(),
                arw: arw.to_string_lossy().into_owned(),
                _temp: temp,
                index,
                writer,
            }
        }

        fn judge(&self, own: &State, cmd: Command, format: SidecarFormat, names: &LabelNames) {
            let after = apply(own, own, cmd);
            let known = label_known(true, own, &after, cmd);
            store(
                &self.index,
                &self.writer,
                &self.dir,
                &self.arw,
                &after,
                known,
                format,
                names.clone(),
            )
            .unwrap();
        }

        fn sidecar(&self, name: &str) -> String {
            fs::read_to_string(Path::new(&self.dir).join(name)).unwrap()
        }
    }

    /// Each key on the file the previous one left, as the view applies them.
    fn press(folder: &Folder, cmds: &[Command], format: SidecarFormat, names: &LabelNames) {
        let mut own = State::default();
        for &cmd in cmds {
            folder.judge(&own, cmd, format, names);
            own = apply(&own, &own, cmd);
        }
    }

    #[test]
    fn stars_pick_and_red_write_an_xmp_and_their_clears_empty_it() {
        let folder = Folder::new();
        let names = label_names_setting(None);
        let set = [Command::Rate(3), Command::Pick, Command::Label("Red")];
        press(&folder, &set, SidecarFormat::Xmp, &names);
        assert!(folder.writer.flush(DRAIN_TIMEOUT));
        let xmp = folder.sidecar("DSC1.xmp");
        for wanted in [
            r#"xmp:Rating="3""#,
            r#"xmpDM:good="True""#,
            r#"photoshop:LabelColor="red""#,
            r#"xmp:Label="Red""#,
        ] {
            assert!(xmp.contains(wanted), "{wanted} in {xmp}");
        }
        let mut own = State::default();
        for &cmd in &set {
            own = apply(&own, &own, cmd);
        }
        for cmd in [Command::ClearRating, Command::Unflag, Command::ClearLabel] {
            folder.judge(&own, cmd, SidecarFormat::Xmp, &names);
            own = apply(&own, &own, cmd);
        }
        assert!(folder.writer.flush(DRAIN_TIMEOUT));
        let xmp = folder.sidecar("DSC1.xmp");
        assert!(xmp.contains(r#"xmp:Rating="0""#), "{xmp}");
        for gone in ["xmpDM:good", "LabelColor", "xmp:Label"] {
            assert!(!xmp.contains(gone), "{gone} in {xmp}");
        }
    }

    #[test]
    fn both_writes_an_xmp_and_a_dop() {
        let folder = Folder::new();
        let names = label_names_setting(None);
        let set = [Command::Rate(3), Command::Pick, Command::Label("Red")];
        press(&folder, &set, SidecarFormat::Both, &names);
        assert!(folder.writer.flush(DRAIN_TIMEOUT));
        let xmp = folder.sidecar("DSC1.xmp");
        assert!(xmp.contains(r#"xmp:Rating="3""#), "{xmp}");
        let dop = folder.sidecar("DSC1.ARW.dop");
        for wanted in [
            "Rating = 3",
            // `0` is PhotoLab's pick.
            "ShouldProcess = 0,",
            r#"ColorLabel = "Red""#,
        ] {
            assert!(dop.contains(wanted), "{wanted} in {dop}");
        }
    }

    #[test]
    fn a_label_is_written_under_its_configured_name() {
        let folder = Folder::new();
        let names = label_names_setting(Some(&json!({ "red": "Rot" })));
        press(
            &folder,
            &[Command::Label("Red")],
            SidecarFormat::Xmp,
            &names,
        );
        assert!(folder.writer.flush(DRAIN_TIMEOUT));
        let xmp = folder.sidecar("DSC1.xmp");
        assert!(xmp.contains(r#"xmp:Label="Rot""#), "{xmp}");
    }

    #[test]
    fn keys_inside_the_debounce_are_written_once_with_the_last_value() {
        let folder = Folder::new();
        let names = label_names_setting(None);
        let xmp = PathBuf::from(&folder.dir).join("DSC1.xmp");
        let mut own = State::default();
        let started = Instant::now();
        for stars in 1..=3 {
            folder.judge(&own, Command::Rate(stars), SidecarFormat::Xmp, &names);
            own = apply(&own, &own, Command::Rate(stars));
        }
        // Only meaningful while the first key's debounce has not run out.
        if started.elapsed() < DEBOUNCE {
            assert!(!xmp.exists());
        }
        thread::sleep(DEBOUNCE * 3);
        let written = fs::read_to_string(&xmp).unwrap();
        assert!(written.contains(r#"xmp:Rating="3""#), "{written}");
        let row = folder
            .index
            .lock()
            .unwrap()
            .dirty_rows(&folder.dir)
            .unwrap();
        assert!(row.is_empty(), "{row:?}");
    }

    #[test]
    fn the_quit_drain_writes_a_key_inside_the_debounce() {
        let folder = Folder::new();
        let names = label_names_setting(None);
        press(&folder, &[Command::Rate(5)], SidecarFormat::Xmp, &names);
        assert!(folder.writer.flush(DRAIN_TIMEOUT));
        assert!(folder.sidecar("DSC1.xmp").contains(r#"xmp:Rating="5""#));
    }

    #[test]
    fn stores_run_in_key_order_while_the_index_is_held() {
        let folder = Folder::new();
        let names = label_names_setting(None);
        let queue = Queue::new();
        let writer = Arc::new(Writer::spawn(folder.index.clone(), |path, message| {
            panic!("sidecar write failed: {} {message}", path.display())
        }));
        let own = State::default();
        let rated = apply(&own, &own, Command::Rate(3));
        let picked = apply(&rated, &rated, Command::Pick);
        let held = folder.index.lock().unwrap();
        let outcomes: Vec<_> = [rated, picked.clone()]
            .into_iter()
            .map(|state| {
                queue.submit(
                    folder.index.clone(),
                    writer.clone(),
                    folder.dir.clone(),
                    folder.arw.clone(),
                    state,
                    true,
                    SidecarFormat::Xmp,
                    names.clone(),
                )
            })
            .collect();
        drop(held);
        for outcome in outcomes {
            assert_eq!(futures::executor::block_on(outcome), Ok(Ok(())));
        }
        assert!(writer.flush(DRAIN_TIMEOUT));
        let xmp = folder.sidecar("DSC1.xmp");
        assert!(xmp.contains(r#"xmp:Rating="3""#), "{xmp}");
        assert!(xmp.contains(r#"xmpDM:good="True""#), "{xmp}");
        let rows = folder.index.lock().unwrap().rows_of(&folder.dir).unwrap();
        let row = rows.iter().find(|r| r.0 == folder.arw).unwrap();
        assert_eq!(
            (row.1, row.2),
            (picked.rating.map(|r| r as i8), picked.flag)
        );
    }

    #[test]
    fn the_quit_drain_waits_for_a_store_held_on_the_index() {
        let folder = Folder::new();
        let names = label_names_setting(None);
        let queue = Queue::new();
        let writer = Arc::new(Writer::spawn(folder.index.clone(), |path, message| {
            panic!("sidecar write failed: {} {message}", path.display())
        }));
        let own = State::default();
        let picked = apply(&own, &own, Command::Pick);
        let held = folder.index.lock().unwrap();
        let _outcome = queue.submit(
            folder.index.clone(),
            writer.clone(),
            folder.dir.clone(),
            folder.arw.clone(),
            picked.clone(),
            true,
            SidecarFormat::Xmp,
            names,
        );
        assert!(!queue.drain(std::time::Duration::from_millis(100)));
        drop(held);
        assert!(queue.drain(DRAIN_TIMEOUT));
        assert!(writer.flush(DRAIN_TIMEOUT));
        let xmp = folder.sidecar("DSC1.xmp");
        assert!(xmp.contains(r#"xmpDM:good="True""#), "{xmp}");
        let rows = folder.index.lock().unwrap().rows_of(&folder.dir).unwrap();
        let row = rows.iter().find(|r| r.0 == folder.arw).unwrap();
        assert_eq!((row.1, row.2), (None, picked.flag));
    }
}
