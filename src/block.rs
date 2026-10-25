use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::Theme;

/// Creates a bordered [`Block`] for a main-content panel.
///
/// - `focused = true`  → uses [`Theme::border_focused`] (accent color, e.g. Green+Bold).
/// - `focused = false` → uses [`Theme::border_unfocused`] (e.g. White).
///
/// The title is any `Line<'static>` — use [`widget_title`] or [`crate::tabs::tab_line`]
/// to build it, or pass `Line::from("My Panel")` for a plain string.
///
/// For a simple focusable widget with an optional digit shortcut, prefer
/// [`focusable_block`] which builds both the title and the block in one call.
pub fn panel_block(title: Line<'static>, focused: bool, theme: &Theme) -> Block<'static> {
    let border_style = if focused {
        theme.border_focused
    } else {
        theme.border_unfocused
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
}

/// Creates a bordered [`Block`] for a floating popup.
///
/// Always uses [`Theme::border_popup`] regardless of focus state,
/// since a visible popup is by definition the active element.
pub fn popup_block(title: Line<'static>, theme: &Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_popup)
        .title(title)
}

/// Convenience wrapper: builds a [`widget_title`] and a [`panel_block`] in one call.
///
/// Use this for any widget that is focusable and optionally has a digit shortcut.
/// The border color and the digit indicator always stay in sync.
///
/// ```ignore
/// let block = focusable_block("Status Log", Some(2), focused, &theme);
/// ```
pub fn focusable_block(title: &str, shortcut: Option<u8>, focused: bool, theme: &Theme) -> Block<'static> {
    let title_line = widget_title(title, shortcut, focused, theme);
    panel_block(title_line, focused, theme)
}

/// Builds a widget title [`Line`] with an optional keyboard-shortcut digit indicator.
///
/// The digit and `─` separator blend into the border line (same color), producing:
///
/// ```text
/// ┌─ 1 ─ Favorites ────────────────────────────────────────────┐
/// ```
///
/// - `shortcut = Some(1)` → ` [1] ─ Label `
/// - `shortcut = None`    → ` Label `
///
/// `active` controls the label style ([`Theme::tab_active`] vs [`Theme::tab_inactive`])
/// and the digit+separator color ([`Theme::border_focused`] vs [`Theme::border_unfocused`]),
/// so they always match the surrounding border.
///
/// # Example
///
/// ```ignore
/// // Simple panel title, always active
/// let title = widget_title("Now Playing", None, true, &theme);
///
/// // Panel with shortcut indicator, focus-aware
/// let title = widget_title("Favorites", Some(1), focused, &theme);
/// ```
/// Render a native ratatui vertical scrollbar over the right border of `area`.
///
/// Pass the **outer** block area (including borders). The scrollbar overlaps the
/// right border column, which is the standard ratatui pattern.
///
/// - `total`    — total number of content rows / lines.
/// - `position` — index of the first visible row (the scroll offset).
///
/// Does nothing when all content fits in the visible area.
pub fn render_scrollbar(f: &mut Frame, area: Rect, total: usize, position: usize) {
    let visible = area.height.saturating_sub(2) as usize; // subtract top+bottom borders
    if total <= visible {
        return;
    }
    // ratatui drops the thumb to the bottom of the track when
    // `position == content_length - 1`, i.e. when the *last row* is at the top
    // of the viewport. Callers scroll only until the last row is at the bottom,
    // so content_length must be the number of scroll offsets — passing the row
    // count instead leaves the thumb stuck mid-track at maximum scroll.
    let max_offset = total - visible;
    let mut state = ScrollbarState::new(max_offset + 1)
        .position(position.min(max_offset))
        .viewport_content_length(visible);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        area,
        &mut state,
    );
}

pub fn widget_title(label: &str, shortcut: Option<u8>, active: bool, theme: &Theme) -> Line<'static> {
    let label_style = if active { theme.tab_active } else { theme.tab_inactive };
    let border_style = if active { theme.border_focused } else { theme.border_unfocused };

    match shortcut {
        Some(n) => Line::from(vec![
            Span::styled(format!("[{}]\u{2500} ", n), border_style),
            Span::styled(label.to_string(), label_style),
            Span::raw(" "),
        ]),
        None => Line::from(vec![
            Span::raw(" "),
            Span::styled(label.to_string(), label_style),
            Span::raw(" "),
        ]),
    }
}
