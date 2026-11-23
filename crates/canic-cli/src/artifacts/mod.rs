//! Artifact evidence and explicit local plans; no live availability inference.

#[cfg(test)]
mod tests;

use crate::Output;
use canic_host::FleetArtifacts;
use ic_host_text::{bordered_table, escape};
use std::fmt::Write as _;

pub(crate) fn output(report: &FleetArtifacts, json: bool, details: bool) -> Output {
    let output = if json {
        Output::Json(serde_json::json!(report))
    } else {
        Output::Text(render(report, details))
    };
    if report.issues.is_empty() {
        output
    } else {
        Output::Partial(Box::new(output))
    }
}

pub(crate) fn run(
    command: &crate::args::FleetArtifactCommand,
    json: bool,
) -> canic_host::Result<Output> {
    use crate::args::{FleetArtifactCommand, RemovalAction};
    let (selection, state_dir) = match command {
        FleetArtifactCommand::List {
            selection,
            state_dir,
            ..
        }
        | FleetArtifactCommand::PlanRemoval {
            selection,
            state_dir,
            ..
        }
        | FleetArtifactCommand::PlanRestoration {
            selection,
            state_dir,
            ..
        } => (selection, state_dir),
    };
    let (_, selected, journal) = selection.resolve_journal(state_dir.as_deref())?;
    let (operation, plan) = match command {
        FleetArtifactCommand::List { details, .. } => {
            return Ok(output(
                &canic_host::fleet_artifacts(&journal, &selected)?,
                json,
                *details,
            ));
        }
        FleetArtifactCommand::PlanRemoval {
            operation, action, ..
        } => {
            let action = match action {
                RemovalAction::Cancel => canic_management_protocol::publication::Removal::Cancel,
                RemovalAction::Delete => canic_management_protocol::publication::Removal::Delete,
            };
            canic_host::create_removal_plan(&journal, operation, &selected, action)?
        }
        FleetArtifactCommand::PlanRestoration { operation, .. } => {
            canic_host::create_restoration_plan(&journal, operation, &selected)?
        }
    };
    if json {
        return Ok(Output::Json(
            serde_json::json!({"operation": operation, "plan": plan}),
        ));
    }
    let text = crate::status::render(&canic_host::status(&journal, &operation)?, &journal)?;
    Ok(Output::Text(format!(
        "{text}\nPlan retained; review before using fleet apply.\n"
    )))
}

fn render(report: &FleetArtifacts, details: bool) -> String {
    let mut output = format!(
        "App: {} · Fleet: {}\nLocal publication evidence\n\n",
        escape(&report.app),
        escape(&report.fleet),
    );
    if report.publications.is_empty() {
        output.push_str("No publication evidence found.\n\n");
    }
    for group in report
        .publications
        .chunk_by(|a, b| a.publication.store.canister == b.publication.store.canister)
    {
        writeln!(
            output,
            "Store: {}",
            escape(&group[0].publication.store.canister)
        )
        .expect("String write");
        let rows: Vec<_> = group
            .iter()
            .map(|entry| {
                vec![
                    entry.publication.release.name.clone(),
                    entry.publication.release.version.clone(),
                    entry.artifact.bytes.to_string(),
                    if entry.publication_retained {
                        "published"
                    } else {
                        "unconfirmed"
                    }
                    .into(),
                ]
            })
            .collect();
        writeln!(
            output,
            "{}\n",
            bordered_table(&["ARTIFACT", "VERSION", "BYTES", "EVIDENCE"], &rows)
        )
        .expect("String write");
        if details {
            for entry in group {
                writeln!(
                    output,
                    "{} / {}",
                    escape(&entry.publication.release.name),
                    escape(&entry.publication.release.version),
                )
                .expect("String write");
                for (label, value) in [
                    ("App canister", &entry.deployment.canister),
                    ("Wasm SHA-256", &entry.artifact.sha256),
                    ("Operation", &entry.operation),
                    ("Root", &entry.publication.root.canister),
                    ("Approved Root module", &entry.publication.root.module),
                    ("Approved Store module", &entry.publication.store.module),
                ] {
                    writeln!(output, "  {label}: {}", escape(value)).expect("String write");
                }
                if entry.publication_operation != entry.operation {
                    writeln!(
                        output,
                        "  Nested publication approval: {}",
                        escape(&entry.publication_operation)
                    )
                    .expect("String write");
                }
                output.push('\n');
            }
        }
    }
    for issue in &report.issues {
        writeln!(output, "Journal issue: {}", escape(issue)).expect("String write");
    }
    output
}
