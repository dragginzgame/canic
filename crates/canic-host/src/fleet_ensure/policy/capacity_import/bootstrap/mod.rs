//! Validate supplied capacity against the exact reviewed Root initialization contract.
//!
//! No observation, record mutation, serialization or effect belongs to this boundary.

use crate::fleet_ensure::model::{
    DesiredCanister, DesiredCanisterKind, DesiredFleet, DesiredFleetBootstrapRoot, DesiredPresence,
};
use candid::Principal;
use canic_contracts::ids::MAX_FLEET_CAPACITY_IMPORT_SOURCES;
use std::collections::BTreeSet;
use thiserror::Error;

/// An inconsistent initialization declaration cannot authorize installation or pool custody.

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CapacityBootstrapError {
    #[error("capacity bootstrap requires exact supplied Root and Store identities")]
    Infrastructure,
    #[error("capacity bootstrap contains an invalid Principal")]
    Identity,
    #[error("capacity bootstrap differs from the reviewed operator")]
    Operator,
    #[error("capacity bootstrap requires a nonzero infrastructure review digest")]
    Review,
    #[error("capacity bootstrap requires the exact bounded, ordered supplied pool identities")]
    Sources,
    #[error("capacity bootstrap source aliases infrastructure, operator or a recovery controller")]
    Collision,
}

/// Require exact physical IDs before the generic initialization compiler can emit a hold.
pub fn validate(
    desired: &DesiredFleet,
    input: &DesiredFleetBootstrapRoot,
) -> Result<(), CapacityBootstrapError> {
    let Some(hold) = &input.capacity_import_bootstrap else {
        return Ok(());
    };
    if hold.review_sha256 == [0; 32] {
        return Err(CapacityBootstrapError::Review);
    }
    let operator = principal(&desired.operator)?;
    if hold.operator != operator {
        return Err(CapacityBootstrapError::Operator);
    }
    let root = configured(desired, &input.root, DesiredCanisterKind::Root)?;
    let store = configured(desired, &input.store, DesiredCanisterKind::Store)?;
    let root_id = supplied(root)?;
    let store_id = supplied(store)?;
    let placement = input.placement_subnet.into_principal();
    if root_id == store_id
        || principal(&root.subnet)? != placement
        || principal(&store.subnet)? != placement
        || store.parent.as_deref() != Some(input.root.as_str())
    {
        return Err(CapacityBootstrapError::Infrastructure);
    }
    let names = input.canister_pool_imports.iter().collect::<BTreeSet<_>>();
    let configured_names = desired
        .canisters
        .iter()
        .filter(|canister| {
            canister.kind == DesiredCanisterKind::Pool
                && canister.parent.as_deref() == Some(input.root.as_str())
        })
        .map(|canister| &canister.name)
        .collect::<BTreeSet<_>>();
    if names != configured_names || names.len() != input.canister_pool_imports.len() {
        return Err(CapacityBootstrapError::Sources);
    }
    let mut expected = Vec::with_capacity(names.len());
    for name in names {
        let source = configured(desired, name, DesiredCanisterKind::Pool)?;
        if principal(&source.subnet)? != placement || source.replace {
            return Err(CapacityBootstrapError::Sources);
        }
        expected.push(supplied(source)?);
    }
    expected.sort_unstable();
    if expected != hold.sources
        || expected.is_empty()
        || expected.len() > MAX_FLEET_CAPACITY_IMPORT_SOURCES
        || expected.len() > input.limits.canister_pool.maximum_size as usize
        || !expected.windows(2).all(|pair| pair[0] < pair[1])
    {
        return Err(CapacityBootstrapError::Sources);
    }
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(CapacityBootstrapError::Infrastructure)?;
    let mut excluded = bootstrap
        .recovery_controllers
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    excluded.insert(operator);
    for canister in &desired.canisters {
        if canister.kind != DesiredCanisterKind::Pool
            && let Some(id) = &canister.principal
        {
            excluded.insert(principal(id)?);
        }
    }
    if hold.sources.iter().any(|id| excluded.contains(id)) {
        return Err(CapacityBootstrapError::Collision);
    }
    Ok(())
}

fn configured<'a>(
    desired: &'a DesiredFleet,
    name: &str,
    kind: DesiredCanisterKind,
) -> Result<&'a DesiredCanister, CapacityBootstrapError> {
    let mut matching = desired
        .canisters
        .iter()
        .filter(|canister| canister.name == name);
    let canister = matching
        .next()
        .ok_or(CapacityBootstrapError::Infrastructure)?;
    if matching.next().is_some()
        || canister.kind != kind
        || canister.presence != DesiredPresence::Present
        || canister.replace
    {
        return Err(CapacityBootstrapError::Infrastructure);
    }
    Ok(canister)
}

fn supplied(canister: &DesiredCanister) -> Result<Principal, CapacityBootstrapError> {
    principal(
        canister
            .principal
            .as_deref()
            .ok_or(CapacityBootstrapError::Infrastructure)?,
    )
}

fn principal(text: &str) -> Result<Principal, CapacityBootstrapError> {
    let id = Principal::from_text(text).map_err(|_| CapacityBootstrapError::Identity)?;
    if id == Principal::anonymous() || id == Principal::management_canister() {
        return Err(CapacityBootstrapError::Identity);
    }
    Ok(id)
}
