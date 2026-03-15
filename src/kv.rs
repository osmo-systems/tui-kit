use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{block::focusable_block, Theme};

/// A single row in a [`render_kv_table`] widget.
pub enum KvRow {
    /// A section header rendered full-width in [`Theme::section_header`] style.
    Header(String),
    /// A key-value pair: key right-aligned, value left-aligned.
    Field { key: String, value: String },
    /// A blank separator line.
    Separator,
}

/// Render a scrollable key-value table inside `area`.
///
/// - Section [`KvRow::Header`]s are rendered full-width in [`Theme::section_header`].
/// - [`KvRow::Field`] rows split the inner width: key column (right-aligned, [`Theme::hint`])
///   and value column (left-aligned, [`Theme::body`]). The key column width is the minimum
///   of the longest key in `rows` and 30% of the inner width.
/// - [`KvRow::Separator`] renders a blank line.
/// - `scroll` skips the first N logical rows before rendering.
pub fn render_kv_table(
    f: &mut Frame,
    area: Rect,
    title: &str,
    shortcut: Option<u8>,
    rows: &[KvRow],
    scroll: u16,
    focused: bool,
    theme: &Theme,
) {
    let block = focusable_block(title, shortcut, focused, theme);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 || rows.is_empty() {
        return;
    }

    // Compute key column width: longest key, capped at 30% of inner width.
    let max_key_len = rows
        .iter()
        .filter_map(|r| match r {
            KvRow::Field { key, .. } => Some(key.chars().count()),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let key_col_width = (max_key_len as u16).min((inner.width as f32 * 0.30) as u16).max(1);

    let visible_height = inner.height as usize;
    let skip = scroll as usize;

    for (i, row) in rows.iter().skip(skip).take(visible_height).enumerate() {
        let row_y = inner.y + i as u16;
        let row_area = Rect { x: inner.x, y: row_y, width: inner.width, height: 1 };

        match row {
            KvRow::Header(text) => {
                let para = Paragraph::new(Line::from(Span::styled(
                    text.clone(),
                    theme.section_header,
                )));
                f.render_widget(para, row_area);
            }
            KvRow::Separator => {
                // Blank line — nothing to render.
            }
            KvRow::Field { key, value } => {
                let val_col_width = inner.width.saturating_sub(key_col_width + 1); // +1 for gap

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(key_col_width),
                        Constraint::Length(1), // gap
                        Constraint::Length(val_col_width),
                    ])
                    .split(row_area);

                let key_para = Paragraph::new(Line::from(Span::styled(
                    key.clone(),
                    theme.hint,
                )))
                .alignment(Alignment::Right);

                let val_para = Paragraph::new(Line::from(Span::styled(
                    value.clone(),
                    theme.body,
                )));

                f.render_widget(key_para, chunks[0]);
                f.render_widget(val_para, chunks[2]);
            }
        }
    }
}
