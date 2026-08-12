//! Terminal-safe activity and styling shared by host-side operator workflows.

mod activity;
mod style;

pub use activity::TerminalActivity;
pub use style::TerminalStyle;
