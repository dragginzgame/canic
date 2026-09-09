//! Module: fleet_ensure::ops::recovery
//!
//! Responsibility: project exact newly observed actions into an operator review difference.
//! Does not own: authority, scheduling, funding decisions, or effects.
//! Boundary: copy only action identities; never serialize Wasm or request payloads for a pause.

use crate::fleet_ensure::model::{
    CurrentFleetProtocolAction, EnsureAction, FleetEnsurePlan, FleetReviewAction,
    FleetSuccessorReview,
};

pub(in crate::fleet_ensure) fn review_details(phase: &FleetEnsurePlan) -> FleetSuccessorReview {
    let actions = phase
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .chain(&phase.protocol_actions)
        .map(action_review)
        .collect();
    FleetSuccessorReview {
        actions,
        maximum_additional_debit_cycles: phase.conservation.maximum_operator_debit_cycles,
        next_review_command: format!(
            "canic fleet ensure {} --environment {}",
            shell_quote(&phase.fleet),
            shell_quote(&phase.environment)
        ),
    }
}

fn action_review(action: &EnsureAction) -> FleetReviewAction {
    let (kind, principal) = match action {
        EnsureAction::Create { .. } => ("create", None),
        EnsureAction::Delete { principal, .. } => ("delete", Some(principal)),
        EnsureAction::Fund { principal, .. } => ("fund", Some(principal)),
        EnsureAction::FundEstate { principal, .. } => ("fund_estate", Some(principal)),
        EnsureAction::Install { principal, .. } => ("install", Some(principal)),
        EnsureAction::Protocol { principal, .. } => ("protocol", Some(principal)),
        EnsureAction::SealAuthority { principal, .. } => ("seal_authority", Some(principal)),
        EnsureAction::SetControllers { principal, .. } => ("set_controllers", Some(principal)),
        EnsureAction::Start { principal, .. } => ("start", Some(principal)),
        EnsureAction::Stop { principal, .. } => ("stop", Some(principal)),
        EnsureAction::Transfer { principal, .. } => ("transfer", Some(principal)),
        EnsureAction::FleetProtocol {
            action, principal, ..
        } => (protocol_kind(action), Some(principal)),
    };
    FleetReviewAction {
        name: action.name().into(),
        principal: principal.cloned(),
        kind: kind.into(),
    }
}

const fn protocol_kind(action: &CurrentFleetProtocolAction) -> &'static str {
    match action {
        CurrentFleetProtocolAction::ActivateRegistry { .. } => "activate_registry",
        CurrentFleetProtocolAction::ActivateRegistryMirror { .. } => "activate_registry_mirror",
        CurrentFleetProtocolAction::AdoptStore { .. } => "adopt_store",
        CurrentFleetProtocolAction::BootstrapStore { .. } => "bootstrap_store",
        CurrentFleetProtocolAction::JoinRoot { .. } => "join_root",
        CurrentFleetProtocolAction::MaintainPoolReadiness { .. } => "maintain_pool_readiness",
        CurrentFleetProtocolAction::ObservePoolReadiness { .. } => "observe_pool_readiness",
        CurrentFleetProtocolAction::PrepareComponentRegistry { .. } => "prepare_component_registry",
        CurrentFleetProtocolAction::PrepareStoreChunkSet { .. } => "prepare_store_chunk_set",
        CurrentFleetProtocolAction::ProvisionComponents { .. } => "provision_components",
        CurrentFleetProtocolAction::PublishStoreChunk { .. } => "publish_store_chunk",
        CurrentFleetProtocolAction::ReconcilePoolAsset { .. } => "reconcile_pool_asset",
        CurrentFleetProtocolAction::StageStoreManifest { .. } => "stage_store_manifest",
        CurrentFleetProtocolAction::SynchronizeRegistry { .. } => "synchronize_registry",
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
