//! Module: backup::tests::verify
//!
//! Responsibility: backup verification command behavior tests.
//! Does not own: backup persistence fixtures or verification implementation.
//! Boundary: CLI verification behavior for dry-run and execution-backed layouts.

mod integrity;
mod rejection;
