//! Read complete bounded Root inventories and reject unstable pages or ambiguous membership.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::capacity_import::admission::{
        CapacityImportAdmissionRecord, CapacityImportInfrastructureKind,
    },
    ops::capacity_import::journal::{CapacityImportInventoryStage, CapacityImportJournalError},
};
use canic_contracts::{
    dto::{
        error::Error,
        fleet_registry::FleetRegistry,
        pool::{CanisterPoolAssetStatus, CanisterPoolResponse},
    },
    protocol,
};
use ic_platform::{agent::Agent, principal::ExternalPrincipal as Principal};
use std::{collections::BTreeSet, time::Duration};

const MAXIMUM_ASSETS: usize = 16_384;
const PAGE_SIZE: u16 = 256;

/// Shared bound for Registry and paginated Root query responses.
pub(in crate::fleet_ensure::ops) const RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Eq, PartialEq)]
pub(in crate::fleet_ensure::ops::capacity_import) struct Inventory {
    pub assigned: BTreeSet<Principal>,
    pub occupied: u32,
    pub maximum: u32,
}

pub(in crate::fleet_ensure::ops) async fn registry(
    agent: &Agent,
    coordinator: Principal,
) -> Result<FleetRegistry, CapacityImportJournalError> {
    let argument = ic_wire::native::encode_request(
        protocol::registry_discovery::snapshot::METHOD_ID,
        &(),
        protocol::registry_discovery::REQUEST_LIMITS,
    )
    .map_err(|_| CapacityImportJournalError::InventoryInvalid)?;
    let bytes = query(
        agent,
        coordinator,
        protocol::registry_discovery::snapshot::METHOD,
        argument,
    )
    .await?;
    let response: Result<canic_contracts::dto::fleet_snapshot::FleetRegistry, Error> =
        ic_wire::native::decode_response(
            protocol::registry_discovery::snapshot::METHOD_ID,
            &bytes,
            protocol::registry_discovery::RESPONSE_LIMITS,
        )
        .map_err(|_| CapacityImportJournalError::InventoryInvalid)?;
    response
        .map(Into::into)
        .map_err(CapacityImportJournalError::CoordinatorRejected)
}

pub(in crate::fleet_ensure::ops::capacity_import) async fn observe(
    agent: &Agent,
    admission: &CapacityImportAdmissionRecord,
    selected_root: Principal,
    registry: &FleetRegistry,
) -> Result<Inventory, CapacityImportJournalError> {
    let mut assigned = admission
        .infrastructure
        .iter()
        .map(|entry| entry.principal)
        .collect::<BTreeSet<_>>();
    let mut assets = BTreeSet::new();
    let mut occupied = None;
    let mut maximum = None;
    for root in &registry.fleet_subnet_roots {
        let store = admission
            .infrastructure
            .iter()
            .find(|entry| {
                entry.kind
                    == CapacityImportInfrastructureKind::Store {
                        root: root.fleet_subnet_root,
                    }
            })
            .ok_or(CapacityImportJournalError::InventoryInvalid)?;
        let scope = if root.fleet_subnet_root == selected_root {
            PoolScope::Destination
        } else {
            PoolScope::OtherRoot
        };
        let inventory = root_inventory(agent, root, store.principal, scope).await?;
        let summary = &inventory.summary;
        for id in &inventory.seen {
            if !assets.insert(*id)
                || (*id != store.principal && assigned.contains(id))
                || assets.len() > MAXIMUM_ASSETS
            {
                return Err(CapacityImportJournalError::InventoryInvalid);
            }
        }
        if root.fleet_subnet_root == selected_root {
            occupied = Some(
                summary
                    .tracked
                    .checked_sub(summary.store)
                    .ok_or(CapacityImportJournalError::InventoryInvalid)?,
            );
            maximum = Some(summary.config.maximum_size);
        }
    }
    assigned.extend(assets);
    Ok(Inventory {
        assigned,
        occupied: occupied.ok_or(CapacityImportJournalError::InventoryInvalid)?,
        maximum: maximum.ok_or(CapacityImportJournalError::InventoryInvalid)?,
    })
}

/// Complete query inventory; this is not a fence or evidence that paid work has settled.
pub(in crate::fleet_ensure::ops) struct RootInventory {
    pub seen: BTreeSet<Principal>,
    pub summary: CanisterPoolResponse,
}

/// Enumerate a Root once, bounding each reply and requiring exact page continuity.
pub(in crate::fleet_ensure::ops) async fn root_inventory(
    agent: &Agent,
    root: &canic_contracts::dto::fleet_registry::FleetSubnetRootEntry,
    store: Principal,
    scope: PoolScope,
) -> Result<RootInventory, CapacityImportJournalError> {
    let canister = root.fleet_subnet_root;
    let mut inventory = Pages::new(store, scope);
    loop {
        let argument = ic_wire::native::encode_request(
            protocol::pool_inventory::METHOD_ID,
            &canic_contracts::dto::pool_inventory::Request {
                start_after: inventory.cursor.map(Into::into),
                limit: PAGE_SIZE,
            },
            protocol::pool_inventory::REQUEST_LIMITS,
        )
        .map_err(|_| CapacityImportJournalError::InventoryInvalid)?;
        let bytes = query(agent, canister, protocol::pool_inventory::METHOD, argument).await?;
        let response = decode(canister, &bytes)?;
        let page =
            CanisterPoolResponse::from(response.map_err(CapacityImportJournalError::RootRejected)?);
        if page.config != root.limits.canister_pool {
            return Err(failed(canister, CapacityImportInventoryStage::Policy));
        }
        if inventory
            .push(page)
            .map_err(|_| failed(canister, CapacityImportInventoryStage::Page))?
        {
            break;
        }
    }
    let summary = inventory
        .finish()
        .map_err(|_| failed(canister, CapacityImportInventoryStage::Summary))?
        .clone();
    Ok(RootInventory {
        seen: inventory.seen,
        summary,
    })
}

struct Pages {
    store: Principal,
    scope: PoolScope,
    summary: Option<CanisterPoolResponse>,
    cursor: Option<Principal>,
    seen: BTreeSet<Principal>,
    stores: BTreeSet<Principal>,
    ready: u32,
    workload: u32,
    failed: u32,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::fleet_ensure::ops) enum PoolScope {
    Destination,
    OtherRoot,
}

fn decode(
    canister: Principal,
    bytes: &[u8],
) -> Result<Result<canic_contracts::dto::pool_inventory::Page, Error>, CapacityImportJournalError> {
    if bytes.len() > RESPONSE_BYTES {
        return Err(failed(canister, CapacityImportInventoryStage::Decode));
    }
    ic_wire::native::decode_response(
        protocol::pool_inventory::METHOD_ID,
        bytes,
        protocol::pool_inventory::RESPONSE_LIMITS,
    )
    .map_err(|_| failed(canister, CapacityImportInventoryStage::Decode))
}

impl Pages {
    const fn new(store: Principal, scope: PoolScope) -> Self {
        Self {
            store,
            scope,
            summary: None,
            cursor: None,
            seen: BTreeSet::new(),
            stores: BTreeSet::new(),
            ready: 0,
            workload: 0,
            failed: 0,
        }
    }

    fn push(&mut self, mut page: CanisterPoolResponse) -> Result<bool, CapacityImportJournalError> {
        if page.entries.len() > usize::from(PAGE_SIZE)
            || page.tracked as usize > MAXIMUM_ASSETS
            || (self.scope == PoolScope::Destination
                && (page.pending_creation.is_some() || page.pending_handoff.is_some()))
        {
            return Err(CapacityImportJournalError::InventoryInvalid);
        }
        let next = page.next_start_after.take();
        let mut previous = self.cursor;
        for entry in std::mem::take(&mut page.entries) {
            if previous.is_some_and(|value| entry.canister_id <= value)
                || !self.seen.insert(entry.canister_id)
            {
                return Err(CapacityImportJournalError::InventoryInvalid);
            }
            previous = Some(entry.canister_id);
            match entry.status {
                CanisterPoolAssetStatus::Store => {
                    self.stores.insert(entry.canister_id);
                }
                CanisterPoolAssetStatus::Ready => self.ready += 1,
                CanisterPoolAssetStatus::Workload { .. } => self.workload += 1,
                CanisterPoolAssetStatus::Failed { .. } => self.failed += 1,
                _ if self.scope == PoolScope::OtherRoot => {}
                _ => return Err(CapacityImportJournalError::InventoryInvalid),
            }
        }
        if self
            .summary
            .as_ref()
            .is_some_and(|summary| *summary != page)
            || self.seen.len() > page.tracked as usize
        {
            return Err(CapacityImportJournalError::InventoryInvalid);
        }
        self.summary = Some(page);
        if next.is_some() && (next != previous || next == self.cursor) {
            return Err(CapacityImportJournalError::InventoryInvalid);
        }
        self.cursor = next;
        Ok(next.is_none())
    }

    fn finish(&self) -> Result<&CanisterPoolResponse, CapacityImportJournalError> {
        let page = self
            .summary
            .as_ref()
            .ok_or(CapacityImportJournalError::InventoryInvalid)?;
        let incomplete = self.cursor.is_some() || self.seen.len() != page.tracked as usize;
        let counts = (page.store, page.ready, page.workload, page.failed);
        if incomplete
            || self.stores != BTreeSet::from([self.store])
            || counts != (1, self.ready, self.workload, self.failed)
            || (self.scope == PoolScope::Destination && page.pending_reset != 0)
        {
            return Err(CapacityImportJournalError::InventoryInvalid);
        }
        Ok(page)
    }
}

async fn query(
    agent: &Agent,
    canister: Principal,
    method: &str,
    argument: Vec<u8>,
) -> Result<Vec<u8>, CapacityImportJournalError> {
    tokio::time::timeout(
        Duration::from_secs(30),
        agent.query(&canister, method).with_arg(argument).call(),
    )
    .await
    .map_err(|_| failed(canister, CapacityImportInventoryStage::Query))?
    .map_err(|_| failed(canister, CapacityImportInventoryStage::Query))
}

const fn failed(
    canister: Principal,
    stage: CapacityImportInventoryStage,
) -> CapacityImportJournalError {
    CapacityImportJournalError::InventoryObservation { canister, stage }
}
