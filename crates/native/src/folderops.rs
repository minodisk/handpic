//! The folder operations of the tree's right-click menu: which flow is under
//! way, the one-flow-at-a-time rule, the shared confirmation dialog
//! (`dialog.rs`) and its drawing, and the dispatch of a folder menu action.
//! So far `Rewrite Sidecars from Index…` and `Delete Sidecars…`
//! (`sidecars.rs`), `Sequence JPEG Timestamps…` (`sequence.rs`), `Import…`
//! (`import.rs`), `Rename by Capture Date…` (`datefolder.rs`) and the tree's
//! inline `Rename…` (`rename.rs`), with what follows a folder rename
//! (`folder_renamed`); the
//! window's view keeps one `FolderOps`, and this module adds to its `impl`,
//! as a child module reaches its private fields.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use futures::channel::oneshot;
use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, FocusHandle, Focusable, FontWeight,
    HighlightStyle, KeyDownEvent, MouseButton, ScrollStrategy, SharedString, StyledText,
    Subscription, Task, WeakEntity, Window, div, prelude::*, px, rgb, transparent_black,
};
use gpui_component::{
    Disableable,
    button::{Button as UiButton, ButtonVariants},
    dialog::{self as component, DialogFooter},
    input::{Input, InputEvent, InputState},
    tooltip::Tooltip,
};
use riffle_backend::{
    datefolder::{self as datefolders, Kind as DateKind, Split},
    foldersidecars::{self, DeletePreview, Preview, Summary},
    import::{self as importer, ImportSettings, Plan, Ran as Imported},
    listing::canonicalize,
    rename::{self as renamer, Context as Moves, Renamed},
    sequence::{self as sequencer, Ran as Sequenced},
    sidecar,
    text::{SCAN_RUNNING, SIZE_BASE, format_bytes},
    trash::Summary as TrashSummary,
};

use crate::datefolder::{self, DateFolderFlow};
use crate::dialog::{self, Button, Content, Dialog, Row, Style};
use crate::import::{self, ImportFlow};
use crate::judgment::State;
use crate::rename::{
    self, Decision, Held, InlineRename, Outcome, PENDING_TITLE, Pending, PendingRename,
    RENAME_REFUSED, RENAMING, RenamesInFlight, SLOW_CLICK_DELAY, SlowClick, commit, display_name,
    edit_outcome,
};
use crate::scan::Event;
use crate::sequence::{self, SequenceFlow};
use crate::sidecars::{self, Kind, SidecarFlow, Target};
use crate::tree::{self, IGNORE_CASE};
use crate::{
    ERROR_COLOR, HeldOp, MUTED_COLOR, PICK_COLOR, PRIMARY_COLOR, Riffle, TREE_ROW_HEIGHT, context,
    keys, trash,
};

/// The folder operations' view state: the flows, the one dialog, laid out
/// only while open, and the sequencing run under way.
#[derive(Default)]
pub struct FolderOps {
    sidecars: SidecarFlow,
    sequence: SequenceFlow,
    dialog: Option<Dialog>,
    /// The sequencing run's cancel flag, which the app's quit sets too
    /// (`sequence_quit`), and how far the run got, in the pane's notes.
    sequence_cancel: Option<Arc<AtomicBool>>,
    sequence_quit: Option<Subscription>,
    sequencing: Option<String>,
    import: ImportFlow,
    /// The plan the import dialog shows, which its run copies.
    import_plan: Option<ImportPlan>,
    import_previews: u64,
    /// The import dialog while its run goes on with the dialog aside.
    aside: Option<Dialog>,
    import_cancel: Option<Arc<AtomicBool>>,
    import_quit: Option<Subscription>,
    /// The import's reading or progress note, then its result, which stays
    /// until the next open of a folder or import.
    importing: Option<String>,
    imported: Option<String>,
    date: DateFolderFlow,
    /// The tree's live inline rename, the rename the gate holds as its row
    /// shows it, the renames in flight and the slow second click.
    editing: Option<Edit>,
    pending: PendingRename,
    renames: RenamesInFlight,
    slow: SlowClick,
}

/// A live inline rename and the text input drawn over its row's name.
struct Edit {
    rename: InlineRename,
    input: Entity<InputState>,
    _events: Subscription,
}

impl FolderOps {
    /// A folder flow is under way, from its preview to its run's end.
    pub fn busy(&self) -> bool {
        self.sidecars.busy() || self.sequence.busy() || self.import_busy() || self.date.busy()
    }

    /// An import is under way, its dialog shown or not; a folder rename is
    /// refused meanwhile.
    pub fn import_busy(&self) -> bool {
        self.import.busy()
    }

    /// A folder was opened afresh: the last import's result goes.
    pub fn clear_imported(&mut self) {
        self.imported = None;
    }

    pub fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    /// The pane's note of a sequencing run under way.
    pub fn note(&self) -> Option<String> {
        self.sequencing.clone()
    }

    /// The pane's import line: the run's note, else its result. It is drawn
    /// as its own line, so it hides no other note.
    pub fn import_note(&self) -> Option<String> {
        self.importing.clone().or_else(|| self.imported.clone())
    }
}

/// A message of a folder operation's run, sent from its thread into the
/// view's channel.
#[derive(Clone, Copy, Debug)]
pub enum Message {
    /// How far the sequencing run `run` got.
    SequenceProgress { run: u64, done: usize, total: usize },
    /// How many files the import run `run` has copied.
    ImportProgress { run: u64, done: usize, total: usize },
}

/// A previewed import: its id, the source and the destination root the run's
/// log line names, and the plan.
struct ImportPlan {
    id: u64,
    source: String,
    root: String,
    plan: Plan,
}

/// What a sidecar preview read.
enum Previewed {
    Rewrite(Preview),
    Delete(DeletePreview),
}

/// What a date run did: a rename, with the folder's canonical path before
/// and after it, or a split of the canonical folder.
enum DateRan {
    Renamed(Renamed, (String, String)),
    Split(Split, String),
}

/// What a sidecar run did.
enum Ran {
    Rewrote(Result<Summary, String>),
    Deleted(Result<TrashSummary, String>),
}

/// A folder as the dialog and the status name it: its last component, else
/// the whole path (a root).
fn name_of(dir: &str) -> String {
    Path::new(dir).file_name().map_or_else(
        || dir.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

impl Riffle {
    /// An item of the folder tree's menu that `folder_action` does not handle
    /// itself.
    pub(crate) fn folder_op(
        &mut self,
        action: context::Action,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let kind = match action {
            context::Action::RewriteSidecars => Kind::Rewrite,
            context::Action::DeleteSidecars => Kind::Delete,
            context::Action::SequenceTimestamps => return self.start_sequence(path, cx),
            context::Action::ImportFolder => return self.start_import(path, cx),
            context::Action::RenameByDate => return self.start_date_folder(path, cx),
            _ => return,
        };
        self.request_sidecars(Target { kind, dir: path }, cx);
    }

    /// Through the gate: the preview is read at once with no listing or scan
    /// under way, else the request waits for the scan's end without
    /// canceling it.
    fn request_sidecars(&mut self, target: Target, cx: &mut Context<Self>) {
        let busy = self.resync.busy() || self.gate.in_flight();
        let label = target.kind.label();
        match self.gate_request(label, HeldOp::Sidecars(target), busy) {
            Some(HeldOp::Sidecars(target)) => self.start_sidecars(target, cx),
            Some(_) => {}
            None => log::info!("sidecars: {label} held for the scan's end"),
        }
        cx.notify();
    }

    /// Read the preview of `target` on a background task, the judgment
    /// stores still queued and the sidecar writer flushed first. Nothing
    /// starts while the settings, the trash dialog or another folder flow
    /// is open or busy.
    pub(crate) fn start_sidecars(&mut self, target: Target, cx: &mut Context<Self>) {
        if self.settings_view.read(cx).is_open()
            || self.trash.busy()
            || self.folderops.sequence.busy()
            || self.folderops.import.busy()
            || self.folderops.date.busy()
            || !self.folderops.sidecars.start(target.clone())
        {
            return;
        }
        let (stores, writer, format) = (
            self.stores.clone(),
            self.writer.clone(),
            self.settings.format,
        );
        let index = self.reader.clone().or_else(|| self.index.clone().ok());
        let Target { kind, dir } = target;
        cx.spawn(async move |this, cx| {
            let name = name_of(&dir);
            let previewed = cx
                .background_spawn(async move {
                    stores.drain(sidecar::DRAIN_TIMEOUT);
                    if let Some(writer) = &writer {
                        writer.flush(sidecar::DRAIN_TIMEOUT);
                    }
                    let dir = canonicalize(&dir);
                    match kind {
                        Kind::Rewrite => {
                            let index = index.ok_or("no index cache available")?;
                            foldersidecars::preview(&dir, &index, format).map(Previewed::Rewrite)
                        }
                        Kind::Delete => foldersidecars::doomed(&dir, format).map(|doomed| {
                            Previewed::Delete(foldersidecars::delete_preview(
                                &doomed,
                                format,
                                |bytes| format_bytes(bytes, SIZE_BASE),
                            ))
                        }),
                    }
                })
                .await;
            this.update(cx, |this, cx| this.sidecars_previewed(&name, previewed, cx))
                .ok();
        })
        .detach();
    }

    /// The preview came back: its dialog opens, or a refusal goes to the
    /// status and ends the flow.
    fn sidecars_previewed(
        &mut self,
        name: &str,
        previewed: Result<Previewed, String>,
        cx: &mut Context<Self>,
    ) {
        match previewed {
            Ok(previewed) if self.folderops.sidecars.previewed() => {
                let (content, run, style) = match previewed {
                    Previewed::Rewrite(preview) => (
                        Content {
                            title: "Rewrite Sidecars from Index".to_owned(),
                            rows: rows(sidecars::rewrite_rows(&preview)),
                            notes: vec![sidecars::rewrite_total_line(&preview, name)],
                            running_note: sidecars::REWRITE_RUNNING_NOTE.to_owned(),
                            ..Content::default()
                        },
                        "Rewrite",
                        Style::Primary,
                    ),
                    Previewed::Delete(preview) => (
                        Content {
                            title: "Delete Sidecars".to_owned(),
                            rows: rows(sidecars::delete_rows(&preview)),
                            notes: vec![sidecars::delete_total_line(&preview, name)],
                            running_note: sidecars::DELETE_RUNNING_NOTE.to_owned(),
                            ..Content::default()
                        },
                        "Move to Trash",
                        Style::Destructive,
                    ),
                };
                self.menus.close();
                self.context.close();
                self.folderops.dialog = Some(Dialog::new(content, run, style, true));
                self.follow_folder_dialog(cx);
            }
            Ok(_) => {}
            Err(e) => {
                self.folderops.sidecars.end();
                self.step_note = Some(e);
            }
        }
        cx.notify();
    }

    /// The run button: refused with the scan message if a listing or scan
    /// started while the dialog was open; else run on a background task, in
    /// flight so no listing or scan starts under it, the buttons disabled.
    fn run_sidecars(&mut self, cx: &mut Context<Self>) {
        let Some(Target { kind, dir }) = self.folderops.sidecars.run() else {
            return;
        };
        if self.resync.busy() || self.gate.in_flight() {
            self.folderops.sidecars.end();
            self.folderops.dialog = None;
            self.follow_folder_dialog(cx);
            self.step_note = Some(SCAN_RUNNING.to_owned());
            cx.notify();
            return;
        }
        if let Some(dialog) = &mut self.folderops.dialog {
            dialog.set_running();
        }
        self.gate.enter_in_flight();
        if self.dir.is_some() {
            self.resync.hold();
        }
        let (stores, writer, index, runs) = (
            self.stores.clone(),
            self.writer.clone(),
            self.index.clone().ok(),
            self.runs.clone(),
        );
        let (format, names) = (self.settings.format, self.settings.names.clone());
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let name = name_of(&dir);
            let (dir, ran) = cx
                .background_spawn(async move {
                    stores.drain(sidecar::DRAIN_TIMEOUT);
                    if let Some(writer) = &writer {
                        writer.flush(sidecar::DRAIN_TIMEOUT);
                    }
                    let dir = canonicalize(&dir);
                    let ran = match kind {
                        Kind::Rewrite => Ran::Rewrote(match (writer, index) {
                            (None, _) => Err("the sidecar writer is not running".to_owned()),
                            (_, None) => Err("no index cache available".to_owned()),
                            (Some(writer), Some(index)) => foldersidecars::rewrite(
                                &dir,
                                &index,
                                &writer,
                                format,
                                &names,
                                foldersidecars::REWRITE_DRAIN,
                            ),
                        }),
                        Kind::Delete => Ran::Deleted(foldersidecars::delete_run(
                            &dir,
                            format,
                            index.as_deref(),
                            &runs,
                        )),
                    };
                    (dir, ran)
                })
                .await;
            log::info!(
                "sidecars: {} of {dir} in {}ms",
                kind.label(),
                started.elapsed().as_millis()
            );
            this.update(cx, |this, cx| this.sidecars_ran(&name, dir, ran, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// A run came back and the dialog closes. A delete ends as a move to the
    /// Trash of sidecars does (`trashed`); a rewrite says what it wrote and
    /// leaves the strip alone, the index being what the strip shows.
    fn sidecars_ran(&mut self, name: &str, dir: String, ran: Ran, cx: &mut Context<Self>) {
        self.folderops.sidecars.end();
        self.folderops.dialog = None;
        self.follow_folder_dialog(cx);
        let result = match ran {
            Ran::Deleted(result) => {
                return self.trashed(
                    vec![dir],
                    trash::Mode::Trash,
                    trash::What::Sidecars,
                    false,
                    false,
                    result,
                    cx,
                );
            }
            Ran::Rewrote(result) => result,
        };
        self.gate.leave_in_flight();
        self.step_note = Some(match result {
            Ok(summary) => {
                log::info!("sidecars: rewrote {dir}: {summary:?}");
                sidecars::rewritten_status(&summary, name)
            }
            Err(e) => {
                log::error!("sidecars: rewrite of {dir} failed: {e}");
                e
            }
        });
        if let Some(folder) = self.deferred_open.take() {
            self.open(folder, None, None, cx).detach();
        } else if self.resync.finished() {
            self.relist(cx);
        }
        cx.notify();
    }

    /// A run of sidecars to the Trash, or its redo, came back: when it moved
    /// something out of the open folder, the strip's judgments go, as
    /// `clearShownJudgments` drops them, and the re-listing reads the
    /// cleared rows back.
    pub(crate) fn sidecars_trashed(&mut self, dirs: &[String], summary: &TrashSummary) {
        let opens = self
            .dir
            .as_deref()
            .is_some_and(|dir| trash::opens_target(dir, dirs, IGNORE_CASE));
        if !opens || summary.moved.is_empty() {
            return;
        }
        for path in &self.all {
            self.touched.remove(path);
            self.judgments.insert(path.clone(), State::default());
        }
        self.mark_strip();
    }

    /// `Sequence JPEG Timestamps…`: read the preview of `dir` on a background
    /// task. It neither waits for a scan nor touches it or the index, and
    /// needs no open folder. Nothing starts while the settings, the trash
    /// dialog or another folder flow is open or busy.
    fn start_sequence(&mut self, dir: String, cx: &mut Context<Self>) {
        if self.settings_view.read(cx).is_open()
            || self.trash.busy()
            || self.folderops.sidecars.busy()
            || self.folderops.import.busy()
            || self.folderops.date.busy()
            || !self.folderops.sequence.start(dir.clone())
        {
            return;
        }
        cx.spawn(async move |this, cx| {
            let read = dir.clone();
            let previewed = cx
                .background_spawn(async move { sequencer::preview(Path::new(&read)) })
                .await;
            this.update(cx, |this, cx| this.sequence_previewed(dir, previewed, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// The preview came back: its dialog opens, or the failure goes to the
    /// pane's notes keyed by the folder and the flow ends.
    fn sequence_previewed(
        &mut self,
        dir: String,
        previewed: Result<sequencer::Preview, String>,
        cx: &mut Context<Self>,
    ) {
        match previewed {
            Ok(preview) if self.folderops.sequence.previewed() => {
                let mut lines = vec![
                    format!("Source: {dir}"),
                    format!("Output: {}", preview.output_dir),
                ];
                lines.extend(sequence::rebuild_notice(&preview).map(str::to_owned));
                let content = Content {
                    title: "Sequence JPEG Timestamps".to_owned(),
                    lines,
                    rows: preview
                        .rows
                        .iter()
                        .map(|row| {
                            let parts = sequence::row_parts(row);
                            Row {
                                text: parts.head + &parts.same,
                                accent: parts.diff,
                                muted: row.old == row.new,
                            }
                        })
                        .collect(),
                    notes: vec![
                        sequence::written_line(&preview.rows, &preview.output_dir),
                        sequence::capture_time_line(&preview.rows),
                    ],
                    errors: preview.failed.iter().map(sequence::failure_text).collect(),
                    running_note: sequence::RUNNING_NOTE.to_owned(),
                    ..Content::default()
                };
                self.menus.close();
                self.context.close();
                self.folderops.dialog = Some(Dialog::new(
                    content,
                    "Run",
                    Style::Primary,
                    !preview.rows.is_empty(),
                ));
                self.follow_folder_dialog(cx);
            }
            Ok(_) => {}
            Err(e) => {
                self.folderops.sequence.fail();
                self.write_failed(dir, e);
            }
        }
        cx.notify();
    }

    /// The run button: the run goes on a thread of its own with a cancel
    /// flag, its progress through the view's channel tagged with its id, the
    /// dialog's Cancel left enabled to cancel it. The app's quit only sets
    /// the flag.
    fn run_sequence(&mut self, cx: &mut Context<Self>) {
        let Some((dir, run)) = self.folderops.sequence.run() else {
            return;
        };
        if let Some(dialog) = &mut self.folderops.dialog {
            dialog.set_running_cancelable();
        }
        self.folderops.sequencing = Some(sequence::progress_status(0, 0));
        let cancel = Arc::new(AtomicBool::new(false));
        self.folderops.sequence_cancel = Some(cancel.clone());
        self.folderops.sequence_quit = Some({
            let cancel = cancel.clone();
            cx.on_app_quit(move |_, _| {
                cancel.store(true, Ordering::Relaxed);
                async {}
            })
        });
        let events = self.events.clone();
        let (reply, ran) = oneshot::channel();
        let started = Instant::now();
        let path = dir.clone();
        std::thread::spawn(move || {
            let ran = sequencer::run(Path::new(&path), &cancel, |done, total| {
                let _ = events.unbounded_send(Event::FolderOp(Message::SequenceProgress {
                    run,
                    done,
                    total,
                }));
            });
            let _ = reply.send(ran);
        });
        cx.spawn(async move |this, cx| {
            let ran = ran.await.unwrap_or_else(|_| Sequenced {
                output_dir: String::new(),
                written: 0,
                total: 0,
                failed: vec![sequencer::Failure {
                    path: dir.clone(),
                    message: "the run stopped unexpectedly".to_owned(),
                }],
                canceled: false,
            });
            log::info!(
                "sequence: {dir} written={} total={} failed={} canceled={} in {}ms",
                ran.written,
                ran.total,
                ran.failed.len(),
                ran.canceled,
                started.elapsed().as_millis()
            );
            this.update(cx, |this, cx| this.sequence_ran(run, ran, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// The run `run` ended: the dialog closes, the failures go to the notes,
    /// the status says what it wrote, and the output folder is revealed
    /// unless the run was canceled or wrote nothing.
    fn sequence_ran(&mut self, run: u64, ran: Sequenced, cx: &mut Context<Self>) {
        if !self.folderops.sequence.done(run) {
            return;
        }
        self.folderops.dialog = None;
        self.follow_folder_dialog(cx);
        self.folderops.sequencing = None;
        self.folderops.sequence_cancel = None;
        self.folderops.sequence_quit = None;
        for failure in &ran.failed {
            self.write_failed(failure.path.clone(), failure.message.clone());
        }
        self.step_note = sequence::done_status(&ran);
        if let Some(out) = sequence::reveal_after(&ran) {
            cx.reveal_path(Path::new(out));
        }
        cx.notify();
    }

    /// `Import…`: while a run is under way with its dialog aside, show the
    /// dialog again; else plan the import of every photo under `dir` into
    /// the import settings' root and pattern on a background task, the
    /// pane saying it reads meanwhile. It neither waits for a scan nor
    /// touches the index, and needs no open folder. Nothing starts while the
    /// settings, the trash dialog or another folder flow is open or busy.
    fn start_import(&mut self, dir: String, cx: &mut Context<Self>) {
        if self.settings_view.read(cx).is_open()
            || self.trash.busy()
            || self.folderops.sidecars.busy()
            || self.folderops.sequence.busy()
            || self.folderops.date.busy()
        {
            return;
        }
        if self.folderops.import.reopen() {
            self.menus.close();
            self.context.close();
            self.folderops.dialog = self.folderops.aside.take();
            self.follow_folder_dialog(cx);
            cx.notify();
            return;
        }
        if !self.folderops.import.start() {
            return;
        }
        self.folderops.imported = None;
        self.folderops.importing = Some(format!("reading {dir}…"));
        self.folderops.import_previews += 1;
        let id = self.folderops.import_previews;
        let settings = ImportSettings {
            root: self.settings.import_root.clone(),
            pattern: self.settings.import_pattern.clone(),
        };
        cx.spawn(async move |this, cx| {
            let read = dir.clone();
            let previewed = cx
                .background_spawn(async move {
                    let source = canonicalize(&read);
                    if let Some(reason) = importer::refusal(&settings) {
                        let empty = Plan {
                            destinations: Vec::new(),
                            undated: Vec::new(),
                        };
                        let mut preview = importer::summarize(None, source, &empty);
                        preview.refusal = Some(reason);
                        return Ok((preview, None));
                    }
                    let plan = importer::plan_dir(Path::new(&source), &settings)?;
                    let preview = importer::summarize(Some(id), source.clone(), &plan);
                    Ok((
                        preview,
                        Some(ImportPlan {
                            id,
                            source,
                            root: settings.root,
                            plan,
                        }),
                    ))
                })
                .await;
            this.update(cx, |this, cx| this.import_previewed(dir, previewed, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// The plan came back: its dialog opens, the plan kept for the run, or
    /// the failure goes to the pane's notes keyed by the folder and the flow
    /// ends.
    fn import_previewed(
        &mut self,
        dir: String,
        previewed: Result<(importer::Preview, Option<ImportPlan>), String>,
        cx: &mut Context<Self>,
    ) {
        self.folderops.importing = None;
        match previewed {
            Ok((preview, plan)) if self.folderops.import.previewed(preview.id) => {
                let mut lines = vec![format!("Source: {}", preview.source)];
                if preview.refusal.is_none() {
                    lines.push(import::total_line(&preview));
                }
                let content = Content {
                    title: "Import".to_owned(),
                    lines,
                    rows: rows(import::folder_rows(&preview)),
                    lists: vec![
                        (
                            import::COLLISIONS_HEADING.to_owned(),
                            import::collision_items(&preview),
                        ),
                        (import::UNDATED_HEADING.to_owned(), preview.undated.clone()),
                    ],
                    notes: import::import_note(&preview).into_iter().collect(),
                    running_note: import::RUNNING_NOTE.to_owned(),
                    ..Content::default()
                };
                self.menus.close();
                self.context.close();
                self.folderops.import_plan = plan;
                self.folderops.dialog = Some(Dialog::new(
                    content,
                    "Run",
                    Style::Primary,
                    import::can_run(&preview),
                ));
                self.follow_folder_dialog(cx);
            }
            Ok(_) => {}
            Err(e) => {
                self.folderops.import.fail();
                self.write_failed(dir, e);
            }
        }
        cx.notify();
    }

    /// The run button: a thread of its own creates the destination folders,
    /// then copies the plan the dialog showed with a cancel flag, its
    /// progress through the view's channel tagged with its id. Once the
    /// folders exist the first opens as a click in the tree opens it and the
    /// dialog steps aside; the run's report waits for that open, which
    /// clears the notes. The app's quit only sets the flag.
    fn run_import(&mut self, cx: &mut Context<Self>) {
        let Some((preview, run)) = self.folderops.import.run() else {
            return;
        };
        let Some(ImportPlan {
            source, root, plan, ..
        }) = self
            .folderops
            .import_plan
            .take()
            .filter(|plan| plan.id == preview)
        else {
            self.folderops.import.fail();
            self.folderops.dialog = None;
            self.follow_folder_dialog(cx);
            return;
        };
        if let Some(dialog) = &mut self.folderops.dialog {
            dialog.set_running_cancelable();
        }
        self.folderops.importing = Some(import::progress_status(0, 0));
        let cancel = Arc::new(AtomicBool::new(false));
        self.folderops.import_cancel = Some(cancel.clone());
        self.folderops.import_quit = Some({
            let cancel = cancel.clone();
            cx.on_app_quit(move |_, _| {
                cancel.store(true, Ordering::Relaxed);
                async {}
            })
        });
        let events = self.events.clone();
        let (folders_sent, folders) = oneshot::channel();
        let (reply, ran) = oneshot::channel();
        let from = source.clone();
        std::thread::spawn(move || {
            let prepared = importer::prepare(&plan);
            let _ = folders_sent.send(prepared.folders.clone());
            let mut last = None;
            let ran = importer::run_plan(prepared, &cancel, |p, _| {
                if last != Some(p.files_done) {
                    last = Some(p.files_done);
                    let _ = events.unbounded_send(Event::FolderOp(Message::ImportProgress {
                        run,
                        done: p.files_done,
                        total: p.files_total,
                    }));
                }
            });
            log::info!("{}", importer::log_line(&from, &root, &ran));
            let _ = reply.send(ran);
        });
        cx.spawn(async move |this, cx| {
            let opening = match folders.await {
                Ok(folders) => this
                    .update(cx, |this, cx| this.import_started(run, &folders, cx))
                    .ok()
                    .flatten(),
                Err(_) => None,
            };
            if let Some(opening) = opening {
                opening.await;
            }
            let ran = ran.await.unwrap_or_else(|_| Imported {
                copied: 0,
                skipped: 0,
                failed: vec![importer::Failure {
                    path: source,
                    message: "the import stopped unexpectedly".to_owned(),
                }],
                canceled: false,
                bytes: 0,
                elapsed_ms: 0,
                folders: Vec::new(),
            });
            this.update(cx, |this, cx| this.import_ran(run, ran, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// The run `run` created its destination `folders`: its dialog steps
    /// aside unless a cancel was asked for, and the first folder opens.
    fn import_started(
        &mut self,
        run: u64,
        folders: &[String],
        cx: &mut Context<Self>,
    ) -> Option<Task<()>> {
        if !self.folderops.import.accepts(run) {
            return None;
        }
        if self.folderops.import.started(run) {
            self.folderops.aside = self.folderops.dialog.take();
            self.follow_folder_dialog(cx);
        }
        cx.notify();
        let first = folders.first()?;
        Some(self.open_from_tree(first.clone(), cx))
    }

    /// The run `run` ended: the dialog closes, the failures go to the notes,
    /// the result stays in them, and the open folder is re-listed when it is
    /// one of the run's, so a file that landed before its watch was set
    /// shows.
    fn import_ran(&mut self, run: u64, ran: Imported, cx: &mut Context<Self>) {
        if !self.folderops.import.done(run) {
            return;
        }
        self.folderops.dialog = None;
        self.folderops.aside = None;
        self.follow_folder_dialog(cx);
        self.folderops.importing = None;
        self.folderops.import_cancel = None;
        self.folderops.import_quit = None;
        for failure in &ran.failed {
            self.write_failed(failure.path.clone(), failure.message.clone());
        }
        self.folderops.imported = Some(import::done_status(&ran));
        let opens = self
            .dir
            .as_deref()
            .is_some_and(|dir| trash::opens_target(dir, &ran.folders, IGNORE_CASE));
        if opens && self.resync.changed() {
            self.relist(cx);
        }
        cx.notify();
    }

    /// `Rename by Capture Date…`: not held by the gate but refused with the
    /// scan message while a listing, a scan or a run is under way, since the
    /// plan could be stale by its end; else planned on a background task
    /// from the index's capture times. Nothing starts while the settings,
    /// the trash dialog or another folder flow is open or busy.
    fn start_date_folder(&mut self, dir: String, cx: &mut Context<Self>) {
        if self.settings_view.read(cx).is_open() || self.trash.busy() || self.folderops.busy() {
            return;
        }
        if self.resync.busy() || self.gate.in_flight() {
            self.step_note = Some(SCAN_RUNNING.to_owned());
            cx.notify();
            return;
        }
        if !self.folderops.date.start(dir.clone()) {
            return;
        }
        let index = self.reader.clone().or_else(|| self.index.clone().ok());
        cx.spawn(async move |this, cx| {
            let previewed = cx
                .background_spawn(async move {
                    let dir = canonicalize(&dir);
                    let plan = datefolders::plan_of(index.as_deref(), &dir)?;
                    Ok::<_, String>(datefolders::preview(Path::new(&dir), plan))
                })
                .await;
            this.update(cx, |this, cx| this.date_folder_previewed(previewed, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// The plan came back: its dialog opens, or the refusal goes to the
    /// status and the flow ends.
    fn date_folder_previewed(
        &mut self,
        previewed: Result<datefolders::Preview, String>,
        cx: &mut Context<Self>,
    ) {
        match previewed {
            Ok(preview) if self.folderops.date.previewed() => {
                let reason = datefolder::disabled_reason(&preview);
                let content = Content {
                    title: "Rename by Capture Date".to_owned(),
                    lines: vec![datefolder::date_title_line(&preview)],
                    rows: rows(datefolder::date_rows(&preview)),
                    notes: [
                        datefolder::undated_line(&preview),
                        datefolder::removal_note(&preview),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                    reason: reason.clone(),
                    running_note: datefolder::RUNNING_NOTE.to_owned(),
                    ..Content::default()
                };
                self.menus.close();
                self.context.close();
                self.folderops.dialog = Some(Dialog::new(
                    content,
                    "Run",
                    Style::Primary,
                    reason.is_none(),
                ));
                self.follow_folder_dialog(cx);
            }
            Ok(_) => {}
            Err(e) => {
                self.folderops.date.end();
                self.step_note = Some(e);
            }
        }
        cx.notify();
    }

    /// The run button: refused with the scan message if a listing or scan
    /// started while the dialog was open; else, the buttons disabled, on a
    /// background task in flight so no listing or scan starts under it: the
    /// queued judgment stores and the sidecar writer flushed, the folder
    /// planned again (the preview is not trusted across the dialog) and
    /// refused on a collision or a folder already named, then renamed to its
    /// one date or split into one folder per date. The folder watcher and the
    /// tree watches on it are released by the backend around the moves.
    fn run_date_folder(&mut self, cx: &mut Context<Self>) {
        let Some(dir) = self.folderops.date.run() else {
            return;
        };
        if self.resync.busy() || self.gate.in_flight() {
            self.folderops.date.end();
            self.folderops.dialog = None;
            self.follow_folder_dialog(cx);
            self.step_note = Some(SCAN_RUNNING.to_owned());
            cx.notify();
            return;
        }
        if let Some(dialog) = &mut self.folderops.dialog {
            dialog.set_running();
        }
        self.gate.enter_in_flight();
        if self.dir.is_some() {
            self.resync.hold();
        }
        let (stores, writer, index, runs) = (
            self.stores.clone(),
            self.writer.clone(),
            self.index.clone().ok(),
            self.runs.clone(),
        );
        let (watch, trees) = (self.watch.clone(), self.tree_watch.clone());
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let caller = dir.clone();
            let ran = cx
                .background_spawn(async move {
                    stores.drain(sidecar::DRAIN_TIMEOUT);
                    if let Some(writer) = &writer {
                        writer.flush(sidecar::DRAIN_TIMEOUT);
                    }
                    let canonical = canonicalize(&caller);
                    let index = index.as_deref();
                    let plan = datefolders::plan_of(index, &canonical)?;
                    if !plan.collisions.is_empty() {
                        return Err(format!("{} already exists", plan.collisions.join(", ")));
                    }
                    let moves = Moves {
                        watch: &watch,
                        trees: &trees,
                        index,
                        runs: &runs,
                    };
                    match plan.kind {
                        DateKind::Named => Err(format!(
                            "{} is already named its capture date",
                            datefolders::file_name(Path::new(&canonical))
                        )),
                        DateKind::Rename => {
                            renamer::rename_folder(&moves, &caller, &plan.groups[0].name).map(
                                |renamed| {
                                    let new = canonicalize(&renamed.path);
                                    DateRan::Renamed(renamed, (canonical, new))
                                },
                            )
                        }
                        DateKind::Split => {
                            datefolders::split_folder(&moves, &canonical, &caller, &plan)
                                .map(|split| DateRan::Split(split, canonical))
                        }
                    }
                })
                .await;
            log::info!(
                "datefolder: {dir} {} in {}ms",
                match &ran {
                    Ok(DateRan::Renamed(renamed, _)) => format!("renamed to {}", renamed.path),
                    Ok(DateRan::Split(split, _)) => format!("split into {}", split.moved.len()),
                    Err(e) => format!("failed: {e}"),
                },
                started.elapsed().as_millis()
            );
            this.update(cx, |this, cx| this.date_folder_ran(dir, ran, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// The run came back and the dialog closes. A rename ends in
    /// `folder_renamed`. A split's failures go to the notes and the status
    /// says what moved; when the split folder was the open one it is
    /// re-listed if it remains, else the first date folder opens; and the
    /// parent is re-listed in the tree.
    fn date_folder_ran(
        &mut self,
        dir: String,
        ran: Result<DateRan, String>,
        cx: &mut Context<Self>,
    ) {
        self.gate.leave_in_flight();
        self.folderops.date.end();
        self.folderops.dialog = None;
        self.follow_folder_dialog(cx);
        let (split, canonical) = match ran {
            Ok(DateRan::Renamed(renamed, canonical)) => {
                return self.folder_renamed(&dir, &renamed.path, canonical, renamed.warning, cx);
            }
            Ok(DateRan::Split(split, canonical)) => (split, canonical),
            Err(e) => {
                self.step_note = Some(e);
                self.settle_folder_run(cx);
                cx.notify();
                return;
            }
        };
        for failure in &split.failed {
            self.write_failed(
                failure.path.clone(),
                format!("could not move: {}", failure.message),
            );
        }
        let status = datefolder::split_status(&split, &name_of(&dir));
        let opens = self
            .dir
            .as_deref()
            .is_some_and(|open| trash::opens_target(open, &[canonical], IGNORE_CASE));
        let first = split.moved.first().filter(|_| split.removed);
        if self.deferred_open.is_some() || !opens {
            self.step_note = Some(status);
            self.settle_folder_run(cx);
        } else if !split.removed {
            self.step_note = Some(status);
            self.resync.open();
            self.relist(cx);
        } else if let Some(first) = first {
            self.open_with_note(PathBuf::from(&first.path), Some(status), cx);
        } else {
            self.step_note = Some(status);
            self.settle_folder_run(cx);
        }
        self.tree_refresh(datefolder::parent_of(&dir), cx);
        cx.notify();
    }

    /// The folder `path` (the tree's spelling) is now `new_path`, its
    /// canonical path going from `canonical.0` to `canonical.1`, as
    /// `main.ts`'s `folderRenamed`: the tree renames the node in place with
    /// its state, its open row and cursor follow, the trash runs of both
    /// histories follow (the backend's recorded runs already did), and the
    /// tree's watched set is handed over afresh. When the open folder is the
    /// renamed one or under it, it is reopened under its new path, its
    /// judgments and last viewed file coming back from the carried index
    /// rows; a warning goes to the status once that open has landed.
    pub(crate) fn folder_renamed(
        &mut self,
        path: &str,
        new_path: &str,
        canonical: (String, String),
        warning: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.tree
            .rename_folder(path, new_path, &name_of(new_path), IGNORE_CASE);
        let moved = |p: &str| tree::rebase(p, path, new_path, IGNORE_CASE);
        self.tree_open = self.tree_open.take().map(|p| moved(&p).unwrap_or(p));
        self.tree_cursor = self.tree_cursor.take().map(|p| moved(&p).unwrap_or(p));
        let (old, new) = canonical;
        self.undo.map_trash_dirs(|dir| {
            tree::rebase(dir, &old, &new, IGNORE_CASE).unwrap_or_else(|| dir.to_owned())
        });
        self.tree_watched = None;
        self.tree_changed(cx);
        let reopen = match self.deferred_open.take() {
            Some(folder) => Some(
                moved(&folder.to_string_lossy())
                    .map(PathBuf::from)
                    .unwrap_or(folder),
            ),
            None => self
                .dir
                .as_deref()
                .and_then(|dir| tree::rebase(dir, &old, &new, IGNORE_CASE))
                .map(|dir| PathBuf::from(context::shown_path(&dir))),
        };
        match reopen {
            Some(folder) => self.open_with_note(folder, warning, cx),
            None => {
                if warning.is_some() {
                    self.step_note = warning;
                }
                self.settle_folder_run(cx);
            }
        }
        cx.notify();
    }

    /// Open `folder` as a click in the tree opens it, then put `note` in the
    /// status, which the open's fresh listing would otherwise clear.
    fn open_with_note(&mut self, folder: PathBuf, note: Option<String>, cx: &mut Context<Self>) {
        let opening = self.open(folder, None, None, cx);
        cx.spawn(async move |this, cx| {
            opening.await;
            let Some(note) = note else { return };
            this.update(cx, |this, cx| {
                this.step_note = Some(note);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// A folder run ended without reopening a folder: the folder the tree
    /// asked for meanwhile opens, else a change the watcher saw meanwhile
    /// re-lists.
    fn settle_folder_run(&mut self, cx: &mut Context<Self>) {
        if let Some(folder) = self.deferred_open.take() {
            self.open(folder, None, None, cx).detach();
        } else if self.resync.finished() {
            self.relist(cx);
        }
    }

    /// A message of a folder operation's run; one of an ended run is
    /// dropped.
    pub(crate) fn folder_op_message(&mut self, message: Message, cx: &mut Context<Self>) {
        match message {
            Message::SequenceProgress { run, done, total } => {
                if self.folderops.sequence.accepts(run) {
                    self.folderops.sequencing = Some(sequence::progress_status(done, total));
                    cx.notify();
                }
            }
            Message::ImportProgress { run, done, total } => {
                if self.folderops.import.accepts(run) {
                    self.folderops.importing = Some(import::progress_status(done, total));
                    cx.notify();
                }
            }
        }
    }

    /// The dialog's Cancel, or Escape when `key`.
    fn dismiss_folder_dialog(&mut self, key: bool, cx: &mut Context<Self>) {
        if self.folderops.import.is_open() {
            match self.folderops.import.dismiss(key) {
                import::Dismiss::Close => {
                    self.folderops.dialog = None;
                    self.folderops.import_plan = None;
                }
                import::Dismiss::Aside => self.folderops.aside = self.folderops.dialog.take(),
                import::Dismiss::Cancel => {
                    if let Some(cancel) = &self.folderops.import_cancel {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
                import::Dismiss::None => {}
            }
            self.follow_folder_dialog(cx);
            cx.notify();
        } else if self.folderops.sequence.is_open() {
            match self.folderops.sequence.dismiss() {
                sequence::Dismiss::Close => self.folderops.dialog = None,
                sequence::Dismiss::Cancel => {
                    if let Some(cancel) = &self.folderops.sequence_cancel {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
                sequence::Dismiss::None => {}
            }
            self.follow_folder_dialog(cx);
            cx.notify();
        } else if self.folderops.sidecars.dismiss() || self.folderops.date.dismiss() {
            self.folderops.dialog = None;
            self.follow_folder_dialog(cx);
            cx.notify();
        }
    }

    fn press_folder_dialog(&mut self, button: Button, cx: &mut Context<Self>) {
        match button {
            Button::Run if self.folderops.import.is_open() => self.run_import(cx),
            Button::Run if self.folderops.sequence.is_open() => self.run_sequence(cx),
            Button::Run if self.folderops.sidecars.is_open() => self.run_sidecars(cx),
            Button::Run if self.folderops.date.is_open() => self.run_date_folder(cx),
            Button::Run => {}
            Button::Cancel => self.dismiss_folder_dialog(false, cx),
        }
    }

    /// A key pressed in the dialog; true when it goes on to the focused
    /// button, which presses itself on Enter or Space. Every other key
    /// stops in the dialog.
    fn folder_dialog_key(&mut self, name: Option<&str>, cx: &mut Context<Self>) -> bool {
        let Some(dialog) = &self.folderops.dialog else {
            return false;
        };
        match dialog.key(name) {
            dialog::Key::Dismiss => {
                self.dismiss_folder_dialog(true, cx);
                false
            }
            dialog::Key::Press => true,
            dialog::Key::Swallow => false,
        }
    }

    /// Open the dialog when a flow shows one and close it when none is
    /// shown (a run's end, a dismissal, the import stepping aside), called
    /// wherever `folderops.dialog` changes, the only places it opens or
    /// closes. The focus goes to `Dialog::focus` on the opening and again
    /// whenever that changes (a cancelable run's start moves it to Cancel).
    pub(crate) fn follow_folder_dialog(&mut self, cx: &mut Context<Self>) {
        if self.folderops.dialog.is_none() {
            self.folder_modal.close(cx);
            return;
        }
        let this = cx.weak_entity();
        let buttons = self.folder_buttons.clone();
        let focused = Cell::new(None);
        self.folder_modal.open(cx, move |shell, window, cx| {
            let focus = this
                .upgrade()
                .and_then(|view| view.read(cx).folderops.dialog.as_ref()?.focus());
            if focus.is_some() && focused.replace(focus) != focus {
                let at = match focus {
                    Some(Button::Run) => 0,
                    _ => 1,
                };
                buttons[at].focus(window, cx);
            }
            folder_dialog(shell, &this, &buttons, cx)
        });
    }
}

/// The open folder dialog, inside `modal`'s shell, rebuilt from the view's
/// `folderops.dialog` on every rebuild of the dialog layer: the title, the
/// lines, the rows (in the component's body, which scrolls under its
/// scrollbar when long, an accent part in the pick color), then in the
/// footer the notes, the errors, the reason, the running note while the run
/// is under way, and the run and Cancel buttons on the focus handles
/// `buttons` holds. Its keys reach `folder_dialog_key` before the focused
/// button sees them.
fn folder_dialog(
    shell: component::Dialog,
    this: &WeakEntity<Riffle>,
    buttons: &[FocusHandle; 2],
    cx: &mut App,
) -> component::Dialog {
    let Some(view) = this.upgrade() else {
        return shell;
    };
    let Some(dialog) = &view.read(cx).folderops.dialog else {
        return shell;
    };
    let content = &dialog.content;
    let button = |id: &'static str, label: String, which: Button| {
        let this = this.clone();
        let at = match which {
            Button::Run => 0,
            Button::Cancel => 1,
        };
        UiButton::new(id)
            .track_focus(&buttons[at])
            .px(px(12.))
            .disabled(!dialog.enabled(which))
            .child(
                div()
                    .text_size(px(13.6))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            )
            .on_click(move |_: &ClickEvent, _, cx| {
                this.update(cx, |this, cx| this.press_folder_dialog(which, cx))
                    .ok();
            })
    };
    let run = button("folder-dialog-run", dialog.run_label.clone(), Button::Run);
    let run = match dialog.style {
        Style::Primary => run.primary(),
        Style::Destructive => run.danger(),
    };
    let muted = |text: &String| div().text_color(rgb(MUTED_COLOR)).child(text.clone());
    let lists_empty =
        content.rows.is_empty() && content.lists.iter().all(|(_, items)| items.is_empty());
    let lines: Vec<SharedString> = content.lines.iter().cloned().map(Into::into).collect();
    let keys = this.clone();
    shell
        .title(content.title.clone())
        .when(!lines.is_empty(), |shell| {
            shell.content(move |body, _, _| {
                body.gap_2()
                    .children(lines.iter().map(|line| div().child(line.clone())))
            })
        })
        .when(!lists_empty, |shell| {
            shell.child(
                div()
                    .flex()
                    .flex_col()
                    .children(content.rows.iter().map(|row| {
                        let text = format!("\u{2022} {}", row.text);
                        let accent = text.len()..text.len() + row.accent.len();
                        div()
                            .when(row.muted, |d| d.text_color(rgb(MUTED_COLOR)))
                            .child(StyledText::new(text + &row.accent).with_highlights([(
                                accent,
                                HighlightStyle {
                                    color: Some(rgb(PICK_COLOR).into()),
                                    ..HighlightStyle::default()
                                },
                            )]))
                    }))
                    .children(
                        content
                            .lists
                            .iter()
                            .filter(|(_, items)| !items.is_empty())
                            .map(|(heading, items)| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .pt_2()
                                    .child(heading.clone())
                                    .children(
                                        items
                                            .iter()
                                            .map(|item| div().child(format!("\u{2022} {item}"))),
                                    )
                            }),
                    ),
            )
        })
        .footer(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .capture_key_down(move |e: &KeyDownEvent, window, cx| {
                    let name = keys::key_name(&e.keystroke);
                    let press = keys
                        .update(cx, |this, cx| this.folder_dialog_key(name.as_deref(), cx))
                        .unwrap_or(false);
                    if !press {
                        window.prevent_default();
                        cx.stop_propagation();
                    }
                })
                .children(content.notes.iter().map(|note| div().child(note.clone())))
                .children(
                    content
                        .errors
                        .iter()
                        .map(|e| div().text_color(rgb(ERROR_COLOR)).child(e.clone())),
                )
                .children(content.reason.as_ref().map(muted))
                .when(dialog.running(), |d| d.child(muted(&content.running_note)))
                .child(DialogFooter::new().child(run).child(button(
                    "folder-dialog-cancel",
                    "Cancel".to_owned(),
                    Button::Cancel,
                ))),
        )
}

/// The tree's inline rename, a port of `folders.ts`'s editor and
/// `main.ts`'s `renameFolder` / `holdRename`.
impl Riffle {
    /// The name of the tree's row `at`: the text input while it is being
    /// renamed, else its name, or the held rename's name in the pending
    /// style. A click on the open folder's name arms the slow click instead
    /// of reopening it.
    pub(crate) fn tree_name(&self, at: usize, cx: &mut Context<Self>) -> AnyElement {
        let row = &self.tree_rows[at];
        if let Some(edit) = self
            .folderops
            .editing
            .as_ref()
            .filter(|edit| edit.rename.path == row.path)
        {
            return div()
                .id(("tree-name", at))
                .flex_1()
                .min_w_0()
                // The input's padding and border take 8 px of the row.
                .line_height(px(TREE_ROW_HEIGHT - 8.))
                .on_click(|_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    // The input propagates an Escape it has no use for.
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.rename_event(false, window, cx);
                    }
                }))
                .child(
                    Input::new(&edit.input)
                        .focus_bordered(false)
                        .bg(transparent_black())
                        .border_color(rgb(PRIMARY_COLOR))
                        .rounded_sm()
                        .px(px(3.))
                        .py(px(1.))
                        .text_size(px(13.)),
                )
                .into_any_element();
        }
        let pending = self
            .folderops
            .pending
            .current()
            .filter(|pending| pending.path == row.path);
        let name = display_name(pending, &row.path, &row.name);
        let name = div()
            .id(("tree-name", at))
            .flex_1()
            .min_w_0()
            .truncate()
            .when(row.failed.is_some(), |s| s.text_color(rgb(ERROR_COLOR)))
            .when(pending.is_some(), |s| {
                s.italic()
                    .text_color(rgb(MUTED_COLOR))
                    .tooltip(|window, cx| Tooltip::new(PENDING_TITLE).build(window, cx))
            })
            .child(if pending.is_some() {
                format!("\u{25f7} {name}")
            } else {
                name.to_owned()
            });
        if self.tree_open_row != Some(at) || row.depth == 0 {
            return name.into_any_element();
        }
        let path = row.path.clone();
        name.on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
            this.tree_name_clicked(&path, e, window, cx);
        }))
        .into_any_element()
    }

    /// A click on the open folder's name: without a modifier it does not
    /// reopen the folder; a single click arms the slow click, whose rename
    /// starts `SLOW_CLICK_DELAY` later unless a press or a key disarms it,
    /// and a double-click disarms it.
    fn tree_name_clicked(
        &mut self,
        path: &str,
        e: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = e.modifiers();
        if modifiers.secondary(cx) || modifiers.shift {
            return;
        }
        cx.stop_propagation();
        if e.click_count() != 1 {
            self.folderops.slow.cancel();
            return;
        }
        self.folderops.slow.click(path, true, Instant::now());
        let path = path.to_owned();
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(SLOW_CLICK_DELAY).await;
            this.update_in(cx, |this, window, cx| {
                if this.folderops.slow.due(&path, Instant::now()) {
                    this.start_rename(path, window, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// A press anywhere disarms the slow click; the arming click comes after
    /// its own press.
    pub(crate) fn disarm_slow_click(&mut self) {
        self.folderops.slow.cancel();
    }

    /// A key disarms the slow click; true while an inline rename is live,
    /// whose input owns every key it does not take itself, so none reaches
    /// the tree or the keymap.
    pub(crate) fn rename_owns_keys(&mut self) -> bool {
        self.folderops.slow.cancel();
        self.folderops.editing.is_some()
    }

    /// Opening a folder whose rename is in flight, or one under it, is
    /// refused with a note: the tree still draws it at a path about to move.
    pub(crate) fn refuse_renaming(&mut self, path: &str, cx: &mut Context<Self>) -> bool {
        if !self.folderops.renames.blocks(path, IGNORE_CASE) {
            return false;
        }
        self.tree_note = Some(RENAMING.to_owned());
        cx.notify();
        true
    }

    /// `Rename…`, or a slow click: the row's name turns into a text input
    /// holding its name (the pending one while a rename of it is held),
    /// selected whole and focused. A live edit of another row is confirmed
    /// first.
    pub(crate) fn start_rename(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.folderops.slow.cancel();
        if self.refuse_renaming(&path, cx) {
            return;
        }
        if self.folderops.editing.is_some() {
            self.finish_rename(Decision::Confirm, cx);
        }
        let Some(at) = self.tree_rows.iter().position(|row| row.path == path) else {
            return;
        };
        let name = display_name(
            self.folderops.pending.current(),
            &path,
            &self.tree_rows[at].name,
        )
        .to_owned();
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |input, cx| {
            input.set_value(name.as_str(), window, cx);
            input.select_all(window, cx);
        });
        let events = cx.subscribe_in(&input, window, |this, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.rename_event(true, window, cx);
            }
        });
        window.focus(&input.focus_handle(cx), cx);
        self.folderops.editing = Some(Edit {
            rename: InlineRename::new(path, name),
            input,
            _events: events,
        });
        self.tree_scroll.scroll_to_item(at, ScrollStrategy::Nearest);
        cx.notify();
    }

    /// The input's Enter or lost focus confirms, its Escape cancels. A focus
    /// lost to another app does not: the edit waits for the window to come
    /// back, as the Tauri editor's blur checks `document.hasFocus()`. After
    /// Enter or Escape the tree has the keyboard again.
    fn rename_event(&mut self, commit: bool, window: &mut Window, cx: &mut Context<Self>) {
        if commit && !window.is_window_active() {
            return;
        }
        let Some(input) = self
            .folderops
            .editing
            .as_ref()
            .map(|edit| edit.input.clone())
        else {
            return;
        };
        let focused = input.focus_handle(cx).is_focused(window);
        self.finish_rename(
            if commit {
                Decision::Confirm
            } else {
                Decision::Cancel
            },
            cx,
        );
        if focused || window.focused(cx).is_none() {
            window.focus(&self.focus, cx);
            self.set_tree_keys(true, cx);
        }
    }

    /// End the live edit with `decision`, once. A confirmed name acts on the
    /// folder as the tree has it now: nothing when it is gone; on a row
    /// whose rename is held, the real name cancels it.
    fn finish_rename(&mut self, decision: Decision, cx: &mut Context<Self>) {
        let Some(mut edit) = self.folderops.editing.take() else {
            return;
        };
        cx.notify();
        if commit(&mut edit.rename, decision) != Some(Decision::Confirm) {
            return;
        }
        let typed = edit.input.read(cx).value().to_string();
        let InlineRename { path, original, .. } = edit.rename;
        if !self.tree.has(&path) {
            return;
        }
        let real = self
            .tree_rows
            .iter()
            .find(|row| row.path == path)
            .map_or(original, |row| row.name.clone());
        match edit_outcome(self.folderops.pending.current(), &path, &real, &typed) {
            Outcome::Keep => {}
            Outcome::Cancel => self.cancel_pending_rename(&path),
            Outcome::Rename(name) => self.request_rename(path, name, cx),
        }
    }

    /// The real name typed back on the row whose rename is held: the held
    /// rename is dropped, the row shows its real name and the status says
    /// so. Only while that rename is still the one held.
    fn cancel_pending_rename(&mut self, path: &str) {
        let held = self.gate.waiting() == Some(rename::LABEL)
            && self
                .folderops
                .pending
                .current()
                .is_some_and(|pending| pending.path == path);
        if let Some(op) = held.then(|| self.gate.discard()).flatten()
            && let Some(note) = self.held_dropped(op, None)
        {
            self.step_note = Some(note.to_owned());
        }
    }

    /// A confirmed name, through the gate under `Rename…`: run at once with
    /// no listing, scan or run under way, else held, its row showing the new
    /// name in the pending style, without canceling the scan. Refused while
    /// an import runs.
    fn request_rename(&mut self, path: String, name: String, cx: &mut Context<Self>) {
        if self.folderops.import_busy() {
            self.step_note = Some(RENAME_REFUSED.to_owned());
            return;
        }
        let busy = self.resync.busy() || self.gate.in_flight();
        let mark = busy.then(|| self.folderops.pending.mark(path.clone(), name.clone()));
        match self.gate_request(
            rename::LABEL,
            HeldOp::Rename(Held { path, name, mark }),
            busy,
        ) {
            Some(HeldOp::Rename(held)) => {
                self.run_rename(held, cx);
            }
            Some(_) => {}
            None => log::info!("rename: held for the scan's end"),
        }
        cx.notify();
    }

    /// The gate's `request`, a held rename it replaces being dropped through
    /// `held_dropped` first, so its row shows its real name again and the
    /// status says it was canceled (unless a rename of the same folder
    /// replaced it).
    pub(crate) fn gate_request(
        &mut self,
        label: &'static str,
        op: HeldOp,
        busy: bool,
    ) -> Option<HeldOp> {
        if let Some(old) = self.gate.discard() {
            let by = match &op {
                HeldOp::Rename(held) => Some(held.path.as_str()),
                _ => None,
            };
            if let Some(note) = self.held_dropped(old, by) {
                self.step_note = Some(note.to_owned());
            }
        }
        self.gate.request(label, op, busy)
    }

    /// A held operation was dropped, by an open or a later request (`by`
    /// naming the folder a replacing rename is of): a rename's row shows
    /// its real name again, and the status note to show, if any.
    pub(crate) fn held_dropped(&mut self, op: HeldOp, by: Option<&str>) -> Option<&'static str> {
        let HeldOp::Rename(held) = op else {
            return None;
        };
        if let Some(mark) = &held.mark {
            self.folderops.pending.clear(mark);
        }
        rename::dropped_status(&held.path, by)
    }

    /// `task` (an open's listing), then `note` in the status, which the
    /// open's fresh listing would otherwise clear.
    pub(crate) fn note_after(
        &mut self,
        task: Task<()>,
        note: &'static str,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        cx.spawn(async move |this, cx| {
            task.await;
            this.update(cx, |this, cx| {
                this.step_note = Some(note.to_owned());
                cx.notify();
            })
            .ok();
        })
    }

    /// Run a rename on a background task, in flight so no listing or scan
    /// starts under it: the queued judgment stores and the sidecar writer
    /// flushed, then the backend's folder rename, which releases the folder
    /// watcher and the tree watches around it. Until it settles, the folder
    /// and everything under it can be neither opened nor renamed. Refused
    /// when an import began while it was held; false when it did not start.
    pub(crate) fn run_rename(&mut self, held: Held, cx: &mut Context<Self>) -> bool {
        let Held { path, name, mark } = held;
        if self.folderops.import_busy() {
            if let Some(mark) = &mark {
                self.folderops.pending.clear(mark);
            }
            self.step_note = Some(RENAME_REFUSED.to_owned());
            cx.notify();
            return false;
        }
        self.folderops.renames.start(path.clone());
        self.gate.enter_in_flight();
        if self.dir.is_some() {
            self.resync.hold();
        }
        let (stores, writer, index, runs) = (
            self.stores.clone(),
            self.writer.clone(),
            self.index.clone().ok(),
            self.runs.clone(),
        );
        let (watch, trees) = (self.watch.clone(), self.tree_watch.clone());
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let dir = path.clone();
            let ran = cx
                .background_spawn(async move {
                    stores.drain(sidecar::DRAIN_TIMEOUT);
                    if let Some(writer) = &writer {
                        writer.flush(sidecar::DRAIN_TIMEOUT);
                    }
                    let old = canonicalize(&dir);
                    let moves = Moves {
                        watch: &watch,
                        trees: &trees,
                        index: index.as_deref(),
                        runs: &runs,
                    };
                    renamer::rename_folder(&moves, &dir, &name).map(|renamed| {
                        let new = canonicalize(&renamed.path);
                        (renamed, (old, new))
                    })
                })
                .await;
            log::info!(
                "rename: {path} {} in {}ms",
                match &ran {
                    Ok((renamed, _)) => format!("renamed to {}", renamed.path),
                    Err(e) => format!("failed: {e}"),
                },
                started.elapsed().as_millis()
            );
            this.update(cx, |this, cx| this.rename_ran(path, mark, ran, cx))
                .ok();
        })
        .detach();
        cx.notify();
        true
    }

    /// The rename came back: the folder can be opened and renamed again,
    /// its pending name goes, and it ends in `folder_renamed`, or the
    /// refusal goes to the status and the row keeps its name.
    fn rename_ran(
        &mut self,
        path: String,
        mark: Option<Pending>,
        ran: Result<(Renamed, (String, String)), String>,
        cx: &mut Context<Self>,
    ) {
        self.gate.leave_in_flight();
        self.folderops.renames.settle(&path);
        if let Some(mark) = &mark {
            self.folderops.pending.clear(mark);
        }
        match ran {
            Ok((renamed, canonical)) => {
                self.folder_renamed(&path, &renamed.path, canonical, renamed.warning, cx);
            }
            Err(e) => {
                self.step_note = Some(e);
                self.settle_folder_run(cx);
                cx.notify();
            }
        }
    }
}

fn rows(texts: Vec<String>) -> Vec<Row> {
    texts.into_iter().map(Row::from).collect()
}
