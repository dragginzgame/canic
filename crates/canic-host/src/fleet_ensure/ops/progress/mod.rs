//! Module: fleet_ensure::ops::progress
//!
//! Responsibility: project reviewed actions into bounded informational progress.
//! Does not own: execution order, completion, polling or retry decisions.
//! Boundary: reads the existing journal; never observes or mutates the platform.

use crate::fleet_ensure::{
    dto::{FleetEnsureActionKind, FleetEnsureActionProgress},
    model::{CurrentFleetProtocolAction, EffectState, EnsureAction, FleetEnsureJournalRecord},
};

/// Find unfinished work by its journal position, never by an aggregate effect count.
pub(in crate::fleet_ensure) fn next_action(
    actions: &[&EnsureAction],
    journal: &FleetEnsureJournalRecord,
) -> Option<FleetEnsureActionProgress> {
    actions.iter().enumerate().find_map(|(index, action)| {
        journal
            .effects
            .get(index)
            .is_none_or(|effect| effect.state != EffectState::Applied)
            .then(|| FleetEnsureActionProgress {
                kind: action_kind(action),
                target: bounded_target(action.name()),
            })
    })
}

fn bounded_target(name: &str) -> String {
    let mut target = name.chars().take(160).collect::<String>();
    if name.chars().count() > 160 {
        target.push_str("...");
    }
    target
}

const fn action_kind(action: &EnsureAction) -> FleetEnsureActionKind {
    match action {
        EnsureAction::SealAuthority { .. } => FleetEnsureActionKind::SealAuthority,
        EnsureAction::Create { .. } => FleetEnsureActionKind::Create,
        EnsureAction::Delete { .. } => FleetEnsureActionKind::Delete,
        EnsureAction::Fund { .. } => FleetEnsureActionKind::Fund,
        EnsureAction::FundEstate { .. } => FleetEnsureActionKind::FundEstate,
        EnsureAction::Install { .. } => FleetEnsureActionKind::Install,
        EnsureAction::Protocol { .. } => FleetEnsureActionKind::Protocol,
        EnsureAction::SetControllers { .. } => FleetEnsureActionKind::SetControllers,
        EnsureAction::Start { .. } => FleetEnsureActionKind::Start,
        EnsureAction::Stop { .. } => FleetEnsureActionKind::Stop,
        EnsureAction::Transfer { .. } => FleetEnsureActionKind::Transfer,
        EnsureAction::FleetProtocol { action, .. } => protocol_kind(action),
    }
}

const fn protocol_kind(action: &CurrentFleetProtocolAction) -> FleetEnsureActionKind {
    match action {
        CurrentFleetProtocolAction::ActivateRegistry { .. } => {
            FleetEnsureActionKind::ActivateRegistry
        }
        CurrentFleetProtocolAction::ActivateRegistryMirror { .. } => {
            FleetEnsureActionKind::ActivateRegistryMirror
        }
        CurrentFleetProtocolAction::AdoptStore { .. } => FleetEnsureActionKind::AdoptStore,
        CurrentFleetProtocolAction::BootstrapStore { .. } => FleetEnsureActionKind::BootstrapStore,
        CurrentFleetProtocolAction::JoinRoot { .. } => FleetEnsureActionKind::JoinRoot,
        CurrentFleetProtocolAction::MaintainPoolReadiness { .. } => {
            FleetEnsureActionKind::MaintainPoolReadiness
        }
        CurrentFleetProtocolAction::ObservePoolReadiness { .. } => {
            FleetEnsureActionKind::ObservePoolReadiness
        }
        CurrentFleetProtocolAction::PrepareComponentRegistry { .. } => {
            FleetEnsureActionKind::PrepareComponentRegistry
        }
        CurrentFleetProtocolAction::PrepareStoreFixture { .. } => {
            FleetEnsureActionKind::PrepareStoreFixture
        }
        CurrentFleetProtocolAction::ProvisionComponents { .. } => {
            FleetEnsureActionKind::ProvisionComponents
        }
        CurrentFleetProtocolAction::PublishStoreChunk { .. } => {
            FleetEnsureActionKind::PublishStoreChunk
        }
        CurrentFleetProtocolAction::PublishStoreFixtureChunk { .. } => {
            FleetEnsureActionKind::PublishStoreFixtureChunk
        }
        CurrentFleetProtocolAction::ReconcilePoolAsset { .. } => {
            FleetEnsureActionKind::ReconcilePoolAsset
        }
        CurrentFleetProtocolAction::SynchronizeRegistry { .. } => {
            FleetEnsureActionKind::SynchronizeRegistry
        }
    }
}
