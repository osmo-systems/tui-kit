//! Pure utility functions for the wizard widget.

use super::types::{WizardStep, WizardStepKind, StepKindRef};

/// DFS walk: find the kind of the step at `target` leaf index.
/// Sections are transparent (not counted as leaves); everything else counts as 1.
pub(super) fn step_kind_at(
    steps: &'static [WizardStep],
    target: usize,
    counter: &mut usize,
) -> Option<StepKindRef> {
    for step in steps {
        match &step.kind {
            WizardStepKind::Section(children) => {
                if let r @ Some(_) = step_kind_at(children, target, counter) {
                    return r;
                }
            }
            _ => {
                if *counter == target {
                    return Some(match &step.kind {
                        WizardStepKind::Leaf                 => StepKindRef::Leaf,
                        WizardStepKind::Optional             => StepKindRef::Optional,
                        WizardStepKind::Select(opts)         => StepKindRef::Select(opts),
                        WizardStepKind::Array(sub_steps)     => StepKindRef::Array(sub_steps),
                        WizardStepKind::Buttons(labels)      => StepKindRef::Buttons(labels),
                        WizardStepKind::Section(_)           => unreachable!(),
                    });
                }
                *counter += 1;
            }
        }
    }
    None
}

/// Count all DFS leaves under `steps`.
/// Section children are counted individually; Array itself counts as 1.
pub(super) fn count_leaves(steps: &[WizardStep]) -> usize {
    steps.iter().map(|s| match &s.kind {
        WizardStepKind::Section(children) => count_leaves(children),
        _ => 1,
    }).sum()
}

/// Maximum label length across sub-steps (for alignment).
pub(super) fn max_sub_label(sub_steps: &[WizardStep]) -> usize {
    sub_steps.iter().map(|s| s.label.chars().count()).max().unwrap_or(4)
}

/// Move backward to the previous UTF-8 char boundary.
pub(super) fn prev_char_boundary(s: &str, pos: usize) -> usize {
    if pos == 0 { return 0; }
    let mut p = pos - 1;
    while !s.is_char_boundary(p) { p -= 1; }
    p
}

/// Move forward to the next UTF-8 char boundary.
pub(super) fn next_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() { return s.len(); }
    let mut p = pos + 1;
    while p < s.len() && !s.is_char_boundary(p) { p += 1; }
    p
}
