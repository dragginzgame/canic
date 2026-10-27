/// Managed initialization encoding and its shared resource allowance.
pub use canic_contracts::protocol::initialization;

/// Public wire-level endpoint names for Canic canisters.
///
/// `canic_contracts::protocol` owns names used by runtime inter-canister calls. This
/// facade adds the maintained convenience names used by tooling and consumers.
pub use canic_contracts::{
    protocol::CANIC_ROOT_FIXTURE_STATUS, protocol::CANIC_ROOT_MEMBERSHIP,
    protocol::CANIC_WASM_STORE_FIXTURE_CHUNK, protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
};

#[cfg(any(
    feature = "control-plane",
    feature = "wasm-store-canister",
    not(target_arch = "wasm32"),
    test
))]
pub use canic_contracts::protocol::CANIC_WASM_STORE_CHUNK;

#[cfg(any(
    feature = "control-plane",
    feature = "wasm-store-canister",
    not(target_arch = "wasm32"),
    test
))]
pub use canic_contracts::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK;

pub use canic_contracts::protocol::ICRC10_SUPPORTED_STANDARDS;
pub use canic_contracts::protocol::ICRC21_CANISTER_CALL_CONSENT_MESSAGE;

pub use crate::__internal::contracts::protocol::CANIC_ADMISSION_STATUS;

/// Typed public responsiveness identity and codec bounds.
pub use canic_contracts::protocol::{CANIC_PUBLIC_HEALTH, public_health, wasm_store_chunk};

pub use canic_contracts::protocol::{CANIC_PUBLIC_METRICS, public_metrics};

pub use canic_contracts::protocol::{CANIC_PUBLIC_HISTORY, public_history};

/// Root checkpoint identities and bounded application hook contract.
pub use canic_contracts::protocol::application_checkpoint;

/// Typed public role identity and bootstrap readiness.
pub use canic_contracts::protocol::{CANIC_PUBLIC_OVERVIEW, public_overview};

/// Typed direct-child inventory.
pub use canic_contracts::protocol::{CANIC_PUBLIC_CHILDREN, public_children};

/// Typed member and controller component-directory reads.
pub use canic_contracts::protocol::component_directory;

/// Authenticated live cycle-balance query.
pub use canic_contracts::protocol::cycle_balance;

/// Controller-authenticated bounded cycle history.
pub use canic_contracts::protocol::cycle_history;
pub use canic_contracts::protocol::cycle_topups;

/// Controller-only runtime health diagnostics.
pub use canic_contracts::protocol::protected_health;

/// Controller-only logs.
pub use canic_contracts::protocol::protected_logs;

/// Controller-only metrics.
pub use canic_contracts::protocol::protected_metrics;

/// Controller-only memory allocations.
pub use canic_contracts::protocol::protected_memory_allocations;

/// Controller-owned child funding observations.
pub use canic_contracts::protocol::child_funding;

/// Controller-only readiness observations.
pub use canic_contracts::protocol::protected_readiness;

/// Controller-only runtime observations.
pub use canic_contracts::protocol::protected_runtime;

/// Controller-only binding observations.
pub use canic_contracts::protocol::protected_binding;

/// Receiver-local original caller-authority publication status.
pub use canic_contracts::protocol::caller_authority_status;

/// Controller-only compiled Root configuration.
pub use canic_contracts::protocol::root_config;

/// Original caller publication transitions.
pub use canic_contracts::protocol::caller_authority;

pub use canic_contracts::protocol::inspection_reserve;

pub use canic_contracts::protocol::coordinator_acknowledgements;

pub use canic_contracts::protocol::observability_relay;

pub use canic_contracts::protocol::root_authority;
pub use canic_contracts::protocol::root_observation;

pub use canic_contracts::protocol::protected_state;

pub use canic_contracts::protocol::registry_discovery;
pub use canic_contracts::protocol::registry_read;

pub use canic_contracts::protocol::release_evidence;

pub use canic_contracts::protocol::admission_control;

/// Native Fleet control and managed lifecycle contracts.
pub use canic_contracts::protocol::{
    cascade, fleet_state_control, pool_retry, runtime_configuration, runtime_reporting,
};

pub use canic_contracts::protocol::issuer_proof;

pub use canic_contracts::protocol::cycles_capability;

pub use canic_contracts::protocol::root_capability;

pub use canic_contracts::protocol::issuer_renewal;

pub use canic_contracts::protocol::role_attestation;

pub use canic_contracts::protocol::issuer_control;

pub use canic_contracts::protocol::component_initialization;

pub use canic_contracts::protocol::store_inventory;

pub use canic_contracts::protocol::store_publication;

pub use canic_contracts::protocol::store_gc;

pub use canic_contracts::protocol::store_activation;

pub use canic_contracts::protocol::store_control;

pub use canic_contracts::protocol::store_catalog;

pub use canic_contracts::protocol::fixture_bytes;

pub use canic_contracts::protocol::store_status;

pub use canic_contracts::protocol::template_bytes;

pub use canic_contracts::protocol::fixture_reply;

pub use canic_contracts::protocol::root_store_reporting;

pub use canic_contracts::protocol::provisioning_census;

pub use canic_contracts::protocol::pool_census;
pub use canic_contracts::protocol::pool_inventory;

pub use canic_contracts::protocol::pool_import_read;

pub use canic_contracts::protocol::root_activation;

pub use canic_contracts::protocol::funding_control;

pub use canic_contracts::protocol::cycle_refill;

pub use canic_contracts::protocol::component_control;

pub use canic_contracts::protocol::root_deletion;

pub use canic_contracts::protocol::pool_control;

pub use canic_contracts::protocol::registry_control;

pub use canic_contracts::protocol::root_store_control;

pub use canic_contracts::protocol::retirement_control;

pub use canic_contracts::protocol::batch_control;

pub use canic_contracts::protocol::inspection_history;

pub use canic_contracts::protocol::import_control;

pub use canic_contracts::protocol::inspection;

pub use canic_contracts::protocol::funding_read;

pub use canic_contracts::protocol::component_read;

pub use canic_contracts::protocol::{component_removal_read, root_removal_read, subtree_read};
