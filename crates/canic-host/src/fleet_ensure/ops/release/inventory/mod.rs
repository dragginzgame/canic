//! Collect the complete reviewed Fleet ownership closure through authenticated queries.
//!
//! Certified owner custody brackets bounded Registry/pool reads. This is a time-local
//! inventory, not producer quiescence, paid-effect settlement or reset authority.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::release::{FleetReleaseReviewRecord, FleetReleaseRole, FleetReleaseSourceRecord},
        ops::{
            capacity_import::{
                admission::observer::inventory::{self, PoolScope},
                journal::CapacityImportJournalError,
            },
            certified_custody::{self, CertifiedCustodyError},
        },
        policy::release::{FleetReleaseError, expected_ownership},
        view::release::FleetReleaseInventoryView,
    },
    icp::IcpCli,
};
use canic_core::{cdk::utils::hash::hex_bytes, dto::fleet_registry::FleetRegistry};
use ic_agent::Agent;
use std::collections::BTreeSet;
use thiserror::Error;

/// No inventory is returned after an authority, membership or observation refusal.
#[derive(Debug, Error)]
pub enum ReleaseInventoryError {
    #[error(transparent)]
    Authentication(#[from] super::observation::ReleaseObservationError),
    #[error(transparent)]
    Evidence(#[from] FleetReleaseError),
    #[error(transparent)]
    Certificate(#[from] CertifiedCustodyError),
    #[error("release ownership query failed: {0}")]
    Query(#[from] CapacityImportJournalError),
}

/// Read selected current-build owners before sealing admission or issuing effects.
/// The caller must separately retain and qualify the selected Registry and artifact bytes.
pub async fn collect(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    expected_registry: &FleetRegistry,
) -> Result<FleetReleaseInventoryView, ReleaseInventoryError> {
    validate_registry_selection(review, expected_registry)?;
    let agent = icp
        .authenticated_agent_with_response_limit(inventory::RESPONSE_BYTES)
        .map_err(|error| {
            super::observation::ReleaseObservationError::Authentication(Box::new(error))
        })?;
    collect_with_agent(&agent, review, expected_registry).await
}

async fn collect_with_agent(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
    expected_registry: &FleetRegistry,
) -> Result<FleetReleaseInventoryView, ReleaseInventoryError> {
    let expected = validate_registry_selection(review, expected_registry)?;
    super::observation::verify_agent(agent, &review.authority)?;
    verify_custody(
        agent,
        review.sources.iter().filter(|source| {
            matches!(
                source.role,
                FleetReleaseRole::Coordinator | FleetReleaseRole::Root
            )
        }),
    )
    .await?;
    verify_registry(agent, review, expected_registry).await?;
    let mut children = expected.children.clone();
    for root in &expected_registry.fleet_subnet_roots {
        let store = review
            .sources
            .iter()
            .find(|source| {
                source.role
                    == FleetReleaseRole::Store {
                        root: root.fleet_subnet_root,
                    }
            })
            .ok_or(FleetReleaseError::Inventory)?;
        // Include every asset even when it is pending. Such work must later be
        // reconciled; hiding it here would turn an incomplete review into authority.
        let observed =
            inventory::root_inventory(agent, root, store.binding.canister_id, PoolScope::OtherRoot)
                .await?;
        children.insert(root.fleet_subnet_root, observed.seen);
    }
    if children != expected.children {
        return Err(FleetReleaseError::Inventory.into());
    }
    verify_registry(agent, review, expected_registry).await?;
    // All selected physical IDs need final custody evidence. Children do not
    // serve inventory queries, so they need no duplicate preliminary certificate.
    verify_custody(agent, &review.sources).await?;
    Ok(FleetReleaseInventoryView { children })
}

fn validate_registry_selection(
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseInventoryView, FleetReleaseError> {
    let children = expected_ownership(review)?;
    let binding = &registry.authority.binding;
    if binding.fleet != review.authority.fleet
        || binding.coordinator != review.authority.coordinator
    {
        return Err(FleetReleaseError::Authority);
    }
    let roots = registry
        .fleet_subnet_roots
        .iter()
        .map(|root| root.fleet_subnet_root)
        .collect::<BTreeSet<_>>();
    if roots.len() != registry.fleet_subnet_roots.len()
        || children.get(&binding.coordinator) != Some(&roots)
    {
        return Err(FleetReleaseError::Inventory);
    }
    for source in &review.sources {
        let expected_subnet = match source.role {
            FleetReleaseRole::Coordinator => binding.coordinator_subnet,
            FleetReleaseRole::Root => {
                registry
                    .fleet_subnet_roots
                    .iter()
                    .find(|root| root.fleet_subnet_root == source.binding.canister_id)
                    .ok_or(FleetReleaseError::Inventory)?
                    .placement_subnet
            }
            FleetReleaseRole::Store { .. } | FleetReleaseRole::Child { .. } => continue,
        };
        if source.binding.subnet != expected_subnet || source.binding.module_sha256.is_none() {
            return Err(FleetReleaseError::Custody {
                canister: source.binding.canister_id,
            });
        }
    }
    Ok(FleetReleaseInventoryView { children })
}

async fn verify_registry(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
    expected: &FleetRegistry,
) -> Result<(), ReleaseInventoryError> {
    if inventory::registry(agent, review.authority.coordinator).await? != *expected {
        return Err(FleetReleaseError::Authority.into());
    }
    Ok(())
}

async fn verify_custody<'a>(
    agent: &Agent,
    sources: impl IntoIterator<Item = &'a FleetReleaseSourceRecord>,
) -> Result<(), ReleaseInventoryError> {
    for source in sources {
        let expected = &source.binding;
        let observed = certified_custody::observe_one(agent, expected.canister_id).await?;
        let mut controllers = expected.controllers.clone();
        controllers.sort_unstable();
        if observed.subnet() != expected.subnet
            || observed.controllers() != controllers
            || observed.module_sha256() != expected.module_sha256.map(hex_bytes).as_deref()
        {
            return Err(FleetReleaseError::Custody {
                canister: expected.canister_id,
            }
            .into());
        }
    }
    Ok(())
}
