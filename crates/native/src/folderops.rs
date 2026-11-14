//! The folder operations of the tree's right-click menu: which flow is under
//! way, the one-flow-at-a-time rule, the shared confirmation dialog
//! (`dialog.rs`) and its drawing, and the dispatch of a folder menu action.
//! So far `Rewrite Sidecars from Index…` and `Delete Sidecars…`
//! (`sidecars.rs`), `Sequence JPEG Timestamps…` (`sequence.rs`), `Import…`
//! (`import.rs`) and `Rename by Capture Date…` (`datefolder.rs`), with what
//! follows a folder rename (`folder_renamed`); the
//! window's view keeps one `FolderOps`, and this module adds to its `impl`,
//! as a child module reaches its private fields.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use futures::channel::oneshot;
use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, HighlightStyle, StyledText, Subscription, Task,
    deferred, div, prelude::*, px, relative, rgb, rgba,
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
use crate::scan::Event;
use crate::sequence::{self, SequenceFlow};
use crate::sidecars::{self, Kind, SidecarFlow, Target};
use crate::tree::{self, IGNORE_CASE};
use crate::{
    BORDER_COLOR, BUTTON_FILL, ERROR_COLOR, FOREGROUND_COLOR, HeldOp, INPUT_BORDER, MUTED_COLOR,
    PANE_COLOR, PICK_COLOR, PRIMARY_COLOR, Riffle, context, trash,
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
        match self.gate.request(label, HeldOp::Sidecars(target), busy) {
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
            cx.notify();
        } else if self.folderops.sidecars.dismiss() || self.folderops.date.dismiss() {
            self.folderops.dialog = None;
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

    /// Every key stops here while the dialog is open.
    pub(crate) fn folder_dialog_key(&mut self, name: Option<&str>, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.folderops.dialog else {
            return;
        };
        match dialog.key(name) {
            dialog::Key::Dismiss => self.dismiss_folder_dialog(true, cx),
            dialog::Key::Press(button) => self.press_folder_dialog(button, cx),
            dialog::Key::Swallow => cx.notify(),
        }
    }

    /// The open folder dialog: a full-window `occlude()`d layer, so no press
    /// reaches what is under it, with the title, the lines, the rows
    /// (scrolling inside the dialog when long, an accent part in the pick
    /// color), the notes, the errors, the reason, the running note while the
    /// run is under way, and the two buttons, the focused one outlined.
    pub(crate) fn folder_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let dialog = self.folderops.dialog.as_ref()?;
        let content = &dialog.content;
        let focused = dialog.focused();
        let button = |id: &'static str, label: String, which: Button| {
            let filled = which == Button::Run;
            let button = div()
                .id(id)
                .px(px(12.))
                .py(px(4.8))
                .rounded_md()
                .border_1()
                .text_size(px(13.6))
                .font_weight(FontWeight::MEDIUM)
                .map(|d| match (focused == Some(which), filled) {
                    (true, true) => d.border_color(rgb(FOREGROUND_COLOR)),
                    (true, false) => d.border_color(rgb(PRIMARY_COLOR)),
                    (false, true) => d.border_color(rgba(0)),
                    (false, false) => d.border_color(rgba(INPUT_BORDER)),
                })
                .map(|d| match (filled, dialog.style) {
                    (true, Style::Destructive) => d.bg(rgb(ERROR_COLOR)),
                    (true, Style::Primary) => d.bg(rgb(PRIMARY_COLOR)).text_color(rgb(PANE_COLOR)),
                    (false, _) => d.bg(rgba(BUTTON_FILL)),
                })
                .child(label);
            if !dialog.enabled(which) {
                return button.opacity(0.5);
            }
            button
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.press_folder_dialog(which, cx);
                }))
        };
        let muted = |text: &String| div().text_color(rgb(MUTED_COLOR)).child(text.clone());
        let lists_empty =
            content.rows.is_empty() && content.lists.iter().all(|(_, items)| items.is_empty());
        Some(
            deferred(
                div()
                    .id("folder-dialog-backdrop")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .occlude()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x00000099))
                    .child(
                        div()
                            .id("folder-dialog-box")
                            .occlude()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .w(px(640.))
                            .max_w(relative(0.9))
                            .max_h(relative(0.9))
                            .px_4()
                            .py_3()
                            .bg(rgb(PANE_COLOR))
                            .border_1()
                            .border_color(rgba(BORDER_COLOR))
                            .rounded_lg()
                            .text_color(rgb(FOREGROUND_COLOR))
                            .text_size(px(13.))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(content.title.clone()),
                            )
                            .children(content.lines.iter().map(|line| div().child(line.clone())))
                            .when(!lists_empty, |d| {
                                d.child(
                                    div()
                                        .id("folder-dialog-rows")
                                        .flex()
                                        .flex_col()
                                        .flex_shrink(1.)
                                        .min_h_0()
                                        .overflow_y_scroll()
                                        .children(content.rows.iter().map(|row| {
                                            let text = format!("\u{2022} {}", row.text);
                                            let accent = text.len()..text.len() + row.accent.len();
                                            div()
                                                .when(row.muted, |d| d.text_color(rgb(MUTED_COLOR)))
                                                .child(
                                                    StyledText::new(text + &row.accent)
                                                        .with_highlights([(
                                                            accent,
                                                            HighlightStyle {
                                                                color: Some(rgb(PICK_COLOR).into()),
                                                                ..HighlightStyle::default()
                                                            },
                                                        )]),
                                                )
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
                                                        .children(items.iter().map(|item| {
                                                            div().child(format!("\u{2022} {item}"))
                                                        }))
                                                }),
                                        ),
                                )
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
                            .child(
                                div()
                                    .flex()
                                    .flex_none()
                                    .justify_end()
                                    .gap_2()
                                    .child(button(
                                        "folder-dialog-run",
                                        dialog.run_label.clone(),
                                        Button::Run,
                                    ))
                                    .child(button(
                                        "folder-dialog-cancel",
                                        "Cancel".to_owned(),
                                        Button::Cancel,
                                    )),
                            ),
                    ),
            )
            .into_any_element(),
        )
    }
}

fn rows(texts: Vec<String>) -> Vec<Row> {
    texts.into_iter().map(Row::from).collect()
}
