//! `tui-kit` — reusable TUI theme, widget frames, and layout helpers.
//!
//! Built on top of [ratatui](https://github.com/ratatui-org/ratatui).
//! Designed to be shared across multiple terminal-UI projects that follow
//! the same visual design language.
//!
//! ## Modules
//!
//! | Module     | Contents |
//! |------------|----------|
//! | [`theme`]  | [`Theme`] struct — the full color/style palette |
//! | [`block`]  | [`block::panel_block`], [`block::popup_block`], [`block::widget_title`], [`block::focusable_block`] |
//! | [`footer`] | [`render_footer`], [`render_footer_with_app`], [`keybind_spans`] — keybind bar |
//! | [`tabs`]   | [`tabs::tab_line`] — horizontal tab-bar line for block titles |
//! | [`popup`]  | [`popup::centered_popup`] — centered overlay area + Clear |
//! | [`leader`] | [`render_leader_bar`] — vim-style leader key popup |
//! | [`mouse`]  | [`mouse_hit`] — hit-test a click against a widget [`Rect`] |
//! | [`toast`]  | [`render_toasts`] — stacked notification toasts |
//! | [`list`]   | [`list::render_list`] — scrollable focusable list |
//! | [`form`]   | [`render_form`] — multi-field input form |
//! | [`kv`]     | [`kv::render_kv_table`] — two-column key/value table |
//!
//! ## Keybind bar convention
//!
//! Every project should show a footer keybind bar using [`render_footer`].
//! The visual format is identical across all projects:
//!
//! ```text
//! Navigate: j/k  |  Open: Enter  |  Leader: space  |  Quit: q
//! ```
//!
//! Keys are styled with [`Theme::shortcut_key`] (yellow), actions and
//! separators with [`Theme::hint`] (dark gray).
//!
//! Build pairs contextually based on the active UI layer (popup vs main screen):
//!
//! ```rust
//! let mut pairs: Vec<(&str, &str)> = Vec::new();
//! if some_popup_open {
//!     pairs.push(("Esc",   "close"));
//!     pairs.push(("Enter", "confirm"));
//! } else {
//!     pairs.push(("j/k",   "navigate"));
//!     pairs.push(("space", "leader"));
//!     pairs.push(("q",     "quit"));
//! }
//! render_footer(f, footer_area, &pairs, &theme);
//! ```

pub mod block;
pub mod footer;
pub mod form;
pub mod kv;
pub mod leader;
pub mod list;
pub mod log;
pub mod mouse;
pub mod picker;
pub mod popup;
pub mod tabs;
pub mod theme;
pub mod toast;

pub use block::render_scrollbar;
pub use footer::{keybind_spans, render_footer, render_footer_with_app};
pub use mouse::{list_item_at, mouse_hit, paragraph_line_at};
pub use form::{FieldInput, FormEvent, FormField, FormState, render_form};
pub use kv::KvRow;
pub use leader::{LeaderNode, LeaderResult, LeaderState, render_leader_bar};
pub use list::{ListItem, ListState};
pub use log::{LogEntry, LogLevel};
pub use picker::{PickerItem, PickerState, render_picker};
pub use theme::Theme;
pub use toast::{Toast, ToastLevel, render_toasts};
