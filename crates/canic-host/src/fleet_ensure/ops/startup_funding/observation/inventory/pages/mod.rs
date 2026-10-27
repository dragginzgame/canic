//! Module: fleet_ensure::ops::startup_funding::observation::inventory::pages
//!
//! Responsibility: consume bounded unfiltered directory pages and join every exact funding edge.
//! Boundary: no retries, no paid calls, no partial inventory promoted to complete evidence.

use crate::{
    canister_protocol::query_wire,
    fleet_ensure::view::startup_funding::{StartupChildFundingBinding, StartupUsageUnavailable},
    icp::IcpCli,
};
use canic_contracts::{
    dto::component_registry::{
        ComponentDirectoryHead, ComponentDirectoryPageRequest, ComponentDirectoryPageResponse,
        ComponentLifecycleStatus,
    },
    ids::ComponentChildBinding,
    protocol,
};
use ic_platform::principal::ExternalPrincipal as Principal;
use std::collections::{BTreeMap, BTreeSet};

const PAGE_LIMIT: u16 = 100;

pub(super) fn observe(
    icp: &IcpCli,
    root: Principal,
    directory: &ComponentDirectoryHead,
    members: &[&StartupChildFundingBinding],
) -> Result<(), StartupUsageUnavailable> {
    collect(directory, members, |request| {
        let page: canic_contracts::dto::component_directory::ComponentDirectoryPageResponse =
            query_wire(
                icp,
                root,
                protocol::component_directory::PAGE,
                protocol::component_directory::PAGE_ID,
                &canic_contracts::dto::component_directory::ComponentDirectoryPageRequest::from(
                    request,
                ),
                protocol::component_directory::REQUEST_LIMITS,
                protocol::component_directory::RESPONSE_LIMITS,
            )
            .map_err(|_| StartupUsageUnavailable::ObservationFailed)?;
        Ok(page.into())
    })
}

pub(super) fn collect(
    directory: &ComponentDirectoryHead,
    members: &[&StartupChildFundingBinding],
    mut read: impl FnMut(
        ComponentDirectoryPageRequest,
    ) -> Result<ComponentDirectoryPageResponse, StartupUsageUnavailable>,
) -> Result<(), StartupUsageUnavailable> {
    let mut remaining = members
        .iter()
        .filter(|member| member.canister_id != member.component.canister_id)
        .map(|member| (member.canister_id, *member))
        .collect::<BTreeMap<_, _>>();
    if remaining.len() as u64 != u64::from(directory.descendant_count) {
        return Err(StartupUsageUnavailable::InventoryIncomplete);
    }
    if remaining.is_empty() {
        return Ok(());
    }
    let mut cursor = None;
    let mut cursors = BTreeSet::new();
    loop {
        let page = read(ComponentDirectoryPageRequest {
            directory: directory.clone(),
            parent_canister_id: None,
            role: None,
            status: None,
            cursor: cursor.clone(),
            limit: PAGE_LIMIT,
        })?;
        if page.directory != *directory {
            return Err(StartupUsageUnavailable::PolicyTransition);
        }
        if page.entries.is_empty() || page.entries.len() > usize::from(PAGE_LIMIT) {
            return Err(StartupUsageUnavailable::InventoryIncomplete);
        }
        for entry in page.entries {
            let expected = remaining
                .remove(&entry.binding.canister_id)
                .ok_or(StartupUsageUnavailable::AuthorityMismatch)?;
            let binding = ComponentChildBinding {
                component: expected.component.clone(),
                parent_canister_id: expected.parent,
                role: expected.role.clone(),
                canister_id: expected.canister_id,
            };
            if entry.binding != binding {
                return Err(StartupUsageUnavailable::AuthorityMismatch);
            }
            if entry.status != ComponentLifecycleStatus::Active {
                return Err(StartupUsageUnavailable::PolicyTransition);
            }
        }
        match page.next_cursor {
            None if remaining.is_empty() => return Ok(()),
            None => return Err(StartupUsageUnavailable::InventoryIncomplete),
            Some(next) => {
                // Every continuation must consume at least one distinct expected member.
                // Thus even arbitrary unique cursors cannot exceed the observed Spec bound.
                if remaining.is_empty() || next.0.is_empty() || !cursors.insert(next.0.clone()) {
                    return Err(StartupUsageUnavailable::InventoryIncomplete);
                }
                cursor = Some(next);
            }
        }
    }
}
