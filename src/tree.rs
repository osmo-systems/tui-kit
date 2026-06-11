//! Foldable tree widget — a hierarchical, scrollable, focusable list.
//!
//! Like [`crate::list`], the widget itself is stateless about your data: the
//! caller owns the tree and its expand/collapse state, flattens the **visible**
//! nodes into a `Vec<TreeRow>` each frame (skipping children of collapsed
//! branches), and passes that slice in. [`TreeState`] only tracks selection and
//! scroll offset over those visible rows.
//!
//! ```text
//! ┌─ Targets ──────────────────────────────────────────────────┐
//! │ ▾ services/api/Makefile                                     │
//! │     build    Build the api                                  │
//! │   ▶ run      Run locally                          PORT       │
//! │ ▸ web/build.mk                              (collapsed)      │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! // 1. Flatten your tree to visible rows (respecting collapsed branches):
//! let rows: Vec<TreeRow> = visible_nodes.iter().map(|n| {
//!     if n.is_branch {
//!         TreeRow::branch(n.depth, n.label.clone(), n.expanded)
//!     } else {
//!         TreeRow::leaf(n.depth, n.label.clone())
//!     }
//! }).collect();
//!
//! // 2. Render, and map state.selected back to your node on Enter.
//! render_tree(f, area, "Targets", None, &rows, &mut tree_state, focused, &theme);
//! ```

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    block::{focusable_block, render_scrollbar},
    Theme,
};

/// Selection + scroll state for a [`render_tree`] widget, over the slice of
/// currently-visible rows.
pub struct TreeState {
    /// Index of the selected row within the visible rows.
    pub selected: usize,
    offset: usize,
}

impl TreeState {
    pub fn new() -> Self {
        Self { selected: 0, offset: 0 }
    }

    /// Move selection to the next visible row, wrapping at the end.
    pub fn select_next(&mut self, row_count: usize) {
        if row_count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % row_count;
    }

    /// Move selection to the previous visible row, wrapping at the start.
    pub fn select_prev(&mut self, row_count: usize) {
        if row_count == 0 {
            return;
        }
        if self.selected == 0 {
            self.selected = row_count - 1;
        } else {
            self.selected -= 1;
        }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Clamp the scroll offset so the selected row stays visible.
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

impl Default for TreeState {
    fn default() -> Self {
        Self::new()
    }
}

/// One visible row of a tree.
pub struct TreeRow {
    /// Indentation level (0 = root). Each level adds two columns.
    pub depth: u16,
    /// Primary label shown after the fold glyph.
    pub label: String,
    /// Optional dimmed text shown right-aligned (e.g. a hint or count).
    pub secondary: Option<String>,
    /// `None` = leaf node; `Some(true)` = expanded branch; `Some(false)` =
    /// collapsed branch. Controls the ▾ / ▸ glyph.
    pub expanded: Option<bool>,
    /// Optional label style override (e.g. accent for directories). Defaults to
    /// [`Theme::body`] when `None`.
    pub style: Option<Style>,
}

impl TreeRow {
    /// A leaf row (no fold glyph).
    pub fn leaf(depth: u16, label: impl Into<String>) -> Self {
        Self {
            depth,
            label: label.into(),
            secondary: None,
            expanded: None,
            style: None,
        }
    }

    /// A branch row that can be expanded or collapsed.
    pub fn branch(depth: u16, label: impl Into<String>, expanded: bool) -> Self {
        Self {
            depth,
            label: label.into(),
            secondary: None,
            expanded: Some(expanded),
            style: None,
        }
    }

    pub fn secondary(mut self, secondary: impl Into<String>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

/// Render a scrollable, focusable tree of `rows` inside `area`.
///
/// - Uses [`focusable_block`] for the outer border + optional digit shortcut.
/// - The selected row is highlighted with [`Theme::selection`].
/// - Branch rows show a ▾ (expanded) or ▸ (collapsed) glyph; leaves are indented
///   to align with branch labels.
/// - `secondary` text is right-aligned using [`Theme::hint`].
/// - Handles an empty `rows` slice without panicking.
pub fn render_tree(
    f: &mut Frame,
    area: Rect,
    title: &str,
    shortcut: Option<u8>,
    rows: &[TreeRow],
    state: &mut TreeState,
    focused: bool,
    theme: &Theme,
) {
    let block = focusable_block(title, shortcut, focused, theme);
    let inner = block.inner(area);
    f.render_widget(block, area);
    render_scrollbar(f, area, rows.len(), state.offset);

    let visible_height = inner.height as usize;

    if !rows.is_empty() && state.selected >= rows.len() {
        state.selected = rows.len() - 1;
    }
    state.clamp_offset(visible_height);

    if rows.is_empty() || visible_height == 0 {
        return;
    }

    for (idx, row) in rows
        .iter()
        .enumerate()
        .skip(state.offset)
        .take(visible_height)
    {
        let row_y = inner.y + (idx - state.offset) as u16;
        let row_area = Rect {
            x: inner.x,
            y: row_y,
            width: inner.width,
            height: 1,
        };

        let is_selected = idx == state.selected;
        let label_style = if is_selected {
            theme.selection
        } else {
            row.style.unwrap_or(theme.body)
        };
        let glyph_style = if is_selected { theme.selection } else { theme.hint };

        let indent = "  ".repeat(row.depth as usize);
        let glyph = match row.expanded {
            Some(true) => "▾ ",
            Some(false) => "▸ ",
            None => "  ",
        };
        let prefix = format!("{indent}{glyph}");

        let left = Line::from(vec![
            Span::styled(prefix, glyph_style),
            Span::styled(row.label.clone(), label_style),
        ]);

        match &row.secondary {
            None => {
                f.render_widget(Paragraph::new(left), row_area);
            }
            Some(sec) => {
                let sec_width = (sec.chars().count() as u16 + 1)
                    .min(inner.width.saturating_sub(1));
                let prim_width = inner.width.saturating_sub(sec_width);

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(prim_width),
                        Constraint::Length(sec_width),
                    ])
                    .split(row_area);

                let sec_style = if is_selected { theme.selection } else { theme.hint };
                let sec_para = Paragraph::new(Line::from(Span::styled(sec.clone(), sec_style)))
                    .alignment(Alignment::Right);

                f.render_widget(Paragraph::new(left), chunks[0]);
                f.render_widget(sec_para, chunks[1]);
            }
        }
    }
}
