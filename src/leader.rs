//! Vim-style leader key bar — generic, reusable across apps.
//!
//! ## Usage
//!
//! 1. Define your menu tree as a `&'static [LeaderNode]`.
//! 2. Create `LeaderState::new(root)`.
//! 3. Call `arm()` when the leader key is pressed.
//! 4. On each key: call `push(c)` → `LeaderResult`.
//! 5. Call `tick()` each event-loop iteration (handles timeout).
//! 6. Call `render_leader_bar(f, &state, screen_area, &theme)` in your draw fn.
//!
//! ## Label rendering
//!
//! Each [`LeaderNode`] takes a full `label` string (e.g. `"navigate"`, `"find"`).
//! The key character is highlighted inline with `[k]` brackets if it appears in
//! the label, otherwise it is displayed as a prefix:
//!
//! ```text
//! [f]ind         ← 'f' found at position 0
//! e[x]ecute      ← 'x' found at position 1
//! [g] navigate   ← 'g' not in "navigate"
//! ```


use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::Theme;


// ── Data model ────────────────────────────────────────────────────────────────

/// One entry in a leader menu level.
///
/// `label` is the full descriptive word shown to the user (e.g. `"navigate"`,
/// `"find"`, `"create"`). The `key` character is highlighted inline if it
/// appears in `label`, otherwise it is shown as a `[k]` prefix.
///
/// Use [`LeaderNode::divider()`] to insert a visual separator between sections.
#[derive(Debug, Clone, Copy)]
pub struct LeaderNode {
    pub key: char,
    pub label: &'static str,
    pub children: &'static [LeaderNode],
    pub is_divider: bool,
}

impl LeaderNode {
    pub const fn new(key: char, label: &'static str, children: &'static [LeaderNode]) -> Self {
        Self { key, label, children, is_divider: false }
    }

    /// A purely visual horizontal divider — not selectable, ignored by key matching.
    pub const fn divider() -> Self {
        Self { key: '\0', label: "", children: &[], is_divider: true }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

/// Delay before the leader popup first appears after arming. Fast typists
/// never see a flash; once the popup is shown, subsequent keystrokes don't
/// re-trigger the delay.
pub const POPUP_DELAY: std::time::Duration = std::time::Duration::from_millis(200);

pub struct LeaderState {
    pub active: bool,
    sequence: Vec<char>,
    pub root: &'static [LeaderNode],
    /// Time of the last key event that still lies within the initial
    /// `POPUP_DELAY` window. Once the popup has been shown, this field is
    /// frozen — further keystrokes don't hide the popup between submenus.
    last_key_at: Option<std::time::Instant>,
}

/// Outcome of pressing a key while the leader is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaderResult {
    /// Sequence matched a complete leaf — contains the key path pressed.
    Matched(Vec<char>),
    /// Valid prefix — more keys expected.
    Pending,
    /// No match — leader cancelled.
    Cancelled,
}

impl LeaderState {
    pub fn new(root: &'static [LeaderNode]) -> Self {
        Self { active: false, sequence: Vec::new(), root, last_key_at: None }
    }

    /// Arm the leader (start listening).
    pub fn arm(&mut self) {
        self.active = true;
        self.sequence.clear();
        self.last_key_at = Some(std::time::Instant::now());
    }

    /// Cancel and reset.
    pub fn cancel(&mut self) {
        self.active = false;
        self.sequence.clear();
        self.last_key_at = None;
    }

    /// Handle a character key press while leader is active.
    pub fn push(&mut self, c: char) -> LeaderResult {
        // Only reset the hide-timer if the popup has NOT yet been revealed —
        // i.e. we're still inside the initial fast-typing window. Once the
        // popup has been visible, further keystrokes navigate submenus
        // without causing a flash between them.
        let still_hiding = self.last_key_at
            .map(|t| t.elapsed() < POPUP_DELAY)
            .unwrap_or(false);
        if still_hiding {
            self.last_key_at = Some(std::time::Instant::now());
        }
        self.sequence.push(c);

        match self.find_node(&self.sequence.clone()) {
            NodeMatch::Leaf => {
                let path = self.sequence.clone();
                self.cancel();
                LeaderResult::Matched(path)
            }
            NodeMatch::Prefix => LeaderResult::Pending,
            NodeMatch::None => {
                self.cancel();
                LeaderResult::Cancelled
            }
        }
    }

    /// Call once per event-loop tick. Always returns `false` (no timeout).
    pub fn tick(&mut self) -> bool {
        false
    }

    /// The entries visible at the current depth (for rendering).
    pub fn current_entries(&self) -> &'static [LeaderNode] {
        let mut level = self.root;
        for &c in &self.sequence {
            if let Some(node) = level.iter().filter(|n| !n.is_divider).find(|n| n.key == c) {
                level = node.children;
            } else {
                return &[];
            }
        }
        level
    }

    // ── private ───────────────────────────────────────────────────────────────

    fn find_node(&self, path: &[char]) -> NodeMatch {
        let mut level = self.root;
        for (i, &c) in path.iter().enumerate() {
            match level.iter().filter(|n| !n.is_divider).find(|n| n.key == c) {
                None => return NodeMatch::None,
                Some(node) => {
                    if i == path.len() - 1 {
                        if node.children.is_empty() {
                            return NodeMatch::Leaf;
                        } else {
                            return NodeMatch::Prefix;
                        }
                    }
                    level = node.children;
                }
            }
        }
        NodeMatch::Prefix
    }
}

enum NodeMatch { Leaf, Prefix, None }

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Build the display [`Line`] for a single leader entry.
///
/// If `key` appears in `label` (case-insensitive), it is highlighted inline:
/// `[f]ind`, `e[x]ecute`. Otherwise the key is shown as a prefix: `[g] navigate`.
/// Entries with children get a `→` suffix.
fn entry_line(entry: &LeaderNode, theme: &Theme) -> Line<'static> {
    let label = entry.label;
    let key = entry.key;
    let has_children = !entry.children.is_empty();

    let key_lower = key.to_lowercase().next().unwrap_or(key);
    let pos = label
        .char_indices()
        .find(|(_, c)| c.to_lowercase().next().unwrap_or(*c) == key_lower);

    let mut spans = vec![Span::raw(" ")]; // left padding

    if let Some((byte_idx, _)) = pos {
        // Inline highlight: prefix[key]suffix
        let before = &label[..byte_idx];
        let char_len = key.len_utf8();
        let after = &label[byte_idx + char_len..];

        if !before.is_empty() {
            spans.push(Span::styled(before.to_string(), theme.body));
        }
        spans.push(Span::styled(format!("[{}]", key), theme.shortcut_key));
        if !after.is_empty() {
            spans.push(Span::styled(after.to_string(), theme.body));
        }
    } else {
        // Prefix: [key] label
        spans.push(Span::styled(format!("[{}]", key), theme.shortcut_key));
        spans.push(Span::styled(format!(" {}", label), theme.body));
    }

    if has_children {
        spans.push(Span::styled(" →".to_string(), theme.hint));
    }

    Line::from(spans)
}

/// Recursively compute the maximum visible character width of any entry in the
/// tree — used so the popup width stays stable across all submenu levels.
fn max_label_width_recursive(nodes: &[LeaderNode]) -> usize {
    nodes.iter().filter(|n| !n.is_divider).map(|n| {
        let own = n.label.chars().count() + 3 + if !n.children.is_empty() { 2 } else { 0 };
        own.max(max_label_width_recursive(n.children))
    }).max().unwrap_or(0)
}

/// Render the leader bar as a centered column popup.
/// Does nothing when `state.active` is false.
pub fn render_leader_bar(f: &mut Frame, state: &LeaderState, area: Rect, theme: &Theme) {
    if !state.active {
        return;
    }
    // Only show the popup once 200 ms have passed since the last keypress.
    // This way any sequence typed faster than that never causes a flash.
    if state.last_key_at.map(|t| t.elapsed() < POPUP_DELAY).unwrap_or(false) {
        return;
    }

    let entries = state.current_entries();
    let breadcrumb: String = state.sequence.iter().collect();

    // Width: stable across all submenu levels (accounts for deepest label)
    // + 1 left padding + 1 right padding + 2 borders
    let max_entry_chars = max_label_width_recursive(state.root).max(10) as u16;
    let bar_width = (max_entry_chars + 4).min(area.width.saturating_sub(4));

    // Inner width for divider lines (bar_width minus 2 borders)
    let inner_w = bar_width.saturating_sub(2) as usize;

    // Build content lines with spacing:
    //   - blank line between same-section entries
    //   - blank line before AND after each divider
    let mut padded: Vec<Line> = vec![Line::from("")]; // top padding
    for (i, entry) in entries.iter().enumerate() {
        if entry.is_divider {
            // Blank before divider (unless it's the very first item)
            if i > 0 {
                padded.push(Line::from(""));
            }
            let rule = "─".repeat(inner_w);
            padded.push(Line::from(Span::styled(rule, theme.separator)));
            // Blank after divider is handled by next entry's leading blank
        } else {
            let prev_is_divider = i > 0 && entries[i - 1].is_divider;
            if i > 0 {
                padded.push(Line::from("")); // blank before every non-first entry
            }
            padded.push(entry_line(entry, theme));
            let _ = prev_is_divider; // blank already pushed above
        }
    }
    padded.push(Line::from("")); // bottom padding

    // Height derived from actual content
    let bar_height = (padded.len() as u16) + 2; // +2 borders

    // Position: centered horizontally and vertically within `area`
    let x = area.x + area.width.saturating_sub(bar_width) / 2;
    let y = area.y + area.height.saturating_sub(bar_height) / 2;
    let bar_area = Rect { x, y, width: bar_width, height: bar_height };

    let title = if breadcrumb.is_empty() {
        " ⌁ ".to_string()
    } else {
        format!(" ⌁ {} › ", breadcrumb)
    };

    f.render_widget(Clear, bar_area);
    let paragraph = Paragraph::new(Text::from(padded))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_popup)
                .title(title)
                .title_style(theme.section_header)
                .title_alignment(Alignment::Center),
        );
    f.render_widget(paragraph, bar_area);
}
