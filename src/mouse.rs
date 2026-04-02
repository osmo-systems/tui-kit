//! Mouse event helpers.
//!
//! ## Enabling mouse support
//!
//! In your terminal setup / teardown, enable and disable mouse capture:
//!
//! ```rust
//! use crossterm::{execute, event::{EnableMouseCapture, DisableMouseCapture}};
//!
//! // setup
//! execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
//!
//! // teardown
//! execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
//! ```
//!
//! ## Hit-testing in the event loop
//!
//! ```rust
//! use crossterm::event::{Event, MouseButton, MouseEventKind};
//! use tui_kit::mouse_hit;
//!
//! if let Event::Mouse(m) = event::read()? {
//!     if m.kind == MouseEventKind::Down(MouseButton::Left) {
//!         app.handle_mouse(m.column, m.row);
//!     }
//! }
//! ```
//!
//! ## Storing panel areas
//!
//! Record each focusable widget's `Rect` during the render pass, then use
//! [`mouse_hit`] to map a click coordinate back to the focused widget:
//!
//! ```rust
//! // In render:
//! app.areas[Panel::List as usize] = Some(list_area);
//!
//! // In handle_mouse:
//! for (i, area) in app.areas.iter().enumerate() {
//!     if let Some(a) = area {
//!         if mouse_hit(*a, col, row) {
//!             app.focus = Panel::from(i);
//!             return;
//!         }
//!     }
//! }
//! ```

use ratatui::{layout::Position, layout::Rect};

/// Returns `true` if the terminal cell `(col, row)` falls inside `area`.
///
/// Use this to map a [`crossterm::event::MouseEvent`] to a widget that was
/// rendered at a known [`Rect`].
#[inline]
pub fn mouse_hit(area: Rect, col: u16, row: u16) -> bool {
    area.contains(Position { x: col, y: row })
}

/// Given the **outer** block area (including borders) of a list widget and a
/// click `row`, returns the item index that was clicked.
///
/// Returns `None` if the click lands on a border row or is outside the area.
/// The returned index accounts for the current scroll `offset` — it is an
/// absolute index into the items slice, ready to assign to `ListState::selected`.
///
/// # Example
/// ```rust
/// if let Some(idx) = list_item_at(app.panel_areas[0], app.list_state.offset(), m.row) {
///     if idx < app.items.len() {
///         app.list_state.selected = idx;
///     }
/// }
/// ```
pub fn list_item_at(area: Rect, offset: usize, row: u16) -> Option<usize> {
    let inner_y = area.y + 1;
    let inner_height = area.height.saturating_sub(2);
    if row < inner_y || row >= inner_y + inner_height {
        return None;
    }
    Some(offset + (row - inner_y) as usize)
}

/// Given the **outer** block area of a scrollable paragraph and a click `row`,
/// returns the absolute content line number that was clicked (i.e. accounting
/// for the scroll offset).
///
/// Returns `None` if the click is on a border row.
/// The caller can then match this line number against their tracked link/item
/// line positions to find which element was clicked.
///
/// # Example
/// ```rust
/// if let Some(line) = paragraph_line_at(area, app.desc_scroll as usize, m.row) {
///     if let Some(i) = app.desc_link_lines.iter().position(|&l| l as usize == line) {
///         app.selected_desc_link = i;
///     }
/// }
/// ```
pub fn paragraph_line_at(area: Rect, scroll: usize, row: u16) -> Option<usize> {
    let inner_y = area.y + 1;
    let inner_height = area.height.saturating_sub(2);
    if row < inner_y || row >= inner_y + inner_height {
        return None;
    }
    Some(scroll + (row - inner_y) as usize)
}
