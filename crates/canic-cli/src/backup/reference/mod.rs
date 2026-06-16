//! Module: backup::reference
//!
//! Responsibility: backup list rows and reference resolution.
//! Does not own: backup rendering, verification, or status reporting.
//! Boundary: converts persisted backup layouts into list entries and paths.

mod entry;
mod list;
mod resolve;
mod timestamp;

pub(super) use list::backup_list;
pub(super) use resolve::resolve_backup_dir;
pub use resolve::resolve_backup_reference;
#[cfg(test)]
pub(super) use resolve::resolve_backup_reference_in;
