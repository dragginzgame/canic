//! Module: fleet_ensure::json
//!
//! Responsibility: encode current Fleet Ensure plan and report JSON projections.
//! Does not own: plan hashing, schema selection, or durable file replacement.
//! Boundary: owning types encode cycle amounts as decimal text; report payloads stay external.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::model::{CanisterPlan, EnsureAction, FleetEnsurePlan, FleetEnsureReport};

use canic_core::cdk::utils::hash::sha256_hex;
use serde::Serialize;
use serde_json::{Map, Value};

const STORE_CHUNK_OBJECT_DIRECTORY: &str = ".canic/fleet-ensure/objects/sha256";

/// Report supplementary registration authority using the existing Store content references.
pub fn registration_recovery_json_value(
    review: &crate::fleet_ensure::model::infrastructure_bootstrap::registration_recovery::BootstrapRegistrationReviewRecord,
) -> Result<Value, serde_json::Error> {
    let mut compact = review.clone();
    compact.protocol_actions.clear();
    let mut projection = serde_json::to_value(&compact)?;
    projection["protocol_actions"] = Value::Array(
        review
            .protocol_actions
            .iter()
            .map(action_json_value)
            .collect::<Result<_, _>>()?,
    );
    Ok(projection)
}

/// Project one complete report without expanding content-addressed Store chunk bytes.
///
/// Each chunk retains a workspace-relative object path, its exact SHA-256 and its byte size.
pub fn report_json_value(report: &FleetEnsureReport) -> Result<Value, serde_json::Error> {
    let mut projection = Map::new();
    insert_serialized(
        &mut projection,
        "continuation_forecast",
        &crate::fleet_ensure::policy::continuation_forecast::forecast(report),
    )?;
    insert_serialized(
        &mut projection,
        "actual_conservation",
        &report.actual_conservation,
    )?;
    insert_serialized(&mut projection, "effects_applied", &report.effects_applied)?;
    insert_serialized(&mut projection, "funding_review", &report.funding_review)?;
    projection.insert("plan".to_string(), plan_json_value(&report.plan)?);
    insert_serialized(&mut projection, "terminal", &report.terminal)?;
    Ok(Value::Object(projection))
}

fn plan_json_value(plan: &FleetEnsurePlan) -> Result<Value, serde_json::Error> {
    let mut projection = Map::new();
    insert_serialized(
        &mut projection,
        "infrastructure_bootstrap",
        &plan.infrastructure_bootstrap,
    )?;
    insert_serialized(&mut projection, "recovery_review", &plan.recovery_review)?;
    insert_serialized(&mut projection, "reinstall", &plan.reinstall)?;
    projection.insert(
        "canisters".to_string(),
        Value::Array(
            plan.canisters
                .iter()
                .map(canister_json_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    insert_serialized(&mut projection, "conservation", &plan.conservation)?;
    insert_serialized(&mut projection, "continuation", &plan.continuation)?;
    insert_serialized(&mut projection, "desired_sha256", &plan.desired_sha256)?;
    insert_serialized(&mut projection, "environment", &plan.environment)?;
    insert_serialized(&mut projection, "fleet", &plan.fleet)?;
    insert_serialized(&mut projection, "operation_id", &plan.operation_id)?;
    insert_serialized(&mut projection, "plan_sha256", &plan.plan_sha256)?;
    insert_serialized(&mut projection, "planned_at_time", &plan.planned_at_time)?;
    projection.insert(
        "protocol_actions".to_string(),
        Value::Array(
            plan.protocol_actions
                .iter()
                .map(action_json_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    insert_serialized(
        &mut projection,
        "root_start_authority",
        &plan.root_start_authority,
    )?;
    insert_serialized(
        &mut projection,
        "root_reinstall_bindings",
        &plan.root_reinstall_bindings,
    )?;
    insert_serialized(&mut projection, "reviewed_desired", &plan.reviewed_desired)?;
    insert_serialized(&mut projection, "schema_version", &plan.schema_version)?;
    insert_serialized(&mut projection, "scope", &plan.scope)?;
    insert_serialized(
        &mut projection,
        "terminal_inventory_operation_id",
        &plan.terminal_inventory_operation_id,
    )?;
    Ok(Value::Object(projection))
}

fn canister_json_value(canister: &CanisterPlan) -> Result<Value, serde_json::Error> {
    let mut projection = Map::new();
    projection.insert(
        "actions".to_string(),
        Value::Array(
            canister
                .actions
                .iter()
                .map(action_json_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    insert_serialized(&mut projection, "disposition", &canister.disposition)?;
    insert_serialized(&mut projection, "name", &canister.name)?;
    insert_serialized(
        &mut projection,
        "observed_cycles",
        &canister.observed_cycles.to_string(),
    )?;
    insert_serialized(&mut projection, "principal", &canister.principal)?;
    Ok(Value::Object(projection))
}

fn action_json_value(action: &EnsureAction) -> Result<Value, serde_json::Error> {
    let EnsureAction::FleetProtocol {
        action: protocol_action,
        ..
    } = action
    else {
        return serde_json::to_value(action);
    };
    let Some(bytes) = protocol_action.publication_bytes() else {
        return serde_json::to_value(action);
    };

    let bytes_sha256 = sha256_hex(bytes);
    let bytes_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut compact = action.clone();
    let EnsureAction::FleetProtocol {
        action: compact_protocol_action,
        ..
    } = &mut compact
    else {
        unreachable!("cloned Fleet protocol action retains its variant")
    };
    compact_protocol_action
        .publication_bytes_mut()
        .expect("cloned publication retains payload")
        .clear();

    let mut projection = serde_json::to_value(&compact)?;
    let projected_request = projection
        .get_mut("action")
        .and_then(|value| value.get_mut("request"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| json_shape_error("Store publication report projection is invalid"))?;
    projected_request.remove("bytes");
    projected_request.insert(
        "bytes_path".to_string(),
        Value::String(format!("{STORE_CHUNK_OBJECT_DIRECTORY}/{bytes_sha256}")),
    );
    projected_request.insert("bytes_sha256".to_string(), Value::String(bytes_sha256));
    projected_request.insert("bytes_size".to_string(), Value::from(bytes_size));
    Ok(projection)
}

fn insert_serialized<T>(
    projection: &mut Map<String, Value>,
    field: &'static str,
    value: &T,
) -> Result<(), serde_json::Error>
where
    T: Serialize,
{
    projection.insert(field.to_string(), serde_json::to_value(value)?);
    Ok(())
}

fn json_shape_error(reason: &'static str) -> serde_json::Error {
    <serde_json::Error as serde::ser::Error>::custom(reason)
}
