//! Module: workflow::state
//!
//! Responsibility: apply root Fleet-state commands and select exact cascade targets.
//! Does not own: endpoint authorization, Fleet-state records, or child transport.
//! Boundary: derives direct children from Store and Component Registry authority.

use crate::ops::{
    component_registry::ComponentRegistryOps,
    storage::state::root_wasm_store::RootWasmStoreStateOps,
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError, ops::ic::IcOps, workflow::state::execute_fleet_command_to,
    },
    dto::state::{FleetCommand, FleetCommandResponse},
};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
enum RootChildAuthority {
    ComponentRegistry,
    StoreInventory,
}

struct RootStateCascadeTargets {
    root: Principal,
    canisters: BTreeSet<Principal>,
}

impl RootStateCascadeTargets {
    fn current() -> Result<Self, InternalError> {
        let mut targets = Self {
            root: IcOps::canister_self(),
            canisters: BTreeSet::new(),
        };
        for store in RootWasmStoreStateOps::wasm_stores() {
            targets.insert(store.pid, RootChildAuthority::StoreInventory)?;
        }
        for component in ComponentRegistryOps::root_component_canisters()? {
            targets.insert(component, RootChildAuthority::ComponentRegistry)?;
        }
        Ok(targets)
    }

    fn insert(
        &mut self,
        canister: Principal,
        authority: RootChildAuthority,
    ) -> Result<(), InternalError> {
        if canister == Principal::anonymous() {
            return Err(invalid_root_child(authority, "is anonymous"));
        }
        if canister == self.root {
            return Err(invalid_root_child(
                authority,
                "equals the Fleet Subnet Root",
            ));
        }
        if !self.canisters.insert(canister) {
            return Err(invalid_root_child(
                authority,
                "appears in more than one root-owned inventory",
            ));
        }
        Ok(())
    }

    fn into_vec(self) -> Vec<Principal> {
        self.canisters.into_iter().collect()
    }
}

///
/// FleetStateWorkflow
///
/// Root workflow that binds Fleet-state fanout to current root-owned inventory.
///

pub struct FleetStateWorkflow;

impl FleetStateWorkflow {
    pub async fn execute_command(cmd: FleetCommand) -> Result<FleetCommandResponse, InternalError> {
        let targets = RootStateCascadeTargets::current()?.into_vec();
        execute_fleet_command_to(cmd, &targets).await
    }
}

fn invalid_root_child(_authority: RootChildAuthority, _reason: &'static str) -> InternalError {
    InternalError::invariant()
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn root_state_targets_are_canonical_and_reject_invalid_authority() {
        let mut targets = RootStateCascadeTargets {
            root: p(1),
            canisters: BTreeSet::new(),
        };
        targets
            .insert(p(3), RootChildAuthority::StoreInventory)
            .expect("insert Store");
        targets
            .insert(p(2), RootChildAuthority::ComponentRegistry)
            .expect("insert Component");
        let duplicate = targets
            .insert(p(3), RootChildAuthority::ComponentRegistry)
            .expect_err("overlapping authority must reject");

        assert_eq!(
            duplicate.code(),
            canic_core::diagnostics::codes::STATE_INVALID
        );
        assert_eq!(targets.into_vec(), vec![p(2), p(3)]);

        let mut invalid_targets = RootStateCascadeTargets {
            root: p(1),
            canisters: BTreeSet::new(),
        };
        let anonymous = invalid_targets
            .insert(Principal::anonymous(), RootChildAuthority::StoreInventory)
            .expect_err("anonymous child must reject");
        let root = invalid_targets
            .insert(p(1), RootChildAuthority::ComponentRegistry)
            .expect_err("root cannot be its own child");

        assert_eq!(
            anonymous.code(),
            canic_core::diagnostics::codes::STATE_INVALID
        );
        assert_eq!(root.code(), canic_core::diagnostics::codes::STATE_INVALID);
    }
}
