//! A single-line text input entity, ported from GPUI's
//! `crates/gpui/examples/input.rs` at the pinned Zed rev `b5cd310b`
//! (Copyright Zed Industries, Inc., licensed under the Apache License,
//! Version 2.0; see `crates/gpui/LICENSE-APACHE` at that rev). Changed from
//! the example: the editing keys are taken in the input's own `on_key_down`
//! (`ctrl+` on Windows and Linux, `cmd+` on macOS) instead of `actions!` and
//! `cx.bind_keys`, so nothing enters GPUI's global keymap; the text and
//! selection state is a plain `Text` with unit tests; no placeholder and no
//! character palette; Enter and a lost focus emit `Event::Commit` and Escape
//! `Event::Cancel` for the owner, which reads the text with `text`.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, KeyDownEvent,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ShapedLine, Style, TextRun, UTF16Selection, UnderlineStyle, Window, div, fill, point,
    prelude::*, px, relative, rgb, rgba, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{FOREGROUND_COLOR, INPUT_BORDER, PRIMARY_COLOR, keys};

/// What the owner hears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Enter, or the focus left the input.
    Commit,
    /// Escape.
    Cancel,
}

/// The modifier of the clipboard and select-all keys, in `keys::key_name`'s
/// terms.
const SECONDARY: &str = if cfg!(target_os = "macos") {
    "meta+"
} else {
    "ctrl+"
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    Backspace,
    Delete,
    Left,
    Right,
    SelectLeft,
    SelectRight,
    SelectAll,
    Home,
    End,
    Copy,
    Cut,
    Paste,
    Commit,
    Cancel,
}

/// The editing key `name` (`keys::key_name`'s) is, if any. Any other key is
/// left to the platform's text input, which types its character.
fn key_for(name: &str) -> Option<Key> {
    Some(match name {
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "arrowleft" => Key::Left,
        "arrowright" => Key::Right,
        "shift+arrowleft" => Key::SelectLeft,
        "shift+arrowright" => Key::SelectRight,
        "home" => Key::Home,
        "end" => Key::End,
        "enter" => Key::Commit,
        "escape" => Key::Cancel,
        _ => match name.strip_prefix(SECONDARY)? {
            "a" => Key::SelectAll,
            "c" => Key::Copy,
            "x" => Key::Cut,
            "v" => Key::Paste,
            _ => return None,
        },
    })
}

/// The text, its selection and the IME's marked range, in UTF-8 offsets;
/// the platform speaks UTF-16, converted at the edges.
#[derive(Default)]
struct Text {
    content: String,
    selected: Range<usize>,
    reversed: bool,
    marked: Option<Range<usize>>,
}

impl Text {
    fn cursor(&self) -> usize {
        if self.reversed {
            self.selected.start
        } else {
            self.selected.end
        }
    }

    fn move_to(&mut self, offset: usize) {
        self.selected = offset..offset;
        self.reversed = false;
    }

    fn select_to(&mut self, offset: usize) {
        if self.reversed {
            self.selected.start = offset;
        } else {
            self.selected.end = offset;
        }
        if self.selected.end < self.selected.start {
            self.reversed = !self.reversed;
            self.selected = self.selected.end..self.selected.start;
        }
    }

    fn select_all(&mut self) {
        self.move_to(0);
        self.select_to(self.content.len());
    }

    /// Replaces the whole text, the caret at its end.
    fn set(&mut self, text: &str) {
        self.content = text.to_owned();
        self.marked = None;
        self.move_to(self.content.len());
    }

    fn left(&mut self) {
        if self.selected.is_empty() {
            self.move_to(self.previous_boundary(self.cursor()));
        } else {
            self.move_to(self.selected.start);
        }
    }

    fn right(&mut self) {
        if self.selected.is_empty() {
            self.move_to(self.next_boundary(self.selected.end));
        } else {
            self.move_to(self.selected.end);
        }
    }

    /// Deletes the selection, else the grapheme before the caret; false at
    /// the start with nothing selected.
    fn backspace(&mut self) -> bool {
        if self.selected.is_empty() {
            let previous = self.previous_boundary(self.cursor());
            if self.cursor() == previous {
                return false;
            }
            self.select_to(previous);
        }
        self.replace(None, "");
        true
    }

    /// Deletes the selection, else the grapheme after the caret; false at
    /// the end with nothing selected.
    fn delete(&mut self) -> bool {
        if self.selected.is_empty() {
            let next = self.next_boundary(self.cursor());
            if self.cursor() == next {
                return false;
            }
            self.select_to(next);
        }
        self.replace(None, "");
        true
    }

    fn selected_text(&self) -> Option<&str> {
        (!self.selected.is_empty()).then(|| &self.content[self.selected.clone()])
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn set_selected_utf16(&mut self, range_utf16: &Range<usize>) {
        self.selected = self.range_from_utf16(range_utf16);
        self.reversed = false;
    }

    /// The range an edit replaces: the given one, else the marked text, else
    /// the selection.
    fn target(&self, range_utf16: Option<&Range<usize>>) -> Range<usize> {
        range_utf16
            .map(|range| self.range_from_utf16(range))
            .or(self.marked.clone())
            .unwrap_or(self.selected.clone())
    }

    fn replace(&mut self, range_utf16: Option<&Range<usize>>, new_text: &str) {
        let range = self.target(range_utf16);
        self.content.replace_range(range.clone(), new_text);
        self.move_to(range.start + new_text.len());
        self.marked = None;
    }

    fn replace_and_mark(
        &mut self,
        range_utf16: Option<&Range<usize>>,
        new_text: &str,
        new_selected_utf16: Option<&Range<usize>>,
    ) {
        let range = self.target(range_utf16);
        self.content.replace_range(range.clone(), new_text);
        self.marked = (!new_text.is_empty()).then(|| range.start..range.start + new_text.len());
        let start_utf16 = self.offset_to_utf16(range.start);
        let inserted_utf16 = new_text.encode_utf16().count();
        self.selected = new_selected_utf16
            .map(|selected| {
                self.range_from_utf16(
                    &(start_utf16 + selected.start.min(inserted_utf16)
                        ..start_utf16 + selected.end.min(inserted_utf16)),
                )
            })
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.reversed = false;
    }
}

/// The one rectangle a range of a single line covers, from its two ends'
/// x positions in the line.
fn span(bounds: Bounds<Pixels>, start_x: Pixels, end_x: Pixels) -> Bounds<Pixels> {
    Bounds::from_corners(
        point(bounds.left() + start_x, bounds.top()),
        point(bounds.left() + end_x, bounds.bottom()),
    )
}

pub struct TextInput {
    focus: FocusHandle,
    text: Text,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    selecting: bool,
}

impl EventEmitter<Event> for TextInput {}

impl TextInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        cx.on_blur(&focus, window, |_, _, cx| cx.emit(Event::Commit))
            .detach();
        Self {
            focus,
            text: Text::default(),
            last_layout: None,
            last_bounds: None,
            selecting: false,
        }
    }

    pub fn text(&self) -> &str {
        &self.text.content
    }

    /// Replaces the text, the caret at its end.
    pub fn set_text(&mut self, text: &str, window: &Window, cx: &mut Context<Self>) {
        if self.text.content == text && self.text.marked.is_none() {
            return;
        }
        self.text.set(text);
        self.changed(window, cx);
    }

    /// Selects the whole text.
    pub fn select_all(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.text.select_all();
        self.selection_changed(window, cx);
    }

    fn changed(&mut self, window: &Window, cx: &mut Context<Self>) {
        window.notify_text_input_changed(&self.focus);
        cx.notify();
    }

    fn selection_changed(&mut self, window: &Window, cx: &mut Context<Self>) {
        window.notify_text_selection_changed(&self.focus);
        cx.notify();
    }

    /// An editing key is taken here and stops; any other key goes on to the
    /// platform's text input, which types its character.
    fn key_down(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = keys::key_name(&e.keystroke).as_deref().and_then(key_for) else {
            return;
        };
        cx.stop_propagation();
        let text = &mut self.text;
        match key {
            Key::Backspace | Key::Delete => {
                let deleted = if key == Key::Backspace {
                    text.backspace()
                } else {
                    text.delete()
                };
                if deleted {
                    self.changed(window, cx);
                } else {
                    window.play_system_bell();
                }
                return;
            }
            Key::Left => text.left(),
            Key::Right => text.right(),
            Key::SelectLeft => text.select_to(text.previous_boundary(text.cursor())),
            Key::SelectRight => text.select_to(text.next_boundary(text.cursor())),
            Key::SelectAll => text.select_all(),
            Key::Home => text.move_to(0),
            Key::End => text.move_to(text.content.len()),
            Key::Copy | Key::Cut => {
                if let Some(selected) = text.selected_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(selected.to_owned()));
                    if key == Key::Cut {
                        text.replace(None, "");
                        self.changed(window, cx);
                    }
                }
                return;
            }
            Key::Paste => {
                if let Some(pasted) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    text.replace(None, &pasted.replace('\n', " "));
                    self.changed(window, cx);
                }
                return;
            }
            Key::Commit | Key::Cancel => {
                cx.emit(if key == Key::Commit {
                    Event::Commit
                } else {
                    Event::Cancel
                });
                return;
            }
        }
        self.selection_changed(window, cx);
    }

    fn mouse_down(&mut self, e: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.selecting = true;
        let at = self.index_for_mouse_position(e.position);
        if e.modifiers.shift {
            self.text.select_to(at);
        } else {
            self.text.move_to(at);
        }
        self.selection_changed(window, cx);
    }

    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn mouse_move(&mut self, e: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            self.text
                .select_to(self.index_for_mouse_position(e.position));
            self.selection_changed(window, cx);
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.text.content.is_empty() {
            return 0;
        }
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.text.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.text.range_from_utf16(&range_utf16);
        actual_range.replace(self.text.range_to_utf16(&range));
        Some(self.text.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.text.range_to_utf16(&self.text.selected),
            reversed: self.text.reversed,
        })
    }

    fn set_selected_text_range(
        &mut self,
        range_utf16: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.text.set_selected_utf16(&range_utf16);
        self.selection_changed(window, cx);
    }

    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.text.content.encode_utf16().count())
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.text
            .marked
            .as_ref()
            .map(|range| self.text.range_to_utf16(range))
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.text.marked = None;
        self.changed(window, cx);
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.text.replace(range_utf16.as_ref(), new_text);
        self.changed(window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.text.replace_and_mark(
            range_utf16.as_ref(),
            new_text,
            new_selected_range_utf16.as_ref(),
        );
        self.changed(window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.last_layout.as_ref()?;
        if line.text.as_ref() != self.text.content {
            return None;
        }
        let range = self.text.range_from_utf16(&range_utf16);
        Some(span(
            bounds,
            line.x_for_index(range.start),
            line.x_for_index(range.end),
        ))
    }

    fn selection_bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Bounds<Pixels>> {
        self.bounds_for_range(range_utf16, bounds, window, cx)
            .into_iter()
            .collect()
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let local = self.last_bounds?.localize(&point)?;
        let line = self.last_layout.as_ref()?;
        if line.text.as_ref() != self.text.content {
            return None;
        }
        Some(self.text.offset_to_utf16(line.closest_index_for_x(local.x)))
    }
}

/// The line, the caret and the selection, painted over the input's bounds.
struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let text = &self.input.read(cx).text;
        let style = window.text_style();
        let run = TextRun {
            len: text.content.len(),
            font: style.font(),
            color: style.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked) = text.marked.as_ref() {
            vec![
                TextRun {
                    len: marked.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked.end - marked.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: text.content.len() - marked.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line =
            window
                .text_system()
                .shape_line(text.content.clone().into(), font_size, &runs, None);
        let (selection, cursor) = if text.selected.is_empty() {
            let x = line.x_for_index(text.cursor());
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + x, bounds.top()),
                        size(px(1.), bounds.bottom() - bounds.top()),
                    ),
                    rgb(FOREGROUND_COLOR),
                )),
            )
        } else {
            (
                Some(fill(
                    span(
                        bounds,
                        line.x_for_index(text.selected.start),
                        line.x_for_index(text.selected.end),
                    ),
                    rgba((PRIMARY_COLOR << 8) | 0x55),
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        let Some(line) = prepaint.line.take() else {
            return;
        };
        if let Err(e) = line.paint(
            bounds.origin,
            window.line_height(),
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        ) {
            log::warn!("text input: {e}");
        }
        if focus.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }
        self.input.update(cx, |input, _| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .w_full()
            .overflow_hidden()
            .track_focus(&self.focus)
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .px(px(8.))
            .py(px(3.))
            .rounded_sm()
            .border_1()
            .border_color(rgba(INPUT_BORDER))
            .focus(|s| s.border_color(rgb(PRIMARY_COLOR)))
            .child(TextElement { input: cx.entity() })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::{Key, SECONDARY, Text, key_for, span};

    fn text(content: &str) -> Text {
        Text {
            content: content.to_owned(),
            ..Text::default()
        }
    }

    // The example's `native_selection_and_length_use_utf16`.
    #[test]
    fn native_selection_and_length_use_utf16() {
        let mut t = text("e🙂漢");
        assert_eq!(t.content.encode_utf16().count(), 4);
        t.set_selected_utf16(&(1..3));
        assert_eq!(t.selected, 1..5);
        assert_eq!(t.range_to_utf16(&t.selected), 1..3);
        t.set_selected_utf16(&(3..99));
        assert_eq!(t.selected, 5..8);
    }

    // The example's `marked_selection_is_relative_to_inserted_text`.
    #[test]
    fn marked_selection_is_relative_to_inserted_text() {
        let mut t = text("A🙂Z");
        t.replace_and_mark(Some(&(1..3)), "é🙂", Some(&(1..3)));
        assert_eq!(t.content, "Aé🙂Z");
        assert_eq!(t.marked, Some(1..7));
        assert_eq!(t.selected, 3..7);

        t.replace_and_mark(None, "漢", Some(&(1..1)));
        assert_eq!(t.content, "A漢Z");
        assert_eq!(t.marked, Some(1..4));
        assert_eq!(t.selected, 4..4);

        t.replace_and_mark(None, "", Some(&(0..0)));
        assert_eq!(t.content, "AZ");
        assert_eq!(t.marked, None);
        assert_eq!(t.selected, 1..1);
    }

    // The example's `long_single_line_selection_uses_one_geometry_fragment`,
    // on the rectangle a range covers once its ends are placed in the line
    // (the shaped line needs GPUI's `test-support`, which this crate does
    // not enable).
    #[test]
    fn long_single_line_selection_uses_one_geometry_fragment() {
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(200.), px(18.)));
        let rect = span(bounds, px(0.), px(70_000.));
        assert!(rect.size.width > px(0.));
        assert_eq!(rect.origin, point(px(10.), px(20.)));
        assert_eq!(rect.size.height, px(18.));
    }

    #[test]
    fn committing_the_ime_s_text_replaces_the_marked_range() {
        let mut t = text("ab");
        t.move_to(1);
        t.replace_and_mark(None, "かな", None);
        assert_eq!(t.marked, Some(1..7));
        t.replace(None, "仮名");
        assert_eq!(t.content, "a仮名b");
        assert_eq!(t.marked, None);
        assert_eq!(t.selected, 7..7);
    }

    #[test]
    fn the_caret_moves_by_grapheme_over_non_ascii_text() {
        // A combining accent, a ZWJ family, a flag and two CJK characters.
        let content = "e\u{301}👨‍👩‍👧🇯🇵漢字";
        let mut t = text(content);
        t.move_to(content.len());
        let mut stops = vec![t.cursor()];
        while t.cursor() > 0 {
            t.left();
            stops.push(t.cursor());
        }
        let family = "👨‍👩‍👧".len();
        let flag = "🇯🇵".len();
        assert_eq!(
            stops,
            [
                content.len(),
                3 + family + flag + 3,
                3 + family + flag,
                3 + family,
                3,
                0
            ]
        );
        t.right();
        assert_eq!(t.cursor(), 3);
        t.right();
        assert_eq!(t.cursor(), 3 + family);
    }

    #[test]
    fn backspace_and_delete_remove_a_whole_grapheme() {
        let mut t = text("a👨‍👩‍👧b");
        t.move_to(1 + "👨‍👩‍👧".len());
        assert!(t.backspace());
        assert_eq!(t.content, "ab");
        assert!(t.delete());
        assert_eq!(t.content, "a");
        assert!(!t.delete());
        t.move_to(0);
        assert!(!t.backspace());
    }

    #[test]
    fn shift_arrows_extend_and_reverse_the_selection() {
        let mut t = text("漢字");
        t.move_to(3);
        t.select_to(t.next_boundary(t.cursor()));
        assert_eq!(t.selected, 3..6);
        t.select_to(t.previous_boundary(t.cursor()));
        t.select_to(t.previous_boundary(t.cursor()));
        assert_eq!(t.selected, 0..3);
        assert!(t.reversed);
        assert_eq!(t.selected_text(), Some("漢"));
    }

    #[test]
    fn typing_over_a_selection_replaces_it() {
        let mut t = text("Red");
        t.select_all();
        assert_eq!(t.selected_text(), Some("Red"));
        t.replace(None, "Rot");
        assert_eq!(t.content, "Rot");
        assert_eq!(t.selected, 3..3);
    }

    #[test]
    fn setting_the_text_drops_the_marked_range_and_puts_the_caret_at_the_end() {
        let mut t = text("a");
        t.replace_and_mark(None, "か", None);
        t.set("レッド");
        assert_eq!(t.marked, None);
        assert_eq!(t.selected, 9..9);
    }

    #[test]
    fn the_clipboard_keys_take_the_platform_s_modifier() {
        let secondary = |key: &str| key_for(&format!("{SECONDARY}{key}"));
        assert_eq!(secondary("a"), Some(Key::SelectAll));
        assert_eq!(secondary("c"), Some(Key::Copy));
        assert_eq!(secondary("x"), Some(Key::Cut));
        assert_eq!(secondary("v"), Some(Key::Paste));
        let other = if SECONDARY == "ctrl+" {
            "meta+"
        } else {
            "ctrl+"
        };
        assert_eq!(key_for(&format!("{other}a")), None);
        if cfg!(target_os = "macos") {
            assert_eq!(SECONDARY, "meta+");
        } else {
            assert_eq!(SECONDARY, "ctrl+");
        }
    }

    #[test]
    fn enter_commits_escape_cancels_and_a_typed_key_is_left_to_the_platform() {
        assert_eq!(key_for("enter"), Some(Key::Commit));
        assert_eq!(key_for("escape"), Some(Key::Cancel));
        assert_eq!(key_for("shift+arrowleft"), Some(Key::SelectLeft));
        for typed in ["a", "shift+a", "space", "1", "tab", "shift+tab", "arrowup"] {
            assert_eq!(key_for(typed), None, "{typed}");
        }
    }
}
