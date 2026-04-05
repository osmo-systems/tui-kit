//! Key-event handling for WizardState.

use crossterm::event::{KeyCode, KeyEvent};

use super::types::{
    ArrayEditSession, ArrayState, StepKindRef, WizardEvent, WizardState, WizardStep,
    WizardStepKind,
};
use super::helpers::{next_char_boundary, prev_char_boundary, step_kind_at};

impl WizardState {
    // ── Public constructor / accessors ────────────────────────────────────────

    /// Create a fresh wizard.  `input_count` ≤ total leaf count.
    pub fn new(steps: &'static [WizardStep], input_count: usize) -> Self {
        Self {
            steps, input_count, current: 0,
            values: Vec::new(), buffer: String::new(), cursor: 0,
            array_states: std::collections::HashMap::new(), button_selected: 0,
        }
    }

    /// Build a pre-completed wizard (edit mode — values already known).
    pub fn completed(
        steps: &'static [WizardStep],
        input_count: usize,
        values: Vec<String>,
        start_current: usize,
    ) -> Self {
        Self {
            steps, input_count, current: start_current,
            values, buffer: String::new(), cursor: 0,
            array_states: std::collections::HashMap::new(), button_selected: 0,
        }
    }

    /// Pre-populate an Array step with existing items (edit mode).
    pub fn preload_array(&mut self, step_idx: usize, items: Vec<Vec<String>>) {
        let arr = self.array_states.entry(step_idx).or_insert_with(ArrayState::new);
        arr.items = items;
        arr.expanded = true;
    }

    /// True when all `input_count` steps have been confirmed.
    pub fn is_complete(&self) -> bool { self.current >= self.input_count }

    /// Set the active leaf index directly (for externally-managed steps).
    pub fn set_current(&mut self, leaf_idx: usize) { self.current = leaf_idx; }

    /// Borrow the state of an Array step by DFS-leaf index.
    pub fn array_state(&self, leaf_idx: usize) -> Option<&ArrayState> {
        self.array_states.get(&leaf_idx)
    }

    /// Mutably borrow the state of an Array step by DFS-leaf index.
    pub fn array_state_mut(&mut self, leaf_idx: usize) -> Option<&mut ArrayState> {
        self.array_states.get_mut(&leaf_idx)
    }

    /// Return the currently highlighted item index (0-based) for an Array step,
    /// or `None` if no item is focused (e.g. array is collapsed).
    pub fn array_selected_item(&self, leaf_idx: usize) -> Option<usize> {
        self.array_states.get(&leaf_idx).and_then(|a| {
            if a.expanded && !a.items.is_empty() { Some(a.selected.min(a.items.len().saturating_sub(1))) }
            else { None }
        })
    }

    /// True when the current step is an Array with an active editing session.
    pub fn is_array_editing(&self) -> bool {
        matches!(self.current_kind(), Some(StepKindRef::Array(_)))
            && self.array_states.get(&self.current)
                   .and_then(|a| a.editing.as_ref()).is_some()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    pub(super) fn current_kind(&self) -> Option<StepKindRef> {
        step_kind_at(self.steps, self.current, &mut 0)
    }

    /// If the step we just landed on is an Array, reset it to collapsed / add focused.
    fn reset_array_on_arrival(&mut self) {
        if matches!(self.current_kind(), Some(StepKindRef::Array(_))) {
            if let Some(arr) = self.array_states.get_mut(&self.current) {
                arr.expanded    = false;
                arr.selected    = 0;
                arr.header_sel  = 0;
                arr.item_btn_sel = 0;
            }
        }
    }

    /// Store `value` at the current leaf index (update-in-place or push).
    fn commit_value(&mut self, value: String) {
        if self.current < self.values.len() {
            self.values[self.current] = value;
        } else {
            self.values.push(value);
        }
    }

    /// Load the stored value for `leaf_idx` into the buffer, restoring cursor to end.
    fn load_buffer_for(&mut self, leaf_idx: usize) {
        self.buffer = self.values.get(leaf_idx).cloned().unwrap_or_default();
        self.cursor = self.buffer.len();
    }

    // ── Public key handler ────────────────────────────────────────────────────

    /// Handle a key event. Only meaningful when `!is_complete()`.
    pub fn handle_key(&mut self, key: KeyEvent) -> WizardEvent {
        if self.is_complete() { return WizardEvent::None; }
        let kind = match self.current_kind() {
            Some(k) => k,
            None    => return WizardEvent::None,
        };
        match kind {
            StepKindRef::Array(sub_steps) => {
                let array_idx  = self.current;
                let is_editing = self.array_states.get(&array_idx)
                    .and_then(|a| a.editing.as_ref()).is_some();
                if is_editing {
                    self.handle_array_edit_key(key, array_idx, sub_steps)
                } else {
                    self.handle_array_browse_key(key, array_idx, sub_steps)
                }
            }
            StepKindRef::Leaf | StepKindRef::Optional => self.handle_leaf_key(key),
            StepKindRef::Select(_) => match key.code {
                KeyCode::Tab | KeyCode::Enter => {
                    let index = self.current;
                    self.commit_value(String::new());
                    self.current += 1;
                    if self.is_complete() { WizardEvent::Done(self.values.clone()) }
                    else { WizardEvent::StepCompleted { index, value: String::new() } }
                }
                KeyCode::Esc => WizardEvent::Cancelled,
                _ => WizardEvent::None,
            },
            StepKindRef::Buttons(labels) => self.handle_buttons_key(key, labels),
        }
    }

    // ── Leaf step ─────────────────────────────────────────────────────────────

    fn handle_leaf_key(&mut self, key: KeyEvent) -> WizardEvent {
        match key.code {
            KeyCode::Char(c) => {
                self.buffer.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                WizardEvent::None
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let start = prev_char_boundary(&self.buffer, self.cursor);
                    let len   = self.cursor - start;
                    self.buffer.drain(start..self.cursor);
                    self.cursor -= len;
                }
                WizardEvent::None
            }
            KeyCode::Delete => {
                if self.cursor < self.buffer.len() { self.buffer.remove(self.cursor); }
                WizardEvent::None
            }
            KeyCode::Left  => { if self.cursor > 0 { self.cursor = prev_char_boundary(&self.buffer, self.cursor); } WizardEvent::None }
            KeyCode::Right => { if self.cursor < self.buffer.len() { self.cursor = next_char_boundary(&self.buffer, self.cursor); } WizardEvent::None }
            KeyCode::Home  => { self.cursor = 0; WizardEvent::None }
            KeyCode::End   => { self.cursor = self.buffer.len(); WizardEvent::None }
            KeyCode::Enter | KeyCode::Tab => {
                let value = std::mem::take(&mut self.buffer);
                self.cursor = 0;
                let index = self.current;
                self.commit_value(value.clone());
                self.current += 1;
                if self.is_complete() {
                    WizardEvent::Done(self.values.clone())
                } else {
                    self.load_buffer_for(self.current);
                    self.reset_array_on_arrival();
                    WizardEvent::StepCompleted { index, value }
                }
            }
            KeyCode::BackTab => {
                if self.current > 0 {
                    self.current -= 1;
                } else {
                    self.current = self.input_count.saturating_sub(1);
                }
                self.load_buffer_for(self.current);
                self.reset_array_on_arrival();
                WizardEvent::None
            }
            KeyCode::Esc => WizardEvent::Cancelled,
            _ => WizardEvent::None,
        }
    }

    // ── Buttons step ──────────────────────────────────────────────────────────

    fn handle_buttons_key(&mut self, key: KeyEvent, labels: &'static [&'static str]) -> WizardEvent {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                if self.button_selected > 0 { self.button_selected -= 1; }
                WizardEvent::None
            }
            KeyCode::BackTab => {
                if self.button_selected > 0 {
                    self.button_selected -= 1;
                } else {
                    self.button_selected = 0;
                    self.current = self.current.saturating_sub(1);
                    self.load_buffer_for(self.current);
                    self.reset_array_on_arrival();
                }
                WizardEvent::None
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                if self.button_selected + 1 < labels.len() {
                    self.button_selected += 1;
                } else {
                    // Wrap forward to the first input step.
                    self.button_selected = 0;
                    self.current = 0;
                    self.load_buffer_for(0);
                    self.reset_array_on_arrival();
                }
                WizardEvent::None
            }
            KeyCode::Enter => {
                let selected = self.button_selected;
                self.button_selected = 0;
                if selected == 0 {
                    self.commit_value(labels[0].to_string());
                    self.current += 1;
                    WizardEvent::Done(self.values.clone())
                } else {
                    WizardEvent::Cancelled
                }
            }
            KeyCode::Esc => {
                self.button_selected = 0;
                WizardEvent::Cancelled
            }
            _ => WizardEvent::None,
        }
    }

    // ── Array browse (not editing) ────────────────────────────────────────────

    fn handle_array_browse_key(
        &mut self, key: KeyEvent,
        array_idx: usize,
        sub_steps: &'static [WizardStep],
    ) -> WizardEvent {
        let expanded   = self.array_states.get(&array_idx).map(|a| a.expanded).unwrap_or(false);
        let items_len  = self.array_states.get(&array_idx).map(|a| a.items.len()).unwrap_or(0);

        if !expanded {
            // ── Collapsed: Left/Right change header_sel, Enter expands ──────
            let header_sel = self.array_states.get(&array_idx).map(|a| a.header_sel).unwrap_or(0);
            return match key.code {
                KeyCode::Left => {
                    if let Some(arr) = self.array_states.get_mut(&array_idx) {
                        if arr.header_sel > 0 { arr.header_sel -= 1; }
                    }
                    WizardEvent::None
                }
                KeyCode::Right => {
                    if let Some(arr) = self.array_states.get_mut(&array_idx) {
                        if arr.header_sel < 1 { arr.header_sel += 1; }
                    }
                    WizardEvent::None
                }
                KeyCode::Enter => {
                    let arr = self.array_states.entry(array_idx).or_insert_with(ArrayState::new);
                    arr.expanded = true;
                    arr.header_sel = 0;
                    if header_sel == 0 {
                        // [+ add] focused — expand and immediately start adding
                        let _ = arr; // end borrow before calling helper
                        self.array_start_add(array_idx, sub_steps)
                    } else {
                        // [n] badge focused — just expand
                        WizardEvent::None
                    }
                }
                KeyCode::Tab  => self.array_advance_wizard(array_idx),
                KeyCode::BackTab => {
                    self.array_retreat_wizard();
                    WizardEvent::None
                }
                KeyCode::Esc => WizardEvent::Cancelled,
                _ => WizardEvent::None,
            };
        }

        // ── Expanded: Tab cycles items, Esc collapses ─────────────────────
        let selected     = self.array_states.get(&array_idx).map(|a| a.selected).unwrap_or(0);
        let item_btn_sel = self.array_states.get(&array_idx).map(|a| a.item_btn_sel).unwrap_or(0);

        match key.code {
            KeyCode::Tab => {
                if items_len > 0 {
                    if let Some(arr) = self.array_states.get_mut(&array_idx) {
                        arr.selected = (arr.selected + 1) % items_len;
                        arr.item_btn_sel = 0;
                    }
                }
                WizardEvent::None
            }
            KeyCode::BackTab | KeyCode::Esc => {
                if let Some(arr) = self.array_states.get_mut(&array_idx) {
                    arr.expanded     = false;
                    arr.header_sel   = 0;
                    arr.item_btn_sel = 0;
                }
                WizardEvent::None
            }
            // Left/Right navigate inline buttons on the focused item header
            KeyCode::Left => {
                if let Some(arr) = self.array_states.get_mut(&array_idx) {
                    if arr.item_btn_sel > 0 { arr.item_btn_sel -= 1; }
                }
                WizardEvent::None
            }
            KeyCode::Right => {
                if let Some(arr) = self.array_states.get_mut(&array_idx) {
                    if arr.item_btn_sel < 1 { arr.item_btn_sel += 1; }
                }
                WizardEvent::None
            }
            KeyCode::Enter => {
                if items_len > 0 {
                    if item_btn_sel == 1 {
                        // [remove] focused
                        let item_idx = selected.min(items_len - 1);
                        if let Some(arr) = self.array_states.get_mut(&array_idx) {
                            arr.items.remove(item_idx);
                            if arr.items.is_empty() {
                                arr.selected = 0;
                            } else {
                                arr.selected = arr.selected.min(arr.items.len() - 1);
                            }
                            arr.item_btn_sel = 0;
                        }
                        WizardEvent::ArrayItemDeleted { array_step_idx: array_idx, item_idx }
                    } else {
                        // Item itself focused — start editing
                        let item_idx = selected.min(items_len - 1);
                        self.array_start_edit(array_idx, sub_steps, item_idx)
                    }
                } else {
                    WizardEvent::None
                }
            }
            // Up/Down do nothing inside an expanded array nest
            KeyCode::Up | KeyCode::Down => WizardEvent::None,
            _ => WizardEvent::None,
        }
    }

    // ── Array item edit ───────────────────────────────────────────────────────

    fn handle_array_edit_key(
        &mut self, key: KeyEvent,
        array_idx: usize,
        sub_steps: &'static [WizardStep],
    ) -> WizardEvent {
        let (sub_step, is_new, item_idx) = {
            let s = self.array_states[&array_idx].editing.as_ref().unwrap();
            (s.sub_step, s.is_new, s.item_idx)
        };
        let is_last_sub = sub_step + 1 >= sub_steps.len();

        let sub_kind = match sub_steps.get(sub_step) {
            None    => return WizardEvent::None,
            Some(s) => match &s.kind {
                WizardStepKind::Select(opts) => StepKindRef::Select(opts),
                WizardStepKind::Optional     => StepKindRef::Optional,
                _                            => StepKindRef::Leaf,
            },
        };

        match sub_kind {
            StepKindRef::Leaf | StepKindRef::Optional => {
                self.handle_array_edit_leaf(key, array_idx, sub_steps, sub_step, item_idx, is_new, is_last_sub)
            }
            StepKindRef::Select(opts) => {
                self.handle_array_edit_select(key, array_idx, sub_steps, sub_step, item_idx, is_new, is_last_sub, opts)
            }
            _ => WizardEvent::None,
        }
    }

    fn handle_array_edit_leaf(
        &mut self, key: KeyEvent,
        array_idx: usize,
        sub_steps: &'static [WizardStep],
        sub_step: usize, item_idx: usize, is_new: bool, is_last_sub: bool,
    ) -> WizardEvent {
        match key.code {
            KeyCode::Char(c) => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                s.buffer.insert(s.buf_cursor, c);
                s.buf_cursor += c.len_utf8();
                WizardEvent::None
            }
            KeyCode::Backspace => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                if s.buf_cursor > 0 {
                    let start = prev_char_boundary(&s.buffer, s.buf_cursor);
                    let len   = s.buf_cursor - start;
                    s.buffer.drain(start..s.buf_cursor);
                    s.buf_cursor -= len;
                }
                WizardEvent::None
            }
            KeyCode::Delete => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                if s.buf_cursor < s.buffer.len() { s.buffer.remove(s.buf_cursor); }
                WizardEvent::None
            }
            KeyCode::Left => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                if s.buf_cursor > 0 { s.buf_cursor = prev_char_boundary(&s.buffer, s.buf_cursor); }
                WizardEvent::None
            }
            KeyCode::Right => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                if s.buf_cursor < s.buffer.len() { s.buf_cursor = next_char_boundary(&s.buffer, s.buf_cursor); }
                WizardEvent::None
            }
            KeyCode::Home => {
                self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap().buf_cursor = 0;
                WizardEvent::None
            }
            KeyCode::End => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                s.buf_cursor = s.buffer.len();
                WizardEvent::None
            }
            KeyCode::Tab | KeyCode::Enter => {
                let buf_val = self.array_states[&array_idx].editing.as_ref().unwrap().buffer.clone();
                self.array_states.get_mut(&array_idx).unwrap().items[item_idx][sub_step] = buf_val;
                if is_last_sub { self.complete_array_item(array_idx) }
                else { self.advance_array_sub_step(array_idx, sub_steps, sub_step, item_idx) }
            }
            KeyCode::Esc => self.cancel_array_edit(array_idx, is_new, item_idx),
            _ => WizardEvent::None,
        }
    }

    fn handle_array_edit_select(
        &mut self, key: KeyEvent,
        array_idx: usize,
        sub_steps: &'static [WizardStep],
        sub_step: usize, item_idx: usize, is_new: bool, is_last_sub: bool,
        opts: &'static [&'static str],
    ) -> WizardEvent {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                s.select_idx = if s.select_idx > 0 { s.select_idx - 1 } else { opts.len() - 1 };
                let si = s.select_idx;
                self.array_states.get_mut(&array_idx).unwrap().items[item_idx][sub_step] = opts[si].to_string();
                WizardEvent::None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                s.select_idx = (s.select_idx + 1) % opts.len();
                let si = s.select_idx;
                self.array_states.get_mut(&array_idx).unwrap().items[item_idx][sub_step] = opts[si].to_string();
                WizardEvent::None
            }
            KeyCode::Tab | KeyCode::Enter => {
                let si = self.array_states[&array_idx].editing.as_ref().unwrap().select_idx;
                self.array_states.get_mut(&array_idx).unwrap().items[item_idx][sub_step] = opts[si].to_string();
                if is_last_sub { self.complete_array_item(array_idx) }
                else { self.advance_array_sub_step(array_idx, sub_steps, sub_step, item_idx) }
            }
            KeyCode::Esc => self.cancel_array_edit(array_idx, is_new, item_idx),
            _ => WizardEvent::None,
        }
    }

    // ── Array helpers ─────────────────────────────────────────────────────────

    fn make_initial_item_vals(&self, sub_steps: &'static [WizardStep]) -> Vec<String> {
        sub_steps.iter().map(|s| match &s.kind {
            WizardStepKind::Select(opts) => opts.first().copied().unwrap_or("").to_string(),
            _ => String::new(),
        }).collect()
    }

    fn array_start_add(&mut self, array_idx: usize, sub_steps: &'static [WizardStep]) -> WizardEvent {
        let initial_vals = self.make_initial_item_vals(sub_steps);
        let first_select_idx = match sub_steps.first().map(|s| &s.kind) {
            Some(WizardStepKind::Select(opts)) =>
                opts.iter().position(|&o| o == initial_vals[0].as_str()).unwrap_or(0),
            _ => 0,
        };
        let sub_count = sub_steps.len();
        let arr = self.array_states.entry(array_idx).or_insert_with(ArrayState::new);
        arr.expanded = true;
        let item_idx = arr.items.len();
        arr.items.push(initial_vals);
        arr.selected = item_idx; // focus the new item
        arr.editing = Some(ArrayEditSession {
            item_idx,
            sub_step: 0,
            is_new: true,
            original_values: vec![String::new(); sub_count],
            buffer: String::new(),
            buf_cursor: 0,
            select_idx: first_select_idx,
        });
        WizardEvent::ArrayItemAdded { array_step_idx: array_idx, item_idx }
    }

    fn array_start_edit(
        &mut self, array_idx: usize,
        sub_steps: &'static [WizardStep],
        item_idx: usize,
    ) -> WizardEvent {
        let arr  = self.array_states.entry(array_idx).or_insert_with(ArrayState::new);
        let orig = arr.items[item_idx].clone();
        let first_val = orig.first().cloned().unwrap_or_default();
        let first_select_idx = match sub_steps.first().map(|s| &s.kind) {
            Some(WizardStepKind::Select(opts)) =>
                opts.iter().position(|&o| o == first_val.as_str()).unwrap_or(0),
            _ => 0,
        };
        arr.editing = Some(ArrayEditSession {
            item_idx,
            sub_step: 0,
            is_new: false,
            original_values: orig,
            buffer: first_val.clone(),
            buf_cursor: first_val.len(),
            select_idx: first_select_idx,
        });
        WizardEvent::None
    }

    fn advance_array_sub_step(
        &mut self, array_idx: usize,
        sub_steps: &'static [WizardStep],
        current_sub: usize, item_idx: usize,
    ) -> WizardEvent {
        let next_sub = current_sub + 1;
        let next_val = self.array_states[&array_idx].items[item_idx][next_sub].clone();
        match sub_steps[next_sub].kind {
            WizardStepKind::Select(opts) => {
                let si = opts.iter().position(|&o| o == next_val.as_str()).unwrap_or(0);
                let s  = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                s.sub_step   = next_sub;
                s.select_idx = si;
                if next_val.is_empty() {
                    self.array_states.get_mut(&array_idx).unwrap().items[item_idx][next_sub] = opts[si].to_string();
                }
            }
            _ => {
                let s = self.array_states.get_mut(&array_idx).unwrap().editing.as_mut().unwrap();
                s.sub_step   = next_sub;
                s.buffer     = next_val;
                s.buf_cursor = s.buffer.len();
            }
        }
        WizardEvent::None
    }

    fn complete_array_item(&mut self, array_idx: usize) -> WizardEvent {
        let arr     = self.array_states.get_mut(&array_idx).unwrap();
        let session = arr.editing.take().unwrap();
        let item_idx = session.item_idx;
        let values   = arr.items[item_idx].clone();
        arr.selected  = item_idx; // keep focus on the just-completed item
        WizardEvent::ArrayItemCompleted { array_step_idx: array_idx, item_idx, values }
    }

    fn cancel_array_edit(&mut self, array_idx: usize, is_new: bool, item_idx: usize) -> WizardEvent {
        let arr     = self.array_states.get_mut(&array_idx).unwrap();
        let session = arr.editing.take().unwrap();
        if is_new {
            arr.items.remove(item_idx);
            arr.selected = if arr.items.is_empty() { 0 } else { arr.selected.min(arr.items.len() - 1) };
            WizardEvent::ArrayItemDeleted { array_step_idx: array_idx, item_idx }
        } else {
            arr.items[item_idx] = session.original_values;
            arr.selected = item_idx;
            WizardEvent::None
        }
    }

    fn array_advance_wizard(&mut self, _array_idx: usize) -> WizardEvent {
        let index = self.current;
        self.commit_value(String::new());
        self.current += 1;
        self.reset_array_on_arrival();
        if self.is_complete() { WizardEvent::Done(self.values.clone()) }
        else { WizardEvent::StepCompleted { index, value: String::new() } }
    }

    fn array_retreat_wizard(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        } else {
            self.current = self.input_count.saturating_sub(1);
        }
        self.load_buffer_for(self.current);
        self.reset_array_on_arrival();
    }
}
