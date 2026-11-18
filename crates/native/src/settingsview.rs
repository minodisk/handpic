//! The settings as their own entity with its own focus handle, over the
//! pure `settingsmodal` model, drawn by the main view over the whole window,
//! outside its key context, while open, its body gpui-component's
//! `Settings`. A change is emitted as a `Change` for the main view, which
//! owns the values, to apply.

use gpui_kit::component::{
    ActiveTheme, Disableable, FocusTrapElement, IndexPath, Sizable, ThemeMode,
    button::{Button, ButtonVariants},
    input::{
        Input, InputEvent, InputGroup, InputGroupAddon, InputGroupAddonAlignment, InputGroupButton,
        InputState,
    },
    kbd::Kbd,
    radio::Radio,
    select::{SearchableVec, Select, SelectEvent, SelectState},
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};
use gpui_kit::{
    ClickEvent, Context, ElementId, Empty, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, KeyBinding, KeyDownEvent, MouseDownEvent, NoAction, PathPromptOptions, Pixels,
    SharedString, Window, div, prelude::*, px, transparent_black,
};

use handpic_backend::{
    import,
    shortcuts::{Binding, Keymap},
    sidecar::SidecarFormat,
};
use handpic_core::{i18n, xmp::LabelNames};

use crate::{
    cache, keys,
    settings::Settings as Stored,
    settingsmodal::{self, Edit, FORMATS, LABEL_COLORS, MODES, ModalKey, SettingsModal, TABS, Tab},
    theme::{self, Mode},
    update::{self, Updates},
};

/// The key context of the box while a shortcut row waits for a key.
pub const CAPTURE_CONTEXT: &str = "SettingsCapture";

/// `Root` binds `tab` / `shift-tab` to the focus trap's walk, dispatched
/// before the box's key listener. These mask them while a row captures, so
/// the keys reach `key_down` and are taken as the shortcut.
pub fn capture_keys() -> [KeyBinding; 2] {
    [
        KeyBinding::new("tab", NoAction, Some(CAPTURE_CONTEXT)),
        KeyBinding::new("shift-tab", NoAction, Some(CAPTURE_CONTEXT)),
    ]
}

/// The key context of a gpui-component `Input`, on the focused path while
/// a text field has the focus.
const INPUT_CONTEXT: &str = "Input";

/// The height of the port the fields replaced: a 13 px line at GPUI's
/// default line height, 3 px of padding and the border on each side.
const FIELD_HEIGHT: Pixels = px(29.);

/// The language picker's items, the presets' names in `i18n::presets()`
/// order.
type Languages = SelectState<SearchableVec<SharedString>>;

/// The Appearance page's theme pickers, `theme::names` of one mode.
type Themes = SelectState<SearchableVec<SharedString>>;

/// What the main view applies.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    AutoAdvance(bool),
    SidecarFormat(SidecarFormat),
    /// The normalized names after a commit that changed them.
    LabelNames(LabelNames),
    /// The keymap after a successful edit.
    Keymap(Keymap),
    /// The destination root chosen in the folder dialog.
    ImportRoot(String),
    /// A committed pattern that is valid and not the stored one.
    ImportPattern(String),
    /// The theme setting after a choice that changed it.
    Theme(theme::Setting),
    /// The MCP page's switch, which only the user flips.
    McpAllowDestructive(bool),
    /// `Clear Cache` pressed.
    ClearCache,
    /// The Updates page's buttons.
    CheckForUpdates,
    RestartNow,
    OpenReleasePage(String),
    /// The main view takes the focus back.
    Closed,
}

pub struct SettingsView {
    /// The box's, so a key inside the modal reaches its handler.
    focus: FocusHandle,
    model: SettingsModal,
    /// The values shown, read from the main view on every open.
    auto_advance: bool,
    format: SidecarFormat,
    keymap: Keymap,
    label_names: LabelNames,
    /// The Sidecar page's name fields, in `LABEL_COLORS` order.
    name_fields: Vec<Entity<InputState>>,
    /// The Lightroom language whose preset names are those in force, none
    /// while no preset's are; a choice puts its preset's names in force.
    languages: Entity<Languages>,
    /// The stored destination root and pattern.
    import_root: String,
    /// The root shown read-only, so it can be selected and copied.
    root_field: Entity<InputState>,
    import_pattern: String,
    pattern_field: Entity<InputState>,
    /// Where today's photos would go under the field's pattern, or why it is
    /// refused, recomputed as the field changes.
    example: Result<String, String>,
    theme: theme::Setting,
    mcp_allow_destructive: bool,
    /// The light and the dark theme's pickers.
    light_themes: Entity<Themes>,
    dark_themes: Entity<Themes>,
    /// The Cache page's size, the press waiting for the running scan and the
    /// clear under way, set by the main view, which runs the clear.
    index_size: SharedString,
    clear_waiting: bool,
    clear_running: bool,
    /// The update flow as the main view last handed it over.
    updates: Option<Updates>,
    status: Option<SharedString>,
}

impl EventEmitter<Change> for SettingsView {}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_fields: Vec<_> = LABEL_COLORS
            .iter()
            .map(|_| cx.new(|cx| InputState::new(window, cx)))
            .collect();
        for field in &name_fields {
            cx.subscribe_in(field, window, |this, _, event, window, cx| {
                if commits(event) {
                    this.commit_name_fields(window, cx);
                }
            })
            .detach();
        }
        let pattern_field = cx.new(|cx| InputState::new(window, cx));
        cx.subscribe_in(&pattern_field, window, |this, _, event, _, cx| {
            if commits(event) {
                this.commit_pattern_field(cx);
            } else if matches!(event, InputEvent::Change) {
                this.show_example(cx);
            }
        })
        .detach();
        let names: Vec<SharedString> = i18n::presets()
            .iter()
            .map(|preset| preset.name.clone().into())
            .collect();
        let languages = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(names),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        cx.subscribe_in(
            &languages,
            window,
            |this, _, event: &SelectEvent<SearchableVec<SharedString>>, window, cx| {
                if let SelectEvent::Confirm(Some(name)) = event
                    && let Some(preset) = i18n::presets()
                        .iter()
                        .find(|preset| preset.name == name.as_ref())
                {
                    this.commit_names(preset.names.clone(), window, cx);
                }
            },
        )
        .detach();
        let root_field = cx.new(|cx| InputState::new(window, cx));
        let light_themes = themes(ThemeMode::Light, window, cx);
        let dark_themes = themes(ThemeMode::Dark, window, cx);
        Self {
            focus: cx.focus_handle(),
            model: SettingsModal::default(),
            auto_advance: true,
            format: SidecarFormat::default(),
            keymap: Keymap::defaults(),
            label_names: LabelNames::default(),
            name_fields,
            languages,
            import_root: String::new(),
            root_field,
            import_pattern: import::DEFAULT_PATTERN.to_owned(),
            pattern_field,
            example: Ok(String::new()),
            theme: theme::Setting::default(),
            mcp_allow_destructive: false,
            light_themes,
            dark_themes,
            index_size: cache::UNKNOWN_SIZE.into(),
            clear_waiting: false,
            clear_running: false,
            updates: None,
            status: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.model.is_open()
    }

    /// Opens on the first page with the box focused, showing `settings`'
    /// values. The component keeps the selected page in its own element
    /// state, dropped while the settings are hidden.
    pub fn open(&mut self, settings: &Stored, window: &mut Window, cx: &mut Context<Self>) {
        if !self.model.open() {
            return;
        }
        self.auto_advance = settings.auto_advance;
        self.format = settings.format;
        self.keymap = settings.keymap.clone();
        self.show_names(settings.names.clone(), window, cx);
        self.import_root = settings.import_root.clone();
        let root = self.import_root.clone();
        show(&self.root_field, &root, window, cx);
        self.import_pattern = settings.import_pattern.clone();
        let pattern = self.import_pattern.clone();
        show(&self.pattern_field, &pattern, window, cx);
        self.show_example(cx);
        self.theme = settings.theme;
        self.mcp_allow_destructive = settings.mcp_allow_destructive;
        for (themes, name) in [
            (&self.light_themes, self.theme.light),
            (&self.dark_themes, self.theme.dark),
        ] {
            themes.update(cx, |state, cx| {
                state.set_selected_value(&SharedString::new_static(name), window, cx)
            });
        }
        self.status = (!settings.writes.is_open()).then(|| settingsmodal::NOT_SAVED.into());
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Shows `names` in the fields and selects the language whose preset
    /// they are, else none. Setting the selection emits no `SelectEvent`, so
    /// only a choice in the menu applies a preset.
    fn show_names(&mut self, names: LabelNames, window: &mut Window, cx: &mut Context<Self>) {
        for (field, text) in self
            .name_fields
            .iter()
            .zip(settingsmodal::label_texts(&names))
        {
            show(field, text, window, cx);
        }
        let preset = settingsmodal::preset_of(&names).map(|at| &i18n::presets()[at]);
        self.languages.update(cx, |state, cx| match preset {
            Some(preset) => {
                state.set_selected_value(&SharedString::from(preset.name.clone()), window, cx)
            }
            None => state.set_selected_index(None, window, cx),
        });
        self.label_names = names;
    }

    /// A name field committed: Enter, the focus left it, or Escape, which
    /// then closes the modal (`key_down`), as the Tauri field's `change`
    /// fires when the closing dialog takes its focus.
    fn commit_name_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let texts: Vec<String> = self
            .name_fields
            .iter()
            .map(|field| field.read(cx).value().to_string())
            .collect();
        let texts = std::array::from_fn(|at| texts[at].as_str());
        self.commit_names(settingsmodal::label_names_of(texts), window, cx);
    }

    /// Shows `names` back in the fields and hands them to the main view when
    /// they differ from those in force.
    fn commit_names(&mut self, names: LabelNames, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.label_names.clone();
        self.show_names(names.clone(), window, cx);
        if let Some(names) = settingsmodal::label_change(&current, names) {
            self.status = None;
            cx.emit(Change::LabelNames(names));
        }
        cx.notify();
    }

    /// The example for the pattern in the field, a local call, so it follows
    /// every edit without the Tauri frontend's debounce.
    fn show_example(&mut self, cx: &mut Context<Self>) {
        let example =
            import::today_example(&self.import_root, &self.pattern_field.read(cx).value());
        if example != self.example {
            self.example = example;
            cx.notify();
        }
    }

    /// The pattern field committed, as a name field does.
    fn commit_pattern_field(&mut self, cx: &mut Context<Self>) {
        let pattern = self.pattern_field.read(cx).value().to_string();
        self.commit_pattern(pattern, cx);
    }

    /// A valid new pattern is handed to the main view to store; an invalid
    /// one shows its reason in the example line and the stored one stays.
    fn commit_pattern(&mut self, pattern: String, cx: &mut Context<Self>) {
        match settingsmodal::pattern_change(&self.import_pattern, &pattern) {
            Ok(Some(pattern)) => {
                self.status = None;
                self.import_pattern = pattern.clone();
                cx.emit(Change::ImportPattern(pattern));
            }
            Ok(None) => {}
            Err(e) => {
                self.status = None;
                self.example = Err(e);
            }
        }
        cx.notify();
    }

    /// An example pattern button: it fills the field and commits.
    fn use_pattern(&mut self, pattern: &str, window: &mut Window, cx: &mut Context<Self>) {
        show(&self.pattern_field, pattern, window, cx);
        self.show_example(cx);
        self.commit_pattern(pattern.to_owned(), cx);
    }

    /// `Choose…`: the OS folder dialog, answered on a later turn. The chosen
    /// root is handed to the main view whether or not the modal is still
    /// open, and the pattern being typed is kept.
    fn choose_root(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status = None;
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => None,
                Ok(Err(e)) => {
                    this.update(cx, |this, cx| this.failed(e.to_string(), cx))
                        .ok();
                    None
                }
            };
            let Some(root) = chosen else { return };
            this.update_in(cx, |this, window, cx| {
                this.import_root = root.to_string_lossy().into_owned();
                show(&this.root_field, &this.import_root, window, cx);
                cx.emit(Change::ImportRoot(this.import_root.clone()));
                this.show_example(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn editing(&self, window: &Window, cx: &gpui_kit::App) -> bool {
        self.name_fields
            .iter()
            .chain([&self.pattern_field])
            .any(|field| field.focus_handle(cx).is_focused(window))
    }

    /// A pointer press on a component control keeps the focus where it was,
    /// so one pressed while a field has it moves it to the box first, and
    /// the field commits on its blur as when the press focused the control.
    fn leave_field(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing(window, cx) {
            window.focus(&self.focus, cx);
        }
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.model.close();
        cx.emit(Change::Closed);
        cx.notify();
    }

    /// Every key stops here, so none reaches the window's keymap, except
    /// that a key typed in a text field goes on to the platform's text input
    /// (the window's own handler returns while the modal is open). The
    /// fields are the view's own and the component's private search field,
    /// so a field is told by the `Input` key context on the focused path.
    fn key_down(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let name = keys::key_name(&e.keystroke);
        let key = self.model.key(name.as_deref());
        let typing = window
            .context_stack()
            .iter()
            .any(|context| context.contains(INPUT_CONTEXT));
        if !settingsmodal::stops(&key, typing) {
            return;
        }
        cx.stop_propagation();
        match key {
            ModalKey::Close => {
                // Escape in a field commits it first: the field's `Escape`
                // action propagates, so the key goes on to here.
                if self.pattern_field.focus_handle(cx).is_focused(window) {
                    self.commit_pattern_field(cx);
                } else if self.editing(window, cx) {
                    self.commit_name_fields(window, cx);
                }
                self.close(cx);
            }
            ModalKey::Add(key) => {
                if let Some(action) = self.model.capturing.take() {
                    self.edit(Edit::Add(action, key), cx);
                }
            }
            ModalKey::Cancel => {
                self.model.capturing = None;
                cx.notify();
            }
            ModalKey::Native => {}
        }
    }

    /// The component's sidebar switches pages in its own state, which the
    /// view cannot observe, so any pointer press inside the box cancels a
    /// capture, before the pressed control's own click runs.
    fn cancel_capture(&mut self, cx: &mut Context<Self>) {
        if self.model.capturing.take().is_some() {
            cx.notify();
        }
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

    fn flip_allow_destructive(&mut self, cx: &mut Context<Self>) {
        self.mcp_allow_destructive = !self.mcp_allow_destructive;
        cx.emit(Change::McpAllowDestructive(self.mcp_allow_destructive));
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

    fn choose_mode(&mut self, chosen: Mode, cx: &mut Context<Self>) {
        let Some(setting) = settingsmodal::mode_change(self.theme, chosen) else {
            return;
        };
        self.theme = setting;
        self.status = None;
        cx.emit(Change::Theme(setting));
        cx.notify();
    }

    fn choose_theme(&mut self, mode: ThemeMode, chosen: &str, cx: &mut Context<Self>) {
        let Some(setting) = settingsmodal::theme_change(self.theme, mode, chosen) else {
            return;
        };
        self.theme = setting;
        self.status = None;
        cx.emit(Change::Theme(setting));
        cx.notify();
    }

    /// `Clear Cache`: the main view holds, asks and runs it.
    fn clear_cache(&mut self, cx: &mut Context<Self>) {
        self.status = None;
        cx.emit(Change::ClearCache);
        cx.notify();
    }

    /// The index's size on disk, formatted.
    pub fn show_index_size(&mut self, size: String, cx: &mut Context<Self>) {
        self.index_size = size.into();
        cx.notify();
    }

    /// Whether a press waits for the running scan and whether a clear runs.
    pub fn show_clear(&mut self, waiting: bool, running: bool, cx: &mut Context<Self>) {
        self.clear_waiting = waiting;
        self.clear_running = running;
        cx.notify();
    }

    /// The update flow's state, on every change of it.
    pub fn show_updates(&mut self, updates: Updates, cx: &mut Context<Self>) {
        self.updates = Some(updates);
        cx.notify();
    }

    /// An Updates page button: the main view runs it.
    fn update_button(&mut self, change: Change, cx: &mut Context<Self>) {
        self.status = None;
        cx.emit(change);
        cx.notify();
    }

    /// A switch failed: its message goes to the status line.
    pub fn failed(&mut self, message: String, cx: &mut Context<Self>) {
        self.status = Some(message.into());
        cx.notify();
    }

    /// The body: gpui-component's `Settings`, one page per tab of `TABS`,
    /// rebuilt on every render. Its closures run outside the view's update,
    /// so they hold the view's entity and reach it through `listener_for`.
    fn body(&self, cx: &mut Context<Self>) -> Settings {
        let view = cx.entity();
        Settings::new("settings").pages(TABS.iter().map(|&tab| {
            let page = SettingPage::new(tab.label());
            match tab {
                Tab::Sidecar => self.sidecar(page, &view),
                Tab::Culling => self.culling(page, &view),
                Tab::Shortcuts => self.shortcuts(page, &view),
                Tab::Cache => self.cache(page, &view),
                Tab::Import => self.import(page, &view),
                Tab::Mcp => self.mcp(page, &view),
                Tab::Appearance => self.appearance(page, &view),
                Tab::Updates => self.updates(page, &view),
            }
        }))
    }

    /// The three format radios and, while the format writes XMP, the label
    /// names: the Tauri paragraph, a name field per color, and the Lightroom
    /// language in a `Select`, whose choice puts its preset's names in force.
    fn sidecar(&self, page: SettingPage, view: &Entity<Self>) -> SettingPage {
        let format = self.format;
        let page = page.group(SettingGroup::new().item(SettingItem::new(
            "Format",
            SettingField::render({
                let view = view.clone();
                move |_, window, _| {
                    div().flex().flex_col().gap_2().children(FORMATS.iter().map(
                        |&(choice, label)| {
                            Radio::new(choice.setting())
                                .small()
                                .checked(choice == format)
                                .label(label)
                                .text_size(px(13.))
                                .on_click(window.listener_for(
                                    &view,
                                    move |this, _: &bool, window, cx| {
                                        this.leave_field(window, cx);
                                        this.choose_format(choice, cx);
                                    },
                                ))
                        },
                    ))
                }
            }),
        )));
        if !settingsmodal::writes_xmp(format) {
            return page;
        }
        let languages = self.languages.clone();
        page.group(
            SettingGroup::new()
                .title("Label names")
                .description(settingsmodal::LABEL_NAMES_NOTE)
                .items(
                    LABEL_COLORS
                        .iter()
                        .zip(&self.name_fields)
                        .map(|(&color, state)| {
                            let state = state.clone();
                            SettingItem::new(
                                color,
                                SettingField::render(move |_, window, cx| {
                                    div().w(px(240.)).child(field(&state, window, cx))
                                }),
                            )
                        }),
                )
                .item(
                    SettingItem::new(
                        "Lightroom language",
                        SettingField::render(move |_, _, _| {
                            div().w(px(160.)).child(
                                Select::new(&languages)
                                    .small()
                                    .accessibility_label("Lightroom language"),
                            )
                        }),
                    )
                    .keywords(["Language", "Lightroom"]),
                ),
        )
    }

    /// The mode radios, drawn as the Sidecar page's format radios, and the
    /// light and the dark theme, each a `Select` like the label names'
    /// language.
    fn appearance(&self, page: SettingPage, view: &Entity<Self>) -> SettingPage {
        let mode = self.theme.mode;
        let view = view.clone();
        let picker = |title: &'static str, themes: &Entity<Themes>| {
            let themes = themes.clone();
            SettingItem::new(
                title,
                SettingField::render(move |_, _, _| {
                    div()
                        .w(px(200.))
                        .child(Select::new(&themes).small().accessibility_label(title))
                }),
            )
        };
        page.group(
            SettingGroup::new()
                .item(SettingItem::new(
                    "Mode",
                    SettingField::render(move |_, window, _| {
                        div().flex().flex_col().gap_2().children(MODES.iter().map(
                            |&(choice, label)| {
                                Radio::new(choice.setting())
                                    .small()
                                    .checked(choice == mode)
                                    .label(label)
                                    .text_size(px(13.))
                                    .on_click(window.listener_for(
                                        &view,
                                        move |this, _: &bool, window, cx| {
                                            this.leave_field(window, cx);
                                            this.choose_mode(choice, cx);
                                        },
                                    ))
                            },
                        ))
                    }),
                ))
                .item(picker("Light theme", &self.light_themes))
                .item(picker("Dark theme", &self.dark_themes)),
        )
    }

    fn culling(&self, page: SettingPage, view: &Entity<Self>) -> SettingPage {
        let checked = self.auto_advance;
        let view = view.clone();
        page.group(SettingGroup::new().item(SettingItem::new(
            "Auto-advance after a star, reject or pick",
            SettingField::switch(
                move |_| checked,
                move |_, cx| view.update(cx, |this, cx| this.flip_auto_advance(cx)),
            ),
        )))
    }

    /// One titled group per `SHORTCUT_GROUPS` entry, each holding one item per
    /// action of that group in `shortcut_rows`' order, titled with its label so the
    /// search finds it: a `Kbd` per key with its remove button, `+` or the
    /// capture prompt, and `Reset`; then `Reset all` in a final untitled group.
    fn shortcuts(&self, page: SettingPage, view: &Entity<Self>) -> SettingPage {
        let bindings = self.keymap.bindings();
        let row = |b: &Binding| {
            let action = b.action;
            let keys = b.keys.clone();
            let capturing = self.model.capturing == Some(action);
            let view = view.clone();
            SettingItem::new(
                settingsmodal::shortcut_label(action),
                SettingField::render(move |_, window, cx| {
                    let muted = cx.theme().muted_foreground;
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_end()
                        .gap(px(4.))
                        .children(keys.iter().enumerate().map(|(k, key)| {
                            let chip = Kbd::new(keys::keystroke_of(key))
                                .text_size(px(13.))
                                .text_color(cx.theme().foreground);
                            let key = key.clone();
                            div().flex().items_center().gap(px(3.)).child(chip).child(
                                small_button(("remove", k), "\u{d7}").on_click(
                                    window.listener_for(
                                        &view,
                                        move |this, _: &ClickEvent, window, cx| {
                                            this.edit(Edit::Remove(action, key.clone()), cx);
                                            // The pressed button is gone with its chip.
                                            window.focus(&this.focus, cx);
                                        },
                                    ),
                                ),
                            )
                        }))
                        .child(if capturing {
                            div()
                                .text_size(px(13.))
                                .text_color(muted)
                                .child("Press a key...")
                                .into_any_element()
                        } else {
                            small_button("add", "+")
                                .on_click(window.listener_for(
                                    &view,
                                    move |this, _: &ClickEvent, window, cx| {
                                        this.capture(action, window, cx);
                                    },
                                ))
                                .into_any_element()
                        })
                        .child(small_button("reset", "Reset").on_click(window.listener_for(
                            &view,
                            move |this, _: &ClickEvent, _, cx| {
                                this.edit(Edit::Reset(action), cx);
                            },
                        )))
                }),
            )
        };
        let page = settingsmodal::SHORTCUT_GROUPS
            .into_iter()
            .fold(page, |page, group| {
                page.group(
                    SettingGroup::new().title(group.title()).items(
                        settingsmodal::shortcut_rows(&bindings, group)
                            .into_iter()
                            .map(&row),
                    ),
                )
            });
        let view = view.clone();
        page.group(
            SettingGroup::new().item(
                SettingItem::render(move |_, window, _| {
                    div().flex().justify_end().child(
                        small_button("shortcuts-reset-all", "Reset all").on_click(
                            window.listener_for(&view, |this, _: &ClickEvent, _, cx| {
                                this.edit(Edit::ResetAll, cx)
                            }),
                        ),
                    )
                })
                .keywords(["Reset all"]),
            ),
        )
    }

    /// The switch that lets MCP clients run the destructive operations
    /// without the app's dialog, as `culling`'s.
    fn mcp(&self, page: SettingPage, view: &Entity<Self>) -> SettingPage {
        let checked = self.mcp_allow_destructive;
        let view = view.clone();
        page.group(
            SettingGroup::new().item(
                SettingItem::new(
                    settingsmodal::ALLOW_DESTRUCTIVE_LABEL,
                    SettingField::switch(
                        move |_| checked,
                        move |_, cx| view.update(cx, |this, cx| this.flip_allow_destructive(cx)),
                    ),
                )
                .description(settingsmodal::ALLOW_DESTRUCTIVE_NOTE),
            ),
        )
    }

    /// The size line with `Clear Cache`, the Tauri paragraph as the page's
    /// description and, while a press waits for the scan, its note.
    fn cache(&self, page: SettingPage, view: &Entity<Self>) -> SettingPage {
        let running = self.clear_running;
        let view = view.clone();
        let mut item = SettingItem::new(
            cache::size_line(&self.index_size),
            SettingField::render(move |_, window, _| {
                small_button("clear-cache", cache::CLEAR_LABEL)
                    .disabled(running)
                    .on_click(
                        window.listener_for(&view, |this, _: &ClickEvent, window, cx| {
                            this.leave_field(window, cx);
                            this.clear_cache(cx);
                        }),
                    )
            }),
        )
        .keywords(["Clear Cache", "Index"]);
        if self.clear_waiting {
            item = item.description(cache::WAITING_NOTE);
        }
        page.description(cache::NOTE)
            .group(SettingGroup::new().item(item))
    }

    /// The running version and its install kind (and a test build's note),
    /// then the phase with `Check for Updates` and, by phase and install
    /// kind, `Restart Now` or `Open Release Page`.
    fn updates(&self, page: SettingPage, view: &Entity<Self>) -> SettingPage {
        let Some(updates) = self.updates.clone() else {
            return page;
        };
        let mut version = SettingItem::new(
            "Version",
            SettingField::render({
                let line = updates.version_line();
                move |_, _, _| div().text_size(px(13.)).child(line.clone())
            }),
        );
        if let Some(note) = updates.test_note() {
            version = version.description(note);
        }
        let can_check = updates.can_check();
        let restart = updates.shows_restart();
        let release_page = updates.release_page().map(str::to_owned);
        let view = view.clone();
        let status = SettingItem::new(
            updates.status_text(),
            SettingField::render(move |_, window, _| {
                let button = |id: &'static str, label: &'static str, change: Change| {
                    small_button(id, label).on_click(window.listener_for(
                        &view,
                        move |this, _: &ClickEvent, window, cx| {
                            this.leave_field(window, cx);
                            this.update_button(change.clone(), cx);
                        },
                    ))
                };
                div()
                    .flex()
                    .gap(px(4.))
                    .children(restart.then(|| {
                        button("update-restart", update::RESTART_LABEL, Change::RestartNow)
                    }))
                    .children(release_page.clone().map(|page| {
                        button(
                            "update-release-page",
                            update::RELEASE_PAGE_LABEL,
                            Change::OpenReleasePage(page),
                        )
                    }))
                    .child(
                        button("update-check", update::CHECK_LABEL, Change::CheckForUpdates)
                            .disabled(!can_check),
                    )
            }),
        )
        .keywords([update::CHECK_LABEL, update::RESTART_LABEL, "Version"]);
        page.group(SettingGroup::new().item(version).item(status))
    }

    /// The Tauri paragraph as the page's description, the read-only root
    /// field with `Choose…` in one `InputGroup` and its note, the pattern field, the example line, the
    /// example patterns and the specifier table.
    fn import(&self, page: SettingPage, view: &Entity<Self>) -> SettingPage {
        let root_field = self.root_field.clone();
        let mut root_item = SettingItem::new(
            "Destination root",
            SettingField::render({
                let view = view.clone();
                move |_, window, _| {
                    InputGroup::new("import-root")
                        .readonly(true)
                        .w(px(360.))
                        .bg(transparent_black())
                        .rounded_sm()
                        .text_size(px(13.))
                        .input(Input::new(&root_field))
                        .addon(
                            InputGroupAddon::new("import-root-addon")
                                .align(InputGroupAddonAlignment::InlineEnd)
                                .child(
                                    InputGroupButton::new("import-root-choose")
                                        .label("Choose\u{2026}")
                                        .on_click(window.listener_for(
                                            &view,
                                            |this, _: &ClickEvent, window, cx| {
                                                this.leave_field(window, cx);
                                                this.choose_root(window, cx);
                                            },
                                        )),
                                ),
                        )
                }
            }),
        );
        if let Some(note) = settingsmodal::root_note(&self.import_root) {
            root_item = root_item.description(note);
        }
        let pattern_field = self.pattern_field.clone();
        let (example, refused) = match &self.example {
            Ok(example) => (example.clone(), false),
            Err(reason) => (reason.clone(), true),
        };
        let view = view.clone();
        page.description(settingsmodal::IMPORT_NOTE).group(
            SettingGroup::new()
                .item(root_item)
                .item(SettingItem::new(
                    "Folder pattern",
                    SettingField::render(move |_, window, cx| {
                        div().w(px(240.)).child(field(&pattern_field, window, cx))
                    }),
                ))
                .item(
                    SettingItem::render(move |_, _, cx| {
                        let danger = cx.theme().danger;
                        div()
                            .text_size(px(13.))
                            .when(refused, |d| d.text_color(danger))
                            .child(example.clone())
                    })
                    .keywords(["Example"]),
                )
                .item(SettingItem::new(
                    "Example patterns",
                    SettingField::render(move |_, window, _| {
                        div().flex().flex_wrap().justify_end().gap(px(4.)).children(
                            settingsmodal::IMPORT_PATTERN_EXAMPLES
                                .iter()
                                .enumerate()
                                .map(|(at, &pattern)| {
                                    small_button(("import-pattern", at), pattern).on_click(
                                        window.listener_for(
                                            &view,
                                            move |this, _: &ClickEvent, window, cx| {
                                                this.leave_field(window, cx);
                                                this.use_pattern(pattern, window, cx);
                                            },
                                        ),
                                    )
                                }),
                        )
                    }),
                ))
                .item(
                    SettingItem::render(|_, _, cx| {
                        let muted = cx.theme().muted_foreground;
                        div().flex().flex_col().text_size(px(13.)).children(
                            settingsmodal::IMPORT_SPECIFIERS.iter().map(
                                |&(spec, meaning, example)| {
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(div().w(px(48.)).flex_none().child(spec))
                                        .child(div().w(px(180.)).flex_none().child(meaning))
                                        .child(div().text_color(muted).child(example))
                                },
                            ),
                        )
                    })
                    .keywords(
                        settingsmodal::IMPORT_SPECIFIERS
                            .iter()
                            .flat_map(|&(spec, meaning, _)| [spec, meaning]),
                    ),
                ),
        )
    }

    /// The box, filling the window, opaque and occluding the main view under
    /// it, with Tab and Shift+Tab kept inside it by its focus trap.
    fn layer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("settings-box")
            .when(self.model.capturing.is_some(), |d| {
                d.key_context(CAPTURE_CONTEXT)
            })
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.key_down(e, window, cx);
            }))
            .capture_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.cancel_capture(cx);
            }))
            .occlude()
            .flex()
            .flex_col()
            .gap_2()
            .size_full()
            .px_4()
            .py_3()
            .bg(cx.theme().popover)
            .text_color(cx.theme().foreground)
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
                        Button::new("settings-close")
                            .ghost()
                            .small()
                            .px(px(6.4))
                            .rounded_sm()
                            .child(div().text_size(px(19.)).child("\u{d7}"))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close(cx))),
                    ),
            )
            // `Settings` fills its parent, which needs a definite size.
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(div().absolute().inset_0().child(self.body(cx))),
            )
            .children(self.status.clone().map(|status| {
                div()
                    .text_size(px(13.))
                    .text_color(cx.theme().danger)
                    .child(status)
            }))
            .focus_trap("settings-trap", &self.focus)
    }
}

/// A picker over `mode`'s themes, Neutral chosen until `open` shows the
/// stored one; a choice is handed to `choose_theme`.
fn themes(mode: ThemeMode, window: &mut Window, cx: &mut Context<SettingsView>) -> Entity<Themes> {
    let names: Vec<_> = theme::names(mode).map(SharedString::new_static).collect();
    let themes = cx.new(|cx| {
        SelectState::new(
            SearchableVec::new(names),
            Some(IndexPath::new(0)),
            window,
            cx,
        )
    });
    cx.subscribe_in(
        &themes,
        window,
        move |this, _, event: &SelectEvent<SearchableVec<SharedString>>, _, cx| {
            if let SelectEvent::Confirm(Some(name)) = event {
                this.choose_theme(mode, name, cx);
            }
        },
    )
    .detach();
    themes
}

/// Enter or the focus leaving a field commits it.
fn commits(event: &InputEvent) -> bool {
    matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur)
}

/// Shows `text` in `field`, left alone when it already does, so a commit
/// that changes nothing keeps the caret and the undo history.
fn show(field: &Entity<InputState>, text: &str, window: &mut Window, cx: &mut gpui_kit::App) {
    if field.read(cx).value().as_ref() != text {
        field.update(cx, |field, cx| field.set_value(text, window, cx));
    }
}

/// A field drawn as the port it replaced: the theme's `input` border, in
/// `primary` while focused, no fill, and its size and radius.
fn field(state: &Entity<InputState>, window: &Window, cx: &gpui_kit::App) -> impl IntoElement {
    let focused = state.focus_handle(cx).is_focused(window);
    let input = Input::new(state)
        .focus_bordered(false)
        .bg(transparent_black())
        .border_color(if focused {
            cx.theme().primary
        } else {
            cx.theme().input
        })
        .rounded_sm()
        .px(px(8.))
        .py(px(3.))
        .text_size(px(13.));
    // `Input::h` is a multi-line height; the frame's own is `Styled`'s.
    Styled::h(input, FIELD_HEIGHT)
}

/// A small outlined button: `Button`'s default variant on the theme's
/// `button` fill and `input` border, at the size and radius of the buttons it
/// replaced, with their 13 px label. `id` is unique within its row, whose own
/// id scopes it.
fn small_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button::new(id)
        .small()
        .px(px(6.))
        .rounded_sm()
        .child(div().text_size(px(13.)).child(label.into()))
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.model.is_open() {
            return Empty.into_any_element();
        }
        self.layer(cx).into_any_element()
    }
}
