//! Internal read-only projections over stored or runtime state.
//!
//! The term `view` is reserved for projections defined under `view/`.
//! DTOs must not use `view` in type or function names.
pub mod env;
pub mod placement;
pub mod topology;
