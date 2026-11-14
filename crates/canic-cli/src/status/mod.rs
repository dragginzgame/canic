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

mod management;
#[cfg(test)]
mod tests;

pub(crate) fn render(state: &serde_json::Value, journal: &Path) -> Result<String> {
    if let Some(value) = state.get("fleet_plan") {
        let plan: canic_host::FleetPlan = serde_json::from_value(value.clone())
            .map_err(|error| Error::Supplier(Box::new(error)))?;
        let operation = state["operation"]
            .as_str()
            .ok_or_else(|| Error::Invalid("missing fleet operation".into()))?;
        let mut output = fleet_preview(operation, &plan, journal)?;
        if !state["cancellation"].is_null() {
            output.push_str("Cancelled before effect preparation. This approval cannot apply.\n");
            return Ok(output);
        }
        let evidence = |step: &serde_json::Value| -> String {
            if step.is_null() {
                return "Waiting for retained Root completion".into();
            }
            if !step["receipt"].is_null() {
                return "Completion receipt retained".into();
            }
            step["management"]["phase"].as_str().map_or_else(
                || "No completion receipt; inspect host effects with --json".into(),
                |phase| format!("{phase}; no completion receipt"),
            )
        };
        output.push_str(&table::render(
            &["STEP", "RETAINED EVIDENCE"],
            &[
                ["Root".into(), evidence(&state["root"])],
                ["Application".into(), evidence(&state["child"])],
            ],
            &[Alignment::Left; 2],
            Options {
                column_gap: 2,
                header_separator: true,
            },
        ));
        output.push_str(
            "\n\nLocal evidence only. Resume this same fleet operation with fleet apply.\n",
        );
        if !state["child"]["management"].is_null() {
            output.push_str(&management::render(
                &state["child"]["management"],
                false,
                !state["child"]["receipt"].is_null(),
            )?);
        }
        return Ok(output);
    }
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
        if plan.management.is_some() && plan.target.is_some() { "Root upgrades its existing child" } else if plan.management.is_some() { "Root creates and installs its child" } else if plan.target.is_some() { "upgrade existing canister" } else { "create and install canister" },
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
        output.push_str(&management::render(
            &state["management"],
            plan.target.is_some(),
            receipt.is_some(),
        )?);
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

pub(crate) fn fleet_preview(
    id: &str,
    plan: &canic_host::FleetPlan,
    journal: &Path,
) -> Result<String> {
    if plan.digest()? != id {
        return Err(Error::Invalid("fleet approval digest mismatch".into()));
    }
    let root = plan
        .root
        .deployment
        .as_ref()
        .ok_or_else(|| Error::Invalid("missing Root slot".into()))?;
    let rows = [
        [
            root.canister.clone(),
            "operator".into(),
            format!("{} debit ceiling", cycles(plan.root.creation_cycles)),
        ],
        [
            plan.child.deployment.canister.clone(),
            format!("{}.result.canister", root.canister),
            format!("{} from Root", cycles(plan.child.creation_cycles)),
        ],
    ];
    Ok(format!(
        "CANIC FLEET PLAN\nOperation: {}\nApp: {}\nFleet: {}\nNetwork: {}\nOperator: {}\nJournal: {}\n\n{}\n\nRoot module: {}\nApplication module: {}\nAdmission before: {}\nChild effect deadline: {}\n\n",
        terminal_text(id),
        terminal_text(&root.app),
        terminal_text(&root.fleet),
        terminal_text(&match &plan.root.network {
            Network::Mainnet => "IC mainnet".into(),
            Network::Local { url, .. } => format!("Local replica: {url}"),
        }),
        terminal_text(&plan.root.controller),
        terminal_text(&journal.to_string_lossy()),
        table::render(
            &["CANISTER", "CONTROLLER", "BUDGET"],
            &rows,
            &[Alignment::Left; 3],
            Options {
                column_gap: 2,
                header_separator: true
            }
        ),
        plan.root.artifact.sha256,
        plan.child.artifact.sha256,
        deadline(plan.child.execute_at_ns),
        deadline(plan.child.execute_at_ns + 600_000_000_000)
    ))
}

fn deadline(ns: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs());
    let seconds = ns / 1_000_000_000;
    let relative = if seconds > now {
        format!("in {}m {}s", (seconds - now) / 60, (seconds - now) % 60)
    } else {
        "expired".into()
    };
    format!(
        "{} ({relative})",
        ic_host_text::timestamp::format_utc_timestamp_secs(seconds)
    )
}

fn cycles(value: u64) -> String {
    let scaled = ic_host_text::quantity::scaled_quantity_text(
        u128::from(value),
        &[
            (1_000_000_000_000, "T cycles"),
            (1_000_000_000, "B cycles"),
            (1, "cycles"),
        ],
    );
    scaled.map_or_else(
        |_| format!("{value} cycles"),
        |text| format!("{text} ({value})"),
    )
}
