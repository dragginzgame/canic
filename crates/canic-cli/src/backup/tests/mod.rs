//! Module: backup::tests
//!
//! Responsibility: backup command behavior tests.
//! Does not own: backup implementation or shared fixture construction.
//! Boundary: integration-style unit tests for the `canic backup` command family.

mod create;
mod fixtures;
mod inspect;
mod list;
mod options;
mod prune;
mod reference;
mod status;
mod verify;
