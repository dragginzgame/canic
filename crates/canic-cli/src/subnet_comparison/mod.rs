//! Fleet-context projection of Query evidence; no ranking or deployment decision.
use crate::Output;
use canic_host::subnet_comparison::FleetSubnetComparison;
use ic_host_text::{bordered_table, escape};

pub(crate) fn output(report: &FleetSubnetComparison, json: bool) -> Output {
    let output = if json {
        Output::Json(serde_json::json!(report))
    } else {
        Output::Text(render(report))
    };
    if report.observation.collection_complete {
        output
    } else {
        Output::Partial(Box::new(output))
    }
}

fn render(report: &FleetSubnetComparison) -> String {
    let observation = &report.observation;
    let mut columns = Vec::new();
    for side in [&observation.left, &observation.right] {
        let mut values = if let Some(d) = side.distribution.report() {
            vec![
                d.configuration.registered_node_ids.len().to_string(),
                d.configuration.replica_version_id.clone(),
                d.configuration.is_halted.to_string(),
                d.configuration.halt_at_cup_height.to_string(),
                format!(
                    "{} known / {} unknown",
                    d.summary.provider_resolved, d.summary.provider_unknown
                ),
                format!(
                    "{} known / {} unknown",
                    d.summary.location_resolved, d.summary.location_unknown
                ),
                d.requested_registry_version.to_string(),
                if d.complete { "complete" } else { "partial" }.into(),
                d.version_observed_at_unix_seconds.to_string(),
            ]
        } else {
            vec!["unavailable".into(); 9]
        };
        if let Some(m) = side.metrics.report() {
            values.extend([
                m.num_canisters.to_string(),
                m.canister_state_bytes.to_string(),
                m.evidence.observed_at_unix_seconds.to_string(),
                m.evidence.state_certificate_verified.to_string(),
            ]);
        } else {
            values.extend(vec!["unavailable".into(); 4]);
        }
        columns.push(values);
    }
    let labels = [
        "Registered members",
        "Configured replica",
        "Halted flag",
        "Halt-at-CUP flag",
        "Provider coverage",
        "Location coverage",
        "Registry version",
        "Distribution coverage",
        "Registry observed (Unix s)",
        "Canisters",
        "Canister state (bytes)",
        "Metrics observed (Unix s)",
        "Metric certificate verified",
    ];
    let rows: Vec<_> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            vec![
                label.to_string(),
                columns[0][i].clone(),
                columns[1][i].clone(),
            ]
        })
        .collect();
    let mut text = format!(
        "App: {}\nFleet: {}\nNetwork: ic / mainnet\nLeft: {}\nRight: {}\nCollection: {} | elapsed: {:.2}s\nCandidates explicitly selected; no fleet ownership or placement approval inferred.\n\n{}\n",
        escape(&report.app),
        escape(&report.fleet),
        observation.left.subnet_id,
        observation.right.subnet_id,
        if observation.collection_complete {
            "complete"
        } else {
            "partial"
        },
        observation.elapsed.as_secs_f64(),
        bordered_table(&["OBSERVATION", "LEFT", "RIGHT"], &rows)
    );
    if let Some(overlap) = observation.infrastructure_overlap() {
        let rows = vec![
            vec![
                "Providers".into(),
                overlap.providers.shared.len().to_string(),
                overlap.providers.left_unknown_nodes.to_string(),
                overlap.providers.right_unknown_nodes.to_string(),
                if overlap.providers.coverage_complete {
                    "complete"
                } else {
                    "partial"
                }
                .into(),
            ],
            vec![
                "Data centers".into(),
                overlap.data_centers.shared.len().to_string(),
                overlap.data_centers.left_unknown_nodes.to_string(),
                overlap.data_centers.right_unknown_nodes.to_string(),
                if overlap.data_centers.coverage_complete {
                    "complete"
                } else {
                    "partial"
                }
                .into(),
            ],
        ];
        text.push_str("\nObserved infrastructure overlap\n\n");
        text.push_str(&bordered_table(
            &[
                "RELATIONSHIP",
                "SHARED IDENTITIES",
                "LEFT UNKNOWN",
                "RIGHT UNKNOWN",
                "COVERAGE",
            ],
            &rows,
        ));
        text.push('\n');
    } else {
        text.push_str("\nInfrastructure overlap unavailable: both distributions are required.\n");
    }
    text.push_str("\nIndependent Registry declarations and certified metrics; no atomic snapshot.\nUnknown coverage is not zero infrastructure; zero observed overlap does not prove independent failure domains.\nNo headroom, capacity ranking or merge-feasibility conclusion.\nUse --json for original sources, component failures and shared identity details.\n");
    text
}

#[cfg(test)]
mod tests;
