//! Verify complete application child directories and their common Root allocation.
//!
//! Queries are bounded passive evidence. Certified custody independently owns module hashes.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        CompletedCanisterInventoryView, CompletedEstateInventoryView, CompletedRootMembershipView,
        CompletedSourceInspectionView,
        model::DesiredCanisterKind,
        ops::reinstall::terminal::inventory::membership::{
            QUERY_TIMEOUT, RESPONSE_BYTES, bounded_query, contract, require_fresh,
        },
    },
    protocol_binding::ResolvedProtocolBinding,
};
use candid::{CandidType, Principal, types::FuncMode};
use canic_core::{
    dto::{
        canister::CanisterInfo,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    protocol,
    role_contract::RoleCapabilityKey,
};
use ic_agent::Agent;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};
use thiserror::Error;

const PAGE_SIZE: u64 = 256;

/// Source application parentage is missing, conflicting or no longer complete.
#[derive(Debug, Error)]
pub enum CompletedParentageError {
    #[error("application {parent} has no exact maintained child-directory query contract")]
    Contract { parent: Principal },
    #[error("application {parent} child-directory encoding or decoding failed: {source}")]
    Decode {
        parent: Principal,
        #[source]
        source: candid::Error,
    },
    #[error("completed application parentage enumeration expired")]
    Expired,
    #[error(
        "application {parent} child directory or Root allocation differs from completed evidence"
    )]
    Membership { parent: Principal },
    #[error("application {parent} returned an incomplete or inconsistent child page")]
    Pagination { parent: Principal },
    #[error("application {parent} child-directory query failed: {source}")]
    Query {
        parent: Principal,
        #[source]
        source: Box<ic_agent::AgentError>,
    },
    #[error("application {parent} rejected child-directory observation: {reason:?}")]
    Rejected {
        parent: Principal,
        reason: Box<canic_core::dto::error::Error>,
    },
}

#[derive(CandidType)]
enum Request {
    Children(PageRequest),
}

#[derive(CandidType, Deserialize)]
enum Response {
    Children(Page<CanisterInfo>),
}

pub(super) fn verify_contract(
    parent: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<(), CompletedParentageError> {
    if !binding
        .binding()
        .capabilities
        .contains(&RoleCapabilityKey::ChildProvisioning)
    {
        return Ok(());
    }
    let text = contract::read(binding).ok_or(CompletedParentageError::Contract { parent })?;
    matches(&text).ok_or(CompletedParentageError::Contract { parent })
}

fn matches(text: &str) -> Option<()> {
    let (mut env, actor) = candid_parser::utils::CandidSource::Text(text).load().ok()?;
    let method = env
        .get_method(actor.as_ref()?, protocol::CANIC_PUBLIC_STATUS)
        .ok()?
        .clone();
    if method.modes != [FuncMode::Query] || method.args.len() != 1 || method.rets.len() != 1 {
        return None;
    }
    let request = contract::variant(&env, &method.args[0], "Children")?;
    let response = contract::variant(&env, &method.rets[0], "Ok")?;
    let response = contract::variant(&env, &response, "Children")?;
    let error = contract::variant(&env, &method.rets[0], "Err")?;
    contract::equal::<PageRequest>(&mut env, &request)?;
    contract::equal::<Page<CanisterInfo>>(&mut env, &response)?;
    contract::equal::<canic_core::dto::error::Error>(&mut env, &error)
}

pub(super) async fn observe(
    agent: &Agent,
    source: &CompletedSourceInspectionView,
    roots: &BTreeMap<String, CompletedRootMembershipView>,
    started: Instant,
) -> Result<(), CompletedParentageError> {
    for (name, parent) in &source.inventory.canisters {
        if parent.kind != DesiredCanisterKind::Component {
            continue;
        }
        let expected = expected_children(&source.inventory, roots, name, parent)?;
        let binding =
            parent
                .protocol_binding
                .as_ref()
                .ok_or(CompletedParentageError::Membership {
                    parent: parent.principal,
                })?;
        // Leaf roles have no Children method; their exact release capability binding
        // excludes provisioning, and expected_children rejects any recorded descendant.
        if !binding
            .capabilities
            .contains(&RoleCapabilityKey::ChildProvisioning)
        {
            continue;
        }
        let mut collector = Collector::new(parent.principal, expected);
        loop {
            require_fresh(started).map_err(|_| CompletedParentageError::Expired)?;
            let page = query(agent, parent.principal, collector.seen.len() as u64).await?;
            if collector.push(page)? {
                break;
            }
        }
    }
    Ok(())
}

fn expected_children(
    inventory: &CompletedEstateInventoryView,
    roots: &BTreeMap<String, CompletedRootMembershipView>,
    name: &str,
    parent: &CompletedCanisterInventoryView,
) -> Result<BTreeMap<Principal, CanisterRole>, CompletedParentageError> {
    let invalid = || CompletedParentageError::Membership {
        parent: parent.principal,
    };
    let root = parent
        .root
        .as_ref()
        .and_then(|name| roots.get(name))
        .ok_or_else(invalid)?;
    let allocation = root
        .assets
        .get(&parent.principal)
        .and_then(|asset| asset.allocation.as_ref())
        .ok_or_else(invalid)?;
    let mut expected = BTreeMap::new();
    for child in inventory
        .canisters
        .values()
        .filter(|child| child.parent.as_deref() == Some(name))
    {
        let child_allocation = root
            .assets
            .get(&child.principal)
            .and_then(|asset| asset.allocation.as_ref())
            .ok_or_else(invalid)?;
        if child.kind != DesiredCanisterKind::Component
            || child.root != parent.root
            || child_allocation.component != allocation.component
        {
            return Err(invalid());
        }
        let role = child
            .protocol_binding
            .as_ref()
            .ok_or_else(invalid)?
            .role
            .clone();
        if expected.insert(child.principal, role).is_some() {
            return Err(invalid());
        }
    }
    let binding = parent.protocol_binding.as_ref().ok_or_else(invalid)?;
    if !expected.is_empty()
        && !binding
            .capabilities
            .contains(&RoleCapabilityKey::ChildProvisioning)
    {
        return Err(invalid());
    }
    Ok(expected)
}

async fn query(
    agent: &Agent,
    parent: Principal,
    offset: u64,
) -> Result<Page<CanisterInfo>, CompletedParentageError> {
    let Response::Children(page) = bounded_query::query(
        agent,
        parent,
        protocol::CANIC_PUBLIC_STATUS,
        Request::Children(PageRequest {
            offset,
            limit: PAGE_SIZE,
        }),
        bounded_query::QueryLimits {
            timeout: QUERY_TIMEOUT,
            response_bytes: RESPONSE_BYTES,
        },
    )
    .await
    .map_err(|error| match error {
        bounded_query::QueryError::Decode(source) => {
            CompletedParentageError::Decode { parent, source }
        }
        bounded_query::QueryError::Expired => CompletedParentageError::Expired,
        bounded_query::QueryError::Query(source) => {
            CompletedParentageError::Query { parent, source }
        }
        bounded_query::QueryError::Rejected(reason) => {
            CompletedParentageError::Rejected { parent, reason }
        }
    })?;
    Ok(page)
}

struct Collector {
    parent: Principal,
    expected: BTreeMap<Principal, CanisterRole>,
    seen: BTreeSet<Principal>,
}

impl Collector {
    const fn new(parent: Principal, expected: BTreeMap<Principal, CanisterRole>) -> Self {
        Self {
            parent,
            expected,
            seen: BTreeSet::new(),
        }
    }

    fn push(&mut self, page: Page<CanisterInfo>) -> Result<bool, CompletedParentageError> {
        let pagination = || CompletedParentageError::Pagination {
            parent: self.parent,
        };
        if page.total != self.expected.len() as u64
            || page.entries.len() as u64 > PAGE_SIZE
            || (page.entries.is_empty() && self.seen.len() != self.expected.len())
        {
            return Err(pagination());
        }
        for child in page.entries {
            if child.parent_pid != Some(self.parent)
                || self.expected.get(&child.pid) != Some(&child.role)
                || !self.seen.insert(child.pid)
            {
                return Err(CompletedParentageError::Membership {
                    parent: self.parent,
                });
            }
        }
        Ok(self.seen.len() == self.expected.len())
    }
}
