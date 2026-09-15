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
    api::fleet_activation::FleetActivationApi,
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError,
        ops::ic::IcOps,
        view::state_cascade::{StateCascadeEndpoint, StateCascadeTarget},
        workflow::state::execute_fleet_command_to,
    },
    dto::{
        fleet_activation::FleetActivationPhase,
        state::{FleetCommand, FleetCommandExecutionResponse},
    },
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
enum RootChildAuthority {
    ComponentRegistry,
    StoreInventory,
}

struct RootStateCascadeTargets {
    root: Principal,
    canisters: BTreeMap<Principal, StateCascadeEndpoint>,
}

impl RootStateCascadeTargets {
    fn current() -> Result<Self, InternalError> {
        let mut targets = Self {
            root: IcOps::canister_self(),
            canisters: BTreeMap::new(),
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
        if self.canisters.contains_key(&canister) {
            return Err(invalid_root_child(
                authority,
                "appears in more than one root-owned inventory",
            ));
        }
        let endpoint = match authority {
            RootChildAuthority::ComponentRegistry => StateCascadeEndpoint::Component,
            RootChildAuthority::StoreInventory => StateCascadeEndpoint::Store,
        };
        self.canisters.insert(canister, endpoint);
        Ok(())
    }

    fn into_vec(self) -> Vec<StateCascadeTarget> {
        self.canisters
            .into_iter()
            .map(|(canister_id, endpoint)| StateCascadeTarget {
                canister_id,
                endpoint,
            })
            .collect()
    }
}

///
/// FleetStateWorkflow
///
/// Root workflow that binds Fleet-state fanout to current root-owned inventory.
///

pub struct FleetStateWorkflow;

impl FleetStateWorkflow {
    pub async fn execute_command(
        cmd: FleetCommand,
    ) -> Result<FleetCommandExecutionResponse, InternalError> {
        let targets = RootStateCascadeTargets::current()?.into_vec();
        let reconcile_funding = if matches!(cmd, FleetCommand::SetCyclesFundingEnabled(_)) {
            let activation =
                FleetActivationApi::status().map_err(InternalError::observed_public)?;
            should_reconcile_root_funding(cmd, activation.phase)
        } else {
            false
        };
        execute_fleet_command_to(cmd, &targets, reconcile_funding).await
    }
}

const fn should_reconcile_root_funding(command: FleetCommand, phase: FleetActivationPhase) -> bool {
    matches!(command, FleetCommand::SetCyclesFundingEnabled(_))
        && matches!(phase, FleetActivationPhase::Active)
}

const fn invalid_root_child(
    _authority: RootChildAuthority,
    _reason: &'static str,
) -> InternalError {
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
            canisters: BTreeMap::new(),
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
        assert_eq!(
            targets.into_vec(),
            vec![
                StateCascadeTarget {
                    canister_id: p(2),
                    endpoint: StateCascadeEndpoint::Component
                },
                StateCascadeTarget {
                    canister_id: p(3),
                    endpoint: StateCascadeEndpoint::Store
                },
            ]
        );

        let mut invalid_targets = RootStateCascadeTargets {
            root: p(1),
            canisters: BTreeMap::new(),
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

    #[test]
    fn root_funding_switch_reconciles_only_after_activation() {
        assert!(should_reconcile_root_funding(
            FleetCommand::SetCyclesFundingEnabled(true),
            FleetActivationPhase::Active,
        ));
        assert!(!should_reconcile_root_funding(
            FleetCommand::SetCyclesFundingEnabled(true),
            FleetActivationPhase::Prepared,
        ));
        assert!(!should_reconcile_root_funding(
            FleetCommand::SetStatus(canic_core::dto::state::FleetStatus::Active),
            FleetActivationPhase::Active,
        ));
    }
}
