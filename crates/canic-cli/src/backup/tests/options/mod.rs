//! Module: backup::tests::options
//!
//! Responsibility: backup CLI usage text and option parsing tests.
//! Does not own: backup execution, persistence, or fixture construction.
//! Boundary: command-line surface validation for the backup command family.

mod parse;
mod selector;
mod usage;
