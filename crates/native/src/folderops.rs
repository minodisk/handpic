//! The folder operations of the tree's right-click menu: which flow is under
//! way, the one-flow-at-a-time rule, the shared confirmation dialog
//! (`dialog.rs`) and its drawing, and the dispatch of a folder menu action.
//! So far `Rewrite Sidecars from Index…` and `Delete Sidecars…`
//! (`sidecars.rs`); the window's view keeps one `FolderOps`, and this module
//! adds to its `impl`, as a child module reaches its private fields.

use std::path::Path;
use std::time::Instant;

use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, deferred, div, prelude::*, px, relative, rgb, rgba,
};
use riffle_backend::{
    foldersidecars::{self, DeletePreview, Preview, Summary},
    listing::canonicalize,
    sidecar,
    text::{SCAN_RUNNING, SIZE_BASE, format_bytes},
    trash::Summary as TrashSummary,
};

use crate::dialog::{self, Button, Content, Dialog, Style};
use crate::judgment::State;
use crate::sidecars::{self, Kind, SidecarFlow, Target};
use crate::tree::IGNORE_CASE;
use crate::{
    BORDER_COLOR, BUTTON_FILL, ERROR_COLOR, FOREGROUND_COLOR, HeldOp, INPUT_BORDER, MUTED_COLOR,
    PANE_COLOR, PRIMARY_COLOR, Riffle, context, trash,
};

/// The folder operations' view state: the flow under way and its dialog,
/// laid out only while open.
#[derive(Default)]
pub struct FolderOps {
    sidecars: SidecarFlow,
    dialog: Option<Dialog>,
}

impl FolderOps {
    /// A folder flow is under way, from its preview to its run's end.
    pub fn busy(&self) -> bool {
        self.sidecars.busy()
    }

    pub fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }
}

/// What a sidecar preview read.
enum Previewed {
    Rewrite(Preview),
    Delete(DeletePreview),
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
                            rows: sidecars::rewrite_rows(&preview),
                            note: Some(sidecars::rewrite_total_line(&preview, name)),
                            running_note: sidecars::REWRITE_RUNNING_NOTE.to_owned(),
                            ..Content::default()
                        },
                        "Rewrite",
                        Style::Primary,
                    ),
                    Previewed::Delete(preview) => (
                        Content {
                            title: "Delete Sidecars".to_owned(),
                            rows: sidecars::delete_rows(&preview),
                            note: Some(sidecars::delete_total_line(&preview, name)),
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

    fn dismiss_folder_dialog(&mut self, cx: &mut Context<Self>) {
        if self.folderops.sidecars.dismiss() {
            self.folderops.dialog = None;
            cx.notify();
        }
    }

    fn press_folder_dialog(&mut self, button: Button, cx: &mut Context<Self>) {
        match button {
            Button::Run if self.folderops.sidecars.is_open() => self.run_sidecars(cx),
            Button::Run => {}
            Button::Cancel => self.dismiss_folder_dialog(cx),
        }
    }

    /// Every key stops here while the dialog is open.
    pub(crate) fn folder_dialog_key(&mut self, name: Option<&str>, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.folderops.dialog else {
            return;
        };
        match dialog.key(name) {
            dialog::Key::Dismiss => self.dismiss_folder_dialog(cx),
            dialog::Key::Press(button) => self.press_folder_dialog(button, cx),
            dialog::Key::Swallow => cx.notify(),
        }
    }

    /// The open folder dialog: a full-window `occlude()`d layer, so no press
    /// reaches what is under it, with the title, the lines, the rows
    /// (scrolling inside the dialog when long), the note, the reason, the
    /// running note while the run is under way, and the two buttons, the
    /// focused one outlined.
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
                            .when(!content.rows.is_empty(), |d| {
                                d.child(
                                    div()
                                        .id("folder-dialog-rows")
                                        .flex()
                                        .flex_col()
                                        .flex_shrink(1.)
                                        .min_h_0()
                                        .overflow_y_scroll()
                                        .children(
                                            content
                                                .rows
                                                .iter()
                                                .map(|row| div().child(format!("\u{2022} {row}"))),
                                        ),
                                )
                            })
                            .children(content.note.as_ref().map(|note| div().child(note.clone())))
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
