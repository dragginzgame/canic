//! Module: infra::ic::nns
//!
//! Responsibility: group raw NNS canister infra adapters.
//! Does not own: topology policy, registry storage, or endpoint response mapping.
//! Boundary: ops topology calls this for raw NNS lookups.

pub mod registry;
