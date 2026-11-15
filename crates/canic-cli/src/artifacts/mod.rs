//! Local publication evidence; no live availability or installed-version inference.
use crate::Output;
use canic_host::FleetArtifacts;
use ic_host_text::{escape::terminal_text, table::render_bordered};
use std::fmt::Write as _;

pub(crate) fn output(report: &FleetArtifacts, json: bool) -> Output {
    let output = if json {
        Output::Json(serde_json::json!(report))
    } else {
        Output::Text(render(report))
    };
    if report.issues.is_empty() {
        output
    } else {
        Output::Partial(Box::new(output))
    }
}

fn render(report: &FleetArtifacts) -> String {
    let mut output = format!(
        "App: {} · Fleet: {}\nLocal publication evidence\n\n",
        terminal_text(&report.app),
        terminal_text(&report.fleet),
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
            terminal_text(&group[0].publication.store.canister)
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
            render_bordered(&["ARTIFACT", "VERSION", "BYTES", "EVIDENCE"], &rows)
        )
        .expect("String write");
    }
    for issue in &report.issues {
        writeln!(output, "Journal issue: {}", terminal_text(issue)).expect("String write");
    }
    output
}

#[cfg(test)]
mod tests;
