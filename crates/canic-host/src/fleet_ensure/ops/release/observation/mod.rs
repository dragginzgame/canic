//! Authenticate bounded physical release samples after independent operator custody.
//!
//! These reads establish no role quiescence, external-account recovery or reset authority.
//! Consuming preparation requires a durably issued allowance from the existing operation journal.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::{capacity_import::CapacityImportSourceBinding, release::FleetReleaseAuthority},
        ops::capacity_import::{
            admission::observer::management::{self, PreparedManagementObservation},
            journal::CapacityImportJournalError,
        },
        policy::release::{FleetReleaseError, validate_physical_sample},
        view::release::FleetReleasePhysicalSourceView,
    },
    icp::{IcpCli, IcpManagementCallError, SNAPSHOT_RESPONSE_BYTES, read_snapshot_ids},
};
use candid::Principal;
use canic_core::ids::{CanonicalNetworkId, SubnetId};
use ic_agent::Agent;
use sha2_host::{Digest, Sha256};
use thiserror::Error;

/// Exact upper bound: two management status calls and two snapshot inventories, with no retries.
pub const PHYSICAL_SAMPLE_PAID_CALLS: u32 = 4;

/// A failed sample grants no custody or destructive authority; the reserved reads remain consumed.
#[derive(Debug, Error)]
pub enum ReleaseObservationError {
    #[error("release observation signer or network differs from the reviewed authority")]
    Authority,
    #[error("release observation authentication failed: {0}")]
    Authentication(#[source] Box<IcpManagementCallError>),
    #[error(transparent)]
    Evidence(#[from] FleetReleaseError),
    #[error("release management observation of {canister} failed: {source}")]
    Management {
        canister: Principal,
        #[source]
        source: Box<CapacityImportJournalError>,
    },
    #[error("release snapshot observation of {canister} failed: {source}")]
    Snapshot {
        canister: Principal,
        #[source]
        source: Box<IcpManagementCallError>,
    },
}

/// Opaque free preparation, consumed once after the caller reserves four paid reads.
pub struct PreparedReleaseSourceObservation {
    agent: Agent,
    authority: FleetReleaseAuthority,
    canister: Principal,
    operator: Principal,
    subnet: SubnetId,
    first: PreparedManagementObservation,
}

/// Authenticate the selected signer/network and certified custody without a management call.
/// Completed journal replay must be selected before invoking this live boundary.
pub async fn prepare(
    icp: &IcpCli,
    authority: &FleetReleaseAuthority,
    source: &CapacityImportSourceBinding,
) -> Result<PreparedReleaseSourceObservation, ReleaseObservationError> {
    let agent = icp
        .authenticated_agent_with_response_limit(SNAPSHOT_RESPONSE_BYTES)
        .map_err(|error| ReleaseObservationError::Authentication(Box::new(error)))?;
    prepare_with_agent(agent, authority, source).await
}

async fn prepare_with_agent(
    agent: Agent,
    authority: &FleetReleaseAuthority,
    source: &CapacityImportSourceBinding,
) -> Result<PreparedReleaseSourceObservation, ReleaseObservationError> {
    verify_agent(&agent, authority)?;
    let canister = source.canister_id;
    let first = management::prepare(&agent, canister)
        .await
        .map_err(|source| management_error(canister, source))?;
    let custody = first.custody();
    if custody.controllers() != [authority.operator] || custody.subnet() != source.subnet {
        return Err(FleetReleaseError::Custody { canister }.into());
    }
    Ok(PreparedReleaseSourceObservation {
        agent,
        authority: authority.clone(),
        canister,
        operator: authority.operator,
        subnet: source.subnet,
        first,
    })
}

pub(super) fn verify_agent(
    agent: &Agent,
    authority: &FleetReleaseAuthority,
) -> Result<(), ReleaseObservationError> {
    let network: [u8; 32] = Sha256::digest(agent.read_root_key()).into();
    if agent.get_principal().ok() != Some(authority.operator)
        || network != authority.network_root_key_sha256
        || CanonicalNetworkId::from_der_root_trust_anchor(&agent.read_root_key()).ok()
            != Some(authority.fleet.fleet.canonical_network_id)
        || [Principal::anonymous(), Principal::management_canister()].contains(&authority.operator)
    {
        return Err(ReleaseObservationError::Authority);
    }
    Ok(())
}

impl PreparedReleaseSourceObservation {
    /// Consume at most four reserved reads. Refusal stops immediately, without an automatic retry.
    /// The result is a time-local physical sample, not a lock against later operator changes.
    pub async fn observe(
        self,
        reservation: super::reservation::ReleaseObservationReservation<'_>,
    ) -> Result<FleetReleasePhysicalSourceView, ReleaseObservationError> {
        if !reservation.authorizes(&self.authority, self.canister, self.subnet) {
            return Err(ReleaseObservationError::Authority);
        }
        let before = self
            .first
            .observe()
            .await
            .map_err(|source| management_error(self.canister, source))?;
        if !before.binding.stopped || before.binding.controllers != [self.operator] {
            return Err(FleetReleaseError::Custody {
                canister: self.canister,
            }
            .into());
        }
        let before_snapshots = snapshots(&self.agent, self.canister).await?;
        let after = management::observe(&self.agent, self.canister)
            .await
            .map_err(|source| management_error(self.canister, source))?;
        let after_snapshots = snapshots(&self.agent, self.canister).await?;
        validate_physical_sample(
            self.operator,
            self.subnet,
            &before,
            &after,
            &before_snapshots,
            &after_snapshots,
        )?;
        Ok(FleetReleasePhysicalSourceView {
            sample: after,
            snapshots: after_snapshots,
        })
    }
}

async fn snapshots(
    agent: &Agent,
    canister: Principal,
) -> Result<Vec<Vec<u8>>, ReleaseObservationError> {
    let mut ids = read_snapshot_ids(agent, canister).await.map_err(|source| {
        ReleaseObservationError::Snapshot {
            canister,
            source: Box::new(source),
        }
    })?;
    ids.sort();
    Ok(ids)
}

fn management_error(
    canister: Principal,
    source: CapacityImportJournalError,
) -> ReleaseObservationError {
    ReleaseObservationError::Management {
        canister,
        source: Box::new(source),
    }
}
