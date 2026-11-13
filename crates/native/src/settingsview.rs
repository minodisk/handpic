//! The settings modal as its own entity with its own focus handle, over the
//! pure `settingsmodal` model: a backdrop over the whole window and a
//! centered box, painted after the rest of the window and occluding it,
//! laid out only while open. A change is emitted as a `Change` for the main
//! view, which owns the values, to apply; the AI switch and its model
//! download (`riffle_backend::ai`) live here.

use std::path::PathBuf;

use futures::{StreamExt, channel::mpsc};
use gpui::{
    ClickEvent, Context, Empty, EventEmitter, FocusHandle, FontWeight, KeyDownEvent, MouseButton,
    SharedString, Window, deferred, div, prelude::*, px, rgb, rgba,
};
use riffle_backend::ai::{Ai, AiState};

use crate::{
    ACCENT_COLOR, BORDER_COLOR, ERROR_COLOR, FOREGROUND_COLOR, INPUT_BORDER, MUTED_COLOR,
    PANE_COLOR, PRIMARY_COLOR, keys,
    settingsmodal::{self, ModalKey, SettingsModal, Step, TABS, Tab},
};

/// What the main view applies.
pub enum Change {
    AutoAdvance(bool),
    AiEnabled(bool),
    /// The focus goes back to the window.
    Closed,
}

pub struct SettingsView {
    /// The box's, so a key inside the modal reaches its handler.
    focus: FocusHandle,
    /// One per tab of `TABS`; only the selected one is a tab stop, as the
    /// Tauri tablist's roving `tabindex`.
    tabs: Vec<FocusHandle>,
    model: SettingsModal,
    /// The value shown, read from the main view on every open.
    auto_advance: bool,
    ai: Ai,
    ai_state: AiState,
    status: Option<SharedString>,
}

impl EventEmitter<Change> for SettingsView {}

impl SettingsView {
    /// `model` is the embedding model's file; the download thread's state
    /// reports reach the view through a channel.
    pub fn new(ai_enabled: bool, model: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        match &model {
            Some(path) => log::info!("embedding model: {}", path.display()),
            None => log::warn!("no local data dir for the embedding model"),
        }
        let (states, mut received) = mpsc::unbounded();
        let ai = Ai::new(ai_enabled, model, move |state| {
            let _ = states.unbounded_send(state);
        });
        cx.spawn(async move |this, cx| {
            while let Some(state) = received.next().await {
                let shown = this.update(cx, |this, cx| {
                    this.ai_state = state;
                    if this.model.is_open() {
                        cx.notify();
                    }
                });
                if shown.is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            focus: cx.focus_handle(),
            tabs: TABS.iter().map(|_| cx.focus_handle()).collect(),
            model: SettingsModal::default(),
            auto_advance: true,
            ai_state: ai.state(),
            ai,
            status: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.model.is_open()
    }

    /// Opens on the tab it was left on, with that tab focused. `saved` is
    /// whether the settings store opened.
    pub fn open(
        &mut self,
        auto_advance: bool,
        saved: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.model.open() {
            return;
        }
        self.auto_advance = auto_advance;
        self.ai_state = self.ai.state();
        self.status = (!saved).then(|| settingsmodal::NOT_SAVED.into());
        window.focus(&self.tabs[self.model.tab.at()], cx);
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.model.close();
        cx.emit(Change::Closed);
        cx.notify();
    }

    /// Every key stops here, so none reaches the window's keymap.
    fn key_down(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        let name = keys::key_name(&e.keystroke);
        match self.model.key(name.as_deref()) {
            ModalKey::Close => self.close(cx),
            ModalKey::Focus(Step::Next) => window.focus_next(cx),
            ModalKey::Focus(Step::Previous) => window.focus_prev(cx),
            ModalKey::Add(key) => log::debug!("settings: no shortcut row takes {key} yet"),
            ModalKey::Cancel => {}
            ModalKey::Native => {
                let Some(name) = name else { return };
                if self.tabs[self.model.tab.at()].is_focused(window) {
                    let tab = settingsmodal::next_tab(self.model.tab, &name);
                    if tab != self.model.tab {
                        self.select(tab, cx);
                        window.focus(&self.tabs[tab.at()], cx);
                    }
                }
            }
        }
    }

    fn select(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.model.tab = tab;
        cx.notify();
    }

    fn flip_auto_advance(&mut self, cx: &mut Context<Self>) {
        self.auto_advance = !self.auto_advance;
        cx.emit(Change::AutoAdvance(self.auto_advance));
        cx.notify();
    }

    fn flip_ai(&mut self, cx: &mut Context<Self>) {
        let on = !self.ai_state.enabled;
        self.ai_state = self.ai.switch(on);
        cx.emit(Change::AiEnabled(on));
        cx.notify();
    }

    fn tab_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(2.))
            .w(px(180.))
            .pr_3()
            .border_r_1()
            .border_color(rgba(BORDER_COLOR))
            .children(TABS.iter().enumerate().map(|(at, &tab)| {
                let selected = tab == self.model.tab;
                div()
                    .id(("settings-tab", at))
                    .track_focus(&self.tabs[at].clone().tab_stop(selected))
                    .px(px(9.6))
                    .py(px(6.4))
                    .rounded_sm()
                    .border_1()
                    .border_color(rgba(0))
                    .text_size(px(13.6))
                    .text_color(rgb(if selected {
                        FOREGROUND_COLOR
                    } else {
                        MUTED_COLOR
                    }))
                    .when(selected, |d| {
                        d.bg(rgb(ACCENT_COLOR)).font_weight(FontWeight::SEMIBOLD)
                    })
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(ACCENT_COLOR)).text_color(rgb(FOREGROUND_COLOR)))
                    .focus_visible(|s| s.border_color(rgb(PRIMARY_COLOR)))
                    .child(tab.label())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select(tab, cx)))
            }))
    }

    fn content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let content = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap_2()
            .text_size(px(13.));
        match self.model.tab {
            Tab::Culling => content
                .child(checkbox(
                    "auto-advance",
                    self.auto_advance,
                    "Auto-advance after a star, reject or pick",
                    cx.listener(|this, _: &ClickEvent, _, cx| this.flip_auto_advance(cx)),
                ))
                .child(checkbox(
                    "ai-enabled",
                    self.ai_state.enabled,
                    "AI analysis of what each photo shows",
                    cx.listener(|this, _: &ClickEvent, _, cx| this.flip_ai(cx)),
                ))
                .child(div().px_2().child(settingsmodal::AI_NOTE))
                .child(
                    div()
                        .px_2()
                        .text_color(rgb(MUTED_COLOR))
                        .child(settingsmodal::ai_status(&self.ai_state)),
                ),
            Tab::Sidecar | Tab::Shortcuts | Tab::Import => content,
        }
    }

    fn dialog(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        div()
            .id("settings-box")
            .occlude()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.key_down(e, window, cx);
            }))
            .flex()
            .flex_col()
            .gap_2()
            .w((viewport.width * 0.9).min(px(860.)))
            .h((viewport.height * 0.8).min(px(720.)))
            .px_4()
            .py_3()
            .bg(rgb(PANE_COLOR))
            .border_1()
            .border_color(rgba(BORDER_COLOR))
            .rounded_lg()
            .text_color(rgb(FOREGROUND_COLOR))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Settings"),
                    )
                    .child(
                        div()
                            .id("settings-close")
                            .tab_index(0)
                            .px(px(6.4))
                            .rounded_sm()
                            .border_1()
                            .border_color(rgba(0))
                            .text_size(px(19.))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(ACCENT_COLOR)))
                            .focus_visible(|s| s.border_color(rgb(PRIMARY_COLOR)))
                            .child("\u{d7}")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_3()
                    .child(self.tab_list(cx))
                    .child(self.content(cx)),
            )
            .children(self.status.clone().map(|status| {
                div()
                    .text_size(px(13.))
                    .text_color(rgb(ERROR_COLOR))
                    .child(status)
            }))
    }
}

/// A checkbox row: the box, checked or not, then its label; a click, Enter or
/// Space on the focused row flips it.
fn checkbox(
    id: &'static str,
    checked: bool,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .tab_index(0)
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py(px(2.))
        .rounded_sm()
        .border_1()
        .border_color(rgba(0))
        .cursor_pointer()
        .focus_visible(|s| s.border_color(rgb(PRIMARY_COLOR)))
        .child(
            div()
                .size(px(14.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(rgba(INPUT_BORDER))
                .text_size(px(11.))
                .when(checked, |d| {
                    d.bg(rgb(PRIMARY_COLOR))
                        .text_color(rgb(PANE_COLOR))
                        .child("\u{2713}")
                }),
        )
        .child(label)
        .on_click(on_click)
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.model.is_open() {
            return Empty.into_any_element();
        }
        // Absolute over the window's root, drawn after the rest of it; the
        // backdrop and the box occlude what is under them, so a press or a
        // wheel never reaches the strip or the menus' buttons.
        deferred(
            div()
                .id("settings-backdrop")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgba(0x00000099))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.close(cx)),
                )
                .child(self.dialog(window, cx)),
        )
        .into_any_element()
    }
}
