//! Module: fleet_ensure::ops::release::receipts
//!
//! Responsibility: collect bounded shared replay evidence from selected Roots and Coordinator.
//! Does not own: pruning, accounting settlement, producer fences or effect authority.
//! Boundary: signed queries retain original metadata within certified custody/Registry brackets.

#[cfg(test)]
pub(super) mod tests;

use crate::{
    fleet_ensure::{
        model::release::{FleetReleaseReviewRecord, FleetReleaseRole},
        ops::release::{
            inventory,
            observation::{ReleaseObservationError, verify_agent},
        },
        view::release::FleetReleaseReceiptsView,
    },
    icp::IcpCli,
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::{
        error::Error as CanicError, fleet_registry::FleetRegistry,
        release_receipts::ReplayReleaseResponse,
    },
    protocol,
};
use ic_agent::Agent;
use serde::Deserialize;
use std::{collections::BTreeMap, time::Duration};
use thiserror::Error;

const RESPONSE_BYTES: usize = 256 * 1024;
const MAXIMUM_CENSUS_BYTES: usize = 8 * 1024 * 1024;
const MAXIMUM_RECEIPTS: usize = 4096;
const QUERY_DEADLINE: Duration = Duration::from_secs(15);
const CENSUS_DEADLINE: Duration = Duration::from_secs(120);

/// Refusal of the complete collection; no partial receipt inventory is returned.
#[derive(Debug, Error)]
pub enum ReleaseReceiptsError {
    #[error(transparent)]
    Authentication(#[from] ReleaseObservationError),
    #[error(transparent)]
    Inventory(#[from] inventory::ReleaseInventoryError),
    #[error("Fleet release replay census exceeded its deadline")]
    Deadline,
    #[error("Owner {owner} replay census failed at {stage:?}")]
    Observation {
        owner: Principal,
        stage: ReleaseReceiptsStage,
    },
    #[error("Owner {owner} refused replay census: {rejection}")]
    Rejected {
        owner: Principal,
        rejection: CanicError,
    },
}

/// Typed observation failure, independent of diagnostic wording.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseReceiptsStage {
    Binding,
    Budget,
    Decode,
    Pagination,
    Query,
}

#[derive(CandidType)]
enum Request {
    ReplayRelease(Option<[u8; 32]>),
}

#[derive(CandidType, Deserialize)]
enum Response {
    ReplayRelease(ReplayReleaseResponse),
}

/// Retain original shared replay evidence without interpreting expiry or completion as settlement.
pub async fn collect(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseReceiptsView, ReleaseReceiptsError> {
    let agent = icp
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| ReleaseObservationError::Authentication(Box::new(error)))?;
    collect_with_agent(&agent, review, registry).await
}

pub(super) async fn collect_with_agent(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseReceiptsView, ReleaseReceiptsError> {
    inventory::validate_registry_selection(review, registry)
        .map_err(inventory::ReleaseInventoryError::from)?;
    verify_agent(agent, &review.authority)?;
    tokio::time::timeout(CENSUS_DEADLINE, async {
        verify_owners(agent, review, registry).await?;
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        let mut owners = BTreeMap::new();
        let targets =
            std::iter::once((review.authority.coordinator, protocol::CANIC_OBSERVABILITY)).chain(
                registry
                    .fleet_subnet_roots
                    .iter()
                    .map(|entry| (entry.fleet_subnet_root, protocol::CANIC_ROOT_STATUS)),
            );
        for (owner, method) in targets {
            let fail = |stage| ReleaseReceiptsError::Observation { owner, stage };
            let mut pages = Pages::new(owner);
            loop {
                let argument = candid::encode_one(Request::ReplayRelease(pages.cursor))
                    .map_err(|_| fail(ReleaseReceiptsStage::Query))?;
                let bytes = tokio::time::timeout(
                    QUERY_DEADLINE,
                    agent.query(&owner, method).with_arg(argument).call(),
                )
                .await
                .map_err(|_| fail(ReleaseReceiptsStage::Query))?
                .map_err(|_| fail(ReleaseReceiptsStage::Query))?;
                if pages
                    .push(decode_response(owner, &bytes, &mut remaining)?)
                    .map_err(fail)?
                {
                    break;
                }
            }
            owners.insert(owner, pages.pages);
        }
        verify_owners(agent, review, registry).await?;
        Ok(FleetReleaseReceiptsView { owners })
    })
    .await
    .map_err(|_| ReleaseReceiptsError::Deadline)?
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

/// Decode one bounded response and debit its raw bytes from the caller's aggregate allowance.
/// Collection separately authenticates provenance and validates owner/cursor bindings.
pub fn decode_response(
    owner: Principal,
    bytes: &[u8],
    remaining: &mut usize,
) -> Result<ReplayReleaseResponse, ReleaseReceiptsError> {
    let fail = |stage| ReleaseReceiptsError::Observation { owner, stage };
    if bytes.len() > RESPONSE_BYTES {
        return Err(fail(ReleaseReceiptsStage::Decode));
    }
    *remaining = remaining
        .checked_sub(bytes.len())
        .ok_or_else(|| fail(ReleaseReceiptsStage::Budget))?;
    let mut config = candid::de::DecoderConfig::new();
    config
        .set_decoding_quota(RESPONSE_BYTES * 8)
        .set_skipping_quota(RESPONSE_BYTES * 8)
        .set_max_type_len(512)
        .set_max_header_len(16 * 1024);
    let response: Result<Response, CanicError> =
        candid::utils::decode_one_with_config(bytes, &config)
            .map_err(|_| fail(ReleaseReceiptsStage::Decode))?;
    let Response::ReplayRelease(page) =
        response.map_err(|rejection| ReleaseReceiptsError::Rejected { owner, rejection })?;
    Ok(page)
}

struct Pages {
    owner: Principal,
    cursor: Option<[u8; 32]>,
    pages: Vec<ReplayReleaseResponse>,
}

impl Pages {
    const fn new(owner: Principal) -> Self {
        Self {
            owner,
            cursor: None,
            pages: Vec::new(),
        }
    }

    fn push(&mut self, page: ReplayReleaseResponse) -> Result<bool, ReleaseReceiptsStage> {
        if page.owner != self.owner {
            return Err(ReleaseReceiptsStage::Binding);
        }
        if self.pages.len() >= MAXIMUM_RECEIPTS {
            return Err(ReleaseReceiptsStage::Budget);
        }
        if let Some(entry) = &page.entry {
            if self.cursor.is_some_and(|cursor| entry.slot <= cursor)
                || page.next_after.is_some_and(|cursor| cursor != entry.slot)
            {
                return Err(ReleaseReceiptsStage::Pagination);
            }
        } else if self.cursor.is_some() || page.next_after.is_some() {
            return Err(ReleaseReceiptsStage::Pagination);
        }
        if page.next_after.is_some() && self.pages.len() + 1 == MAXIMUM_RECEIPTS {
            return Err(ReleaseReceiptsStage::Budget);
        }
        self.cursor = page.next_after;
        self.pages.push(page);
        Ok(self.cursor.is_none())
    }
}
