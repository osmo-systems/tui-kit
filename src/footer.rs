//! Footer / keybind bar widget.
//!
//! Renders a single-line row of keyboard shortcut hints at the bottom of the screen.
//! Each hint is a `(key, action)` pair styled as:
//!
//! ```text
//! j/k :Navigate  |  Enter :Open  |  space :Leader  |  q :Quit
//! ^^^^ ^^^^^^^^^^    ^^^^^                           ^
//! key  action         key                            separator
//! (yellow)  (dark gray)                            (dark gray)
//! ```
//!
//! ## Usage
//!
//! ```rust
//! // In your render function, at the bottom of the layout:
//! render_footer(f, chunks[FOOTER], &[
//!     ("j/k",   "navigate"),
//!     ("enter", "open"),
//!     ("space", "leader"),
//!     ("q",     "quit"),
//! ], &theme);
//! ```
//!
//! The pairs are context-aware — build them based on the current app state
//! (e.g. show different hints when a popup is open):
//!
//! ```rust
//! let mut pairs: Vec<(&str, &str)> = Vec::new();
//! if popup_open {
//!     pairs.push(("Esc",   "close"));
//!     pairs.push(("Enter", "confirm"));
//! } else {
//!     pairs.push(("j/k",   "navigate"));
//!     pairs.push(("space", "leader"));
//!     pairs.push(("q",     "quit"));
//! }
//! render_footer(f, footer_area, &pairs, &theme);
//! ```
//!
//! ## Optional right-hand label
//!
//! Use [`render_footer_with_app`] to also show an app name + version pinned to
//! the right edge (as rad does):
//!
//! ```rust
//! render_footer_with_app(f, chunks[FOOTER], &pairs, "myapp", "1.0.0", &theme);
//! ```

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::theme::Theme;

/// Render a keybind bar from a list of `(key, action)` pairs.
///
/// Pairs are separated by `" | "` and styled with the theme's
/// `shortcut_key` (yellow) for keys and `hint` (dark gray) for actions and
/// separators. A 1-column margin is applied on both sides so the bar aligns
/// with bordered widgets that have their own borders.
///
/// # Arguments
/// * `pairs` — slice of `(key_label, action_label)`. Action labels should be
///   lowercase, no leading colon needed (the colon is added automatically).
pub fn render_footer(f: &mut Frame, area: Rect, pairs: &[(&str, &str)], theme: &Theme) {
    let inner = horizontal_margin(area);
    f.render_widget(Paragraph::new(Line::from(keybind_spans(pairs, theme))), inner);
}

/// Like [`render_footer`] but also renders `"name version"` right-aligned.
///
/// Useful for showing the app name and version in the bottom-right corner,
/// exactly as the rad project does.
pub fn render_footer_with_app(
    f: &mut Frame,
    area: Rect,
    pairs: &[(&str, &str)],
    app_name: &str,
    app_version: &str,
    theme: &Theme,
) {
    let inner = horizontal_margin(area);
    let right_width = (app_name.len() + 1 + app_version.len()) as u16;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_width)])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(keybind_spans(pairs, theme))),
        chunks[0],
    );

    let right_line = Line::from(vec![
        Span::styled(app_name.to_string(), theme.shortcut_key),
        Span::styled(format!(" {}", app_version), theme.hint),
    ]);
    f.render_widget(
        Paragraph::new(right_line).alignment(Alignment::Right),
        chunks[1],
    );
}

/// Build the `Vec<Span>` for a keybind bar without rendering it.
///
/// Useful when you need to embed the hints inside a larger `Line` or a block
/// title rather than a full-width footer row.
pub fn keybind_spans<'a>(pairs: &[(&'a str, &'a str)], theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    for (i, (key, action)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" | ", theme.hint));
        }
        spans.push(Span::styled(*key, theme.shortcut_key));
        spans.push(Span::styled(format!(" :{}", action), theme.hint));
    }
    spans
}

/// Apply a 1-column left+right margin so the footer aligns with bordered panels.
fn horizontal_margin(area: Rect) -> Rect {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);
    chunks[1]
}
