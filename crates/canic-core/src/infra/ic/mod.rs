//! Module: infra::ic
//!
//! Responsibility: group raw IC call, NNS, and management adapters.
//! Does not own: ops conversion, workflow policy, or endpoint DTO shaping.
//! Boundary: ops calls this namespace for low-level IC platform interactions.

pub mod build_network;
pub mod cycles_ledger;
mod external_candid;
pub mod icp_refill;
pub mod known;
pub mod mgmt;
pub mod nns;
pub mod release_build;

use ic_platform::candid::contract::Error as CandidError;
use ic_platform::cdk::call::CallFailed;
use thiserror::Error as ThisError;

///
/// IcInfraError
///
/// IC infra failure surface for raw IC transport and protocol adapters.
/// Owned by `infra::ic` and converted at the calling layer boundary.
///

#[derive(Debug, ThisError)]
pub enum IcInfraError {
    #[error(transparent)]
    EmbeddedReleaseBuild(#[from] release_build::EmbeddedReleaseBuildError),

    #[error(transparent)]
    CyclesLedgerInfra(#[from] cycles_ledger::CyclesLedgerInfraError),

    #[error(transparent)]
    IcpRefillInfra(#[from] icp_refill::IcpRefillInfraError),

    #[error(transparent)]
    MgmtInfra(#[from] mgmt::MgmtInfraError),

    #[error(transparent)]
    NnsRegistryInfra(#[from] nns::registry::NnsRegistryInfraError),

    // candid catch-all errors
    #[error(transparent)]
    CallFailed(#[from] CallFailed),

    #[error(transparent)]
    Candid(#[from] CandidError),

    #[error(transparent)]
    CandidDecode(CandidError),
}

impl IcInfraError {
    /// Whether a management observation proved that its target Canister is absent.
    #[must_use]
    pub fn is_canister_not_found(&self) -> bool {
        matches!(
            self,
            Self::CallFailed(CallFailed::CallRejected(rejection))
                if matches!(
                    rejection.reject_code(),
                    Ok(ic_platform::cdk::call::RejectCode::DestinationInvalid)
                )
        )
    }
}
