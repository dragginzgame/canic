//! Module: fleet_ensure::ops::reinstall::source
//!
//! Responsibility: inspect retained activation evidence without making it executable authority.
//! Does not own: supersession, reset admission, source mutation or remote effects.
//! Boundary: exact source bytes and issued-effect hashes are evidence, not reset authority.

use crate::fleet_ensure::{
    model::{
        CurrentFleetProtocolAction, DesiredCanisterKind, EffectState, EnsureAction,
        FleetActivationSourceRecord, FleetProtocolReadRecord,
    },
    ops::{
        EnsurePaths, EnsureStateError, action_sha256, is_sha256, plan_content, read_document_bytes,
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use serde_json::Value;
use std::collections::BTreeMap;

/// Inspect source evidence independently of the replacement plan's executable schema.
/// A source plan is never converted into a current plan or passed to the effect driver.
pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<FleetActivationSourceRecord, EnsureStateError> {
    let plan_bytes = read_document_bytes(&paths.plan)?.ok_or_else(invalid)?;
    let journal_bytes = read_document_bytes(&paths.journal)?.ok_or_else(invalid)?;
    let state_bytes = read_document_bytes(&paths.state)?.ok_or_else(invalid)?;
    let mut plan: Value = serde_json::from_slice(&plan_bytes).map_err(|_| invalid())?;
    let journal: Value = serde_json::from_slice(&journal_bytes).map_err(|_| invalid())?;
    plan_content::hydrate(paths, &mut plan)?;
    let mut source = inspect(&plan, &journal, environment, fleet)?;
    source.infrastructure = infrastructure(paths, &plan, fleet)?;
    source.plan_document_sha256 = sha256_hex(&plan_bytes);
    source.journal_document_sha256 = sha256_hex(&journal_bytes);
    source.state_document_sha256 = sha256_hex(&state_bytes);
    Ok(source)
}

#[expect(
    clippy::too_many_lines,
    reason = "one evidence boundary validates the complete retained prefix without executing it"
)]
fn inspect(
    plan: &Value,
    journal: &Value,
    environment: &str,
    fleet: &str,
) -> Result<FleetActivationSourceRecord, EnsureStateError> {
    let operation_id = string(plan, "operation_id")?;
    let desired = plan
        .pointer("/reviewed_desired/desired")
        .ok_or_else(invalid)?;
    let conservation = plan.get("conservation").ok_or_else(invalid)?;
    for field in [
        "maximum_new_funding_cycles",
        "maximum_operator_debit_cycles",
        "maximum_unavoidable_fee_cycles",
        "scheduled_transfer_cycles",
    ] {
        if amount(conservation, field)? != 0 {
            return Err(invalid());
        }
    }
    let funding = journal
        .get("initial_estate_funding_cycles_by_root")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    let initial_estate_funding_cycles_by_root = funding
        .iter()
        .map(|(root, value)| {
            Ok((
                root.clone(),
                value
                    .as_str()
                    .ok_or_else(invalid)?
                    .parse::<u128>()
                    .map_err(|_| invalid())?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, EnsureStateError>>()?;
    let plan_sha256 = string(plan, "plan_sha256")?;
    let identity_matches = [
        plan.get("schema_version").and_then(Value::as_u64) == Some(1),
        journal.get("schema_version").and_then(Value::as_u64) == Some(1),
        string(plan, "scope")? == "full",
        string(plan, "environment")? == environment,
        string(plan, "fleet")? == fleet,
        string(journal, "fleet")? == fleet,
        string(journal, "operation_id")? == operation_id,
        string(journal, "plan_sha256")? == plan_sha256,
        string(journal, "completion")? == "in_progress",
        is_sha256(operation_id),
        is_sha256(plan_sha256),
    ];
    if !identity_matches.into_iter().all(|fact| fact)
        || !array(journal, "successor_phases")?.is_empty()
        || !array(journal, "funding_reviews")?.is_empty()
        || journal.get("estate_funding_required") != Some(&Value::Null)
    {
        return Err(invalid());
    }
    let canisters = array(plan, "canisters")?;
    if canisters.is_empty()
        || canisters.len() > crate::fleet_ensure::model::MAX_FLEET_ENSURE_CANISTERS
    {
        return Err(invalid());
    }
    for canister in canisters {
        if !array(canister, "actions")?.is_empty() {
            return Err(invalid());
        }
    }
    let actions = array(plan, "protocol_actions")?;
    let effects = array(journal, "effects")?;
    if actions.is_empty()
        || actions.len() > crate::fleet_ensure::model::MAX_FLEET_ENSURE_PROTOCOL_STEPS
        || effects.is_empty()
        || effects.len() >= actions.len()
    {
        return Err(invalid());
    }
    let mut provisioning = None;
    let mut registry_preparations = Vec::new();
    let mut stores = BTreeMap::new();
    for (index, raw_action) in actions.iter().enumerate() {
        let action: EnsureAction =
            serde_json::from_value(raw_action.clone()).map_err(|_| invalid())?;
        let EnsureAction::FleetProtocol {
            action: protocol, ..
        } = &action
        else {
            return Err(invalid());
        };
        if let Some(effect) = effects.get(index) {
            if string(effect, "action_sha256")? != action_sha256(&action) {
                return Err(invalid());
            }
            let state: EffectState =
                serde_json::from_value(effect.get("state").cloned().ok_or_else(invalid)?)
                    .map_err(|_| invalid())?;
            if index + 1 < effects.len() {
                if state != EffectState::Applied {
                    return Err(invalid());
                }
                if protocol.target_kind() == DesiredCanisterKind::Store {
                    let EnsureAction::FleetProtocol {
                        candid,
                        candid_sha256,
                        principal,
                        ..
                    } = &action
                    else {
                        return Err(invalid());
                    };
                    let binding = FleetProtocolReadRecord {
                        candid: candid.clone(),
                        candid_sha256: candid_sha256.clone(),
                        principal: principal.clone(),
                    };
                    if let Some(previous) = stores.insert(principal.clone(), binding.clone())
                        && previous != binding
                    {
                        return Err(invalid());
                    }
                }
                if matches!(
                    protocol.as_ref(),
                    CurrentFleetProtocolAction::PrepareComponentRegistry { .. }
                ) {
                    registry_preparations.push(action);
                }
            } else {
                let CurrentFleetProtocolAction::ProvisionComponents { request, .. } =
                    protocol.as_ref()
                else {
                    return Err(invalid());
                };
                if state != EffectState::Issued
                    || canic_core::cdk::utils::hash::hex_bytes(request.operation_id) != operation_id
                {
                    return Err(invalid());
                }
                provisioning = Some(action);
            }
        } else if !matches!(
            protocol.as_ref(),
            CurrentFleetProtocolAction::MaintainPoolReadiness { .. }
                | CurrentFleetProtocolAction::ObservePoolReadiness { .. }
        ) {
            return Err(invalid());
        }
    }
    if registry_preparations.is_empty() {
        return Err(invalid());
    }
    Ok(FleetActivationSourceRecord {
        infrastructure: Vec::new(),
        operator: string(desired, "operator")?.to_string(),
        cycles_ledger: string(desired, "cycles_ledger")?.to_string(),
        initial_controlled_cycles: amount(journal, "initial_controlled_cycles")?,
        maximum_execution_burn_cycles: amount(conservation, "maximum_execution_burn_cycles")?,
        initial_estate_funding_cycles_by_root,
        operation_id: operation_id.to_string(),
        plan_sha256: plan_sha256.to_string(),
        plan_document_sha256: String::new(),
        journal_document_sha256: String::new(),
        state_document_sha256: String::new(),
        provisioning: provisioning.ok_or_else(invalid)?,
        registry_preparations,
        stores: stores.into_values().collect(),
    })
}

fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str, EnsureStateError> {
    value.get(field).and_then(Value::as_str).ok_or_else(invalid)
}

fn amount(value: &Value, field: &str) -> Result<u128, EnsureStateError> {
    string(value, field)?.parse().map_err(|_| invalid())
}

fn array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>, EnsureStateError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(invalid)
}

const fn invalid() -> EnsureStateError {
    EnsureStateError::InvalidActivationSource
}

/// Read only exact artifact and controller bindings; no source desired plan becomes executable.
fn infrastructure(
    paths: &EnsurePaths,
    plan: &Value,
    fleet: &str,
) -> Result<Vec<crate::fleet_ensure::model::RootManagementBinding>, EnsureStateError> {
    let desired = plan
        .pointer("/reviewed_desired/desired")
        .ok_or_else(invalid)?;
    let canisters = array(desired, "canisters")?;
    let state = crate::fleet_ensure::ops::read_state(paths, fleet)?;
    let mut principals = state.principals.clone();
    for configured in canisters {
        if let Some(principal) = configured.get("principal").and_then(Value::as_str) {
            principals.insert(
                string(configured, "name")?.to_string(),
                principal.to_string(),
            );
        }
    }
    let mut bindings = Vec::new();
    for configured in canisters {
        let kind: DesiredCanisterKind =
            serde_json::from_value(configured.get("kind").cloned().ok_or_else(invalid)?)
                .map_err(|_| invalid())?;
        if kind == DesiredCanisterKind::Pool {
            continue;
        }
        if !matches!(
            kind,
            DesiredCanisterKind::Root
                | DesiredCanisterKind::Store
                | DesiredCanisterKind::Coordinator
        ) {
            return Err(invalid());
        }
        let name = string(configured, "name")?;
        let module_sha256 = crate::fleet_ensure::ops::artifact_sha256(
            &paths.workspace,
            string(configured, "wasm")?,
        )?;
        if state
            .topology
            .get(name)
            .and_then(|entry| entry.module_hash.as_deref())
            != Some(module_sha256.as_str())
        {
            return Err(invalid());
        }
        let mut controllers = array(configured, "controllers")?
            .iter()
            .map(|value| value.as_str().map(str::to_string).ok_or_else(invalid))
            .collect::<Result<Vec<_>, _>>()?;
        for controller in array(configured, "controller_canisters")? {
            controllers.push(
                principals
                    .get(controller.as_str().ok_or_else(invalid)?)
                    .cloned()
                    .ok_or_else(invalid)?,
            );
        }
        controllers.sort();
        controllers.dedup();
        bindings.push(crate::fleet_ensure::model::RootManagementBinding {
            controllers,
            module_sha256,
            name: name.to_string(),
            principal: principals.get(name).cloned().ok_or_else(invalid)?,
            subnet: string(configured, "subnet")?.to_string(),
        });
    }
    bindings.sort_by(|left, right| left.name.cmp(&right.name));
    if bindings.is_empty() {
        return Err(invalid());
    }
    Ok(bindings)
}
