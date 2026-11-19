//! Standalone application storage and lifecycle; Canic owns deployment only.
#[cfg(feature = "canister")]
mod canister;
/// The application's persistent owner/revision rules, independently of its actor exports.
pub mod state;
/// Explicit standalone bootstrap, separate from the reusable state engine.
pub mod storage;
