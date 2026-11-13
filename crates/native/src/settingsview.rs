//! The settings modal as its own entity with its own focus handle, over the
//! pure `settingsmodal` model: a backdrop over the whole window and a
//! centered box, painted after the rest of the window and occluding it,
//! laid out only while open. A change is emitted as a `Change` for the main
//! view, which owns the values, to apply.

use gpui::{
    ClickEvent, Context, ElementId, Empty, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, KeyDownEvent, MouseButton, SharedString, Window, deferred, div, prelude::*, px,
    rgb, rgba,
};

use riffle_backend::{shortcuts::Keymap, sidecar::SidecarFormat};
use riffle_core::{i18n, xmp::LabelNames};

use crate::{
    ACCENT_COLOR, BORDER_COLOR, BUTTON_FILL, ERROR_COLOR, FOREGROUND_COLOR, INPUT_BORDER,
    MUTED_COLOR, PANE_COLOR, PRIMARY_COLOR, keys,
    settings::Settings,
    settingsmodal::{self, Edit, FORMATS, LABEL_COLORS, ModalKey, SettingsModal, Step, TABS, Tab},
    textinput::{self, TextInput},
};

/// What the main view applies.
pub enum Change {
    AutoAdvance(bool),
    SidecarFormat(SidecarFormat),
    /// The normalized names after a commit that changed them.
    LabelNames(LabelNames),
    /// The keymap after a successful edit.
    Keymap(Keymap),
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
    /// The values shown, read from the main view on every open.
    auto_advance: bool,
    format: SidecarFormat,
    keymap: Keymap,
    label_names: LabelNames,
    /// The Sidecar tab's name fields, in `LABEL_COLORS` order.
    name_fields: Vec<Entity<TextInput>>,
    /// The language `Reset` fills in, an index into `i18n::presets()`.
    language: usize,
    status: Option<SharedString>,
}

impl EventEmitter<Change> for SettingsView {}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_fields: Vec<_> = LABEL_COLORS
            .iter()
            .map(|_| cx.new(|cx| TextInput::new(window, cx)))
            .collect();
        for field in &name_fields {
            cx.subscribe_in(field, window, |this, _, event, window, cx| {
                this.name_field(*event, window, cx);
            })
            .detach();
        }
        Self {
            focus: cx.focus_handle(),
            tabs: TABS.iter().map(|_| cx.focus_handle()).collect(),
            model: SettingsModal::default(),
            auto_advance: true,
            format: SidecarFormat::default(),
            keymap: Keymap::defaults(),
            label_names: LabelNames::default(),
            name_fields,
            language: 0,
            status: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.model.is_open()
    }

    /// Opens on the tab it was left on, with that tab focused, showing
    /// `settings`' values.
    pub fn open(&mut self, settings: &Settings, window: &mut Window, cx: &mut Context<Self>) {
        if !self.model.open() {
            return;
        }
        self.auto_advance = settings.auto_advance;
        self.format = settings.format;
        self.keymap = settings.keymap.clone();
        self.show_names(settings.names.clone(), window, cx);
        self.status = (!settings.writes.is_open()).then(|| settingsmodal::NOT_SAVED.into());
        window.focus(&self.tabs[self.model.tab.at()], cx);
        cx.notify();
    }

    fn show_names(&mut self, names: LabelNames, window: &Window, cx: &mut Context<Self>) {
        for (field, text) in self
            .name_fields
            .iter()
            .zip(settingsmodal::label_texts(&names))
        {
            field.update(cx, |field, cx| field.set_text(text, window, cx));
        }
        self.label_names = names;
    }

    /// A name field committed (Enter, or it lost the focus) or was left with
    /// Escape. Escape commits too, then closes the modal: the Tauri field's
    /// `change` fires when the closing dialog takes its focus.
    fn name_field(&mut self, event: textinput::Event, window: &mut Window, cx: &mut Context<Self>) {
        let texts: Vec<String> = self
            .name_fields
            .iter()
            .map(|field| field.read(cx).text().to_owned())
            .collect();
        let texts = std::array::from_fn(|at| texts[at].as_str());
        self.commit_names(settingsmodal::label_names_of(texts), window, cx);
        if event == textinput::Event::Cancel && self.model.is_open() {
            self.close(cx);
        }
    }

    /// Shows `names` back in the fields and hands them to the main view when
    /// they differ from those in force.
    fn commit_names(&mut self, names: LabelNames, window: &Window, cx: &mut Context<Self>) {
        let current = self.label_names.clone();
        self.show_names(names.clone(), window, cx);
        if let Some(names) = settingsmodal::label_change(&current, names) {
            self.status = None;
            cx.emit(Change::LabelNames(names));
        }
        cx.notify();
    }

    /// `Reset`: the chosen language's Lightroom names.
    fn reset_names(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(preset) = i18n::presets().get(self.language) {
            self.commit_names(preset.names.clone(), window, cx);
        }
    }

    fn editing(&self, window: &Window, cx: &gpui::App) -> bool {
        self.name_fields
            .iter()
            .any(|field| field.focus_handle(cx).is_focused(window))
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.model.close();
        cx.emit(Change::Closed);
        cx.notify();
    }

    /// Every key stops here, so none reaches the window's keymap, except
    /// that a key typed in a name field goes on to the platform's text input
    /// (the window's own handler returns while the modal is open).
    fn key_down(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let name = keys::key_name(&e.keystroke);
        let key = self.model.key(name.as_deref());
        if !settingsmodal::stops(&key, self.editing(window, cx)) {
            return;
        }
        cx.stop_propagation();
        match key {
            ModalKey::Close => self.close(cx),
            ModalKey::Focus(Step::Next) => window.focus_next(cx),
            ModalKey::Focus(Step::Previous) => window.focus_prev(cx),
            ModalKey::Add(key) => {
                if let Some(action) = self.model.capturing.take() {
                    self.edit(Edit::Add(action, key), cx);
                }
            }
            ModalKey::Cancel => {
                self.model.capturing = None;
                cx.notify();
            }
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
        self.model.select(tab);
        cx.notify();
    }

    /// The row's `+`: the next key goes to `action`. The box takes the focus,
    /// since the pressed `+` is replaced by the prompt and a focus left on a
    /// node no longer drawn would send the key to the window's root.
    fn capture(&mut self, action: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.model.capturing = Some(action);
        self.status = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Applies `edit` to the shown keymap and hands the result to the main
    /// view; a refusal goes to the status line and changes nothing.
    fn edit(&mut self, edit: Edit, cx: &mut Context<Self>) {
        self.model.capturing = None;
        match settingsmodal::edit(&mut self.keymap, edit) {
            Ok(()) => {
                self.status = None;
                cx.emit(Change::Keymap(self.keymap.clone()));
            }
            Err(e) => self.status = Some(e.into()),
        }
        cx.notify();
    }

    fn flip_auto_advance(&mut self, cx: &mut Context<Self>) {
        self.auto_advance = !self.auto_advance;
        cx.emit(Change::AutoAdvance(self.auto_advance));
        cx.notify();
    }

    fn choose_format(&mut self, chosen: SidecarFormat, cx: &mut Context<Self>) {
        let Some(format) = settingsmodal::format_change(self.format, chosen) else {
            return;
        };
        self.format = format;
        self.status = None;
        cx.emit(Change::SidecarFormat(format));
        cx.notify();
    }

    /// A switch failed: its message goes to the status line.
    pub fn failed(&mut self, message: String, cx: &mut Context<Self>) {
        self.status = Some(message.into());
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
            Tab::Culling => content.child(checkbox(
                "auto-advance",
                self.auto_advance,
                "Auto-advance after a star, reject or pick",
                cx.listener(|this, _: &ClickEvent, _, cx| this.flip_auto_advance(cx)),
            )),
            Tab::Sidecar => content
                .children(FORMATS.iter().map(|&(format, label)| {
                    radio(
                        format.setting(),
                        format == self.format,
                        label,
                        cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.choose_format(format, cx);
                        }),
                    )
                }))
                .when(settingsmodal::writes_xmp(self.format), |d| {
                    d.child(self.label_names(cx))
                }),
            Tab::Shortcuts => {
                content
                    .child(self.shortcuts(cx))
                    .child(div().flex().justify_end().child(button(
                        "shortcuts-reset-all".into(),
                        "Reset all",
                        cx.listener(|this, _: &ClickEvent, _, cx| this.edit(Edit::ResetAll, cx)),
                    )))
            }
            Tab::Import => content,
        }
    }

    /// The Tauri paragraph, a name field per color, the languages `Reset`
    /// chooses from (GPUI has no select element: a wrapping row of them,
    /// the chosen one lit) and `Reset`.
    fn label_names(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_2()
            .child(
                div()
                    .text_color(rgb(MUTED_COLOR))
                    .child(settingsmodal::LABEL_NAMES_NOTE),
            )
            .children(
                LABEL_COLORS
                    .iter()
                    .zip(&self.name_fields)
                    .map(|(color, field)| {
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(64.)).flex_none().child(*color))
                            .child(div().w(px(240.)).child(field.clone()))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(4.))
                    .children(i18n::presets().iter().enumerate().map(|(at, preset)| {
                        let chosen = at == self.language;
                        div()
                            .id(("label-language", at))
                            .tab_index(0)
                            .px(px(6.))
                            .rounded_sm()
                            .border_1()
                            .border_color(rgba(if chosen { 0 } else { INPUT_BORDER }))
                            .when(chosen, |d| {
                                d.bg(rgb(ACCENT_COLOR)).border_color(rgb(PRIMARY_COLOR))
                            })
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(ACCENT_COLOR)))
                            .focus_visible(|s| s.border_color(rgb(PRIMARY_COLOR)))
                            .child(preset.name.clone())
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.language = at;
                                cx.notify();
                            }))
                    }))
                    .child(button(
                        "label-names-reset".into(),
                        "Reset",
                        cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.reset_names(window, cx)
                        }),
                    )),
            )
    }

    /// One row per action: its label, a chip per key with its remove button,
    /// `+` or the capture prompt, and `Reset`. Laid out only while the tab is
    /// shown, so the rows are plain `div`s in a scrolling list.
    fn shortcuts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("shortcuts")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .children(
                self.keymap
                    .bindings()
                    .into_iter()
                    .enumerate()
                    .map(|(at, b)| {
                        let action = b.action;
                        let capturing = self.model.capturing == Some(action);
                        div()
                            .id(("shortcut", at))
                            .flex()
                            .items_center()
                            .gap_2()
                            .py(px(3.))
                            .child(
                                div()
                                    .w(px(220.))
                                    .flex_none()
                                    .child(settingsmodal::shortcut_label(action)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(4.))
                                    .children(b.keys.into_iter().enumerate().map(|(k, key)| {
                                        let shown = settingsmodal::display_key(&key).to_owned();
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(3.))
                                            .pl(px(6.))
                                            .rounded_sm()
                                            .bg(rgb(ACCENT_COLOR))
                                            .child(shown)
                                            .child(button(
                                                ("remove", k).into(),
                                                "\u{d7}",
                                                cx.listener(
                                                    move |this, _: &ClickEvent, window, cx| {
                                                        this.edit(
                                                            Edit::Remove(action, key.clone()),
                                                            cx,
                                                        );
                                                        // The pressed button is gone with its chip.
                                                        window.focus(&this.focus, cx);
                                                    },
                                                ),
                                            ))
                                    }))
                                    .child(if capturing {
                                        div()
                                            .text_color(rgb(MUTED_COLOR))
                                            .child("Press a key...")
                                            .into_any_element()
                                    } else {
                                        button(
                                            "add".into(),
                                            "+",
                                            cx.listener(move |this, _: &ClickEvent, window, cx| {
                                                this.capture(action, window, cx);
                                            }),
                                        )
                                        .into_any_element()
                                    }),
                            )
                            .child(button(
                                "reset".into(),
                                "Reset",
                                cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.edit(Edit::Reset(action), cx);
                                }),
                            ))
                    }),
            )
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

/// A small outlined button; a click, Enter or Space on it when focused runs
/// `on_click`. `id` is unique within its row, whose own id scopes it.
fn button(
    id: ElementId,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .tab_index(0)
        .flex_none()
        .px(px(6.))
        .rounded_sm()
        .border_1()
        .border_color(rgba(INPUT_BORDER))
        .bg(rgba(BUTTON_FILL))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(ACCENT_COLOR)))
        .focus_visible(|s| s.border_color(rgb(PRIMARY_COLOR)))
        .child(label)
        .on_click(on_click)
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

/// A radio row: the round box, filled when selected, then its label; a click,
/// Enter or Space on the focused row selects it.
fn radio(
    id: &'static str,
    selected: bool,
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
                .rounded_full()
                .border_1()
                .border_color(rgba(INPUT_BORDER))
                .when(selected, |d| {
                    d.border_color(rgb(PRIMARY_COLOR))
                        .child(div().size(px(6.)).rounded_full().bg(rgb(PRIMARY_COLOR)))
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
