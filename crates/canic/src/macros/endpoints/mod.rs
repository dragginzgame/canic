//! Module: macros::endpoints
//!
//! Responsibility: collect endpoint emitter and bundle macro modules.
//! Does not own: endpoint implementations, generated endpoint bodies, or lifecycle wiring.
//! Boundary: module discovery only; exported macros are defined by child modules.

mod blob_storage;
mod blob_storage_billing;
mod bundles;
mod fleet_coordinator;
mod role;
mod root;
mod standards;
mod wasm_store;
