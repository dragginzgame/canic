//! Module: ops::replay::model
//!
//! Responsibility: re-export replay model types for existing ops callers.
//! Does not own: model definitions or storage conversion.
//! Boundary: compatibility shim while callers move to `model::replay`.

pub use crate::model::replay::*;
