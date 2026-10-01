//! Authenticate bounded physical release samples after independent operator custody.
//!
//! These reads establish no role quiescence, external-account recovery or reset authority.
//! Workflow must reserve the complete read allowance durably before consuming preparation.

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
    icp::{IcpCli, IcpManagementCallError},
};
use candid::{CandidType, Principal};
use canic_core::ids::SubnetId;
use ic_agent::{Agent, AgentError};
use serde::Deserialize;
use sha2_host::{Digest, Sha256};
use std::time::Duration;
use thiserror::Error;

const RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const READ_DEADLINE: Duration = Duration::from_secs(45);

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
    Transport {
        canister: Principal,
        #[source]
        source: Box<AgentError>,
    },
    #[error("release snapshot observation of {canister} exceeded its deadline")]
    Deadline { canister: Principal },
    #[error("release snapshot response from {canister} cannot be decoded: {source}")]
    Decode {
        canister: Principal,
        #[source]
        source: candid::Error,
    },
}

/// Opaque free preparation, consumed once after the caller reserves four paid reads.
pub struct PreparedReleaseSourceObservation {
    agent: Agent,
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
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| ReleaseObservationError::Authentication(Box::new(error)))?;
    prepare_with_agent(agent, authority, source).await
}

async fn prepare_with_agent(
    agent: Agent,
    authority: &FleetReleaseAuthority,
    source: &CapacityImportSourceBinding,
) -> Result<PreparedReleaseSourceObservation, ReleaseObservationError> {
    let canister = source.canister_id;
    let network: [u8; 32] = Sha256::digest(agent.read_root_key()).into();
    if agent.get_principal().ok() != Some(authority.operator)
        || network != authority.network_root_key_sha256
        || [Principal::anonymous(), Principal::management_canister()].contains(&authority.operator)
    {
        return Err(ReleaseObservationError::Authority);
    }
    let first = management::prepare(&agent, canister)
        .await
        .map_err(|source| management_error(canister, source))?;
    let custody = first.custody();
    if custody.controllers() != [authority.operator] || custody.subnet() != source.subnet {
        return Err(FleetReleaseError::Custody { canister }.into());
    }
    Ok(PreparedReleaseSourceObservation {
        agent,
        canister,
        operator: authority.operator,
        subnet: source.subnet,
        first,
    })
}

impl PreparedReleaseSourceObservation {
    /// Consume at most four reserved reads. Refusal stops immediately, without an automatic retry.
    /// The result is a time-local physical sample, not a lock against later operator changes.
    pub async fn observe(self) -> Result<FleetReleasePhysicalSourceView, ReleaseObservationError> {
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

#[derive(CandidType)]
struct SnapshotRequest {
    canister_id: Principal,
}

#[derive(CandidType, Deserialize)]
struct Snapshot {
    id: Vec<u8>,
}

async fn snapshots(
    agent: &Agent,
    canister: Principal,
) -> Result<Vec<Vec<u8>>, ReleaseObservationError> {
    let argument = candid::encode_one(SnapshotRequest {
        canister_id: canister,
    })
    .map_err(|source| ReleaseObservationError::Decode { canister, source })?;
    let bytes = tokio::time::timeout(
        READ_DEADLINE,
        agent
            .update(&Principal::management_canister(), "list_canister_snapshots")
            .with_effective_canister_id(canister)
            .with_arg(argument)
            .call_and_wait(),
    )
    .await
    .map_err(|_| ReleaseObservationError::Deadline { canister })?
    .map_err(|source| ReleaseObservationError::Transport {
        canister,
        source: Box::new(source),
    })?;
    let mut config = candid::de::DecoderConfig::new();
    config
        .set_decoding_quota(RESPONSE_BYTES * 64)
        .set_skipping_quota(RESPONSE_BYTES * 64);
    let snapshots: Vec<Snapshot> = candid::utils::decode_one_with_config(&bytes, &config)
        .map_err(|source| ReleaseObservationError::Decode { canister, source })?;
    let mut ids = snapshots
        .into_iter()
        .map(|snapshot| snapshot.id)
        .collect::<Vec<_>>();
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
