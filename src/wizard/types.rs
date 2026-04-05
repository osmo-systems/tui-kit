//! All data types for the wizard widget.

use std::collections::HashMap;

// ── Step definitions ──────────────────────────────────────────────────────────

/// The kind of a wizard step, controlling how it is rendered and how keys are handled.
pub enum WizardStepKind {
    /// Free-text input.  Tab/Enter advances to the next step.
    Leaf,
    /// Free-text input that can be skipped by pressing Enter on an empty buffer.
    Optional,
    /// Cycle through a fixed list of options with Left/Right.
    Select(&'static [&'static str]),
    /// Visual grouping that expands its children in place.  Sections are not
    /// counted as leaves themselves — their children are counted instead.
    Section(&'static [WizardStep]),
    /// Variable-length list of items, each with the given sub-steps.
    /// Counts as **one leaf** in the DFS order (the array header).
    Array(&'static [WizardStep]),
    /// One or more action buttons rendered inline.
    ///
    /// Left/Right cycles the highlighted button.
    /// Enter on the **first** button (index 0) fires [`WizardEvent::Done`].
    /// Enter on any other button fires [`WizardEvent::Cancelled`].
    /// BackTab retreats to the previous step.  Esc fires `Cancelled`.
    Buttons(&'static [&'static str]),
}

/// One node in the wizard step tree.
pub struct WizardStep {
    pub label: &'static str,
    pub kind: WizardStepKind,
}

impl WizardStep {
    pub const fn leaf(label: &'static str) -> Self {
        Self { label, kind: WizardStepKind::Leaf }
    }
    pub const fn optional(label: &'static str) -> Self {
        Self { label, kind: WizardStepKind::Optional }
    }
    pub const fn select(label: &'static str, opts: &'static [&'static str]) -> Self {
        Self { label, kind: WizardStepKind::Select(opts) }
    }
    pub const fn section(label: &'static str, children: &'static [WizardStep]) -> Self {
        Self { label, kind: WizardStepKind::Section(children) }
    }
    pub const fn array(label: &'static str, sub_steps: &'static [WizardStep]) -> Self {
        Self { label, kind: WizardStepKind::Array(sub_steps) }
    }
    pub const fn buttons(labels: &'static [&'static str]) -> Self {
        Self { label: "", kind: WizardStepKind::Buttons(labels) }
    }
    pub fn is_section(&self) -> bool { matches!(self.kind, WizardStepKind::Section(_)) }
    pub fn is_array(&self)   -> bool { matches!(self.kind, WizardStepKind::Array(_)) }
    /// Children for Section or Array steps; empty for all others.
    pub fn children(&self) -> &'static [WizardStep] {
        match &self.kind {
            WizardStepKind::Section(c) | WizardStepKind::Array(c) => c,
            _ => &[],
        }
    }
}

// ── Array state ───────────────────────────────────────────────────────────────

/// State for an in-progress edit of one array item.
pub struct ArrayEditSession {
    /// Which item (0-based index into `ArrayState::items`).
    pub item_idx: usize,
    /// Which sub-step is currently active.
    pub sub_step: usize,
    /// `true` if this item was just created — Esc will delete it.
    pub is_new: bool,
    /// Original sub-step values for Esc-restore on existing items.
    pub original_values: Vec<String>,
    /// Text buffer for Leaf sub-steps.
    pub buffer: String,
    /// Byte-cursor within `buffer`.
    pub buf_cursor: usize,
    /// Selected index for Select sub-steps.
    pub select_idx: usize,
}

/// Mutable state for one Array step.
pub struct ArrayState {
    /// Completed items; `items[i][j]` is the value for sub-step `j` of item `i`.
    pub items: Vec<Vec<String>>,
    /// Browse cursor when expanded — direct item index (0-based).
    pub selected: usize,
    /// Active edit session, if any.
    pub editing: Option<ArrayEditSession>,
    /// Whether the array is expanded (showing items) or collapsed (showing badge only).
    pub expanded: bool,
    /// Which header button is focused when collapsed: 0 = [+ add], 1 = [n] badge.
    pub header_sel: usize,
    /// Which inline button is focused on the selected item: 0 = item itself, 1 = [remove].
    pub item_btn_sel: usize,
}

impl ArrayState {
    pub fn new() -> Self {
        Self {
            items: vec![],
            selected: 0,
            editing: None,
            expanded: false,
            header_sel: 0,
            item_btn_sel: 0,
        }
    }
}

// ── Wizard state ──────────────────────────────────────────────────────────────

/// All mutable wizard state.
pub struct WizardState {
    /// The full step tree (static, defined by the caller).
    pub steps: &'static [WizardStep],
    /// How many leading DFS-leaf steps are wizard-managed.
    /// When `current >= input_count`, the wizard is complete.
    pub input_count: usize,
    /// DFS-leaf index of the currently active step.
    pub current: usize,
    /// Confirmed string values for non-array leaf steps.
    /// Indexed by step leaf index; may be sparse if the user cycled backward.
    pub values: Vec<String>,
    /// Text buffer for the currently active Leaf/Optional input step.
    pub buffer: String,
    /// Byte-cursor within `buffer`.
    pub cursor: usize,
    /// State for each Array step, keyed by its DFS-leaf index.
    pub array_states: HashMap<usize, ArrayState>,
    /// Which button is highlighted when the current step is a Buttons step.
    pub button_selected: usize,
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Event returned by [`WizardState::handle_key`].
///
/// `StepCompleted` doubles as the "field blurred / onChange" hook: validate
/// the value here and show an error toast if needed.
#[derive(Debug)]
pub enum WizardEvent {
    None,
    /// A non-array leaf step was confirmed (also the validate / onChange hook).
    StepCompleted { index: usize, value: String },
    /// All `input_count` steps have been confirmed.
    Done(Vec<String>),
    /// The user cancelled (Esc at any top-level position).
    Cancelled,
    /// An item was added to an array step (editing session started).
    ArrayItemAdded { array_step_idx: usize, item_idx: usize },
    /// An item was deleted from an array step.
    ArrayItemDeleted { array_step_idx: usize, item_idx: usize },
    /// An item's edit session was confirmed with all sub-step values.
    ArrayItemCompleted { array_step_idx: usize, item_idx: usize, values: Vec<String> },
}

// ── Internal copy-friendly kind reference ─────────────────────────────────────

/// Lightweight `Copy` view of a step kind, carrying only `'static` refs.
/// Used to avoid borrow conflicts when dispatching `handle_key`.
#[derive(Clone, Copy)]
pub(super) enum StepKindRef {
    Leaf,
    Optional,
    Select(&'static [&'static str]),
    Array(&'static [WizardStep]),
    Buttons(&'static [&'static str]),
}
