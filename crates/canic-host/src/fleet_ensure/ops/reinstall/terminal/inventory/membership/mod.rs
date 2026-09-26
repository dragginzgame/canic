//! Read-only enumeration of completed-source Root pools under certified physical custody.
//!
//! No management update, reset, payment or source execution journal is used. Pool
//! queries are sampled membership evidence; cached cycles never become live balances.

mod contract;
mod coordinator;
mod parentage;
#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        CompletedEstateInventoryView, CompletedEstateMembershipView, CompletedPoolAssetView,
        CompletedRootMembershipView, CompletedWorkloadAllocationView,
        model::DesiredCanisterKind,
        ops::{
            reinstall::terminal::inventory::{custody, ledger},
            retained_contract::{
                CompletedCustodyError, RetainedContractError, inspect_completed_source,
            },
        },
    },
    icp::IcpCli,
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::pool::{CanisterPoolAssetStatus, CanisterPoolResponse, CanisterPoolStatusRequest},
    ids::FleetSubnetCanisterPoolConfig,
    protocol,
};
use ic_agent::Agent;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};

use thiserror::Error;

pub use coordinator::CompletedCoordinatorError;
pub use parentage::CompletedParentageError;

const PAGE_SIZE: u16 = 256;
const RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const PASS_TIMEOUT: Duration = Duration::from_secs(60);
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Failure to account for exactly the recorded physical pool members.
/// No failure permits ignoring, replacing or deleting an asset.
#[derive(Debug, Error)]
pub enum CompletedMembershipError {
    #[error("completed source evidence failed: {0}")]
    Source(#[from] Box<RetainedContractError>),
    #[error("completed estate custody failed: {0}")]
    Custody(#[from] CompletedCustodyError),
    #[error("completed source Ledger accounting failed: {0}")]
    Ledger(#[from] ledger::CompletedLedgerError),
    #[error("completed source Coordinator membership failed: {0}")]
    Coordinator(#[from] CompletedCoordinatorError),
    #[error("completed source application parentage failed: {0}")]
    Parentage(#[from] CompletedParentageError),
    #[error("completed source evidence or physical authority changed during enumeration")]
    SourceChanged,
    #[error(
        "Root {root} source interface does not bind the exact maintained read-only pool contract"
    )]
    Contract { root: Principal },
    #[error("completed Root membership enumeration expired")]
    Expired,
    #[error("Root {root} pool query failed: {source}")]
    Query {
        root: Principal,
        #[source]
        source: Box<ic_agent::AgentError>,
    },
    #[error("Root {root} pool response could not be decoded: {source}")]
    Decode {
        root: Principal,
        #[source]
        source: candid::Error,
    },
    #[error("Root {root} rejected its protected pool query: {reason:?}")]
    Rejected {
        root: Principal,
        reason: Box<canic_core::dto::error::Error>,
    },
    #[error("Root {root} pool membership differs from completed source evidence")]
    Membership { root: Principal },
    #[error("Root {root} pool has an unfinished creation, handoff or lifecycle transition")]
    Unsettled { root: Principal },
    #[error("Root {root} pool pagination or summary changed or is incomplete")]
    Pagination { root: Principal },
}

#[derive(CandidType)]
enum Request {
    Pool(CanisterPoolStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum Response {
    Pool(CanisterPoolResponse),
}

/// Read complete source Root inventories without changing any canister or local record.
///
/// Source modules/interfaces and certified custody are checked before querying and
/// checked again after enumeration. A journaled mutation fence and fresh native,
/// reserved balances are still required before destructive admission. Default
/// Ledger accounts are observed without payments or historical balance rebasing.
pub async fn inspect(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    icp: &IcpCli,
) -> Result<CompletedEstateMembershipView, CompletedMembershipError> {
    let started = Instant::now();
    let source = inspect_completed_source(workspace, environment, fleet).map_err(Box::new)?;
    for (name, entry) in &source.inventory.canisters {
        if entry.kind == DesiredCanisterKind::Component {
            let binding = source
                .source_protocols
                .get(name)
                .ok_or(CompletedMembershipError::SourceChanged)?;
            parentage::verify_contract(entry.principal, binding)?;
        }
        if matches!(
            entry.kind,
            DesiredCanisterKind::Root | DesiredCanisterKind::Coordinator
        ) {
            let binding = source
                .source_protocols
                .get(name)
                .ok_or(CompletedMembershipError::SourceChanged)?;
            if entry.kind == DesiredCanisterKind::Root {
                contract::verify(entry.principal, binding)?;
            } else {
                coordinator::verify_contract(entry.principal, binding)?;
            }
        }
    }
    let agent = icp
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| CompletedCustodyError::Identity(Box::new(error)))?;
    let before = custody::observe(&agent, &source.inventory).await?;
    let before = custody::capture(&before, &source.inventory.receipts.documents)?;
    coordinator::observe(&agent, &source.inventory.coordinator_registry).await?;
    let mut roots = BTreeMap::new();
    for (name, entry) in &source.inventory.canisters {
        if entry.kind != DesiredCanisterKind::Root {
            continue;
        }
        if !source.source_protocols.contains_key(name) {
            return Err(CompletedMembershipError::SourceChanged);
        }
        let expected = expected_members(&source.inventory, name)?;
        let mut collector = Collector::new(entry.principal, expected);
        loop {
            require_fresh(started)?;
            let page = query(&agent, entry.principal, collector.cursor).await?;
            if collector.push(page)? {
                break;
            }
        }
        roots.insert(name.clone(), collector.finish()?);
    }
    parentage::observe(&agent, &source, &roots, started).await?;
    let ledger = ledger::observe(&agent, &source.inventory, started).await?;
    let coordinator = coordinator::observe(&agent, &source.inventory.coordinator_registry).await?;
    let after = custody::observe(&agent, &source.inventory).await?;
    let after_record = custody::capture(&after, &source.inventory.receipts.documents)?;
    let refreshed = inspect_completed_source(workspace, environment, fleet).map_err(Box::new)?;
    if !custody::same_authority(&before, &after_record)
        || refreshed.inventory.receipts.documents != source.inventory.receipts.documents
        || refreshed.source_protocols != source.source_protocols
    {
        return Err(CompletedMembershipError::SourceChanged);
    }
    require_fresh(started)?;
    Ok(CompletedEstateMembershipView {
        custody: after,
        coordinator,
        roots,
        ledger,
    })
}

fn require_fresh(started: Instant) -> Result<(), CompletedMembershipError> {
    if started.elapsed() > PASS_TIMEOUT {
        Err(CompletedMembershipError::Expired)
    } else {
        Ok(())
    }
}

async fn query(
    agent: &Agent,
    root: Principal,
    cursor: Option<Principal>,
) -> Result<CanisterPoolResponse, CompletedMembershipError> {
    let argument = candid::encode_one(Request::Pool(CanisterPoolStatusRequest {
        start_after: cursor,
        limit: PAGE_SIZE,
    }))
    .map_err(|source| CompletedMembershipError::Decode { root, source })?;
    let bytes = tokio::time::timeout(
        QUERY_TIMEOUT,
        agent
            .query(&root, protocol::CANIC_ROOT_STATUS)
            .with_arg(argument)
            .call(),
    )
    .await
    .map_err(|_| CompletedMembershipError::Expired)?
    .map_err(|source| CompletedMembershipError::Query {
        root,
        source: Box::new(source),
    })?;
    let mut config = candid::de::DecoderConfig::new();
    config.set_decoding_quota(RESPONSE_BYTES * 64);
    config.set_skipping_quota(RESPONSE_BYTES);
    let response: Result<Response, canic_core::dto::error::Error> =
        candid::utils::decode_one_with_config(&bytes, &config)
            .map_err(|source| CompletedMembershipError::Decode { root, source })?;
    let Response::Pool(page) = response.map_err(|source| CompletedMembershipError::Rejected {
        root,
        reason: Box::new(source),
    })?;
    Ok(page)
}

fn expected_members(
    source: &CompletedEstateInventoryView,
    name: &str,
) -> Result<BTreeMap<Principal, DesiredCanisterKind>, CompletedMembershipError> {
    let root = source.canisters[name].principal;
    let mut expected = BTreeMap::new();
    for entry in source
        .canisters
        .values()
        .filter(|entry| entry.root.as_deref() == Some(name) && entry.principal != root)
    {
        if expected.insert(entry.principal, entry.kind).is_some() {
            return Err(CompletedMembershipError::Membership { root });
        }
    }
    if expected.is_empty() {
        return Err(CompletedMembershipError::Membership { root });
    }
    Ok(expected)
}

/// All accepted terminal counts must remain identical across every page.
#[derive(Eq, PartialEq)]
struct Summary {
    config: FleetSubnetCanisterPoolConfig,
    completed_handoffs: u64,
    tracked: u32,
    store: u32,
    workload: u32,
    ready: u32,
}

struct Collector {
    root: Principal,
    expected: BTreeMap<Principal, DesiredCanisterKind>,
    assets: BTreeMap<Principal, CompletedPoolAssetView>,
    summary: Option<Summary>,
    cursor: Option<Principal>,
    complete: bool,
}

impl Collector {
    const fn new(root: Principal, expected: BTreeMap<Principal, DesiredCanisterKind>) -> Self {
        Self {
            root,
            expected,
            assets: BTreeMap::new(),
            summary: None,
            cursor: None,
            complete: false,
        }
    }

    fn push(&mut self, page: CanisterPoolResponse) -> Result<bool, CompletedMembershipError> {
        let pagination = || CompletedMembershipError::Pagination { root: self.root };
        let membership = || CompletedMembershipError::Membership { root: self.root };
        if page.pending_creation.is_some()
            || page.pending_handoff.is_some()
            || [
                page.store_deletion_pending,
                page.pending_reset,
                page.claimed,
                page.recycling,
                page.handing_off,
                page.failed,
            ]
            .iter()
            .any(|count| *count != 0)
        {
            return Err(CompletedMembershipError::Unsettled { root: self.root });
        }
        if self.complete
            || page.entries.len() > usize::from(PAGE_SIZE)
            || page.pooled != page.ready
            || page.surplus != page.ready.saturating_sub(page.config.maximum_size)
        {
            return Err(pagination());
        }
        let summary = Summary {
            config: page.config,
            completed_handoffs: page.completed_handoffs,
            tracked: page.tracked,
            store: page.store,
            workload: page.workload,
            ready: page.ready,
        };
        if self
            .summary
            .as_ref()
            .is_some_and(|expected| *expected != summary)
        {
            return Err(pagination());
        }
        if usize::try_from(summary.tracked).ok() != Some(self.expected.len()) {
            return Err(membership());
        }
        self.summary = Some(summary);
        let mut last = self.cursor;
        for asset in page.entries {
            if last.is_some_and(|last| asset.canister_id.as_slice() <= last.as_slice()) {
                return Err(pagination());
            }
            let (kind, allocation) = match asset.status {
                CanisterPoolAssetStatus::Store => (DesiredCanisterKind::Store, None),
                CanisterPoolAssetStatus::Ready => (DesiredCanisterKind::Pool, None),
                CanisterPoolAssetStatus::Workload { claim } => (
                    DesiredCanisterKind::Component,
                    Some(CompletedWorkloadAllocationView {
                        component: claim.component,
                        operation_id: claim.operation_id,
                    }),
                ),
                _ => return Err(CompletedMembershipError::Unsettled { root: self.root }),
            };
            if self.expected.get(&asset.canister_id) != Some(&kind) {
                return Err(membership());
            }
            last = Some(asset.canister_id);
            self.assets.insert(
                asset.canister_id,
                CompletedPoolAssetView { kind, allocation },
            );
        }
        if let Some(next) = page.next_start_after {
            if Some(next) != last
                || Some(next) == self.cursor
                || self.assets.len() >= self.expected.len()
            {
                return Err(pagination());
            }
            self.cursor = Some(next);
        } else {
            self.complete = true;
        }
        Ok(self.complete)
    }

    fn finish(self) -> Result<CompletedRootMembershipView, CompletedMembershipError> {
        let invalid = || CompletedMembershipError::Pagination { root: self.root };
        let summary = self.summary.ok_or_else(invalid)?;
        let count = |kind| {
            self.assets
                .values()
                .filter(|asset| asset.kind == kind)
                .count()
        };
        let counts_match = count(DesiredCanisterKind::Store) == summary.store as usize
            && count(DesiredCanisterKind::Pool) == summary.ready as usize
            && count(DesiredCanisterKind::Component) == summary.workload as usize;
        if !self.complete
            || self.assets.len() != self.expected.len()
            || summary.store != 1
            || !counts_match
        {
            return Err(invalid());
        }
        Ok(CompletedRootMembershipView {
            root: self.root,
            config: summary.config,
            completed_handoffs: summary.completed_handoffs,
            assets: self.assets,
        })
    }
}
