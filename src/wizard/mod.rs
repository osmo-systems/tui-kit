//! Step-by-step wizard widget with tree-structured steps, optional steps,
//! select (cycling) steps, and variable-length array steps.
//!
//! ## Visual layout (example)
//!
//! ```text
//!   │
//!   ●  name             my-blueprint
//!   │
//!   ●  description      (none)
//!   │
//!   ○  fields    [ + add ]  [0]        ← active array, collapsed
//!   │
//!   ◌  [ save ]                        ← pending Buttons step
//!   │
//! ```
//!
//! When the array is expanded:
//!
//! ```text
//!   ◉  fields    [ + add ]  [2]        ← expanded, 2 items
//!   │  │
//!   │  ●  #1                           ← item (not selected)
//!   │  │  │
//!   │  │  ●  field name   title
//!   │  │  │
//!   │  │  ●  field type   String
//!   │  │
//!   │  ►  #2  [ remove ]               ← item selected; [ remove ] focusable via Right
//!   │  │  │
//!   │  │  ●  field name   count
//!   │  │  │
//!   │  │  ●  field type   Integer
//!   │
//! ```
//!
//! ## Navigation
//!
//! | Context | Key | Effect |
//! |---------|-----|--------|
//! | Any leaf | Tab / Enter | Confirm value, advance |
//! | Any leaf | BackTab | Retreat (wraps) |
//! | Any leaf | Esc | Cancel (dismiss wizard) |
//! | Array collapsed | Left / Right | Switch between `[+ add]` and `[n]` |
//! | Array collapsed | Enter on `[+ add]` | Expand + start new item |
//! | Array collapsed | Enter on `[n]` | Expand only |
//! | Array collapsed | Tab | Advance past array |
//! | Array collapsed | Esc | Cancel (dismiss wizard) |
//! | Array expanded | Tab | Cycle through items |
//! | Array expanded | Enter | Edit focused item |
//! | Array expanded | Right → `[remove]` → Enter | Delete focused item |
//! | Array expanded | Esc / BackTab | Collapse |
//! | Item editing leaf | Tab / Enter | Confirm sub-step / complete item |
//! | Item editing leaf | Esc | Cancel edit (delete if new, restore if existing) |
//! | Item editing select | Left / Right | Cycle option |
//! | Item editing select | Tab / Enter | Confirm, advance sub-step |
//! | Buttons | Left / Right / Tab | Cycle buttons (Tab wraps to first step) |
//! | Buttons | BackTab | Retreat to previous step |
//! | Buttons | Enter | Fire primary (Done) or secondary (Cancelled) |
//! | Buttons | Esc | Cancel |

mod types;
mod helpers;
mod input;
mod render;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use types::{
    ArrayEditSession, ArrayState, WizardEvent, WizardState, WizardStep, WizardStepKind,
};
pub use render::render_wizard;
