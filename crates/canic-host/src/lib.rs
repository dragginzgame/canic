//! Native deployment planning and retained attempts; no application runtime dependency.

#![forbid(unsafe_code)]

mod app_build;
pub mod application;
mod artifact;
mod auth;
#[cfg(all(unix, feature = "backup"))]
pub mod backup;
mod config;
mod fleet;
mod fleet_plan;
mod maintenance;
mod managed;
mod network;
mod operation;
mod output_path;
mod output_publication;
pub mod placements;
mod plan;
mod process;
mod publication;
mod root_custody;
mod sidecar_lock;
mod signed;
mod staging_directory;
mod starter;
mod store;
mod store_bootstrap;
mod store_upgrade;
pub mod subnet_comparison;
#[cfg(test)]
mod tests;
mod wallet;

pub use app_build::{BuiltCanister, build_app};
pub use artifact::{Artifact, build, inspect};
pub use auth::{
    AuthApproval, AuthConfig, AuthReceipt, ReceiverController, create_auth_plan,
    create_auth_revocation,
};
pub use canic_management_protocol::Phase as ManagementPhase;
pub use config::{
    AppConfig, BuildInput, CanisterBinding, CanisterConfig, CanisterDefinition, Config,
    DeploymentName, DiscoveredConfig, FleetConfig, FleetDeclaration, SelectedFleet,
    ShardPoolConfig, add_app_canister, bind_canister, create_configured_plan, discover_configs,
    init_app, init_config, init_fleet, load_config, resolve_config_path,
};
pub use fleet::{
    Fleet, FleetCanister, FleetManager, FleetMetrics, FleetOperation, FleetStoreMaintenance,
    FleetUpgrade, fleet, fleet_with_config, fleet_with_managers, save_fleet_report,
};
pub use fleet_plan::{
    Cancellation, CreatedCanister, FleetChild, FleetPlan, FleetPlanning, FleetStore,
    cancel_fleet_plan, plan_fleet,
};
pub use managed::{activate, create_managed_plan, stored::StoredArtifact};
pub use network::Network;
pub use operation::{
    ApplyOutcome, CanisterStatus, ChildOutcome, FleetChildStatus, FleetStatus, FleetStepStatus,
    ManagementStatus, OperationStatus, Receipt, StatusDetail, apply, reconcile, status,
};
pub use plan::{
    ManagedOwner, Plan, PlanAction, PlanInput, StoreMaintenance, UpgradeTarget, create_plan,
};
pub use publication::list::{FleetArtifact, FleetArtifacts, fleet_artifacts};
pub use publication::removal::{
    PublicationRemoval, RemovalReceipt, create_removal_plan, create_restoration_plan,
};
pub use publication::{
    Publication, PublicationInput, PublicationProgress, create_publication_plan,
};
pub use signed::signing_principal;
pub use starter::create_starter;
pub use store_bootstrap::create_store_plan;
pub use store_upgrade::{StoreContinuation, store_continuation};
pub use wallet::{Wallet, WalletBalance, wallet, wallet_address};

use std::time::{SystemTime, UNIX_EPOCH};

/// Recoverable failures preserve whether an external attempt may have executed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Required external Candid request encoding failed.
    #[error("external request encoding failed: {0:?}")]
    Encode(ic_platform_protocol::management::EncodeError),
    /// Required external Candid response decoding failed.
    #[error("external reply decoding failed: {0:?}")]
    Decode(ic_platform_protocol::candid::Error),
    /// Input or retained-state admission failed before the next mutation.
    #[error("invalid input or state: {0}")]
    Invalid(String),
    /// An original attempt remains unresolved; never replace it automatically.
    #[error("blocked: {0}")]
    Blocked(String),
    /// A supplier failed. Retained attempt files determine external certainty.
    #[error("supplier failure: {0}")]
    Supplier(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Result of a Canic host operation.
pub type Result<T> = std::result::Result<T, Error>;

fn supplier(error: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::Supplier(Box::new(error))
}

fn now() -> Result<u64> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(supplier)?
        .as_nanos();
    u64::try_from(nanos).map_err(supplier)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Result<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(Error::Invalid("expected hexadecimal bytes".into()));
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(supplier)?;
            u8::from_str_radix(text, 16).map_err(supplier)
        })
        .collect()
}
