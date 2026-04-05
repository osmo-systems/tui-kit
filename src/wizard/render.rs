//! Rendering logic for the wizard widget.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::Theme;
use super::types::{WizardState, WizardStep, WizardStepKind};
use super::helpers::max_sub_label;

// ── Depth prefix ──────────────────────────────────────────────────────────────

/// Build the indentation prefix spans and column offset for any nesting depth.
///
/// Depth 0: `"  "`  (2 cols)
/// Each additional level appends `"│  "` (+3 cols).
fn depth_prefix(depth: u8, theme: &Theme) -> (Vec<Span<'static>>, usize) {
    let mut spans: Vec<Span<'static>> = vec![Span::raw("  ")];
    let mut cols = 2usize;
    for _ in 0..depth {
        spans.push(Span::styled("│", theme.hint));
        spans.push(Span::raw("  "));
        cols += 3;
    }
    (spans, cols)
}

// ── Bullet helpers ────────────────────────────────────────────────────────────

fn bullet(completed: bool, active: bool, section_active: bool) -> &'static str {
    if section_active   { "◉" }
    else if completed   { "●" }
    else if active      { "○" }
    else                { "◌" }
}

// ── RenderCtx ─────────────────────────────────────────────────────────────────

pub(super) struct RenderCtx<'a> {
    pub state:       &'a WizardState,
    /// DFS leaf counter — mutated as we walk the tree.
    pub leaf:        usize,
    pub lines:       Vec<Line<'static>>,
    /// Content-line index of the active input row (for terminal cursor placement).
    pub cursor_line: Option<usize>,
    /// Display-column offset for the terminal cursor within that line.
    pub cursor_col:  usize,
    pub theme:       &'a Theme,
}

impl<'a> RenderCtx<'a> {
    // ── Low-level line builders ───────────────────────────────────────────────

    fn push_connector(&mut self, depth: u8) {
        let (spans, _) = depth_prefix(depth, self.theme);
        // Connectors show the vertical bar for the *next* level — so they use
        // depth+1 prefix but we just want to append a │ at the current depth.
        // Simplest correct form: re-use depth_prefix but emit connector bar.
        let line = {
            let (mut s, _) = depth_prefix(depth, self.theme);
            s.push(Span::styled("│", self.theme.hint));
            Line::from(s)
        };
        // discard unused spans from first call
        let _ = spans;
        self.lines.push(line);
    }

    /// Push one labelled step row.  Returns the column where the value starts.
    fn push_step_row(
        &mut self,
        label: &'static str,
        value_text: &str,
        bul: &'static str,
        bul_style: ratatui::style::Style,
        depth: u8,
        label_pad: usize,
    ) -> usize {
        let (mut spans, prefix_cols) = depth_prefix(depth, self.theme);
        let value_col = prefix_cols + 1 + 2 + label_pad + 2;
        spans.push(Span::styled(bul, bul_style));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{:<pad$}  ", label, pad = label_pad),
            self.theme.shortcut_key,
        ));
        if !value_text.is_empty() {
            spans.push(Span::styled(value_text.to_string(), self.theme.body));
        }
        self.lines.push(Line::from(spans));
        value_col
    }

    // ── Render a slice of steps ───────────────────────────────────────────────

    pub(super) fn render_steps(&mut self, steps: &'static [WizardStep], depth: u8, skip_first_connector: bool) {
        let max_label = steps.iter().map(|s| s.label.chars().count()).max().unwrap_or(4);

        for (i, step) in steps.iter().enumerate() {
            if !skip_first_connector || i > 0 {
                self.push_connector(depth);
            }

            match &step.kind {

                // ── Section ──────────────────────────────────────────────────
                WizardStepKind::Section(children) => {
                    let first_leaf = self.leaf;
                    let n_leaves   = super::helpers::count_leaves(children);
                    let last_leaf  = first_leaf + n_leaves.saturating_sub(1);

                    let completed  = self.state.current > last_leaf;
                    let active     = self.state.current >= first_leaf && self.state.current <= last_leaf;
                    let bul        = bullet(completed, active, active);
                    let bul_style  = if active { self.theme.shortcut_key } else { self.theme.hint };

                    self.push_step_row(step.label, "", bul, bul_style, depth, max_label);

                    if active || completed {
                        self.render_steps(children, depth + 1, false);
                        self.push_connector(depth);
                    }
                    if !active && !completed { self.leaf += n_leaves; }
                }

                // ── Array ─────────────────────────────────────────────────────
                WizardStepKind::Array(sub_steps) => {
                    let leaf_idx   = self.leaf;
                    self.leaf += 1;

                    let completed  = self.state.current > leaf_idx;
                    let active     = self.state.current == leaf_idx;
                    let arr        = self.state.array_states.get(&leaf_idx);
                    let item_count = arr.map(|a| a.items.len()).unwrap_or(0);
                    let is_expanded = arr.map(|a| a.expanded).unwrap_or(false);
                    let header_sel  = arr.map(|a| a.header_sel).unwrap_or(0);
                    let arr_selected = arr.map(|a| a.selected).unwrap_or(0);
                    let item_btn_sel = arr.map(|a| a.item_btn_sel).unwrap_or(0);
                    let editing_item = arr.and_then(|a| a.editing.as_ref()).map(|s| s.item_idx);

                    let has_items  = item_count > 0 || editing_item.is_some();
                    let bul        = bullet(completed, active, active && has_items && is_expanded);
                    let bul_style  = if active || completed { self.theme.shortcut_key } else { self.theme.hint };

                    // Header row: "○  fields    [ + add ]  [n]"
                    // When collapsed & active: the focused header button is highlighted.
                    // When expanded or completed: buttons shown dimly.
                    {
                        let (add_style, badge_style) = if active && !is_expanded {
                            (
                                if header_sel == 0 { self.theme.selection } else { self.theme.hint },
                                if header_sel == 1 { self.theme.selection } else { self.theme.hint },
                            )
                        } else {
                            (self.theme.hint, self.theme.hint)
                        };
                        let (mut spans, _) = depth_prefix(depth, self.theme);
                        spans.push(Span::styled(bul, bul_style));
                        spans.push(Span::raw("  "));
                        spans.push(Span::styled(
                            format!("{:<pad$}  ", step.label, pad = max_label),
                            self.theme.shortcut_key,
                        ));
                        if active || completed {
                            spans.push(Span::styled("[ + add ]", add_style));
                            spans.push(Span::raw("  "));
                        }
                        spans.push(Span::styled(format!("[{}]", item_count), badge_style));
                        self.lines.push(Line::from(spans));
                    }

                    // Items — only shown when expanded (respects collapsed state even when completed)
                    if is_expanded && (active || completed) {
                        let items = self.state.array_states.get(&leaf_idx)
                            .map(|a| a.items.clone()).unwrap_or_default();
                        let lp = max_sub_label(sub_steps);

                        for j in 0..items.len() {
                            self.push_connector(depth + 1);
                            let is_editing  = editing_item == Some(j);
                            let is_selected = active && editing_item.is_none() && j == arr_selected;

                            // Item header: "●  #j  [ remove ]" (remove only when selected)
                            let (item_bul, item_bul_style) = if is_editing {
                                ("○", self.theme.shortcut_key)
                            } else if is_selected {
                                ("►", self.theme.selection)
                            } else {
                                ("●", self.theme.hint)
                            };
                            let (mut header_spans, _) = depth_prefix(depth + 1, self.theme);
                            header_spans.push(Span::styled(item_bul, item_bul_style));
                            header_spans.push(Span::raw("  "));
                            header_spans.push(Span::styled(format!("#{}", j + 1), item_bul_style));
                            if is_selected && !is_editing {
                                header_spans.push(Span::raw("  "));
                                let remove_style = if item_btn_sel == 1 { self.theme.selection } else { self.theme.hint };
                                header_spans.push(Span::styled("[ remove ]", remove_style));
                            }
                            self.lines.push(Line::from(header_spans));

                            // Sub-steps beneath item header
                            if is_editing {
                                let session    = self.state.array_states[&leaf_idx].editing.as_ref().unwrap();
                                let sub_step   = session.sub_step;
                                let buffer     = session.buffer.clone();
                                let buf_cursor = session.buf_cursor;
                                let select_idx = session.select_idx;
                                let item_vals  = items[j].clone();
                                self.push_edit_rows(sub_steps, depth + 2, lp, sub_step, &buffer, buf_cursor, select_idx, &item_vals);
                            } else {
                                for (k, sub) in sub_steps.iter().enumerate() {
                                    self.push_connector(depth + 2);
                                    let val     = items[j].get(k).cloned().unwrap_or_default();
                                    let display = if val.is_empty() { "(none)".to_string() } else { val };
                                    self.push_step_row(sub.label, &display, "●", self.theme.hint, depth + 2, lp);
                                }
                            }
                        }

                        self.push_connector(depth);
                    }
                }

                // ── Buttons ───────────────────────────────────────────────────
                WizardStepKind::Buttons(labels) => {
                    let leaf_idx = self.leaf;
                    self.leaf += 1;
                    let completed = self.state.current > leaf_idx;
                    let active    = self.state.current == leaf_idx;

                    let (mut spans, _) = depth_prefix(depth, self.theme);
                    let bul_ch = if active { "○" } else if completed { "●" } else { "◌" };
                    let bul_st = if active { self.theme.shortcut_key } else { self.theme.hint };
                    spans.push(Span::styled(bul_ch, bul_st));
                    spans.push(Span::raw("  "));
                    for (j, &lbl) in labels.iter().enumerate() {
                        if j > 0 { spans.push(Span::raw("  ")); }
                        let is_sel = active && j == self.state.button_selected;
                        let style = if is_sel { self.theme.selection }
                                    else if active || completed { self.theme.shortcut_key }
                                    else { self.theme.hint };
                        spans.push(Span::styled(format!("[ {} ]", lbl), style));
                    }
                    self.lines.push(Line::from(spans));
                }

                // ── Regular leaf (Leaf, Optional, Select) ─────────────────────
                _ => {
                    let leaf_idx = self.leaf;
                    self.leaf += 1;

                    let active = self.state.current == leaf_idx;
                    let has_stored = leaf_idx < self.state.values.len()
                        && !self.state.values[leaf_idx].is_empty();
                    // Show as completed if past OR if it has a stored value (cycling support)
                    let completed = !active && (self.state.current > leaf_idx || has_stored);
                    let bul       = bullet(completed, active, false);
                    let bul_style = if active || completed { self.theme.shortcut_key } else { self.theme.hint };

                    let value_text: String = if completed {
                        let v = self.state.values.get(leaf_idx).map(|s| s.as_str()).unwrap_or("");
                        if v.is_empty() { "(none)".into() } else { v.into() }
                    } else if active && leaf_idx < self.state.input_count {
                        match &step.kind {
                            WizardStepKind::Select(opts) => opts.first().copied().unwrap_or("").to_string(),
                            _ => self.state.buffer.clone(),
                        }
                    } else {
                        String::new()
                    };

                    let value_col = self.push_step_row(step.label, &value_text, bul, bul_style, depth, max_label);

                    if active && leaf_idx < self.state.input_count
                        && matches!(step.kind, WizardStepKind::Leaf | WizardStepKind::Optional)
                    {
                        let cursor_display = UnicodeWidthStr::width(&self.state.buffer[..self.state.cursor]);
                        self.cursor_line = Some(self.lines.len() - 1);
                        self.cursor_col  = value_col + cursor_display;
                    }
                }
            }
        }
    }

    // ── In-progress item edit rows ────────────────────────────────────────────

    fn push_edit_rows(
        &mut self,
        sub_steps: &'static [WizardStep],
        depth: u8,
        label_pad: usize,
        sub_step: usize,
        buffer: &str,
        buf_cursor: usize,
        select_idx: usize,
        item_values: &[String],
    ) {
        for (i, sub) in sub_steps.iter().enumerate() {
            if i > 0 { self.push_connector(depth); }

            let is_active = i == sub_step;
            let bul       = if is_active { "○" } else if i < sub_step { "●" } else { "◌" };
            let bul_style = if is_active { self.theme.shortcut_key } else { self.theme.hint };

            let value_text: String = match &sub.kind {
                WizardStepKind::Select(opts) if is_active => format!("◀ {} ▶", opts[select_idx]),
                _ if is_active                            => buffer.to_string(),
                _ if i < sub_step                         => item_values.get(i).cloned().unwrap_or_default(),
                _                                         => String::new(),
            };

            let (mut spans, prefix_cols) = depth_prefix(depth, self.theme);
            spans.push(Span::styled(bul, bul_style));
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("{:<pad$}  ", sub.label, pad = label_pad),
                if is_active { self.theme.shortcut_key } else { self.theme.hint },
            ));
            if !value_text.is_empty() {
                spans.push(Span::styled(value_text.clone(), self.theme.body));
            }
            self.lines.push(Line::from(spans));

            if is_active && matches!(sub.kind, WizardStepKind::Leaf | WizardStepKind::Optional) {
                let value_col  = prefix_cols + 1 + 2 + label_pad + 2;
                let cursor_disp = UnicodeWidthStr::width(&buffer[..buf_cursor]);
                self.cursor_line = Some(self.lines.len() - 1);
                self.cursor_col  = value_col + cursor_disp;
            }
        }
    }
}

// ── Public render entry point ─────────────────────────────────────────────────

/// Render the wizard as a popup inside `area`.
pub fn render_wizard(f: &mut Frame, state: &WizardState, area: Rect, title: &str, theme: &Theme) {
    let mut ctx = RenderCtx {
        state,
        leaf: 0,
        lines: vec![],
        cursor_line: None,
        cursor_col: 0,
        theme,
    };

    ctx.render_steps(state.steps, 0, true);

    let paragraph = Paragraph::new(ratatui::text::Text::from(ctx.lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_popup)
                .title(title.to_string())
                .title_style(theme.border_popup.add_modifier(ratatui::style::Modifier::BOLD)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);

    if let Some(line) = ctx.cursor_line {
        let inner = area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
        let cx = inner.x + ctx.cursor_col as u16;
        let cy = inner.y + line as u16;
        if cx < inner.x + inner.width && cy < inner.y + inner.height {
            f.set_cursor_position((cx, cy));
        }
    }
}
