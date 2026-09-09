//! Module: fleet_ensure::policy::reinstall::activation
//!
//! Responsibility: bind a non-destructive activation preparation to the complete observed estate.
//! Does not own: observation, local supersession, effects or callback settlement claims.
//! Boundary: the review stops the Coordinator and restarts unchanged Roots before any reset review.

#[cfg(test)]
pub(in crate::fleet_ensure) mod tests;

use super::super::*;
use crate::fleet_ensure::model::{
    FleetActivationResetRecord, FleetActivationSourceRecord, FleetReinstallAssetRecord,
    FleetReinstallRecord, RootActivationResetRecord,
};

/// Exact source and physical observations used by the preparation compiler.
pub(in crate::fleet_ensure) struct ActivationPreparationInput<'a> {
    pub root: RootStartPlanInput<'a>,
    pub artifacts: &'a DesiredFleetArtifacts,
    pub source: &'a FleetActivationSourceRecord,
    pub roots: &'a [RootActivationResetRecord],
    pub assets: &'a [FleetReinstallAssetRecord],
    pub observation: &'a FleetObservation,
}

/// Compile stop/restart preparation without granting any install or paid creation authority.
#[expect(
    clippy::too_many_lines,
    reason = "one pure review binds action order, exact estate and bounded conservation"
)]
pub(in crate::fleet_ensure) fn preparation(
    input: ActivationPreparationInput<'_>,
) -> Result<FleetEnsurePlan, EnsurePolicyError> {
    let desired = input.root.desired;
    let management = input.root.observation;
    let bounds = cycle_bounds(desired)?;
    if input.roots.len() != 1 {
        return Err(conflict("one affected source Root"));
    }
    for bootstrap in desired
        .bootstrap
        .as_ref()
        .into_iter()
        .flat_map(|b| &b.roots)
    {
        let pool = input
            .observation
            .estate_funding_domains
            .get(&bootstrap.root)
            .and_then(|domain| domain.pool.as_ref())
            .ok_or_else(|| conflict("complete source pool"))?;
        if pool.maximum_size != bootstrap.limits.canister_pool.maximum_size {
            return Err(conflict("unchanged physical capacity before reset"));
        }
    }
    if input.source.operator != desired.operator
        || input.source.cycles_ledger != desired.cycles_ledger
    {
        return Err(conflict("source operator and Ledger"));
    }
    // The next review must have a real replacement module and complete reviewed imports.
    let mut plan = root_reinstall::compile(input.root, input.artifacts)?
        .ok_or_else(|| conflict("corrected-release Root modules"))?;
    if plan.root_reinstall_bindings.len() != input.roots.len()
        || plan.operation_id == input.source.operation_id
    {
        return Err(conflict("complete changed Root set"));
    }
    let mut authorities = Vec::new();
    let mut canisters = Vec::new();
    for configured in &desired.canisters {
        if configured.kind == DesiredCanisterKind::Pool {
            continue;
        }
        let observed = management
            .roots
            .get(&configured.name)
            .ok_or_else(|| conflict("complete infrastructure"))?;
        let live = &observed.live;
        let mut controllers = live.controllers.clone();
        controllers.sort();
        let binding = RootManagementBinding {
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
            controllers: resolved_controllers(configured, input.observation)?,
            module_sha256: binding.module_sha256.clone(),
            name: configured.name.clone(),
            principal: configured
                .principal
                .clone()
                .ok_or_else(|| conflict("captured physical identity"))?,
            subnet: configured.subnet.clone(),
        };
        if binding != expected
            || configured.replace
            || configured.presence != DesiredPresence::Present
            || live.status != CanisterRuntimeStatus::Running
        {
            return Err(conflict("retained running infrastructure"));
        }
        let actions = match configured.kind {
            DesiredCanisterKind::Coordinator => vec![EnsureAction::Stop {
                name: configured.name.clone(),
                principal: live.principal.clone(),
            }],
            DesiredCanisterKind::Root => vec![
                EnsureAction::Stop {
                    name: configured.name.clone(),
                    principal: live.principal.clone(),
                },
                EnsureAction::Start {
                    name: configured.name.clone(),
                    principal: live.principal.clone(),
                },
            ],
            DesiredCanisterKind::Store => Vec::new(),
            _ => return Err(conflict("generated infrastructure")),
        };
        authorities.push(binding);
        canisters.push(CanisterPlan {
            actions,
            disposition: CanisterDisposition::Reuse,
            name: configured.name.clone(),
            observed_cycles: live.cycles,
            principal: Some(live.principal.clone()),
        });
    }
    if desired
        .canisters
        .iter()
        .filter(|c| c.kind == DesiredCanisterKind::Coordinator)
        .count()
        != 1
    {
        return Err(conflict("one Coordinator"));
    }
    let mut installed = authorities.clone();
    installed.sort_by(|left, right| left.name.cmp(&right.name));
    if installed != input.source.infrastructure {
        return Err(conflict("exact source artifacts and controllers"));
    }
    canisters.sort_by_key(|c| {
        let coordinator = c.actions.as_slice().len() == 1;
        (!coordinator, c.name.clone())
    });
    let infrastructure = authorities
        .iter()
        .map(|a| a.principal.as_str())
        .collect::<BTreeSet<_>>();
    let assets = input
        .assets
        .iter()
        .map(|a| a.principal.as_str())
        .collect::<BTreeSet<_>>();
    let observed = input
        .observation
        .canisters
        .values()
        .filter_map(Option::as_ref)
        .map(|live| live.principal.as_str())
        .collect::<BTreeSet<_>>();
    let complete = infrastructure
        .union(&assets)
        .copied()
        .collect::<BTreeSet<_>>();
    if infrastructure.len() != authorities.len()
        || assets.len() != input.assets.len()
        || !infrastructure.is_disjoint(&assets)
        || observed != complete
        || !input.observation.additional_controlled_cycles.is_empty()
    {
        return Err(conflict("unique complete physical estate"));
    }
    let native = input
        .observation
        .canisters
        .values()
        .filter_map(Option::as_ref)
        .try_fold(0_u128, |total, live| {
            checked_add(total, live.cycles, "source native cycles")
        })?;
    let mut domains = compile_estate_funding_domains(desired, input.observation, bounds, true)?;
    let mut ledger = 0_u128;
    for domain in &mut domains {
        ledger = checked_add(
            ledger,
            domain
                .available_cycles
                .ok_or_else(|| conflict("observed Ledger balance"))?,
            "source Ledger cycles",
        )?;
        domain.required_creation_count = 0;
        domain.maximum_creation_debit_cycles = 0;
        domain.maximum_creation_fee_cycles = 0;
        domain.maximum_funding_cycles = 0;
        domain.shortfall_cycles = 0;
    }
    let cycles = checked_add(native, ledger, "source controlled cycles")?;
    if input
        .source
        .initial_controlled_cycles
        .checked_sub(cycles)
        .is_none_or(|burn| burn > input.source.maximum_execution_burn_cycles)
    {
        return Err(conflict("bounded source protocol debit"));
    }
    let observed_accounts = domains
        .iter()
        .map(|domain| {
            (
                domain.root.clone(),
                domain.available_cycles.unwrap_or_default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if observed_accounts != input.source.initial_estate_funding_cycles_by_root {
        return Err(conflict("unchanged source Ledger accounts"));
    }
    let observations = (authorities.len() + input.assets.len()) as u128;
    let actions = canisters
        .iter()
        .map(|c| c.actions.len() as u128)
        .sum::<u128>();
    let burn = bounds
        .observation_burn
        .checked_mul(observations)
        .and_then(|b| b.checked_mul(8))
        .and_then(|b| {
            bounds
                .update_burn
                .checked_mul(actions)
                .and_then(|updates| b.checked_add(updates))
        })
        .ok_or_else(|| conflict("preparation burn bound"))?;
    let remaining = cycles
        .checked_sub(burn)
        .ok_or_else(|| conflict("preparation cycle budget"))?;
    plan.canisters = canisters;
    plan.conservation = CycleConservation {
        estate_funding_domains: domains,
        expected_post_operation_cycles: remaining,
        maximum_execution_burn_cycles: burn,
        maximum_new_funding_cycles: 0,
        maximum_operator_debit_cycles: 0,
        maximum_unavoidable_fee_cycles: 0,
        observed_controlled_cycles: cycles,
        retained_in_reused_canisters_cycles: cycles,
        scheduled_transfer_cycles: 0,
    };
    plan.scope = FleetEnsurePlanScope::ReinstallPreparation;
    plan.root_reinstall_bindings.clear();
    plan.recovery_review = None;
    plan.reinstall = Some(Box::new(FleetReinstallRecord {
        operation_id: plan.operation_id.clone(),
        source_operation_id: input.source.operation_id.clone(),
        authorities,
        assets: input.assets.to_vec(),
        activation_reset: Some(Box::new(FleetActivationResetRecord {
            preparation: None,
            source: input.source.clone(),
            roots: input.roots.to_vec(),
        })),
    }));
    plan.plan_sha256 = expected_plan_sha256(&plan);
    Ok(plan)
}

fn conflict(field: &'static str) -> EnsurePolicyError {
    EnsurePolicyError::RootManagementAuthorityMismatch {
        field,
        name: "inactive activation preparation".into(),
    }
}

/// Compile the separate Root wipe only after the retained preparation has completed.
pub(in crate::fleet_ensure) fn reset(
    input: RootStartPlanInput<'_>,
    artifacts: &DesiredFleetArtifacts,
    prepared: &FleetEnsurePlan,
    evidence: crate::fleet_ensure::model::ActivationPreparationEvidenceRecord,
    observation: &FleetObservation,
) -> Result<FleetEnsurePlan, EnsurePolicyError> {
    let bounds = cycle_bounds(input.desired)?;
    let mut plan = root_reinstall::compile(input, artifacts)?
        .ok_or_else(|| conflict("corrected Root reset"))?;
    if plan.operation_id != prepared.operation_id || plan.desired_sha256 != prepared.desired_sha256
    {
        return Err(conflict("prepared operation identity"));
    }
    let mut intent = prepared
        .reinstall
        .clone()
        .ok_or_else(|| conflict("completed preparation"))?;
    let activation = intent
        .activation_reset
        .as_mut()
        .ok_or_else(|| conflict("source activation"))?;
    if activation.preparation.is_some()
        || plan.root_reinstall_bindings.len() != activation.roots.len()
    {
        return Err(conflict("prepared Root set"));
    }
    activation.preparation = Some(evidence);
    let native = observation
        .canisters
        .values()
        .filter_map(Option::as_ref)
        .try_fold(0_u128, |total, live| {
            checked_add(total, live.cycles, "reset native cycles")
        })?;
    let total = observation
        .estate_funding_domains
        .values()
        .try_fold(native, |total, domain| {
            checked_add(
                total,
                domain
                    .balance_cycles
                    .ok_or_else(|| conflict("reset Ledger observation"))?,
                "reset controlled cycles",
            )
        })?;
    let extra = bounds
        .update_burn
        .checked_mul(plan.root_reinstall_bindings.len() as u128)
        .ok_or_else(|| conflict("reset update budget"))?;
    let burn = checked_add(
        prepared.conservation.maximum_execution_burn_cycles,
        extra,
        "reset burn budget",
    )?;
    plan.conservation = prepared.conservation.clone();
    plan.conservation.maximum_execution_burn_cycles = burn;
    plan.conservation.observed_controlled_cycles = total;
    plan.conservation.retained_in_reused_canisters_cycles = total;
    plan.conservation.expected_post_operation_cycles = total
        .checked_sub(burn)
        .ok_or_else(|| conflict("reset cycle budget"))?;
    if let Some(review) = &mut plan.recovery_review {
        review.base_execution_burn_cycles = burn;
    }
    plan.reinstall = Some(intent);
    plan.plan_sha256 = expected_plan_sha256(&plan);
    Ok(plan)
}
