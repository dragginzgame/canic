//! Module: fleet_ensure::ops::release::intents
//!
//! Responsibility: collect bounded canonical accounting evidence from selected Roots and Coordinator.
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
        view::release::FleetReleaseIntentsView,
    },
    icp::IcpCli,
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::{
        error::Error as CanicError,
        fleet_registry::FleetRegistry,
        release_intents::{IntentReleaseEntry, IntentReleaseKey, IntentReleaseResponse},
    },
    protocol,
};
use ic_agent::Agent;
use serde::Deserialize;
use std::{collections::BTreeMap, time::Duration};
use thiserror::Error;

const RESPONSE_BYTES: usize = 256 * 1024;
const MAXIMUM_CENSUS_BYTES: usize = 8 * 1024 * 1024;
const MAXIMUM_INTENTS: usize = 4096;
const QUERY_DEADLINE: Duration = Duration::from_secs(15);
const CENSUS_DEADLINE: Duration = Duration::from_secs(120);

/// Refusal of the complete collection; no partial intent inventory is returned.
#[derive(Debug, Error)]
pub enum ReleaseIntentsError {
    #[error(transparent)]
    Authentication(#[from] ReleaseObservationError),
    #[error(transparent)]
    Inventory(#[from] inventory::ReleaseInventoryError),
    #[error("Fleet release intent census exceeded its deadline")]
    Deadline,
    #[error("Owner {owner} intent census failed at {stage:?}")]
    Observation {
        owner: Principal,
        stage: ReleaseIntentsStage,
    },
    #[error("Owner {owner} refused intent census: {rejection}")]
    Rejected {
        owner: Principal,
        rejection: CanicError,
    },
}

/// Typed observation failure, independent of diagnostic wording.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseIntentsStage {
    Binding,
    Budget,
    Decode,
    Pagination,
    Query,
}

#[derive(CandidType)]
enum Request {
    IntentRelease(Option<IntentReleaseKey>),
}

#[derive(CandidType, Deserialize)]
enum Response {
    IntentRelease(IntentReleaseResponse),
}

/// Retain original canonical accounting evidence without interpreting expiry or completion as settlement.
pub async fn collect(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseIntentsView, ReleaseIntentsError> {
    let agent = icp
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| ReleaseObservationError::Authentication(Box::new(error)))?;
    collect_with_agent(&agent, review, registry).await
}

pub(super) async fn collect_with_agent(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseIntentsView, ReleaseIntentsError> {
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
            let fail = |stage| ReleaseIntentsError::Observation { owner, stage };
            let mut pages = Pages::new(owner);
            loop {
                let argument = candid::encode_one(Request::IntentRelease(pages.cursor))
                    .map_err(|_| fail(ReleaseIntentsStage::Query))?;
                let bytes = tokio::time::timeout(
                    QUERY_DEADLINE,
                    agent.query(&owner, method).with_arg(argument).call(),
                )
                .await
                .map_err(|_| fail(ReleaseIntentsStage::Query))?
                .map_err(|_| fail(ReleaseIntentsStage::Query))?;
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
        Ok(FleetReleaseIntentsView { owners })
    })
    .await
    .map_err(|_| ReleaseIntentsError::Deadline)?
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
) -> Result<IntentReleaseResponse, ReleaseIntentsError> {
    let fail = |stage| ReleaseIntentsError::Observation { owner, stage };
    if bytes.len() > RESPONSE_BYTES {
        return Err(fail(ReleaseIntentsStage::Decode));
    }
    *remaining = remaining
        .checked_sub(bytes.len())
        .ok_or_else(|| fail(ReleaseIntentsStage::Budget))?;
    let mut config = candid::de::DecoderConfig::new();
    config
        .set_decoding_quota(RESPONSE_BYTES * 8)
        .set_skipping_quota(RESPONSE_BYTES * 8)
        .set_max_type_len(512)
        .set_max_header_len(16 * 1024);
    let response: Result<Response, CanicError> =
        candid::utils::decode_one_with_config(bytes, &config)
            .map_err(|_| fail(ReleaseIntentsStage::Decode))?;
    let Response::IntentRelease(page) =
        response.map_err(|rejection| ReleaseIntentsError::Rejected { owner, rejection })?;
    Ok(page)
}

struct Pages {
    owner: Principal,
    cursor: Option<IntentReleaseKey>,
    pages: Vec<IntentReleaseResponse>,
}

impl Pages {
    const fn new(owner: Principal) -> Self {
        Self {
            owner,
            cursor: None,
            pages: Vec::new(),
        }
    }

    fn push(&mut self, page: IntentReleaseResponse) -> Result<bool, ReleaseIntentsStage> {
        if page.owner != self.owner {
            return Err(ReleaseIntentsStage::Binding);
        }
        if self.pages.len() >= MAXIMUM_INTENTS {
            return Err(ReleaseIntentsStage::Budget);
        }
        if let Some(entry) = &page.entry {
            let key = entry_key(entry);
            if self.cursor.is_some_and(|cursor| key <= cursor)
                || page.next_after.is_some_and(|cursor| cursor != key)
            {
                return Err(ReleaseIntentsStage::Pagination);
            }
        } else if self.cursor.is_some() || page.next_after.is_some() {
            return Err(ReleaseIntentsStage::Pagination);
        }
        if page.next_after.is_some() && self.pages.len() + 1 == MAXIMUM_INTENTS {
            return Err(ReleaseIntentsStage::Budget);
        }
        self.cursor = page.next_after;
        self.pages.push(page);
        Ok(self.cursor.is_none())
    }
}

/// Derive the store-qualified key without trusting a separately supplied wire cursor.
#[must_use]
pub const fn entry_key(entry: &IntentReleaseEntry) -> IntentReleaseKey {
    match entry {
        IntentReleaseEntry::Local { intent_id, .. } => IntentReleaseKey::Local(*intent_id),
        IntentReleaseEntry::ReceiptBacked(record) => {
            IntentReleaseKey::ReceiptBacked(record.operation_id)
        }
    }
}
