//! Module: observatory::ops::comparison::source
//!
//! Responsibility: admit exact source bindings, metric identities and reset windows.
//! Boundary: local evidence validation and numeric conversion, without filling missing rows.

use crate::observatory::{
    model::cost::{CostCounterReading, CostInterval, CostIntervalIssue, CounterMovement},
    policy::cost,
    view::*,
};
use std::collections::BTreeMap;

#[derive(PartialEq)]
struct SnapshotBinding<'a> {
    environment: &'a str,
    fleet: &'a str,
    authority: &'a ObservatoryAuthorityView,
}

#[derive(PartialEq)]
struct RoleBinding<'a> {
    role: &'a str,
    canister_id: &'a str,
    parent: Option<&'a str>,
    subnet: Option<&'a str>,
    release: &'a str,
    module: &'a str,
}

///
/// BalanceWindow
///
/// Parsed balance endpoints owned by host ops for interval projection.
///

#[derive(Clone, Copy)]
pub(super) struct BalanceWindow {
    pub interval: CostInterval,
    pub opening: u128,
    pub closing: u128,
}

pub(super) fn same_authority(
    before: &ObservatorySnapshotView,
    after: &ObservatorySnapshotView,
) -> Result<(), CostComparisonFailure> {
    if before.schema_version != 1 || after.schema_version != 1 {
        return Err(CostComparisonFailure::UnsupportedSchema);
    }
    if binding(before)? != binding(after)? {
        return Err(CostComparisonFailure::BindingChanged);
    }
    if before.collected_at_unix_ms >= after.collected_at_unix_ms {
        return Err(CostComparisonFailure::NonAdvancingWindow);
    }
    Ok(())
}

fn binding(
    snapshot: &ObservatorySnapshotView,
) -> Result<SnapshotBinding<'_>, CostComparisonFailure> {
    let Observation::Observed {
        source: ObservationSource::RetainedTerminalReview,
        value,
        ..
    } = &snapshot.authority
    else {
        return Err(CostComparisonFailure::AuthorityUnavailable);
    };
    Ok(SnapshotBinding {
        environment: &snapshot.environment,
        fleet: &snapshot.fleet,
        authority: value,
    })
}

pub(super) fn roles(
    snapshot: &ObservatorySnapshotView,
) -> Result<BTreeMap<&str, &ObservatoryRoleView>, CostComparisonFailure> {
    if snapshot.roles.is_empty()
        || snapshot.roles.len() > crate::fleet_ensure::model::MAX_FLEET_ENSURE_CANISTERS
    {
        return Err(CostComparisonFailure::InvalidSnapshot);
    }
    let mut roles = BTreeMap::new();
    for role in &snapshot.roles {
        if role.canister_id.len() > 128 {
            return Err(CostComparisonFailure::InvalidSnapshot);
        }
        let pid = candid::Principal::from_text(&role.canister_id)
            .map_err(|_| CostComparisonFailure::InvalidSnapshot)?;
        if pid.to_text() != role.canister_id
            || roles.insert(role.canister_id.as_str(), role).is_some()
        {
            return Err(CostComparisonFailure::InvalidSnapshot);
        }
    }
    Ok(roles)
}

pub(super) fn same_role(
    before: &ObservatoryRoleView,
    after: &ObservatoryRoleView,
) -> Result<(), CostComparisonFailure> {
    if role_binding(before)? != role_binding(after)? {
        return Err(CostComparisonFailure::BindingChanged);
    }
    Ok(())
}

fn role_binding(role: &ObservatoryRoleView) -> Result<RoleBinding<'_>, CostComparisonFailure> {
    Ok(RoleBinding {
        role: &role.role,
        canister_id: &role.canister_id,
        parent: role.parent_canister_id.as_deref(),
        subnet: role.subnet_id.as_deref(),
        release: required(role.release_identity.as_deref())?,
        module: required(role.expected_module_sha256.as_deref())?,
    })
}

fn required(value: Option<&str>) -> Result<&str, CostComparisonFailure> {
    value
        .filter(|text| !text.is_empty())
        .ok_or(CostComparisonFailure::BindingUnavailable)
}

pub(super) fn evidence(
    role: &ObservatoryRoleView,
) -> Result<&RoleCostEvidenceView, CostComparisonFailure> {
    role.costs
        .as_ref()
        .ok_or(CostComparisonFailure::MissingCosts)
}

pub(super) fn window(role: &ObservatoryRoleView) -> Result<&CostWindowView, CostComparisonFailure> {
    let evidence = evidence(role)?;
    if evidence
        .limitations
        .contains(&CostEvidenceLimitation::SourceWindowChanged)
    {
        return Err(CostComparisonFailure::SourceWindowChanged);
    }
    if evidence
        .limitations
        .contains(&CostEvidenceLimitation::SourceWindowUnavailable)
    {
        return Err(CostComparisonFailure::SourceWindowUnavailable);
    }
    let Observation::Observed {
        source: ObservationSource::PublicMetricCache,
        value,
        ..
    } = &evidence.window
    else {
        return Err(CostComparisonFailure::SourceWindowUnavailable);
    };
    if value.heap_started_at_ns.is_none() {
        return Err(CostComparisonFailure::SourceWindowUnavailable);
    }
    Ok(value)
}

pub(super) fn same_window(
    before: &ObservatoryRoleView,
    after: &ObservatoryRoleView,
) -> Result<(), CostComparisonFailure> {
    if window(before)? != window(after)? {
        return Err(CostComparisonFailure::SourceWindowChanged);
    }
    Ok(())
}

fn metric<'a>(
    role: &ObservatoryRoleView,
    observation: &'a Observation<CostSamplesView>,
    name: &str,
    canister_id: Option<&str>,
) -> Result<&'a CostMetricView, CostComparisonFailure> {
    let Observation::Observed {
        source: ObservationSource::PublicMetricCache,
        value,
        ..
    } = observation
    else {
        return Err(CostComparisonFailure::SnapshotUnavailable);
    };
    match value.state {
        CostSampleState::Fresh => (),
        CostSampleState::Stale => return Err(CostComparisonFailure::StaleSample),
        _ => return Err(CostComparisonFailure::SnapshotUnavailable),
    }
    if value.truncated {
        return Err(CostComparisonFailure::TruncatedSample);
    }
    if value.rows.len() > 256 {
        return Err(CostComparisonFailure::InvalidMetric);
    }
    let mut rows = value
        .rows
        .iter()
        .filter(|row| row.name == name && row.canister_id.as_deref() == canister_id);
    let row = rows.next().ok_or(CostComparisonFailure::MissingMetric)?;
    let exact_sample = value.sampled_at_ns == Some(row.observed_at_ns);
    if rows.next().is_some() || row.unit != "cycles" || !exact_sample {
        return Err(CostComparisonFailure::InvalidMetric);
    }
    if window(role)?
        .heap_started_at_ns
        .is_some_and(|start| row.observed_at_ns < start)
    {
        return Err(CostComparisonFailure::SourceWindowChanged);
    }
    Ok(row)
}

pub(super) fn value(row: &CostMetricView) -> Result<u128, CostComparisonFailure> {
    if row.value.len() > 39 {
        return Err(CostComparisonFailure::InvalidMetric);
    }
    let number = row
        .value
        .parse::<u128>()
        .map_err(|_| CostComparisonFailure::InvalidMetric)?;
    if row.value != number.to_string() {
        return Err(CostComparisonFailure::InvalidMetric);
    }
    Ok(number)
}

pub(super) fn balances(
    before: &ObservatoryRoleView,
    after: &ObservatoryRoleView,
) -> Result<BalanceWindow, CostComparisonFailure> {
    same_window(before, after)?;
    let old = metric(
        before,
        &evidence(before)?.balance,
        "balance",
        Some(&before.canister_id),
    )?;
    let new = metric(
        after,
        &evidence(after)?.balance,
        "balance",
        Some(&after.canister_id),
    )?;
    if old.measurement != CostMetricKind::Gauge || new.measurement != CostMetricKind::Gauge {
        return Err(CostComparisonFailure::InvalidMetric);
    }
    Ok(BalanceWindow {
        interval: cost::interval(old.observed_at_ns, new.observed_at_ns)
            .map_err(numeric_failure)?,
        opening: value(old)?,
        closing: value(new)?,
    })
}

pub(super) fn grants(
    before: &ObservatoryRoleView,
    after: &ObservatoryRoleView,
    child: Option<&str>,
) -> Result<CounterMovement, CostComparisonFailure> {
    same_window(before, after)?;
    let name = if child.is_some() {
        "cycles_funding.cycles_granted_to_child"
    } else {
        "cycles_funding.cycles_granted_total"
    };
    let old = metric(
        before,
        &evidence(before)?.funding_and_callbacks,
        name,
        child,
    )?;
    let new = metric(after, &evidence(after)?.funding_and_callbacks, name, child)?;
    cost::counter(counter_reading(old)?, counter_reading(new)?).map_err(numeric_failure)
}

fn counter_reading(row: &CostMetricView) -> Result<CostCounterReading, CostComparisonFailure> {
    let CostMetricKind::Counter {
        window_id,
        saturated,
    } = row.measurement
    else {
        return Err(CostComparisonFailure::InvalidMetric);
    };
    Ok(CostCounterReading {
        value: value(row)?,
        observed_at_ns: row.observed_at_ns,
        window_id,
        saturated,
    })
}

pub(super) const fn numeric_failure(issue: CostIntervalIssue) -> CostComparisonFailure {
    match issue {
        CostIntervalIssue::CounterDecreased => CostComparisonFailure::CounterDecreased,
        CostIntervalIssue::CounterWindowChanged => CostComparisonFailure::CounterWindowChanged,
        CostIntervalIssue::NonAdvancingWindow => CostComparisonFailure::NonAdvancingWindow,
        CostIntervalIssue::Overflow => CostComparisonFailure::Overflow,
        CostIntervalIssue::SaturatedCounter => CostComparisonFailure::SaturatedCounter,
    }
}
