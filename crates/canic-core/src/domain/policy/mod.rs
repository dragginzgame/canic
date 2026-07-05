//! Domain policy namespace.
//!
//! Pure, side-effect-free policy decisions live under [`pure`]. Keeping the
//! implementation behind that namespace makes policy call sites auditably
//! distinct from workflow, ops, storage, runtime, and endpoint layers.

pub mod pure;
