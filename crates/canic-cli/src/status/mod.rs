//! Human projection of retained operation evidence; never a live status check.

use canic_host::{Error, Network, Plan, Receipt, Result};
use ic_host_text::{
    escape::terminal_text,
    table::{self, Alignment, Options},
};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(test)]
mod tests;

pub(crate) fn render(state: &serde_json::Value, journal: &Path) -> Result<String> {
    let plan: Plan = serde_json::from_value(state["plan"].clone())
        .map_err(|error| Error::Supplier(Box::new(error)))?;
    let receipt: Option<Receipt> = serde_json::from_value(state["receipt"].clone())
        .map_err(|error| Error::Supplier(Box::new(error)))?;
    let effects: BTreeMap<String, String> = serde_json::from_value(state["effects"].clone())
        .map_err(|error| Error::Supplier(Box::new(error)))?;
    let operation = state["operation"]
        .as_str()
        .ok_or_else(|| Error::Invalid("operation status has no digest".into()))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Supplier(Box::new(error)))?
        .as_nanos();
    let mut output = format!(
        "CANIC OPERATION\nOperation: {}\nJournal: {}\nLocal evidence only; no network observation.\n",
        terminal_text(operation),
        terminal_text(&journal.to_string_lossy()),
    );
    if let Some(name) = &plan.deployment {
        writeln!(
            output,
            "App: {}\nFleet: {}\nCanister slot: {}",
            terminal_text(&name.app),
            terminal_text(&name.fleet),
            terminal_text(&name.canister),
        )
        .expect("writing to String");
    }
    let network = match &plan.network {
        Network::Mainnet => "IC mainnet".into(),
        Network::Local { url, .. } => format!("Local replica: {url}"),
    };
    writeln!(
        output,
        "\nPlanned action: {}\nNetwork: {}\nNetwork binding: {}\nController: {}\nModule: {} ({} bytes)\nCreation debit ceiling: {} cycles\nNew-effect deadline: {} Unix seconds ({})",
        if plan.management.is_some() { "Root creates and installs its child" } else if plan.target.is_some() { "upgrade existing canister" } else { "create and install canister" },
        terminal_text(&network),
        plan.network.binding_id()?,
        terminal_text(plan.application_controller()),
        terminal_text(&plan.artifact.sha256),
        plan.artifact.bytes,
        plan.creation_cycles,
        plan.expires_at_ns / 1_000_000_000,
        if now >= u128::from(plan.expires_at_ns) { "expired" } else { "within original deadline" },
    )
    .expect("writing to String");
    if let Some(owner) = &plan.management {
        writeln!(
            output,
            "Root: {}\nRoot module: {}\nOperator: {}\nAutonomous dispatch: {} Unix seconds",
            terminal_text(&owner.root),
            terminal_text(&owner.root_module),
            terminal_text(&plan.controller),
            owner.execute_at_ns / 1_000_000_000
        )
        .expect("writing to String");
        if let Some(phase) = state["management"]["phase"].as_str() {
            writeln!(output, "Retained Root phase: {}", terminal_text(phase))
                .expect("writing to String");
        }
    }
    if let Some(target) = &plan.target {
        writeln!(output, "Upgrade target: {}", terminal_text(target)).expect("writing to String");
    }
    if let Some(module) = &plan.previous_module {
        writeln!(
            output,
            "Required previous module: {}",
            terminal_text(module)
        )
        .expect("writing to String");
    }
    let rows: Vec<_> = effects
        .iter()
        .map(|(effect, evidence)| {
            let description = match evidence.as_str() {
                "not_submitted" if effect == "create" && plan.target.is_some() => {
                    "Not applicable to this upgrade (not submitted)"
                }
                "not_submitted" => "Not submitted",
                "signed_attempt_retained_outcome_unknown" => {
                    "Signed attempt retained; outcome UNKNOWN"
                }
                "reply_retained" => "Reply retained; not itself proof of success",
                "rejection_retained" => "Rejection retained",
                _ => evidence,
            };
            [effect.clone(), description.into()]
        })
        .collect();
    writeln!(
        output,
        "\n{}",
        table::render(
            &["EFFECT", "RETAINED EVIDENCE"],
            &rows,
            &[Alignment::Left, Alignment::Left],
            Options {
                column_gap: 2,
                header_separator: true
            },
        )
    )
    .expect("writing to String");
    if let Some(receipt) = receipt {
        writeln!(
            output,
            "Receipt: retained (historical deployment evidence)\n  Operation: {}\n  Canister: {}\n  Module: {}\n  Controller: {}",
            terminal_text(&receipt.operation),
            terminal_text(&receipt.canister),
            terminal_text(&receipt.module_hash),
            terminal_text(&receipt.controller),
        )
        .expect("writing to String");
    } else {
        output.push_str("Receipt: none retained; deployment completion is not established.\n");
    }
    if effects
        .values()
        .any(|value| value == "signed_attempt_retained_outcome_unknown")
    {
        output.push_str("An uncertain attempt may have executed. Preserve this operation and its signed attempt; expiry does not establish failure or authorize a replacement.\n");
    }
    output.push_str("Current code, controllers, readiness and balances are not checked here.\n");
    Ok(output)
}
