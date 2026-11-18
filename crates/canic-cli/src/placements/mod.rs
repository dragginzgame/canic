//! Pure presentation of configured slots and Query's independent observations.

#[cfg(test)]
mod tests;

use crate::Output;
use canic_host::placements::{FleetPlacements, PlacementOutcome};
use ic_host_text::{bordered_table, escape};
use std::collections::BTreeSet;

pub(crate) fn output(report: &FleetPlacements, json: bool) -> Output {
    let output = if json {
        Output::Json(serde_json::json!(report))
    } else {
        Output::Text(render(report))
    };
    if report.collection_complete() {
        output
    } else {
        Output::Partial(Box::new(output))
    }
}

fn render(report: &FleetPlacements) -> String {
    let configured = report
        .slots
        .iter()
        .filter(|slot| slot.canister_id.is_some())
        .count();
    let mut read = 0;
    let mut subnets = BTreeSet::new();
    let rows: Vec<_> = report
        .slots
        .iter()
        .map(|slot| {
            let outcome = slot
                .observation_index
                .and_then(|i| report.observation.as_ref()?.canisters.get(i))
                .map(|row| &row.outcome);
            let (subnet, state) = match outcome {
                Some(PlacementOutcome::Read { report }) => {
                    read += 1;
                    subnets.insert(report.subnet_id);
                    (report.subnet_id.to_string(), "read".to_owned())
                }
                Some(PlacementOutcome::Failed { error }) => ("unknown".into(), error.to_string()),
                Some(PlacementOutcome::IdentityMismatch { .. }) => {
                    ("unknown".into(), "identity mismatch".into())
                }
                Some(PlacementOutcome::OverallDeadline) => {
                    ("unknown".into(), "collection deadline".into())
                }
                Some(PlacementOutcome::NotStarted) => ("unknown".into(), "not started".into()),
                None if slot.canister_id.is_none() => ("—".into(), "unbound".into()),
                None => ("unknown".into(), "observation unavailable".into()),
            };
            vec![
                slot.name.clone(),
                slot.role.clone().unwrap_or_else(|| "—".into()),
                slot.parent.clone().unwrap_or_else(|| "—".into()),
                slot.canister_id.clone().unwrap_or_else(|| "—".into()),
                subnet,
                state,
            ]
        })
        .collect();
    format!(
        "App: {}\nFleet: {}\nNetwork: ic / mainnet\nCoverage: configured bindings only; retained deployment outputs are not included.\nCollection: {}\nBindings: {} read / {} configured | {} unresolved | {} unbound slots\nObserved subnets: {} (successful reads only)\n\n{}\n\nIndependent Registry routing evidence; not consensus-certified or an atomic snapshot.\nRoles and parents are application declarations, not controllers or deployment authority.\nUse --json for each lookup's source evidence.\n",
        escape(&report.app),
        escape(&report.fleet),
        if report.observation.is_none() {
            "not attempted (no configured bindings)"
        } else if report.collection_complete() {
            "complete"
        } else {
            "partial"
        },
        read,
        configured,
        configured.saturating_sub(read),
        report.slots.len() - configured,
        subnets.len(),
        bordered_table(
            &["SLOT", "ROLE", "PARENT", "CANISTER", "SUBNET", "OUTCOME"],
            &rows
        )
    )
}
