use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::Theme;

/// The interactive value held by a [`FormField`].
#[derive(Debug, Clone)]
pub enum FieldInput {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    /// Inline selector cycling through `options`; `selected` is the current index.
    Enum { options: Vec<String>, selected: usize },
    /// Ordered list of strings. Display-only for now — editing deferred.
    List(Vec<String>),
}

/// A single field in a [`FormState`].
#[derive(Debug, Clone)]
pub struct FormField {
    /// Label displayed to the left of the input.
    pub label: String,
    /// Current input value.
    pub input: FieldInput,
    /// If true, a `*` is appended to the label.
    pub required: bool,
    /// Optional hint shown below the field when it is focused.
    pub description: Option<String>,
}

/// Event returned by [`FormState::handle_key`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormEvent {
    /// No state change visible to the caller.
    None,
    /// User confirmed the form (Enter on last field, or explicit submit key).
    Submit,
    /// User cancelled the form (Esc).
    Cancel,
}

/// State for the [`render_form`] widget.
pub struct FormState {
    /// The form fields.
    pub fields: Vec<FormField>,
    /// Index of the currently focused field.
    pub focused: usize,
    /// Cursor position (byte offset in the string representation) per field.
    cursors: Vec<usize>,
}

impl FormState {
    /// Create a new [`FormState`] from a list of fields.
    pub fn new(fields: Vec<FormField>) -> Self {
        let len = fields.len();
        Self { fields, focused: 0, cursors: vec![0; len] }
    }

    /// Move focus to the next field, stopping at the last.
    pub fn focus_next(&mut self) {
        if self.focused + 1 < self.fields.len() {
            self.focused += 1;
        }
    }

    /// Move focus to the previous field, stopping at the first.
    pub fn focus_prev(&mut self) {
        self.focused = self.focused.saturating_sub(1);
    }

    /// Handle a key event, updating internal state and returning a [`FormEvent`].
    pub fn handle_key(&mut self, key: KeyEvent) -> FormEvent {
        if self.fields.is_empty() {
            return FormEvent::None;
        }

        match key.code {
            // Navigation
            KeyCode::Tab if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.focus_next();
                FormEvent::None
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus_prev();
                FormEvent::None
            }
            KeyCode::Down => { self.focus_next(); FormEvent::None }
            KeyCode::Up   => { self.focus_prev(); FormEvent::None }

            // Cancel
            KeyCode::Esc => FormEvent::Cancel,

            // Confirm
            KeyCode::Enter => {
                if self.focused + 1 >= self.fields.len() {
                    FormEvent::Submit
                } else {
                    self.focus_next();
                    FormEvent::None
                }
            }

            // Field-specific input
            _ => {
                self.handle_field_key(key);
                FormEvent::None
            }
        }
    }

    fn handle_field_key(&mut self, key: KeyEvent) {
        let idx = self.focused;
        match &mut self.fields[idx].input {
            FieldInput::Text(s) => match key.code {
                KeyCode::Char(c) => { s.push(c); self.cursors[idx] = s.len(); }
                KeyCode::Backspace => { s.pop(); self.cursors[idx] = s.len(); }
                _ => {}
            },
            FieldInput::Integer(n) => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || (c == '-' && *n == 0) => {
                    let mut s = n.to_string();
                    s.push(c);
                    if let Ok(v) = s.parse::<i64>() { *n = v; }
                    self.cursors[idx] = n.to_string().len();
                }
                KeyCode::Backspace => {
                    let mut s = n.to_string();
                    s.pop();
                    *n = s.parse::<i64>().unwrap_or(0);
                    self.cursors[idx] = n.to_string().len();
                }
                _ => {}
            },
            FieldInput::Float(f) => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || (c == '-' && *f == 0.0) => {
                    // Work via a temporary string representation
                    let mut s = format!("{}", f);
                    s.push(c);
                    if let Ok(v) = s.parse::<f64>() { *f = v; }
                    self.cursors[idx] = format!("{}", f).len();
                }
                KeyCode::Backspace => {
                    let mut s = format!("{}", f);
                    s.pop();
                    *f = s.parse::<f64>().unwrap_or(0.0);
                    self.cursors[idx] = format!("{}", f).len();
                }
                _ => {}
            },
            FieldInput::Boolean(b) => {
                if key.code == KeyCode::Char(' ') {
                    *b = !*b;
                }
            }
            FieldInput::Enum { options, selected } => match key.code {
                KeyCode::Right | KeyCode::Char('l') => {
                    if !options.is_empty() {
                        *selected = (*selected + 1) % options.len();
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if !options.is_empty() && *selected > 0 {
                        *selected -= 1;
                    } else if !options.is_empty() {
                        *selected = options.len() - 1;
                    }
                }
                _ => {}
            },
            FieldInput::List(_) => {
                // Editing deferred — display-only for now.
            }
        }
    }

    /// Borrow the fields slice.
    pub fn fields(&self) -> &[FormField] {
        &self.fields
    }

    /// Consume the state and return the fields with their current input values.
    pub fn into_fields(self) -> Vec<FormField> {
        self.fields
    }
}

/// Render the form inside `area`.
///
/// Each field takes up to 3 rows:
/// 1. `Label [*]: <input>`
/// 2. Description hint (only when focused, in [`Theme::hint`])
/// 3. Blank separator
///
/// The caller is responsible for wrapping this in a [`crate::popup::centered_popup`]
/// + [`crate::block::popup_block`] — no outer border is drawn here.
pub fn render_form(f: &mut Frame, area: Rect, state: &FormState, theme: &Theme) {
    if area.height == 0 {
        return;
    }

    let mut y = area.y;

    for (idx, field) in state.fields.iter().enumerate() {
        if y >= area.y + area.height {
            break;
        }

        let focused = idx == state.focused;
        let label_style = if focused { theme.tab_active } else { theme.hint };
        let bracket_style = if focused { theme.border_focused } else { theme.border_unfocused };

        let label = if field.required {
            format!("{}*", field.label)
        } else {
            field.label.clone()
        };

        let input_repr = field_repr(&field.input);

        // Line 1: label + input
        let line = Line::from(vec![
            Span::styled(format!("{}: ", label), label_style),
            Span::styled(input_repr, bracket_style),
        ]);
        let row = Rect { x: area.x, y, width: area.width, height: 1 };
        f.render_widget(Paragraph::new(line), row);
        y += 1;

        // Line 2: description hint (focused only)
        if focused {
            if let Some(desc) = &field.description {
                if y < area.y + area.height {
                    let hint_row = Rect { x: area.x + 2, y, width: area.width.saturating_sub(2), height: 1 };
                    f.render_widget(
                        Paragraph::new(Line::from(Span::styled(desc.clone(), theme.hint))),
                        hint_row,
                    );
                    y += 1;
                }
            }
        }

        // Line 3: blank separator
        y += 1;
    }
}

/// Produce the bracketed string representation of a field input.
fn field_repr(input: &FieldInput) -> String {
    match input {
        FieldInput::Text(s)    => format!("[ {} ]", if s.is_empty() { "_" } else { s }),
        FieldInput::Integer(n) => format!("[ {} ]", n),
        FieldInput::Float(f)   => format!("[ {} ]", f),
        FieldInput::Boolean(b) => if *b { "[x]".into() } else { "[ ]".into() },
        FieldInput::Enum { options, selected } => {
            if options.is_empty() {
                "< >".into()
            } else {
                format!("< {} >", options[*selected])
            }
        }
        FieldInput::List(items) => {
            if items.is_empty() {
                "[ (empty) ]".into()
            } else {
                format!("[ {} ]", items.join(", "))
            }
        }
    }
}
