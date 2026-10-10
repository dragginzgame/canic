//! Collect Coordinator and Root funding evidence without settling effects or fencing producers.
//!
//! Certified custody and Registry bindings bracket bounded signed queries; no partial
//! census escapes a refusal. Destructive admission remains with the release workflow.

mod assessment;
mod coordinator;

#[cfg(test)]
pub(super) mod tests;

use crate::{
    fleet_ensure::{
        model::release::{FleetReleaseReviewRecord, FleetReleaseRole},
        ops::release::{inventory, observation::verify_agent},
        view::release::{FleetReleaseFundingView, FleetReleaseRootFundingView},
    },
    icp::IcpCli,
};
use candid::{CandidType, Principal};
use canic_contracts::{
    dto::{
        error::Error as CanicError,
        fleet_registry::FleetRegistry,
        root::RootFundingReleaseResponse,
        wire::projection::release_funding::{Request, Response},
    },
    protocol,
};
use canic_core::shared_support::fleet_funding_policy::fleet_subnet_root_funding_policy_hash;
use ic_agent::Agent;
use serde::Deserialize;
use std::{collections::BTreeSet, time::Duration};
use thiserror::Error;

pub(in crate::fleet_ensure) use assessment::assessment_facts;

const RESPONSE_BYTES: usize = 256 * 1024;
const MAXIMUM_CENSUS_BYTES: usize = 8 * 1024 * 1024;
const MAXIMUM_REFILLS: usize = 4096;
const PAGE_SIZE: usize = 32;
const QUERY_DEADLINE: Duration = Duration::from_secs(15);
const CENSUS_DEADLINE: Duration = Duration::from_secs(120);

/// Failed observation returns no partial funding history or release authority.
#[derive(Debug, Error)]
pub enum ReleaseFundingError {
    #[error(transparent)]
    Inventory(#[from] inventory::ReleaseInventoryError),
    #[error(transparent)]
    Authentication(#[from] super::observation::ReleaseObservationError),
    #[error("Fleet release funding census exceeded its deadline")]
    Deadline,
    #[error("Canister {root} funding census failed at {stage:?}")]
    Observation {
        root: Principal,
        stage: ReleaseFundingStage,
    },
    #[error("Canister {root} refused funding census: {rejection}")]
    Rejected {
        root: Principal,
        rejection: CanicError,
    },
}

/// Exact boundary that refused a funding page, independent of explanatory prose.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseFundingStage {
    Binding,
    Budget,
    Decode,
    Pagination,
    Query,
}

/// Collect Coordinator treasury and every selected Root's funding evidence under reviewed custody.
///
/// This observes all outcomes, including exhausted notifications. It neither certifies
/// an atomic snapshot nor resolves default Ledger IDs, balances or paid obligations.
pub async fn collect(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseFundingView, ReleaseFundingError> {
    let agent = icp
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| {
            super::observation::ReleaseObservationError::Authentication(Box::new(error))
        })?;
    collect_with_agent(&agent, review, registry).await
}

pub(super) async fn collect_with_agent(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseFundingView, ReleaseFundingError> {
    inventory::validate_registry_selection(review, registry)
        .map_err(inventory::ReleaseInventoryError::from)?;
    verify_agent(agent, &review.authority)?;
    tokio::time::timeout(CENSUS_DEADLINE, async {
        inventory::verify_custody(
            agent,
            review.sources.iter().filter(|s| {
                matches!(
                    s.role,
                    FleetReleaseRole::Coordinator | FleetReleaseRole::Root
                )
            }),
        )
        .await?;
        inventory::verify_registry(agent, review, registry).await?;
        let mut remaining_bytes = MAXIMUM_CENSUS_BYTES;
        let coordinator = coordinator::collect(
            agent,
            review.authority.coordinator,
            registry,
            &mut remaining_bytes,
        )
        .await?;
        let mut roots = Vec::new();
        for root in &registry.fleet_subnet_roots {
            roots.push(
                read_root(
                    agent,
                    root.fleet_subnet_root,
                    fleet_subnet_root_funding_policy_hash(&root.funding),
                    &mut remaining_bytes,
                )
                .await?,
            );
        }
        inventory::verify_registry(agent, review, registry).await?;
        inventory::verify_custody(
            agent,
            review.sources.iter().filter(|s| {
                matches!(
                    s.role,
                    FleetReleaseRole::Coordinator | FleetReleaseRole::Root
                )
            }),
        )
        .await?;
        Ok(FleetReleaseFundingView { coordinator, roots })
    })
    .await
    .map_err(|_| ReleaseFundingError::Deadline)?
}

async fn read_root(
    agent: &Agent,
    root: Principal,
    policy_hash: [u8; 32],
    remaining_bytes: &mut usize,
) -> Result<FleetReleaseRootFundingView, ReleaseFundingError> {
    let mut pages = Pages::new(root, policy_hash);
    loop {
        let fail = |stage| ReleaseFundingError::Observation { root, stage };
        let argument = candid::encode_one(Request::FundingRelease(pages.cursor))
            .map_err(|_| fail(ReleaseFundingStage::Query))?;
        let bytes = tokio::time::timeout(
            QUERY_DEADLINE,
            agent
                .query(&root, protocol::CANIC_ROOT_STATUS)
                .with_arg(argument)
                .call(),
        )
        .await
        .map_err(|_| fail(ReleaseFundingStage::Query))?
        .map_err(|_| fail(ReleaseFundingStage::Query))?;
        let page = decode(root, &bytes, remaining_bytes)?;
        if pages.push(page).map_err(fail)? {
            return Ok(FleetReleaseRootFundingView {
                root,
                pages: pages.pages,
            });
        }
    }
}

fn decode(
    root: Principal,
    bytes: &[u8],
    remaining: &mut usize,
) -> Result<RootFundingReleaseResponse, ReleaseFundingError> {
    let Response::FundingRelease(page) = decode_response(root, bytes, remaining)?;
    Ok(*page)
}

fn decode_response<T: CandidType + for<'de> Deserialize<'de>>(
    root: Principal,
    bytes: &[u8],
    remaining: &mut usize,
) -> Result<T, ReleaseFundingError> {
    let fail = |stage| ReleaseFundingError::Observation { root, stage };
    if bytes.len() > RESPONSE_BYTES {
        return Err(fail(ReleaseFundingStage::Decode));
    }
    *remaining = remaining
        .checked_sub(bytes.len())
        .ok_or_else(|| fail(ReleaseFundingStage::Budget))?;
    let mut config = candid::de::DecoderConfig::new();
    config
        .set_decoding_quota(RESPONSE_BYTES * 8)
        .set_skipping_quota(RESPONSE_BYTES * 8)
        .set_max_type_len(512)
        .set_max_header_len(16 * 1024);
    let response: Result<T, CanicError> = candid::utils::decode_one_with_config(bytes, &config)
        .map_err(|_| fail(ReleaseFundingStage::Decode))?;
    response.map_err(|rejection| ReleaseFundingError::Rejected { root, rejection })
}

struct Pages {
    fleet_subnet_root: Principal,
    policy_hash: [u8; 32],
    cursor: Option<u64>,
    header: Option<Vec<u8>>,
    operations: BTreeSet<[u8; 32]>,
    pages: Vec<RootFundingReleaseResponse>,
}

impl Pages {
    const fn new(root: Principal, policy_hash: [u8; 32]) -> Self {
        Self {
            fleet_subnet_root: root,
            policy_hash,
            cursor: None,
            header: None,
            operations: BTreeSet::new(),
            pages: Vec::new(),
        }
    }

    fn push(&mut self, page: RootFundingReleaseResponse) -> Result<bool, ReleaseFundingStage> {
        if page.fleet_subnet_root != self.fleet_subnet_root || page.policy_hash != self.policy_hash
        {
            return Err(ReleaseFundingStage::Binding);
        }
        if page.icp_refills.len() > PAGE_SIZE
            || self.operations.len() + page.icp_refills.len() > MAXIMUM_REFILLS
        {
            return Err(ReleaseFundingStage::Budget);
        }
        // Encode only the bounded header to compare exact policy/request/receipt authority.
        let header = candid::encode_args((
            page.policy_generation,
            &page.icp_refill_policy,
            &page.current_request,
            &page.accepted_grant,
            &page.rotation_current,
        ))
        .map_err(|_| ReleaseFundingStage::Decode)?;
        if self
            .header
            .as_ref()
            .is_some_and(|expected| *expected != header)
        {
            return Err(ReleaseFundingStage::Binding);
        }
        let mut previous = self.cursor;
        for entry in &page.icp_refills {
            if entry.source_canister != self.fleet_subnet_root
                || entry.target_canister != self.fleet_subnet_root
            {
                return Err(ReleaseFundingStage::Binding);
            }
            if previous.is_some_and(|id| entry.record_id <= id)
                || !self.operations.insert(entry.response.operation_id)
            {
                return Err(ReleaseFundingStage::Pagination);
            }
            previous = Some(entry.record_id);
        }
        let premature_end = self.cursor.is_some() && page.icp_refills.is_empty();
        let continuation_matches = page.icp_refills.len() == PAGE_SIZE
            && page.next_after == previous
            && self.operations.len() < MAXIMUM_REFILLS;
        if premature_end || (page.next_after.is_some() && !continuation_matches) {
            return Err(ReleaseFundingStage::Pagination);
        }
        self.cursor = page.next_after;
        self.header = Some(header);
        self.pages.push(page);
        Ok(self.cursor.is_none())
    }
}
