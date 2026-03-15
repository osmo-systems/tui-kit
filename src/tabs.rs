use ratatui::text::{Line, Span};

use crate::Theme;

/// Builds a horizontal tab-bar [`Line`] from a list of `(label, is_active)` pairs.
///
/// Active tab uses [`Theme::tab_active`], inactive tabs use [`Theme::tab_inactive`].
/// Tabs are separated by two spaces. A leading and trailing space is added so the
/// line reads cleanly inside a block title.
///
/// The returned `Line` is `'static` (all content is owned) so it can be used
/// freely as a block title without lifetime concerns.
///
/// # Example
///
/// ```ignore
/// let title = tab_line(&[
///     ("Browse",    matches!(tab, Tab::Browse)),
///     ("Favorites", matches!(tab, Tab::Favorites)),
///     ("History",   matches!(tab, Tab::History)),
/// ], &theme);
/// ```
pub fn tab_line(tabs: &[(&str, bool)], theme: &Theme) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];

    for (i, (label, active)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let style = if *active { theme.tab_active } else { theme.tab_inactive };
        spans.push(Span::styled(label.to_string(), style));
    }

    spans.push(Span::raw(" "));
    Line::from(spans)
}
