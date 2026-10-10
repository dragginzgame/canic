//! Module: fleet_ensure::ops::release::provisioning
//!
//! Responsibility: collect bounded signed pages from every reviewed Root provisioning journal.
//! Does not own: settlement, producer fences, execution or new spending authority.
//! Boundary: certified custody and Registry observations bracket the complete collection.

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
        view::release::{FleetReleaseProvisioningView, FleetReleaseRootProvisioningView},
    },
    icp::IcpCli,
};
use candid::Principal;
use canic_contracts::{
    dto::{
        error::Error as CanicError,
        fleet_registry::FleetRegistry,
        root::{
            RootProvisioningReleaseKey as Key, RootProvisioningReleasePhase as Phase,
            RootProvisioningReleaseResponse,
        },
        wire::projection::release_provisioning::{Request, Response},
    },
    protocol,
};
use ic_agent::Agent;
use std::time::Duration;
use thiserror::Error;

pub(in crate::fleet_ensure) use assessment::assessment_facts;

const RESPONSE_BYTES: usize = 256 * 1024;
const MAXIMUM_CENSUS_BYTES: usize = 8 * 1024 * 1024;
const MAXIMUM_OPERATIONS: usize = 4096;
const QUERY_DEADLINE: Duration = Duration::from_secs(15);
const CENSUS_DEADLINE: Duration = Duration::from_secs(120);

/// A refused collection returns no partial journal evidence.
#[derive(Debug, Error)]
pub enum ReleaseProvisioningError {
    #[error(transparent)]
    Authentication(#[from] ReleaseObservationError),
    #[error(transparent)]
    Inventory(#[from] inventory::ReleaseInventoryError),
    #[error("Fleet release provisioning census exceeded its deadline")]
    Deadline,
    #[error("Root {root} provisioning census failed at {stage:?}")]
    Observation {
        root: Principal,
        stage: ReleaseProvisioningStage,
    },
    #[error("Root {root} refused provisioning census: {rejection}")]
    Rejected {
        root: Principal,
        rejection: CanicError,
    },
}

/// Machine-readable collection refusal independent of presentation wording.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseProvisioningStage {
    Binding,
    Budget,
    Decode,
    Pagination,
    Query,
}

/// Discover both journal kinds without treating historical completion as a release blocker.
pub async fn collect(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseProvisioningView, ReleaseProvisioningError> {
    let agent = icp
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| ReleaseObservationError::Authentication(Box::new(error)))?;
    collect_with_agent(&agent, review, registry).await
}

pub(super) async fn collect_with_agent(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseProvisioningView, ReleaseProvisioningError> {
    inventory::validate_registry_selection(review, registry)
        .map_err(inventory::ReleaseInventoryError::from)?;
    verify_agent(agent, &review.authority)?;
    tokio::time::timeout(CENSUS_DEADLINE, async {
        verify_owners(agent, review, registry).await?;
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        let mut roots = Vec::new();
        for entry in &registry.fleet_subnet_roots {
            let root = entry.fleet_subnet_root;
            let fail = |stage| ReleaseProvisioningError::Observation { root, stage };
            let mut pages = Pages::new(root);
            loop {
                let argument = candid::encode_one(Request::ProvisioningRelease(pages.cursor))
                    .map_err(|_| fail(ReleaseProvisioningStage::Query))?;
                let bytes = tokio::time::timeout(
                    QUERY_DEADLINE,
                    agent
                        .query(&root, protocol::CANIC_ROOT_STATUS)
                        .with_arg(argument)
                        .call(),
                )
                .await
                .map_err(|_| fail(ReleaseProvisioningStage::Query))?
                .map_err(|_| fail(ReleaseProvisioningStage::Query))?;
                if pages
                    .push(decode_response(root, &bytes, &mut remaining)?)
                    .map_err(fail)?
                {
                    break;
                }
            }
            roots.push(FleetReleaseRootProvisioningView {
                root,
                pages: pages.pages,
            });
        }
        verify_owners(agent, review, registry).await?;
        Ok(FleetReleaseProvisioningView { roots })
    })
    .await
    .map_err(|_| ReleaseProvisioningError::Deadline)?
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

/// Decode one bounded response and charge its bytes to the caller's aggregate allowance.
/// The collection owner separately authenticates provenance and validates pagination.
pub fn decode_response(
    root: Principal,
    bytes: &[u8],
    remaining: &mut usize,
) -> Result<RootProvisioningReleaseResponse, ReleaseProvisioningError> {
    let fail = |stage| ReleaseProvisioningError::Observation { root, stage };
    if bytes.len() > RESPONSE_BYTES {
        return Err(fail(ReleaseProvisioningStage::Decode));
    }
    *remaining = remaining
        .checked_sub(bytes.len())
        .ok_or_else(|| fail(ReleaseProvisioningStage::Budget))?;
    let mut config = candid::de::DecoderConfig::new();
    config
        .set_decoding_quota(RESPONSE_BYTES * 8)
        .set_skipping_quota(RESPONSE_BYTES * 8)
        .set_max_type_len(512)
        .set_max_header_len(16 * 1024);
    let response: Result<Response, CanicError> =
        candid::utils::decode_one_with_config(bytes, &config)
            .map_err(|_| fail(ReleaseProvisioningStage::Decode))?;
    let Response::ProvisioningRelease(page) =
        response.map_err(|rejection| ReleaseProvisioningError::Rejected { root, rejection })?;
    Ok(page)
}

struct Pages {
    root: Principal,
    cursor: Option<Key>,
    pages: Vec<RootProvisioningReleaseResponse>,
}

impl Pages {
    const fn new(root: Principal) -> Self {
        Self {
            root,
            cursor: None,
            pages: Vec::new(),
        }
    }

    fn push(
        &mut self,
        page: RootProvisioningReleaseResponse,
    ) -> Result<bool, ReleaseProvisioningStage> {
        if page.root != self.root
            || self.pages.first().is_some_and(|first| {
                first.active_provisioning != page.active_provisioning
                    || first.active_directory_synchronization
                        != page.active_directory_synchronization
            })
        {
            return Err(ReleaseProvisioningStage::Binding);
        }
        if self.pages.len() >= MAXIMUM_OPERATIONS {
            return Err(ReleaseProvisioningStage::Budget);
        }
        if let Some(entry) = &page.entry {
            let directory = matches!(
                entry.phase,
                Phase::DirectoryPlanned
                    | Phase::DirectorySynchronizing
                    | Phase::DirectorySynchronized
            );
            if directory != matches!(entry.key, Key::DirectorySynchronization(_)) {
                return Err(ReleaseProvisioningStage::Binding);
            }
            if self.cursor.is_some_and(|cursor| entry.key <= cursor)
                || page.next_after.is_some_and(|cursor| cursor != entry.key)
            {
                return Err(ReleaseProvisioningStage::Pagination);
            }
        } else if self.cursor.is_some() || page.next_after.is_some() {
            return Err(ReleaseProvisioningStage::Pagination);
        }
        if page.next_after.is_some() && self.pages.len() + 1 == MAXIMUM_OPERATIONS {
            return Err(ReleaseProvisioningStage::Budget);
        }
        self.cursor = page.next_after;
        self.pages.push(page);
        Ok(self.cursor.is_none())
    }
}
