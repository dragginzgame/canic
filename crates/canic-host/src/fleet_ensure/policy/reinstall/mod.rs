//! Module: fleet_ensure::policy::reinstall
//!
//! Responsibility: admit explicit resets against exact observed authority.
//! Does not own: transport, inventory capture, persistence or effect sequencing.
//! Boundary: selected-build wipes and partial-activation repairs bind the complete physical estate.

pub(in crate::fleet_ensure) mod activation;

use super::*;
use crate::fleet_ensure::model::{DesiredCanisterInit, FleetReinstallRecord};

pub(in crate::fleet_ensure) fn validate_reset(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    observation: &FleetObservation,
    intent: &FleetReinstallRecord,
    operation_id: &str,
) -> Result<BTreeSet<String>, EnsurePolicyError> {
    validate_evidence_owner(intent)?;
    if intent.operation_id != operation_id || intent.source_operation_id == operation_id {
        return Err(conflict("operation identity"));
    }
    let mut targets = BTreeSet::new();
    for configured in &desired.canisters {
        if configured.presence != DesiredPresence::Present || configured.replace {
            return Err(conflict("unchanged topology"));
        }
        let live = observation
            .canisters
            .get(&configured.name)
            .and_then(Option::as_ref)
            .ok_or_else(|| conflict("existing physical estate"))?;
        if configured.kind == DesiredCanisterKind::Pool {
            let asset = intent
                .assets
                .iter()
                .find(|asset| asset.principal == live.principal)
                .ok_or_else(|| conflict("complete pool inventory"))?;
            let mut controllers = live.controllers.clone();
            controllers.sort();
            if controllers != asset.controllers {
                return Err(conflict("pool management authority"));
            }
            continue;
        }
        if !matches!(
            configured.kind,
            DesiredCanisterKind::Coordinator
                | DesiredCanisterKind::Root
                | DesiredCanisterKind::Store
        ) {
            return Err(conflict("generated infrastructure and pool estate"));
        }
        let binding = intent
            .authorities
            .iter()
            .find(|binding| binding.name == configured.name)
            .ok_or_else(|| conflict("infrastructure authority"))?;
        let mut controllers = live.controllers.clone();
        controllers.sort();
        let actual = RootManagementBinding {
            controllers,
            module_sha256: live
                .module_sha256
                .clone()
                .ok_or_else(|| conflict("installed module"))?,
            name: configured.name.clone(),
            principal: live.principal.clone(),
            subnet: configured.subnet.clone(),
        };
        let module_matches = source_module_matches(binding, configured, artifacts)?;
        let controllers_match =
            binding.controllers == resolved_controllers(configured, observation)?;
        if actual != *binding
            || !module_matches
            || !controllers_match
            || (live.status != CanisterRuntimeStatus::Running
                && !(configured.kind != DesiredCanisterKind::Root
                    && live.status == CanisterRuntimeStatus::Stopped))
        {
            return Err(conflict("source infrastructure authority"));
        }
        if configured.kind != DesiredCanisterKind::Root {
            targets.insert(configured.name.clone());
        }
    }
    let pools = desired
        .canisters
        .iter()
        .filter(|c| c.kind == DesiredCanisterKind::Pool)
        .count();
    let root_count = desired
        .canisters
        .iter()
        .filter(|c| c.kind == DesiredCanisterKind::Root)
        .count();
    if targets.len() + root_count != intent.authorities.len()
        || pools != intent.assets.len()
        || artifacts.continuation.is_none()
    {
        return Err(conflict("complete generated Fleet"));
    }
    Ok(targets)
}

fn validate_evidence_owner(intent: &FleetReinstallRecord) -> Result<(), EnsurePolicyError> {
    if intent.activation_reset.is_none() {
        return Err(conflict("exact reset evidence owner"));
    }
    Ok(())
}

fn source_module_matches(
    binding: &RootManagementBinding,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    artifacts: &DesiredFleetArtifacts,
) -> Result<bool, EnsurePolicyError> {
    Ok(configured.kind != DesiredCanisterKind::Root
        || binding.module_sha256 == wasm_sha256(artifacts, &configured.name)?)
}

fn conflict(field: &'static str) -> EnsurePolicyError {
    EnsurePolicyError::RootManagementAuthorityMismatch {
        field,
        name: "Fleet reinstall".to_string(),
    }
}

/// Bind each explicit reinstall to its reviewed, running Root history witness.
pub(super) fn bind_history_witness(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    intent: &FleetReinstallRecord,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    plan: &mut CanisterPlan,
) -> Result<(), EnsurePolicyError> {
    for action in &mut plan.actions {
        let EnsureAction::Install {
            reinstall_witness,
            mode: InstallMode::Reinstall,
            ..
        } = action
        else {
            continue;
        };
        let root_name = match &configured.canic_init {
            Some(DesiredCanisterInit::Root { root } | DesiredCanisterInit::Store { root }) => {
                root.as_str()
            }
            Some(DesiredCanisterInit::Coordinator) => desired
                .canisters
                .iter()
                .find(|c| c.kind == DesiredCanisterKind::Root)
                .map(|c| c.name.as_str())
                .ok_or_else(|| conflict("history witness Root"))?,
            None => return Err(conflict("history witness role")),
        };
        let authority = intent
            .authorities
            .iter()
            .find(|a| a.name == root_name)
            .cloned()
            .ok_or_else(|| conflict("history witness authority"))?;
        let protocol = desired
            .protocol
            .as_ref()
            .ok_or_else(|| conflict("history witness Candid"))?;
        let continuation = artifacts
            .continuation
            .as_ref()
            .ok_or_else(|| conflict("history witness hash"))?;
        let candid = protocol.root_candid.clone();
        let candid_sha256 = continuation.root_candid_sha256.clone();
        let replacement = if configured.kind == DesiredCanisterKind::Root {
            let mut installed = authority.clone();
            installed.module_sha256 = wasm_sha256(artifacts, &configured.name)?;
            Some(Box::new(
                crate::fleet_ensure::model::ReinstallRootWitnessRecord {
                    authority: installed,
                    candid: protocol.root_candid.clone(),
                    candid_sha256: continuation.root_candid_sha256.clone(),
                },
            ))
        } else {
            None
        };
        *reinstall_witness = Some(Box::new(
            crate::fleet_ensure::model::ReinstallHistoryWitness {
                prior_module_sha256: intent
                    .authorities
                    .iter()
                    .find(|binding| binding.name == configured.name)
                    .ok_or_else(|| conflict("history target module"))?
                    .module_sha256
                    .clone(),
                authority,
                candid,
                candid_sha256,
                replacement,
            },
        ));
    }
    Ok(())
}

/// Select an exact source witness, or the intended replacement while checking Root itself.
pub(in crate::fleet_ensure) fn history_authority(
    witness: &crate::fleet_ensure::model::ReinstallHistoryWitness,
    current: &RootManagementBinding,
    principal: &str,
    target_hash: &str,
    state: &crate::fleet_ensure::model::EffectState,
) -> Option<crate::fleet_ensure::model::ReinstallRootWitnessRecord> {
    if current == &witness.authority {
        return Some(crate::fleet_ensure::model::ReinstallRootWitnessRecord {
            authority: witness.authority.clone(),
            candid: witness.candid.clone(),
            candid_sha256: witness.candid_sha256.clone(),
        });
    }
    let replacement = witness.replacement.as_deref()?;
    // Intent is persisted before the call; a lost response can leave a completed install in Intent.
    let pending = matches!(
        state,
        crate::fleet_ensure::model::EffectState::Intent
            | crate::fleet_ensure::model::EffectState::Issued
    );
    let own_reinstall = pending && principal == witness.authority.principal;
    if own_reinstall && current == &replacement.authority && current.module_sha256 == target_hash {
        Some(replacement.clone())
    } else {
        None
    }
}
