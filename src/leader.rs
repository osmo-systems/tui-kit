//! Vim-style leader key bar — generic, reusable across apps.
//!
//! ## Usage
//!
//! 1. Define your menu tree as a `&'static [LeaderNode]`.
//! 2. Create `LeaderState::new(root)`.
//! 3. Call `arm()` when the leader key is pressed.
//! 4. On each key: call `push(c)` → `LeaderResult`.
//! 5. Call `tick()` each event-loop iteration (handles timeout).
//! 6. Call `render_leader_bar(f, &state, screen_area)` in your draw fn.

use std::time::{Duration, Instant};

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const TIMEOUT: Duration = Duration::from_millis(2000);

// ── Data model ────────────────────────────────────────────────────────────────

/// One entry in a leader menu level.
/// The displayed label is `[key]suffix`, e.g. key='f' suffix="ind" → `[f]ind`.
#[derive(Debug, Clone, Copy)]
pub struct LeaderNode {
    pub key: char,
    pub suffix: &'static str,
    pub children: &'static [LeaderNode],
}

impl LeaderNode {
    pub const fn new(key: char, suffix: &'static str, children: &'static [LeaderNode]) -> Self {
        Self { key, suffix, children }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

pub struct LeaderState {
    pub active: bool,
    sequence: Vec<char>,
    armed_at: Option<Instant>,
    pub root: &'static [LeaderNode],
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
        Self { active: false, sequence: Vec::new(), armed_at: None, root }
    }

    /// Arm the leader (start listening).
    pub fn arm(&mut self) {
        self.active = true;
        self.sequence.clear();
        self.armed_at = Some(Instant::now());
    }

    /// Cancel and reset.
    pub fn cancel(&mut self) {
        self.active = false;
        self.sequence.clear();
        self.armed_at = None;
    }

    /// Handle a character key press while leader is active.
    pub fn push(&mut self, c: char) -> LeaderResult {
        self.armed_at = Some(Instant::now());
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

    /// Call once per event-loop tick. Returns `true` if the sequence timed out.
    pub fn tick(&mut self) -> bool {
        if let Some(t) = self.armed_at {
            if t.elapsed() > TIMEOUT {
                self.cancel();
                return true;
            }
        }
        false
    }

    /// The entries visible at the current depth (for rendering).
    pub fn current_entries(&self) -> &'static [LeaderNode] {
        let mut level = self.root;
        for &c in &self.sequence {
            if let Some(node) = level.iter().find(|n| n.key == c) {
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
            match level.iter().find(|n| n.key == c) {
                None => return NodeMatch::None,
                Some(node) => {
                    if i == path.len() - 1 {
                        // Last key in path
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

/// Render the leader bar as a centered overlay at the bottom of `area`.
/// Does nothing when `state.active` is false.
pub fn render_leader_bar(f: &mut Frame, state: &LeaderState, area: Rect) {
    if !state.active {
        return;
    }

    let entries = state.current_entries();

    // Build breadcrumb prefix: show pressed keys so far
    let breadcrumb: String = state.sequence.iter().collect();

    // Build entry spans: `[f]ind  `
    let mut entry_spans: Vec<Span> = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            entry_spans.push(Span::raw("   "));
        }
        entry_spans.push(Span::styled(
            format!("[{}]", entry.key),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        entry_spans.push(Span::styled(
            entry.suffix,
            Style::default().fg(Color::White),
        ));
    }

    if entry_spans.is_empty() {
        entry_spans.push(Span::styled(
            "(no actions)",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Measure content width
    let content: String = entries.iter().enumerate().map(|(i, e)| {
        format!("{}[{}]{}", if i > 0 { "   " } else { "" }, e.key, e.suffix)
    }).collect();
    let inner_width = content.len().max(12) as u16 + 4; // +4 for padding

    // Bar height: 1 line of entries + borders = 3 rows
    let bar_height = 3u16;
    let bar_width = inner_width.min(area.width.saturating_sub(4));

    // Position: centered horizontally, just above the bottom of `area`
    let x = area.x + area.width.saturating_sub(bar_width) / 2;
    let y = area.y + area.height.saturating_sub(bar_height + 1); // +1 to sit above status bar

    let bar_area = Rect { x, y, width: bar_width, height: bar_height };

    // Build title: ` ⌁ ` or ` ⌁ f › `
    let title = if breadcrumb.is_empty() {
        " ⌁ ".to_string()
    } else {
        format!(" ⌁ {} › ", breadcrumb)
    };

    f.render_widget(Clear, bar_area);
    let paragraph = Paragraph::new(Line::from(entry_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .title_alignment(Alignment::Center),
        )
        .alignment(Alignment::Center);
    f.render_widget(paragraph, bar_area);
}
