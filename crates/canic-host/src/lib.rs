//! Native deployment planning and retained attempts; no application runtime dependency.

mod artifact;
mod config;
mod fleet;
mod network;
mod operation;
mod plan;
mod signed;
mod store;
#[cfg(test)]
mod tests;
mod wallet;

pub use artifact::{Artifact, build, inspect};
pub use canic_profile_protocol::growth::Phase as ProfileGrowthPhase;
pub use config::{
    AppConfig, CanisterConfig, Config, DeploymentName, FleetConfig, SelectedFleet,
    create_configured_plan, init_config, load_config,
};
pub use fleet::{
    Fleet, FleetCanister, FleetManager, FleetOperation, fleet, fleet_with_config,
    fleet_with_managers,
};
pub use network::{Network, greet};
pub use operation::{Receipt, apply, status};
pub use plan::{Plan, PlanInput, create_plan};
pub use wallet::{Wallet, WalletBalance, wallet, wallet_address};

use std::time::{SystemTime, UNIX_EPOCH};

/// Recoverable failures preserve whether an external attempt may have executed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Required external Candid request encoding failed.
    #[error("external request encoding failed: {0:?}")]
    Encode(ic_platform::candid_codec::Error),
    /// Required external Candid response decoding failed.
    #[error("external reply decoding failed: {0:?}")]
    Decode(ic_platform::candid_codec::decoder::Error),
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
