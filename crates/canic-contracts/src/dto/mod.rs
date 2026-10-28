//! DTO boundary definitions.
//!
//! ## Design rules
//!
//! - DTOs are **passive transport types**. They contain no domain logic, policy,
//!   validation, normalization, or invariant enforcement.
//!
//! - Decoding does not establish application-specific uniqueness, sort order,
//!   or keyed semantics. Validate those properties at their owning boundary.
//!
//! - All authoritative invariants (e.g. ownership, uniqueness, replacement
//!   semantics) live in **storage and ops layers**, never in DTOs.
//! - Data exported from stable registries is treated as a **snapshot** of state,
//!   not a live or authoritative registry.
//!
//! - DTOs therefore prefer simple list representations (`Vec<T>`). When keyed
//!   data is needed, define small entry structs (`FooEntry`) instead of
//!   tuples or maps. More structured shapes are used only when they materially
//!   reduce complexity for consumers, not to mirror storage internals or
//!   reintroduce lost semantics.
//!
//! - Avoid `HashMap` in DTOs. Use ordered `Vec<...Entry>` or explicit list types
//!   with application-owned uniqueness and ordering validation.

pub mod abi;
pub mod application_checkpoint;
pub mod auth;
pub mod authority_restore;
pub mod caller_authority;
pub mod canister;
pub mod capability;
pub mod cascade;
pub mod component_deployment;
pub mod component_directory;
pub mod component_provisioning;
pub mod component_registry;
pub mod cycle_topups;
pub mod cycles;
pub mod env;
pub mod error;
pub mod fixture_provisioning;
pub mod fleet_activation;
pub mod fleet_admission;
pub mod fleet_funding;
pub mod fleet_registry;
pub mod fleet_subnet_root;
pub mod icp_refill;
pub mod log;
pub mod memory;
pub mod metadata;
pub mod metrics;
pub mod observability;
pub mod page;
pub mod placement;
pub mod pool;
pub mod pool_import;
pub mod public_status;
pub mod release_intents;
pub mod release_receipts;
pub mod role;
pub mod root_authority;
pub mod root_observation;
pub mod root_store;
pub mod rpc;
pub mod runtime;
pub mod state;

///
/// Prelude
///

pub mod prelude {
    pub use crate::ids::{CanisterRole, ComponentSpecId};
    pub use ic_platform::{candid::contract::Nat, principal::ExternalPrincipal as Principal};
    pub use serde::{Deserialize, Serialize};
}

pub mod fleet_coordinator;
pub mod root;
pub mod template;

pub mod metric_page;

pub mod child_funding;

pub mod runtime_observation;

pub mod caller_authority_status;

pub mod inspection_reserve;

pub mod coordinator_acknowledgements;

pub mod authority_restore_status;

pub mod registry_discovery;
pub mod registry_read;

pub mod fleet_snapshot;

pub mod admission_control;

pub mod pool_retry;

pub mod fleet_state_control;

pub mod cascade_transport;

pub mod runtime_configuration;

pub mod runtime_reporting;

pub mod issuer_proof;

pub mod root_capability;

pub mod issuer_renewal;

pub mod role_attestation;

pub mod issuer_control;

pub mod component_initialization;

pub mod store_control;

pub mod fixture_bytes;

pub mod store_status;

pub mod template_bytes;

pub mod fixture_reply;

pub mod root_store_reporting;

pub mod provisioning_census;

pub mod pool_census;
pub mod pool_inventory;

pub mod pool_import_read;

pub mod funding_control;

pub mod component_control;
pub mod component_read;

pub mod root_deletion;

pub mod pool_control;

pub mod registry_control;

pub mod root_store_control;

pub mod retirement_control;

pub mod batch_control;

pub mod inspection_history;

pub mod import_control;

pub mod inspection;

pub mod funding_read;

pub mod subtree_read;

pub mod component_removal_read;

pub mod root_removal_read;
