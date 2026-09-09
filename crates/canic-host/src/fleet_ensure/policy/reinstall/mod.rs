//! Module: fleet_ensure::policy::reinstall
//!
//! Responsibility: admit explicit resets against exact observed authority.
//! Does not own: transport, inventory capture, persistence or effect sequencing.
//! Boundary: same-release wipes and partial-activation repairs bind the complete physical estate.

pub(in crate::fleet_ensure) mod activation;

use super::*;
use crate::fleet_ensure::model::{DesiredCanisterInit, FleetReinstallRecord};

/// Immutable inputs for the read-only preparation review.
pub(in crate::fleet_ensure) struct PreparationInput<'a> {
    pub desired: &'a DesiredFleet,
    pub artifacts: &'a DesiredFleetArtifacts,
    pub observation: &'a RootManagementObservation,
    pub candid_hashes: &'a BTreeMap<String, String>,
    pub source_operation_id: &'a str,
    pub desired_sha256: &'a str,
    pub operation_id: &'a str,
    pub time: u64,
}

#[expect(
    clippy::too_many_lines,
    reason = "one pure boundary binds preparation actions, authority and their conservation budget"
)]
pub(in crate::fleet_ensure) fn preparation(
    input: PreparationInput<'_>,
) -> Result<FleetEnsurePlan, EnsurePolicyError> {
    let desired = input.desired;
    validate_authority(desired, &desired.fleet)?;
    let protocol = desired
        .protocol
        .as_ref()
        .ok_or_else(|| conflict("typed generated protocol"))?;
    let bounds = cycle_bounds(desired)?;
    let observation = FleetObservation {
        additional_controlled_cycles: BTreeMap::new(),
        canisters: input
            .observation
            .roots
            .iter()
            .map(|(name, entry)| (name.clone(), Some(entry.live.clone())))
            .collect(),
        estate_funding_domains: BTreeMap::new(),
        ledger_fee_cycles: bounds.ledger_fee,
        operator_cycles: input.observation.operator_cycles,
        protocol_ready: BTreeMap::new(),
    };
    let mut authorities = Vec::new();
    let mut canisters = Vec::new();
    let mut cycles = 0_u128;
    for configured in &desired.canisters {
        if configured.kind == DesiredCanisterKind::Pool {
            continue;
        }
        let observed = input
            .observation
            .roots
            .get(&configured.name)
            .ok_or_else(|| conflict("complete infrastructure"))?;
        let live = &observed.live;
        let mut controllers = live.controllers.clone();
        controllers.sort();
        let expected_hash = wasm_sha256(input.artifacts, &configured.name)?;
        let actual = RootManagementBinding {
            controllers,
            module_sha256: live
                .module_sha256
                .clone()
                .ok_or_else(|| conflict("installed module"))?,
            name: observed.name.clone(),
            principal: live.principal.clone(),
            subnet: observed.subnet.clone(),
        };
        let expected = RootManagementBinding {
            controllers: resolved_controllers(configured, &observation)?,
            module_sha256: expected_hash,
            name: configured.name.clone(),
            principal: configured
                .principal
                .clone()
                .unwrap_or_else(|| live.principal.clone()),
            subnet: configured.subnet.clone(),
        };
        let retained = !configured.replace && configured.presence == DesiredPresence::Present;
        let running = live.status == CanisterRuntimeStatus::Running;
        if actual != expected || !retained || !running {
            return Err(conflict("unchanged running infrastructure"));
        }
        authorities.push(actual);
        let candid = match configured.kind {
            DesiredCanisterKind::Coordinator => &protocol.coordinator_candid,
            DesiredCanisterKind::Root => &protocol.root_candid,
            DesiredCanisterKind::Store => continue,
            _ => return Err(conflict("generated infrastructure")),
        };
        cycles = checked_add(cycles, live.cycles, "seal cycles")?;
        canisters.push(CanisterPlan {
            actions: vec![EnsureAction::SealAuthority {
                candid: candid.clone(),
                candid_sha256: input
                    .candid_hashes
                    .get(candid)
                    .cloned()
                    .ok_or_else(|| conflict("seal Candid"))?,
                authority_kind: configured.kind,
                name: configured.name.clone(),
                principal: live.principal.clone(),
            }],
            disposition: CanisterDisposition::Reuse,
            name: configured.name.clone(),
            observed_cycles: live.cycles,
            principal: Some(live.principal.clone()),
        });
    }
    let burn = bounds
        .observation_burn
        .checked_mul(8)
        .and_then(|b| b.checked_add(bounds.update_burn))
        .and_then(|b| b.checked_mul(canisters.len() as u128))
        .ok_or_else(|| conflict("seal budget"))?;
    let remaining = cycles
        .checked_sub(burn)
        .ok_or_else(|| conflict("seal funding"))?;
    let mut plan = FleetEnsurePlan {
        canisters,
        continuation: None,
        conservation: CycleConservation {
            estate_funding_domains: Vec::new(),
            expected_post_operation_cycles: remaining,
            maximum_execution_burn_cycles: burn,
            maximum_new_funding_cycles: 0,
            maximum_operator_debit_cycles: 0,
            maximum_unavoidable_fee_cycles: 0,
            observed_controlled_cycles: cycles,
            retained_in_reused_canisters_cycles: cycles,
            scheduled_transfer_cycles: 0,
        },
        desired_sha256: input.desired_sha256.to_string(),
        environment: desired.environment.clone(),
        fleet: desired.fleet.clone(),
        operation_id: input.operation_id.to_string(),
        plan_sha256: String::new(),
        planned_at_time: input.time,
        protocol_actions: Vec::new(),
        recovery_review: None,
        reinstall: Some(Box::new(FleetReinstallRecord {
            activation_reset: None,
            operation_id: input.operation_id.to_string(),
            source_operation_id: input.source_operation_id.to_string(),
            authorities,
            assets: Vec::new(),
        })),
        root_reinstall_bindings: Vec::new(),
        root_start_authority: None,
        reviewed_desired: Some(Box::new(
            crate::fleet_ensure::model::ReviewedDesiredFleetRecord::capture(desired),
        )),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        scope: FleetEnsurePlanScope::ReinstallPreparation,
        terminal_inventory_operation_id: Some(input.source_operation_id.to_string()),
    };
    plan.plan_sha256 = expected_plan_sha256(&plan);
    Ok(plan)
}

pub(in crate::fleet_ensure) fn validate_reset(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    observation: &FleetObservation,
    intent: &FleetReinstallRecord,
    operation_id: &str,
) -> Result<BTreeSet<String>, EnsurePolicyError> {
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
            if controllers != asset.controllers
                || (intent.activation_reset.is_none() && live.module_sha256 != asset.module_sha256)
            {
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
        let module_matches = (intent.activation_reset.is_some()
            && configured.kind != DesiredCanisterKind::Root)
            || binding.module_sha256 == wasm_sha256(artifacts, &configured.name)?;
        let controllers_match =
            binding.controllers == resolved_controllers(configured, observation)?;
        if actual != *binding
            || !module_matches
            || !controllers_match
            || (live.status != CanisterRuntimeStatus::Running
                && !(intent.activation_reset.is_some()
                    && configured.kind != DesiredCanisterKind::Root
                    && live.status == CanisterRuntimeStatus::Stopped))
        {
            return Err(conflict("same-release infrastructure authority"));
        }
        if intent.activation_reset.is_none() || configured.kind != DesiredCanisterKind::Root {
            targets.insert(configured.name.clone());
        }
    }
    let pools = desired
        .canisters
        .iter()
        .filter(|c| c.kind == DesiredCanisterKind::Pool)
        .count();
    let root_count = if intent.activation_reset.is_some() {
        desired
            .canisters
            .iter()
            .filter(|c| c.kind == DesiredCanisterKind::Root)
            .count()
    } else {
        0
    };
    if targets.len() + root_count != intent.authorities.len()
        || pools != intent.assets.len()
        || artifacts.continuation.is_none()
    {
        return Err(conflict("complete generated Fleet"));
    }
    Ok(targets)
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
                candid: protocol.root_candid.clone(),
                candid_sha256: continuation.root_candid_sha256.clone(),
            },
        ));
    }
    Ok(())
}
