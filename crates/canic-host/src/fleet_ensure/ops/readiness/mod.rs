//! Module: fleet_ensure::ops::readiness
//!
//! Responsibility: collect bounded pre-build native facts and project their limits.
//! Does not own: deployment authority, conversion, funding effects or artifact compilation.
//! Boundary: retained identities guide observation; exact selected controllers must still match.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::{
            DesiredCanister, DesiredCanisterKind, DesiredFleet, DesiredPresence,
            FleetEnsureStateRecord, MAX_FLEET_ENSURE_CANISTERS,
        },
        ops::{EnsurePaths, EnsureStateError, read_state, startup_funding},
        policy::EnsurePolicyError,
        view::readiness::{
            PrebuildFundingReadiness, ReadinessUnresolved, RootFundingReadiness,
            RootReadinessUnavailable,
        },
    },
    icp::{IcpCanisterStatusReport, IcpCli},
};
use candid::Principal;
use canic_core::cdk::types::Cycles;
use std::{collections::BTreeSet, path::Path};

/// Unknowns are explicit even when no desired input or caller estimate is supplied.
pub(in crate::fleet_ensure) fn unknown() -> PrebuildFundingReadiness {
    PrebuildFundingReadiness {
        desired_sha256: None,
        app_config_sha256: None,
        roots: Vec::new(),
        per_step_execution_allowance_cycles: None,
        conversion: None,
        unresolved: vec![
            ReadinessUnresolved::DesiredNotSelected,
            ReadinessUnresolved::ArtifactExecutionReserve,
            ReadinessUnresolved::PoolAndCurrentGrantUsage,
            ReadinessUnresolved::SelectedPlanOperatorDebit,
            ReadinessUnresolved::ConversionNotRequested,
            ReadinessUnresolved::OperatorIcpBalance,
            ReadinessUnresolved::FreshPlanAdmission,
        ],
    }
}

/// Compile configuration demand and observe only explicitly selected Root identities.
pub(in crate::fleet_ensure) fn roots(
    workspace: &Path,
    desired: &DesiredFleet,
    icp: &IcpCli,
) -> Result<PrebuildFundingReadiness, EnsureStateError> {
    roots_with(workspace, desired, |principal| {
        icp.canister_status_report(principal)
            .map_err(|_| RootReadinessUnavailable::ObservationFailed)
    })
}

fn roots_with(
    workspace: &Path,
    desired: &DesiredFleet,
    mut observe: impl FnMut(&str) -> Result<IcpCanisterStatusReport, RootReadinessUnavailable>,
) -> Result<PrebuildFundingReadiness, EnsureStateError> {
    let invalid = || EnsureStateError::StartupConfigurationMismatch;
    if desired.canisters.len() > MAX_FLEET_ENSURE_CANISTERS {
        return Err(invalid());
    }
    let mut names = BTreeSet::new();
    if desired
        .canisters
        .iter()
        .any(|entry| !names.insert(&entry.name))
    {
        return Err(invalid());
    }
    let paths = EnsurePaths::under(workspace, &desired.environment, &desired.fleet);
    let state = read_state(&paths, &desired.fleet)?;
    let config_hash = || {
        desired
            .protocol
            .as_ref()
            .map(|protocol| super::artifact_sha256(workspace, &protocol.app_config))
            .transpose()
    };
    let before_config = config_hash()?;
    let requirements = startup_funding::resolve(workspace, desired)?;
    let mut result = unknown();
    result
        .unresolved
        .retain(|reason| *reason != ReadinessUnresolved::DesiredNotSelected);
    let update = cycles(
        "maximum_update_burn_cycles",
        &desired.maximum_update_burn_cycles,
    )?;
    let observation = cycles(
        "maximum_observation_burn_cycles",
        &desired.maximum_observation_burn_cycles,
    )?;
    result.per_step_execution_allowance_cycles =
        Some(update.checked_add(observation).ok_or_else(invalid)?);
    for root in desired.canisters.iter().filter(|entry| {
        entry.kind == DesiredCanisterKind::Root && entry.presence == DesiredPresence::Present
    }) {
        let configured = cycles("minimum_cycles", &root.minimum_cycles)?;
        let requirement = requirements.get(&root.name);
        let startup = requirement.map(|value| value.minimum_native_cycles);
        if startup.is_none()
            && !result
                .unresolved
                .contains(&ReadinessUnresolved::StartupConfiguration)
        {
            result
                .unresolved
                .push(ReadinessUnresolved::StartupConfiguration);
        }
        let minimum = configured.max(startup.unwrap_or(0));
        let principal = selected_principal(desired, &state, &root.name);
        let observed = principal
            .as_ref()
            .map_err(|reason| *reason)
            .and_then(|principal| {
                let controllers = controllers(desired, &state, root)?;
                let status = observe(principal)?;
                native_balance(principal, &controllers, &status)
            });
        result.roots.push(RootFundingReadiness {
            root: root.name.clone(),
            principal: principal.ok(),
            available_native_cycles: observed.as_ref().ok().copied(),
            configured_minimum_cycles: configured,
            startup_minimum_cycles: startup,
            required_native_floor_cycles: minimum,
            floor_shortfall_cycles: observed
                .as_ref()
                .ok()
                .map(|available| minimum.saturating_sub(*available)),
            unfunded_role: requirement.and_then(|value| value.unfunded_role.clone()),
            unavailable: observed.err(),
        });
    }
    if config_hash()? != before_config || read_state(&paths, &desired.fleet)? != state {
        return Err(invalid());
    }
    result.app_config_sha256 = before_config;
    Ok(result)
}

fn cycles(field: &'static str, value: &str) -> Result<u128, EnsureStateError> {
    value
        .parse::<Cycles>()
        .map(|cycles| cycles.to_u128())
        .map_err(|_| {
            EnsureStateError::StartupFunding(Box::new(EnsurePolicyError::InvalidFleetCycles {
                field,
                value: value.into(),
            }))
        })
}

fn selected_principal(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    name: &str,
) -> Result<String, RootReadinessUnavailable> {
    let configured = desired
        .canisters
        .iter()
        .find(|entry| entry.name == name)
        .ok_or(RootReadinessUnavailable::PrincipalUnresolved)?;
    let retained = state.principals.get(name);
    if configured
        .principal
        .as_ref()
        .zip(retained)
        .is_some_and(|(selected, prior)| selected != prior)
    {
        return Err(RootReadinessUnavailable::AuthorityMismatch);
    }
    let principal = configured
        .principal
        .as_ref()
        .or(retained)
        .ok_or(RootReadinessUnavailable::PrincipalUnresolved)?;
    Principal::from_text(principal).map_err(|_| RootReadinessUnavailable::AuthorityMismatch)?;
    Ok(principal.clone())
}

fn controllers(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    configured: &DesiredCanister,
) -> Result<BTreeSet<Principal>, RootReadinessUnavailable> {
    configured
        .controllers
        .iter()
        .cloned()
        .map(Ok)
        .chain(
            configured
                .controller_canisters
                .iter()
                .map(|name| selected_principal(desired, state, name)),
        )
        .map(|value| {
            Principal::from_text(value?).map_err(|_| RootReadinessUnavailable::AuthorityMismatch)
        })
        .collect()
}

fn native_balance(
    principal: &str,
    expected: &BTreeSet<Principal>,
    status: &IcpCanisterStatusReport,
) -> Result<u128, RootReadinessUnavailable> {
    let invalid = RootReadinessUnavailable::AuthorityMismatch;
    if status.id != principal || expected.is_empty() {
        return Err(invalid);
    }
    let controllers = status
        .settings
        .as_ref()
        .map(|settings| &settings.controllers)
        .or(status.public_controllers.as_ref())
        .ok_or(invalid)?;
    let observed: BTreeSet<_> = controllers
        .iter()
        .map(Principal::from_text)
        .collect::<Result<_, _>>()
        .map_err(|_| invalid)?;
    if observed.len() != controllers.len() || &observed != expected {
        return Err(invalid);
    }
    status
        .cycles
        .as_deref()
        .and_then(parse_native_cycles)
        .ok_or(RootReadinessUnavailable::BalanceUnavailable)
}

// ICP status uses decimal digits with optional three-digit underscore groups.
// Bound the external text to a grouped u128 and reject overflow without allocating.
fn parse_native_cycles(value: &str) -> Option<u128> {
    const MAX_DIGITS: usize = 39;
    const MAX_GROUPED_BYTES: usize = MAX_DIGITS + (MAX_DIGITS - 1) / 3;

    if value.len() > MAX_GROUPED_BYTES {
        return None;
    }
    let first_group_limit = if value.contains('_') { 3 } else { MAX_DIGITS };
    let mut cycles = 0_u128;
    for (index, group) in value.split('_').enumerate() {
        let valid_width = if index == 0 {
            (1..=first_group_limit).contains(&group.len())
        } else {
            group.len() == 3
        };
        if !valid_width {
            return None;
        }
        for byte in group.bytes() {
            if !byte.is_ascii_digit() {
                return None;
            }
            cycles = cycles
                .checked_mul(10)?
                .checked_add(u128::from(byte - b'0'))?;
        }
    }
    Some(cycles)
}
