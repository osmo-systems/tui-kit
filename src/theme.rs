use ratatui::style::{Color, Modifier, Style};

/// Color and style palette for tui-kit components.
///
/// ## Usage
///
/// Use [`Theme::native()`] (the default) for a palette that adapts to the
/// user's terminal color scheme, or [`Theme::dark()`] for a fixed
/// lazygit-inspired dark palette.
///
/// ## Extending for your app
///
/// Wrap `Theme` in your own struct for app-specific semantic roles:
///
/// ```rust
/// pub struct AppTheme {
///     pub base: Theme,            // passed to all tui-kit calls
///     pub my_semantic: Style,     // app-specific roles
/// }
///
/// impl AppTheme {
///     pub fn native() -> Self {
///         Self {
///             base: Theme::native(),
///             my_semantic: Style::default().fg(Color::LightGreen),
///         }
///     }
/// }
/// ```
///
/// `Theme` is `Copy` so it is cheap to pass by value or reference.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Border of the currently-focused interactive panel.
    pub border_focused: Style,
    /// Border of passive / display-only panels.
    pub border_unfocused: Style,
    /// Border of floating popups (always treated as focused).
    pub border_popup: Style,
    /// Border of error popups.
    pub border_error: Style,
    /// Border of warning popups.
    pub border_warning: Style,

    /// Label of the active tab.
    pub tab_active: Style,
    /// Label of an inactive tab.
    pub tab_inactive: Style,

    /// Style applied to the selected row in a list (fg + bg combined).
    pub selection: Style,

    /// Inline keyboard shortcut labels (e.g. "Enter", "Tab").
    pub shortcut_key: Style,
    /// The `-[n]-` digit indicator shown in widget titles.
    pub shortcut_indicator: Style,

    /// Section / group headers inside popups.
    pub section_header: Style,
    /// Normal body text.
    pub body: Style,
    /// Dimmed hint and footer text.
    pub hint: Style,
    /// Separator lines (─ characters).
    pub separator: Style,

    /// Positive / success signal (e.g. toast success, CI pass).
    pub success: Style,
}

impl Theme {
    /// 16-color native palette — adapts to the user's terminal color scheme.
    ///
    /// Uses only the 16 named ANSI colors so the result respects the terminal
    /// theme (dark/light, Solarized, Nord, etc.).
    ///
    /// | Role               | Color          |
    /// |--------------------|----------------|
    /// | Accent / focused   | LightBlue      |
    /// | Tabs / headers     | LightBlue      |
    /// | Shortcuts          | Yellow         |
    /// | Selection          | Black on LightBlue |
    /// | Positive signal    | LightGreen     |
    /// | Negative signal    | LightRed       |
    /// | Muted / hints      | DarkGray       |
    pub fn native() -> Self {
        Self {
            border_focused:   Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
            border_unfocused: Style::default().fg(Color::DarkGray),
            border_popup:     Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
            border_error:     Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
            border_warning:   Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),

            tab_active:   Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(Color::DarkGray),

            selection: Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),

            shortcut_key:       Style::default().fg(Color::Yellow),
            shortcut_indicator: Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),

            section_header: Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
            body:           Style::default().fg(Color::White),
            hint:           Style::default().fg(Color::DarkGray),
            separator:      Style::default().fg(Color::DarkGray),

            success: Style::default().fg(Color::LightGreen),
        }
    }

    /// Fixed dark palette with green accents — does not adapt to terminal theme.
    ///
    /// Use when you know the terminal has a dark background and want a
    /// consistent lazygit-inspired look regardless of terminal settings.
    ///
    /// | Role               | Color        |
    /// |--------------------|--------------|
    /// | Accent / focused   | Green + Bold |
    /// | Unfocused border   | White        |
    /// | Active tab         | Green + Bold |
    /// | Selection          | White on Blue + Bold |
    /// | Shortcut keys      | Yellow       |
    /// | Section headers    | Cyan + Bold  |
    /// | Hints              | DarkGray     |
    pub fn dark() -> Self {
        Self {
            border_focused:   Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            border_unfocused: Style::default().fg(Color::White),
            border_popup:     Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            border_error:     Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            border_warning:   Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),

            tab_active:   Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(Color::White),

            selection: Style::default()
                .fg(Color::Indexed(7))
                .bg(Color::Indexed(6))
                .add_modifier(Modifier::BOLD),

            shortcut_key:       Style::default().fg(Color::Yellow),
            shortcut_indicator: Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),

            section_header: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            body:           Style::default().fg(Color::White),
            hint:           Style::default().fg(Color::DarkGray),
            separator:      Style::default().fg(Color::Green),

            success: Style::default().fg(Color::Green),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::native()
    }
}
