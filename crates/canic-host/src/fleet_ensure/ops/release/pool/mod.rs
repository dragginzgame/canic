//! Collect retained Root pool obligations through bounded signed queries.
//!
//! Custody and Registry reads bracket observation; neither history nor exhaustion
//! grants settlement, new spending authority or permission to clear an owner.

mod assessment;
#[cfg(test)]
pub(super) mod tests;

use crate::{
    fleet_ensure::{
        model::release::{FleetReleaseReviewRecord, FleetReleaseRole},
        ops::release::{
            inventory,
            observation::{ReleaseObservationError, verify_agent},
        },
        view::release::FleetReleasePoolView,
    },
    icp::IcpCli,
};
use std::time::Duration;

use candid::{CandidType, Principal};
use canic_control_plane::dto::root::RootPoolReleaseResponse;
use canic_core::{
    dto::{error::Error as CanicError, fleet_registry::FleetRegistry},
    protocol,
};
use ic_agent::Agent;
use serde::Deserialize;
use thiserror::Error;

pub(in crate::fleet_ensure) use assessment::assessment_facts;

const RESPONSE_BYTES: usize = 1024 * 1024;
const MAXIMUM_CENSUS_BYTES: usize = 16 * RESPONSE_BYTES;
const QUERY_DEADLINE: Duration = Duration::from_secs(15);
const CENSUS_DEADLINE: Duration = Duration::from_secs(120);

/// A refused census returns no partial evidence or release authority.
#[derive(Debug, Error)]
pub enum ReleasePoolError {
    #[error(transparent)]
    Authentication(#[from] ReleaseObservationError),

    #[error("Fleet release pool census exceeded its deadline")]
    Deadline,

    #[error(transparent)]
    Inventory(#[from] inventory::ReleaseInventoryError),

    #[error("Root {root} pool census failed at {stage:?}")]
    Observation {
        root: Principal,
        stage: ReleasePoolStage,
    },

    #[error("Root {root} refused pool census: {rejection}")]
    Rejected {
        root: Principal,
        rejection: CanicError,
    },
}

/// Machine-readable refusal boundary, independent of presentation wording.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleasePoolStage {
    Binding,
    Budget,
    Decode,
    Query,
}

#[derive(CandidType)]
enum Request {
    PoolRelease,
}

#[derive(CandidType, Deserialize)]
enum Response {
    PoolRelease(Box<RootPoolReleaseResponse>),
}

/// Preserve every selected Root's retained pool evidence, including exhausted and released imports.
///
/// This does not infer quiescence, reconcile paid effects or discard historical authority.
pub async fn collect(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleasePoolView, ReleasePoolError> {
    let agent = icp
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| ReleaseObservationError::Authentication(Box::new(error)))?;
    collect_with_agent(&agent, review, registry).await
}

pub(super) async fn collect_with_agent(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleasePoolView, ReleasePoolError> {
    inventory::validate_registry_selection(review, registry)
        .map_err(inventory::ReleaseInventoryError::from)?;
    verify_agent(agent, &review.authority)?;
    tokio::time::timeout(CENSUS_DEADLINE, async {
        verify_owners(agent, review, registry).await?;
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        let mut roots = Vec::new();
        for entry in &registry.fleet_subnet_roots {
            let root = entry.fleet_subnet_root;
            let fail = |stage| ReleasePoolError::Observation { root, stage };
            let argument = candid::encode_one(Request::PoolRelease)
                .map_err(|_| fail(ReleasePoolStage::Query))?;
            let bytes = tokio::time::timeout(
                QUERY_DEADLINE,
                agent
                    .query(&root, protocol::CANIC_ROOT_STATUS)
                    .with_arg(argument)
                    .call(),
            )
            .await
            .map_err(|_| fail(ReleasePoolStage::Query))?
            .map_err(|_| fail(ReleasePoolStage::Query))?;
            let status = decode(root, &bytes, &mut remaining)?;
            validate(&status, root, entry.placement_subnet.as_principal()).map_err(fail)?;
            roots.push(status);
        }
        verify_owners(agent, review, registry).await?;
        Ok(FleetReleasePoolView { roots })
    })
    .await
    .map_err(|_| ReleasePoolError::Deadline)?
}

async fn verify_owners(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<(), inventory::ReleaseInventoryError> {
    inventory::verify_custody(
        agent,
        review.sources.iter().filter(|source| {
            matches!(
                source.role,
                FleetReleaseRole::Coordinator | FleetReleaseRole::Root
            )
        }),
    )
    .await?;
    inventory::verify_registry(agent, review, registry).await
}

fn decode(
    root: Principal,
    bytes: &[u8],
    remaining: &mut usize,
) -> Result<RootPoolReleaseResponse, ReleasePoolError> {
    let fail = |stage| ReleasePoolError::Observation { root, stage };
    if bytes.len() > RESPONSE_BYTES {
        return Err(fail(ReleasePoolStage::Decode));
    }
    *remaining = remaining
        .checked_sub(bytes.len())
        .ok_or_else(|| fail(ReleasePoolStage::Budget))?;
    let mut config = candid::de::DecoderConfig::new();
    config
        .set_decoding_quota(RESPONSE_BYTES * 8)
        .set_skipping_quota(RESPONSE_BYTES * 8)
        .set_max_type_len(512)
        .set_max_header_len(16 * 1024);
    let response: Result<Response, CanicError> =
        candid::utils::decode_one_with_config(bytes, &config)
            .map_err(|_| fail(ReleasePoolStage::Decode))?;
    let Response::PoolRelease(status) =
        response.map_err(|rejection| ReleasePoolError::Rejected { root, rejection })?;
    Ok(*status)
}

fn validate(
    status: &RootPoolReleaseResponse,
    root: Principal,
    subnet: &Principal,
) -> Result<(), ReleasePoolStage> {
    let import_matches = status.capacity_import.as_ref().is_none_or(|import| {
        import.reservation.root == root && import.reservation.subnet == *subnet
    });
    let creation_matches = status
        .creation
        .as_ref()
        .is_none_or(|creation| creation.root == root && creation.placement_subnet == *subnet);
    if status.root != root || !import_matches || !creation_matches {
        return Err(ReleasePoolStage::Binding);
    }
    Ok(())
}
