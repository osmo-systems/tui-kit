use ratatui::{layout::Rect, widgets::Clear, Frame};

/// Computes a centered popup [`Rect`], renders a [`Clear`] over it (erasing
/// whatever is behind), and returns the area to draw into.
///
/// # Parameters
///
/// - `width_pct`  — fraction of the screen width (0.0–1.0).
/// - `max_width`  — hard cap in columns.
/// - `height`     — fixed row count (clamped to screen height).
///
/// # Example
///
/// ```ignore
/// let area = centered_popup(f, 0.7, 90, 26);
/// f.render_widget(popup_block(title, &theme), area);
/// ```
pub fn centered_popup(f: &mut Frame, width_pct: f32, max_width: u16, height: u16) -> Rect {
    let screen = f.area();
    let width = (screen.width as f32 * width_pct).min(max_width as f32) as u16;
    let height = height.min(screen.height);
    let x = screen.width.saturating_sub(width) / 2;
    let y = screen.height.saturating_sub(height) / 2;
    let area = Rect { x, y, width, height };
    f.render_widget(Clear, area);
    area
}
