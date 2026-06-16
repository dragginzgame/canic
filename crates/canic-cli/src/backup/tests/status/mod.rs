//! Module: backup::tests::status
//!
//! Responsibility: backup status and completion-gate behavior tests.
//! Does not own: backup persistence fixtures or command option parsing.
//! Boundary: status report behavior for download and execution-backed layouts.

mod execution;
mod json;
mod read;
mod requirements;
