//! Authenticate production import observations using the selected ICP signer and network.
//!
//! Workflow owns durable observation allowances. Each method issues at most one management call.

pub(in crate::fleet_ensure::ops) mod inventory;
pub(in crate::fleet_ensure::ops) mod management;

use crate::{
    fleet_ensure::{
        model::capacity_import::CapacityImportPlanRecord,
        ops::{
            capacity_import::{
                admission::{declarations::hash, registry},
                journal::CapacityImportJournalError,
                observation::CapacityImportObserver,
                transport::{CapacityImportTransport, verify_agent},
                validate_destination_authority,
            },
            reinstall::terminal::inventory::custody,
        },
        policy::capacity_import::disposition_digest,
        view::capacity_import::{
            CapacityImportDestinationView, CapacityImportOwnershipView, CapacityImportSourceView,
        },
    },
    icp::IcpCli,
};
use candid::Principal;
use canic_core::dto::{
    fleet_registry::FleetRegistry,
    pool_import::{PoolImportIdentity, PoolImportSourceProgress},
};

/// Production admission reader. Construction performs no management effect.
pub struct CapacityImportLiveObserver {
    transport: CapacityImportTransport,
}

impl CapacityImportLiveObserver {
    /// Resolve operator authentication. Completed replay should precede this call.
    pub fn from_icp(icp: &IcpCli) -> Result<Self, CapacityImportJournalError> {
        Ok(Self {
            transport: CapacityImportTransport::from_icp(icp)?,
        })
    }

    async fn infrastructure(
        &self,
        plan: &CapacityImportPlanRecord,
    ) -> Result<FleetRegistry, CapacityImportJournalError> {
        verify_agent(&self.transport.agent, plan)?;
        let admission = plan
            .admission
            .as_ref()
            .ok_or(CapacityImportJournalError::InfrastructureRequired)?;
        for expected in &admission.infrastructure {
            let observed = custody::observe_one(&self.transport.agent, expected.principal)
                .await
                .map_err(|_| CapacityImportJournalError::InfrastructureChanged)?;
            let module = observed.module_sha256.as_deref().map(hash).transpose()?;
            if (observed.subnet, &observed.controllers, module)
                != (
                    expected.subnet,
                    &expected.controllers,
                    Some(expected.module_sha256),
                )
            {
                return Err(CapacityImportJournalError::InfrastructureChanged);
            }
        }
        let expected = registry(admission)?;
        let observed =
            inventory::registry(&self.transport.agent, plan.authority.coordinator).await?;
        if observed != expected {
            return Err(CapacityImportJournalError::InfrastructureChanged);
        }
        let context = self.transport.root_context(plan.authority.root).await?;
        validate_destination_authority(plan, &context, &observed)?;
        let identity = PoolImportIdentity {
            sequence: plan.authority.import_sequence,
            plan_sha256: plan.plan_sha256,
        };
        let operation_matches = match context.active_import {
            Some(active) => active == identity,
            None => context.next_sequence == plan.authority.import_sequence,
        };
        if !operation_matches {
            return Err(CapacityImportJournalError::Conflict);
        }
        if context.active_import.is_some() {
            let status = self.transport.root_status(plan).await?;
            if status
                .progress
                .iter()
                .any(|progress| *progress != PoolImportSourceProgress::AwaitingHandoff)
            {
                return Err(CapacityImportJournalError::Unresolved);
            }
        }
        Ok(observed)
    }
}

/// Recheck frozen physical custody before Root effects, including after restart.
pub(in crate::fleet_ensure::ops::capacity_import) async fn verify_infrastructure(
    agent: &ic_agent::Agent,
    plan: &CapacityImportPlanRecord,
    observed_registry: &FleetRegistry,
) -> Result<(), CapacityImportJournalError> {
    let Some(admission) = &plan.admission else {
        return Ok(());
    };
    if registry(admission)? != *observed_registry {
        return Err(CapacityImportJournalError::InfrastructureChanged);
    }
    for expected in &admission.infrastructure {
        let observed = custody::observe_one(agent, expected.principal)
            .await
            .map_err(|_| CapacityImportJournalError::InfrastructureChanged)?;
        let module = observed.module_sha256.as_deref().map(hash).transpose()?;
        if (observed.subnet, &observed.controllers, module)
            != (
                expected.subnet,
                &expected.controllers,
                Some(expected.module_sha256),
            )
        {
            return Err(CapacityImportJournalError::InfrastructureChanged);
        }
    }
    Ok(())
}

impl CapacityImportObserver for CapacityImportLiveObserver {
    async fn destination(
        &mut self,
        plan: &CapacityImportPlanRecord,
    ) -> Result<CapacityImportDestinationView, CapacityImportJournalError> {
        let registry = self.infrastructure(plan).await?;
        let inventory = inventory::observe(
            &self.transport.agent,
            plan.admission
                .as_ref()
                .ok_or(CapacityImportJournalError::InfrastructureRequired)?,
            plan.authority.root,
            &registry,
        )
        .await?;
        let status = management::observe(&self.transport.agent, plan.authority.root).await?;
        self.infrastructure(plan).await?;
        Ok(CapacityImportDestinationView {
            authority: plan.authority.clone(),
            ready: true,
            draining: false,
            competing_operation: false,
            occupied_capacity: inventory.occupied,
            maximum_capacity: inventory.maximum,
            controlled_cycles: status.cycles,
            reserved_cycles: status.reserved_cycles,
            minimum_retained_cycles: plan.root_budget.minimum_retained_cycles,
            assigned_canisters: inventory.assigned,
        })
    }

    async fn source(
        &mut self,
        plan: &CapacityImportPlanRecord,
        canister: Principal,
    ) -> Result<CapacityImportSourceView, CapacityImportJournalError> {
        let source = plan
            .sources
            .iter()
            .find(|source| source.binding.canister_id == canister)
            .ok_or(CapacityImportJournalError::Integrity)?;
        let registry = self.infrastructure(plan).await?;
        let inventory = inventory::observe(
            &self.transport.agent,
            plan.admission
                .as_ref()
                .ok_or(CapacityImportJournalError::InfrastructureRequired)?,
            plan.authority.root,
            &registry,
        )
        .await?;
        if inventory.assigned.contains(&canister) {
            return Err(CapacityImportJournalError::InventoryInvalid);
        }
        let status = management::observe(&self.transport.agent, canister).await?;
        // A concurrent membership or infrastructure change cannot be hidden by a paid sample.
        let refreshed = inventory::observe(
            &self.transport.agent,
            plan.admission
                .as_ref()
                .ok_or(CapacityImportJournalError::InfrastructureRequired)?,
            plan.authority.root,
            &registry,
        )
        .await?;
        self.infrastructure(plan).await?;
        if refreshed != inventory {
            return Err(CapacityImportJournalError::InventoryInvalid);
        }
        Ok(CapacityImportSourceView {
            binding: status.binding,
            cycles: status.cycles,
            reserved_cycles: status.reserved_cycles,
            ownership: CapacityImportOwnershipView::Unassigned,
            disposition_evidence_sha256: Some(disposition_digest(source)),
        })
    }
}
