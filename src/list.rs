use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{block::{focusable_block, render_scrollbar}, Theme};

/// Scroll and selection state for a [`render_list`] widget.
pub struct ListState {
    /// Index of the currently selected item.
    pub selected: usize,
    offset: usize,
}

impl ListState {
    /// Create a new [`ListState`] with nothing selected and no scroll offset.
    pub fn new() -> Self {
        Self { selected: 0, offset: 0 }
    }

    /// Move selection to the next item, wrapping around at the end.
    pub fn select_next(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % item_count;
    }

    /// Move selection to the previous item, wrapping around at the beginning.
    pub fn select_prev(&mut self) {
        if self.selected == 0 {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
    }

    /// Return the index of the currently selected item.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Return the current scroll offset (index of the first visible item).
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Clamp the scroll offset so the selected item stays within the visible area.
    fn clamp_offset(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + visible_height {
            self.offset = self.selected - visible_height + 1;
        }
    }
}

impl Default for ListState {
    fn default() -> Self {
        Self::new()
    }
}

/// A single row in a [`render_list`] widget.
pub struct ListItem {
    /// Primary text shown left-aligned.
    pub primary: String,
    /// Optional secondary text shown right-aligned in a dimmed style.
    pub secondary: Option<String>,
}

/// Render a scrollable, focusable list inside `area`.
///
/// - Uses [`focusable_block`] for the outer border.
/// - The selected row is highlighted with [`Theme::selection`].
/// - `primary` text is left-aligned using [`Theme::body`].
/// - `secondary` text is right-aligned using [`Theme::hint`].
/// - Handles an empty `items` slice without panicking.
pub fn render_list(
    f: &mut Frame,
    area: Rect,
    title: &str,
    shortcut: Option<u8>,
    items: &[ListItem],
    state: &mut ListState,
    focused: bool,
    theme: &Theme,
) {
    let block = focusable_block(title, shortcut, focused, theme);
    let inner = block.inner(area);
    f.render_widget(block, area);
    render_scrollbar(f, area, items.len(), state.offset);

    let visible_height = inner.height as usize;

    // Clamp the selection to valid item indices before adjusting the offset.
    if !items.is_empty() && state.selected >= items.len() {
        state.selected = items.len() - 1;
    }
    state.clamp_offset(visible_height);

    if items.is_empty() || visible_height == 0 {
        return;
    }

    let visible_items = items
        .iter()
        .enumerate()
        .skip(state.offset)
        .take(visible_height);

    for (idx, item) in visible_items {
        let row_y = inner.y + (idx - state.offset) as u16;
        let row_area = Rect {
            x: inner.x,
            y: row_y,
            width: inner.width,
            height: 1,
        };

        let is_selected = idx == state.selected;
        let row_style = if is_selected { theme.selection } else { theme.body };

        match &item.secondary {
            None => {
                let para = Paragraph::new(Line::from(Span::styled(
                    item.primary.clone(),
                    row_style,
                )));
                f.render_widget(para, row_area);
            }
            Some(sec) => {
                // Split the row into primary (left) and secondary (right) columns.
                let sec_width = (sec.chars().count() as u16).min(inner.width.saturating_sub(1));
                let prim_width = inner.width.saturating_sub(sec_width);

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(prim_width),
                        Constraint::Length(sec_width),
                    ])
                    .split(row_area);

                let prim_style = if is_selected { theme.selection } else { theme.body };
                let sec_style = if is_selected { theme.selection } else { theme.hint };

                let prim_para = Paragraph::new(Line::from(Span::styled(
                    item.primary.clone(),
                    prim_style,
                )));
                let sec_para = Paragraph::new(Line::from(Span::styled(
                    sec.clone(),
                    sec_style,
                )))
                .alignment(ratatui::layout::Alignment::Right);

                f.render_widget(prim_para, chunks[0]);
                f.render_widget(sec_para, chunks[1]);
            }
        }
    }
}
