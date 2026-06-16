//! Module: macros::endpoints
//!
//! Responsibility: collect endpoint emitter and bundle macro modules.
//! Does not own: endpoint implementations, generated endpoint bodies, or lifecycle wiring.
//! Boundary: module discovery only; exported macros are defined by child modules.

mod bundles;
mod cycles;
mod icp_refill;
mod nonroot;
mod root;
mod shared;
mod topology;
mod wasm_store;
