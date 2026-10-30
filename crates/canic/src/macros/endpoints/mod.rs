//! Module: macros::endpoints
//!
//! Responsibility: collect endpoint emitter and bundle macro modules.
//! Does not own: endpoint implementations, generated endpoint bodies, or lifecycle wiring.
//! Boundary: module discovery only; exported macros are defined by child modules.

mod bundles;
mod cycle_balance;
mod cycle_history;
mod cycle_topups;
mod fixture_reply;
mod fleet_coordinator;
mod protected_health;
mod role;
mod root;
mod wasm_store;

mod protected_logs;

mod protected_metrics;

mod protected_memory_allocations;

mod child_funding;

mod protected_readiness;

mod protected_runtime;

mod protected_binding;

mod observation_updates;

mod root_observation;

mod protected_state;

mod release_evidence;

mod cascade;

mod cycles_capability;

mod root_capability;
