//! Multi-field input form with a stable two-column layout.
//!
//! Two entry points:
//!
//! 1. **High-level**: build a [`FormState`] from a list of [`FormField`],
//!    route keys through [`FormState::handle_key`], and render with
//!    [`render_form`]. Suitable for self-contained settings/wizard-style
//!    screens.
//!
//! 2. **Low-level primitives**: [`label_prefix`], [`input_row`],
//!    [`select_row`], [`error_lines`]. Use these when you want the same
//!    visual style but manage focus/buffer state yourself (e.g. ygg's
//!    blueprint detail panel, which stores its form values inside a
//!    snapshot-backed buffer).
//!
//! ## Visual language
//!
//! - `  label   value`                       ← row, inactive
//! - `  label   [ lhs│rhs ]`                 ← row, active, software caret
//! - `  label   ◀ option ▶`                  ← row, active, enum / bool cycle
//! - `  label * value`                       ← row with validation error (red `*`)
//! - `        └ message wrapped over…`      ← per-row error, red italic
//! - `          …more text`                  ← continuation, aligned under message
//!
//! The `│` caret is drawn in-band (software cursor). Hide the terminal's
//! hardware cursor (`Frame::set_cursor_position` omitted) so only one caret
//! is visible.

use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::Theme;

// ── Data ──────────────────────────────────────────────────────────────────────

/// The interactive value held by a [`FormField`].
#[derive(Debug, Clone, PartialEq)]
pub enum FieldInput {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    /// Inline selector cycling through `options`; `selected` is the current index.
    Enum { options: Vec<String>, selected: usize },
    /// Display-only ordered list (editing deferred).
    List(Vec<String>),
    /// Non-editable display value (e.g. a derived label shown as a row).
    ReadOnly(String),
}

/// One row in a form.
#[derive(Debug, Clone)]
pub struct FormField {
    /// Stable identifier used to key errors and touched state. Should be unique.
    pub id: String,
    /// Label drawn in the left column.
    pub label: String,
    /// Current input value.
    pub input: FieldInput,
    /// If true, the label is suffixed with a subtle marker.
    pub required: bool,
    /// Optional help text (rendered under the row when focused).
    pub description: Option<String>,
    /// When false, the row is skipped by focus navigation and not rendered.
    /// Callers toggle this dynamically for contextual fields (e.g. "default"
    /// hidden when type=Boolean has its own cycle).
    pub visible: bool,
}

impl FormField {
    /// Build a new field with sensible defaults (`required = false`,
    /// `description = None`, `visible = true`).
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        input: FieldInput,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            input,
            required: false,
            description: None,
            visible: true,
        }
    }

    pub fn required(mut self, required: bool) -> Self { self.required = required; self }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into()); self
    }
    pub fn visible(mut self, visible: bool) -> Self { self.visible = visible; self }
}

/// Outcome of routing a key through [`FormState::handle_key`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormEvent {
    None,
    /// Focus moved to a different (visible) row. Callers typically
    /// re-validate and refresh derived visibility.
    FocusMoved,
    /// The value of the focused field changed (insert/delete/cycle/toggle).
    FieldChanged(String),
    /// User requested submit (Enter on the last visible row).
    Submit,
    /// User pressed Esc.
    Cancel,
}

/// Form state: fields + focus + per-field cursor + touched set.
#[derive(Debug, Clone)]
pub struct FormState {
    pub fields: Vec<FormField>,
    pub focused: usize,
    /// Char-index cursor per field (relevant for Text / Integer / Float).
    pub cursors: Vec<usize>,
    /// Field ids that the user has visited and then left. Callers render
    /// validation errors only for ids in this set (error-on-leave pattern).
    pub touched: HashSet<String>,
}

impl FormState {
    /// Build a new state. The cursor for each Text field starts at the end
    /// of its value; other kinds start at 0.
    pub fn new(fields: Vec<FormField>) -> Self {
        let cursors = fields.iter().map(cursor_end_of).collect();
        let focused = fields
            .iter()
            .position(|f| f.visible)
            .unwrap_or(0);
        Self { fields, focused, cursors, touched: HashSet::new() }
    }

    /// Move focus to the next visible field. Records the previously focused
    /// field as touched. Returns true if focus actually changed.
    pub fn focus_next(&mut self) -> bool {
        let from = self.focused;
        let n = self.fields.len();
        if n == 0 { return false; }
        let mut i = from;
        while i + 1 < n {
            i += 1;
            if self.fields[i].visible {
                self.on_focus_change(from, i);
                return true;
            }
        }
        false
    }

    /// Move focus to the previous visible field.
    pub fn focus_prev(&mut self) -> bool {
        let from = self.focused;
        let mut i = from;
        while i > 0 {
            i -= 1;
            if self.fields[i].visible {
                self.on_focus_change(from, i);
                return true;
            }
        }
        false
    }

    fn on_focus_change(&mut self, from: usize, to: usize) {
        if from < self.fields.len() {
            self.touched.insert(self.fields[from].id.clone());
        }
        self.focused = to;
        // Snap cursor to end of newly-focused value so Backspace / Left work
        // predictably — matches the UX of the ygg bp-detail panel.
        self.cursors[to] = cursor_end_of(&self.fields[to]);
    }

    /// Route a key event. Returns a [`FormEvent`] describing the outcome.
    pub fn handle_key(&mut self, key: KeyEvent) -> FormEvent {
        if self.fields.is_empty() {
            return FormEvent::None;
        }
        match key.code {
            KeyCode::Esc => FormEvent::Cancel,
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.focus_prev() { FormEvent::FocusMoved } else { FormEvent::None }
            }
            KeyCode::BackTab => {
                if self.focus_prev() { FormEvent::FocusMoved } else { FormEvent::None }
            }
            KeyCode::Tab | KeyCode::Down => {
                if self.focus_next() { FormEvent::FocusMoved } else { FormEvent::None }
            }
            KeyCode::Up => {
                if self.focus_prev() { FormEvent::FocusMoved } else { FormEvent::None }
            }
            KeyCode::Enter => {
                // Submit on the last visible row; otherwise just advance.
                if self.is_on_last_visible() {
                    FormEvent::Submit
                } else if self.focus_next() {
                    FormEvent::FocusMoved
                } else {
                    FormEvent::Submit
                }
            }
            _ => self.handle_field_key(key),
        }
    }

    fn is_on_last_visible(&self) -> bool {
        self.fields
            .iter()
            .enumerate()
            .filter(|(_, f)| f.visible)
            .last()
            .map(|(i, _)| i == self.focused)
            .unwrap_or(false)
    }

    fn handle_field_key(&mut self, key: KeyEvent) -> FormEvent {
        let idx = self.focused;
        let id = self.fields[idx].id.clone();
        let mut changed = false;
        match &mut self.fields[idx].input {
            FieldInput::Text(s) => {
                if edit_text(key, s, &mut self.cursors[idx]) { changed = true; }
            }
            FieldInput::Integer(n) => {
                let mut s = n.to_string();
                let mut cur = self.cursors[idx].min(s.chars().count());
                if edit_numeric(key, &mut s, &mut cur, true) {
                    *n = s.parse::<i64>().unwrap_or(*n);
                    self.cursors[idx] = cur;
                    changed = true;
                }
            }
            FieldInput::Float(f) => {
                let mut s = format!("{}", f);
                let mut cur = self.cursors[idx].min(s.chars().count());
                if edit_numeric(key, &mut s, &mut cur, false) {
                    *f = s.parse::<f64>().unwrap_or(*f);
                    self.cursors[idx] = cur;
                    changed = true;
                }
            }
            FieldInput::Boolean(b) => match key.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                    *b = !*b;
                    changed = true;
                }
                _ => {}
            },
            FieldInput::Enum { options, selected } => {
                if options.is_empty() { return FormEvent::None; }
                match key.code {
                    KeyCode::Right | KeyCode::Char('l') => {
                        *selected = (*selected + 1) % options.len();
                        changed = true;
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        *selected = if *selected == 0 { options.len() - 1 } else { *selected - 1 };
                        changed = true;
                    }
                    _ => {}
                }
            }
            FieldInput::List(_) | FieldInput::ReadOnly(_) => {}
        }
        if changed { FormEvent::FieldChanged(id) } else { FormEvent::None }
    }

    /// Mark the focused field as touched without moving focus. Useful when
    /// a caller commits a form (e.g. on Enter-save) and wants every row's
    /// error visible.
    pub fn touch_all(&mut self) {
        for f in &self.fields {
            self.touched.insert(f.id.clone());
        }
    }

    /// Reset touched state — typically called when re-opening a form fresh.
    pub fn untouch_all(&mut self) { self.touched.clear(); }
}

fn cursor_end_of(f: &FormField) -> usize {
    match &f.input {
        FieldInput::Text(s)     => s.chars().count(),
        FieldInput::Integer(n)  => n.to_string().chars().count(),
        FieldInput::Float(v)    => format!("{}", v).chars().count(),
        _ => 0,
    }
}

/// Apply a key to a string buffer with a char-indexed cursor. Returns true
/// if the buffer or cursor changed.
fn edit_text(key: KeyEvent, buf: &mut String, cursor: &mut usize) -> bool {
    let len_chars = buf.chars().count();
    match key.code {
        KeyCode::Char(c) => {
            let byte = char_index_to_byte(buf, *cursor);
            buf.insert(byte, c);
            *cursor += 1;
            true
        }
        KeyCode::Backspace if *cursor > 0 => {
            let from = char_index_to_byte(buf, *cursor - 1);
            let to = char_index_to_byte(buf, *cursor);
            buf.replace_range(from..to, "");
            *cursor -= 1;
            true
        }
        KeyCode::Delete if *cursor < len_chars => {
            let from = char_index_to_byte(buf, *cursor);
            let to = char_index_to_byte(buf, *cursor + 1);
            buf.replace_range(from..to, "");
            true
        }
        KeyCode::Left if *cursor > 0 => { *cursor -= 1; true }
        KeyCode::Right if *cursor < len_chars => { *cursor += 1; true }
        KeyCode::Home if *cursor != 0 => { *cursor = 0; true }
        KeyCode::End if *cursor != len_chars => { *cursor = len_chars; true }
        _ => false,
    }
}

/// Apply a key to a numeric-string buffer. `allow_minus_sign` selects integer
/// vs float semantics (float additionally accepts `.`).
fn edit_numeric(key: KeyEvent, buf: &mut String, cursor: &mut usize, integer: bool) -> bool {
    match key.code {
        KeyCode::Char(c) => {
            let is_digit = c.is_ascii_digit();
            let is_sign = c == '-' && *cursor == 0 && !buf.starts_with('-');
            let is_dot = !integer && c == '.' && !buf.contains('.');
            if is_digit || is_sign || is_dot {
                let byte = char_index_to_byte(buf, *cursor);
                buf.insert(byte, c);
                *cursor += 1;
                true
            } else {
                false
            }
        }
        _ => edit_text(key, buf, cursor),
    }
}

fn char_index_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(s.len())
}

// ── Rendering — low-level primitives ──────────────────────────────────────────

/// Build the label-column spans for a row. Output:
/// `"  <label><*|space><filler>"` — two-space left indent, label, then a red
/// bold `*` if `error` is true (else a space), padded so all values line up
/// at column `indent + 2 + pad + 2`.
pub fn label_prefix(
    label: &str,
    pad: usize,
    error: bool,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let marker = if error {
        Span::styled(
            "*",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(" ")
    };
    let total = pad + 2;
    let filler_width = total.saturating_sub(label.chars().count() + 1);
    vec![
        Span::raw("  "),
        Span::styled(label.to_string(), theme.hint),
        marker,
        Span::raw(" ".repeat(filler_width)),
    ]
}

/// Split a string at a character index, clamping to the bounds.
fn split_at_char(s: &str, char_idx: usize) -> (&str, &str) {
    for (i, (b, _)) in s.char_indices().enumerate() {
        if i == char_idx {
            return (&s[..b], &s[b..]);
        }
    }
    (s, "")
}

/// Render a text-input row.
///
/// When `active` is true, the value is wrapped in brackets. If `caret` is
/// `Some(idx)`, a bold `│` is inserted at character index `idx` (software
/// caret). When `active` is false, the value renders plain.
pub fn input_row(
    label: &str,
    pad: usize,
    value: &str,
    active: bool,
    error: bool,
    caret: Option<usize>,
    theme: &Theme,
) -> Line<'static> {
    let mut spans = label_prefix(label, pad, error, theme);
    if active {
        spans.push(Span::styled("[", theme.shortcut_key));
        match caret {
            Some(pos) => {
                let (lhs, rhs) = split_at_char(value, pos);
                spans.push(Span::styled(lhs.to_string(), theme.body));
                spans.push(Span::styled(
                    "│".to_string(),
                    theme.shortcut_key.add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(rhs.to_string(), theme.body));
            }
            None => spans.push(Span::styled(value.to_string(), theme.body)),
        }
        spans.push(Span::styled("]", theme.shortcut_key));
    } else {
        spans.push(Span::styled(value.to_string(), theme.body));
    }
    Line::from(spans)
}

/// Render a select-style row: when active, the value is framed with
/// `◀ … ▶` cycle glyphs. When inactive, it renders plain.
pub fn select_row(
    label: &str,
    pad: usize,
    value: &str,
    active: bool,
    error: bool,
    theme: &Theme,
) -> Line<'static> {
    let mut spans = label_prefix(label, pad, error, theme);
    if active {
        spans.push(Span::styled("◀ ", theme.shortcut_key));
        spans.push(Span::styled(value.to_string(), theme.body));
        spans.push(Span::styled(" ▶", theme.shortcut_key));
    } else {
        spans.push(Span::styled(value.to_string(), theme.body));
    }
    Line::from(spans)
}

/// Render a per-row inline error message, wrapped to `max_width`. The first
/// line is prefixed with `└ `, continuations with two spaces so the message
/// text stays aligned. Indented by `indent` columns so it sits under the
/// row's label.
pub fn error_lines(msg: &str, indent: usize, max_width: usize) -> Vec<Line<'static>> {
    let style = Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC);
    let prefix_cols = 2;
    let avail = max_width.saturating_sub(indent + prefix_cols).max(1);
    let chunks = wrap_chars(msg, avail);
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let marker = if i == 0 { "└ " } else { "  " };
            Line::from(vec![
                Span::raw(" ".repeat(indent)),
                Span::styled(format!("{}{}", marker, chunk), style),
            ])
        })
        .collect()
}

/// Word-wrap `s` into chunks of at most `width` chars each. Breaks on
/// whitespace; hard-breaks words longer than `width`.
pub fn wrap_chars(s: &str, width: usize) -> Vec<String> {
    if width == 0 { return vec![s.to_string()]; }
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut line_len = 0usize;
    for word in s.split_whitespace() {
        let wlen = word.chars().count();
        if wlen > width {
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
                line_len = 0;
            }
            let mut buf = String::new();
            let mut n = 0;
            for c in word.chars() {
                buf.push(c);
                n += 1;
                if n == width {
                    out.push(std::mem::take(&mut buf));
                    n = 0;
                }
            }
            if !buf.is_empty() { line = buf; line_len = n; }
            continue;
        }
        let sep = if line_len == 0 { 0 } else { 1 };
        if line_len + sep + wlen > width {
            out.push(std::mem::take(&mut line));
            line_len = 0;
        }
        if line_len > 0 {
            line.push(' ');
            line_len += 1;
        }
        line.push_str(word);
        line_len += wlen;
    }
    if !line.is_empty() { out.push(line); }
    if out.is_empty() { out.push(String::new()); }
    out
}

// ── Rendering — high-level form widget ────────────────────────────────────────

/// Styling knobs for [`render_form`].
#[derive(Debug, Clone)]
pub struct FormStyle {
    /// Width reserved for the label column. `0` = auto (max label length).
    pub label_pad: usize,
    /// Render a `│` software caret in the focused Text / Integer / Float field.
    pub show_caret: bool,
    /// When true, the focused field's description is drawn on the line
    /// immediately below it, indented to align under the value column.
    pub show_description: bool,
}

impl Default for FormStyle {
    fn default() -> Self {
        Self { label_pad: 0, show_caret: true, show_description: true }
    }
}

/// Render a form inside `area`, applying the default styling.
///
/// This thin wrapper preserves the original public API; use
/// [`render_form_with`] for error messages and style overrides.
pub fn render_form(f: &mut Frame, area: Rect, state: &FormState, theme: &Theme) {
    render_form_with(f, area, state, &FormStyle::default(), &[], theme);
}

/// Render a form with explicit style options and per-field errors.
///
/// `errors` is a slice of `(field_id, message)` pairs. A given error is
/// rendered only if `field_id` is in `state.touched` — so newly-opened
/// fields do not flash errors before the user has a chance to type.
pub fn render_form_with(
    f: &mut Frame,
    area: Rect,
    state: &FormState,
    style: &FormStyle,
    errors: &[(String, String)],
    theme: &Theme,
) {
    if area.height == 0 { return; }

    let pad = if style.label_pad == 0 {
        state
            .fields
            .iter()
            .filter(|f| f.visible)
            .map(|f| f.label.chars().count())
            .max()
            .unwrap_or(0)
    } else {
        style.label_pad
    };

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (idx, field) in state.fields.iter().enumerate() {
        if !field.visible { continue; }

        let active = idx == state.focused;
        let touched = state.touched.contains(&field.id);
        let has_error = touched
            && errors.iter().any(|(id, _)| id == &field.id);

        let label_with_req = if field.required {
            format!("{}*", field.label)
        } else {
            field.label.clone()
        };

        let caret = if style.show_caret && active {
            Some(state.cursors.get(idx).copied().unwrap_or(0))
        } else {
            None
        };

        let line = match &field.input {
            FieldInput::Text(s) => input_row(&label_with_req, pad, s, active, has_error, caret, theme),
            FieldInput::Integer(n) => {
                let s = n.to_string();
                input_row(&label_with_req, pad, &s, active, has_error, caret, theme)
            }
            FieldInput::Float(v) => {
                let s = format!("{}", v);
                input_row(&label_with_req, pad, &s, active, has_error, caret, theme)
            }
            FieldInput::Boolean(b) => {
                let v = if *b { "yes" } else { "no" };
                select_row(&label_with_req, pad, v, active, has_error, theme)
            }
            FieldInput::Enum { options, selected } => {
                let v = options.get(*selected).map(String::as_str).unwrap_or("");
                select_row(&label_with_req, pad, v, active, has_error, theme)
            }
            FieldInput::ReadOnly(s) => {
                let mut spans = label_prefix(&label_with_req, pad, has_error, theme);
                spans.push(Span::styled(s.clone(), theme.hint));
                Line::from(spans)
            }
            FieldInput::List(items) => {
                let v = if items.is_empty() {
                    "(empty)".to_string()
                } else {
                    items.join(", ")
                };
                let mut spans = label_prefix(&label_with_req, pad, has_error, theme);
                spans.push(Span::styled(v, theme.body));
                Line::from(spans)
            }
        };
        lines.push(line);

        // Inline error lines (wrapped under the label).
        if active && has_error {
            if let Some((_, msg)) = errors.iter().find(|(id, _)| id == &field.id) {
                let indent = 2;
                lines.extend(error_lines(msg, indent, area.width as usize));
            }
        }

        // Description hint for focused row.
        if style.show_description && active {
            if let Some(desc) = &field.description {
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(2 + pad + 2)),
                    Span::styled(desc.clone(), theme.hint),
                ]));
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn text_state() -> FormState {
        FormState::new(vec![
            FormField::new("name", "name", FieldInput::Text("ab".into())),
            FormField::new("age", "age", FieldInput::Integer(42)),
        ])
    }

    #[test]
    fn text_insertion_tracks_cursor() {
        let mut s = FormState::new(vec![FormField::new(
            "x", "x", FieldInput::Text("".into()),
        )]);
        assert_eq!(s.cursors[0], 0);
        let r = s.handle_key(key(KeyCode::Char('a')));
        assert!(matches!(r, FormEvent::FieldChanged(_)));
        assert_eq!(s.cursors[0], 1);
        match &s.fields[0].input {
            FieldInput::Text(v) => assert_eq!(v, "a"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn tab_moves_forward_and_touches_previous() {
        let mut s = text_state();
        assert_eq!(s.focused, 0);
        let r = s.handle_key(key(KeyCode::Tab));
        assert_eq!(r, FormEvent::FocusMoved);
        assert_eq!(s.focused, 1);
        assert!(s.touched.contains("name"));
    }

    #[test]
    fn backtab_moves_back() {
        let mut s = text_state();
        s.focused = 1;
        let r = s.handle_key(key(KeyCode::BackTab));
        assert_eq!(r, FormEvent::FocusMoved);
        assert_eq!(s.focused, 0);
    }

    #[test]
    fn hidden_rows_skipped_by_navigation() {
        let mut s = FormState::new(vec![
            FormField::new("a", "a", FieldInput::Text("a".into())),
            FormField::new("b", "b", FieldInput::Text("b".into())).visible(false),
            FormField::new("c", "c", FieldInput::Text("c".into())),
        ]);
        assert_eq!(s.focused, 0);
        s.handle_key(key(KeyCode::Tab));
        assert_eq!(s.focused, 2, "should skip the hidden row");
    }

    #[test]
    fn enum_cycles_with_arrows() {
        let mut s = FormState::new(vec![FormField::new(
            "k",
            "kind",
            FieldInput::Enum {
                options: vec!["A".into(), "B".into(), "C".into()],
                selected: 0,
            },
        )]);
        s.handle_key(key(KeyCode::Right));
        match &s.fields[0].input {
            FieldInput::Enum { selected, .. } => assert_eq!(*selected, 1),
            _ => unreachable!(),
        }
        s.handle_key(key(KeyCode::Left));
        s.handle_key(key(KeyCode::Left));
        match &s.fields[0].input {
            FieldInput::Enum { selected, .. } => assert_eq!(*selected, 2, "wraps"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn bool_toggles_with_left_right() {
        let mut s = FormState::new(vec![FormField::new(
            "b", "b", FieldInput::Boolean(false),
        )]);
        s.handle_key(key(KeyCode::Left));
        assert!(matches!(s.fields[0].input, FieldInput::Boolean(true)));
    }

    #[test]
    fn esc_cancels() {
        let mut s = text_state();
        assert_eq!(s.handle_key(key(KeyCode::Esc)), FormEvent::Cancel);
    }

    #[test]
    fn enter_on_last_submits() {
        let mut s = text_state();
        s.focused = 1;
        assert_eq!(s.handle_key(key(KeyCode::Enter)), FormEvent::Submit);
    }

    #[test]
    fn enter_on_middle_advances() {
        let mut s = text_state();
        assert_eq!(s.handle_key(key(KeyCode::Enter)), FormEvent::FocusMoved);
        assert_eq!(s.focused, 1);
    }

    #[test]
    fn wrap_chars_respects_width_and_hard_breaks() {
        let out = wrap_chars("one two three four", 8);
        assert_eq!(out, vec!["one two", "three", "four"]);

        let out = wrap_chars("abcdefghijk", 3);
        assert_eq!(out, vec!["abc", "def", "ghi", "jk"]);
    }

    #[test]
    fn error_lines_align_under_indent() {
        let lines = error_lines("oops bad value", 4, 20);
        assert!(!lines.is_empty());
        // First line carries the └ marker; subsequent lines are plain-indented.
        let first = format!("{:?}", lines[0]);
        assert!(first.contains("└"), "expected └ marker in first error line");
    }
}
