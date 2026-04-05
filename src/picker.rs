//! Telescope-style two-column picker widget.
//!
//! Layout:
//! ```text
//! ┌─ Title ──────────────────────────────────────────────────┐
//! │ ┌─ search ──────────┐  ┌─ Detail ─────────────────────┐ │
//! │ │ query_            │  │ selected item detail…        │ │
//! │ └───────────────────┘  │                              │ │
//! │ ┌─ 3/12 ────────────┐  │                              │ │
//! │ │   item one        │  └──────────────────────────────┘ │
//! │ │ ▶ item two        │                                   │
//! │ │   item three      │                                   │
//! │ └───────────────────┘                                   │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! // 1. Create/manage PickerState in your app state
//! // 2. Filter your items externally based on state.search
//! // 3. Compute detail lines for state.selected item
//! // 4. Call render_picker each frame
//!
//! let filtered: Vec<PickerItem> = all_items
//!     .iter()
//!     .filter(|i| i.label.to_lowercase().contains(&state.search.to_lowercase()))
//!     .cloned()
//!     .collect();
//!
//! let detail = if let Some(item) = filtered.get(state.selected) {
//!     vec![Line::from(item.label.clone())]
//! } else {
//!     vec![]
//! };
//!
//! let area = tui_kit::popup::centered_popup(f, 0.88, 120, 30);
//! render_picker(f, area, "Find instance", &state, &filtered, detail, &theme);
//! ```

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::Theme;

/// Navigation and search state for a [`render_picker`] widget.
#[derive(Debug, Clone)]
pub struct PickerState {
    /// Current search string (shown in the search box).
    pub search: String,
    /// Index of the selected item in the **filtered** list.
    pub selected: usize,
    /// First visible item index (scroll offset).
    pub scroll_offset: usize,
}

impl PickerState {
    pub fn new() -> Self {
        Self { search: String::new(), selected: 0, scroll_offset: 0 }
    }

    /// Move selection down by one, clamped to `item_count`.
    pub fn select_next(&mut self, item_count: usize) {
        if item_count == 0 { return; }
        if self.selected + 1 < item_count {
            self.selected += 1;
        }
    }

    /// Move selection up by one.
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Reset selection and scroll when the search changes.
    pub fn reset_selection(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Clamp scroll so the selected row is always visible.
    pub fn clamp_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 { return; }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }
}

impl Default for PickerState {
    fn default() -> Self { Self::new() }
}

/// A single row in a [`render_picker`] list.
#[derive(Debug, Clone)]
pub struct PickerItem {
    /// Primary label shown in the list.
    pub label: String,
    /// Optional short tag shown before the label, e.g. `"[C]"`, `"[R]"`.
    pub tag: Option<String>,
    /// Style applied to the tag when the row is not selected. Ignored when tag is None.
    pub tag_style: Option<Style>,
}

impl PickerItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), tag: None, tag_style: None }
    }

    pub fn with_tag(label: impl Into<String>, tag: impl Into<String>) -> Self {
        Self { label: label.into(), tag: Some(tag.into()), tag_style: None }
    }

    pub fn with_tag_styled(label: impl Into<String>, tag: impl Into<String>, style: Style) -> Self {
        Self { label: label.into(), tag: Some(tag.into()), tag_style: Some(style) }
    }
}

/// Render a two-column picker inside `area`.
///
/// - `title`       — label shown on the search box border.
/// - `items`       — pre-filtered list of items to display.
/// - `state`       — current search/selection state.
/// - `detail`      — lines to render in the right-hand detail pane.
///                   Computed by the caller from `items[state.selected]`.
/// - `total_count` — total (unfiltered) item count, shown as `n/total` in list title.
pub fn render_picker<'a>(
    f: &mut Frame,
    area: Rect,
    title: &str,
    items: &[PickerItem],
    state: &PickerState,
    detail: Vec<Line<'a>>,
    total_count: usize,
    theme: &Theme,
) {
    // Two columns: 45% list | 55% detail
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // ── Left: search box + list ──────────────────────────────────────────────

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(cols[0]);

    // Search box
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused)
        .title(format!(" {} ", title))
        .title_style(theme.tab_active);
    f.render_widget(
        Paragraph::new(state.search.as_str()).block(search_block),
        left_rows[0],
    );

    // List
    let visible_height = left_rows[1].height.saturating_sub(2) as usize;
    let scroll = state.scroll_offset.min(items.len().saturating_sub(1));

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(idx, item)| {
            let selected = idx == state.selected;
            let row_style = if selected { theme.selection } else { theme.body };
            let prefix = if selected { "▶ " } else { "  " };
            let line = match &item.tag {
                Some(tag) => {
                    let tag_style = if selected {
                        row_style
                    } else {
                        item.tag_style.unwrap_or(row_style)
                    };
                    Line::from(vec![
                        Span::styled(prefix.to_string(), row_style),
                        Span::styled(format!("{} ", tag), tag_style),
                        Span::styled(item.label.clone(), row_style),
                    ])
                }
                None => Line::from(vec![
                    Span::styled(prefix.to_string(), row_style),
                    Span::styled(item.label.clone(), row_style),
                ]),
            };
            ListItem::new(line)
        })
        .collect();

    let count_title = format!(" {}/{} ", items.len(), total_count);
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_unfocused)
        .title(count_title)
        .title_style(theme.hint);
    f.render_widget(List::new(list_items).block(list_block), left_rows[1]);

    // ── Right: detail ────────────────────────────────────────────────────────

    let detail_content = if detail.is_empty() {
        vec![Line::from(Span::styled(
            "(no selection)",
            theme.hint.add_modifier(Modifier::ITALIC),
        ))]
    } else {
        detail
    };

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_unfocused)
        .title(" Detail ")
        .title_style(theme.hint);

    f.render_widget(
        Paragraph::new(Text::from(detail_content))
            .block(detail_block)
            .wrap(Wrap { trim: false }),
        cols[1],
    );
}
