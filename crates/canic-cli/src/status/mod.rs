//! Human projection of retained operation evidence; never a live status check.

use canic_host::{
    CanisterStatus, Error, ManagementStatus, Network, OperationStatus, Result, StatusDetail,
};
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

pub(crate) mod management;
pub(crate) mod store;
#[cfg(test)]
mod tests;

pub(crate) fn render(state: &OperationStatus, journal: &Path) -> Result<String> {
    let state = match state {
        OperationStatus::Fleet(state) => return render_fleet(state, journal),
        OperationStatus::Canister(state) => state,
    };
    let plan = &state.plan;
    let receipt = state.receipt.as_ref();
    let effects = &state.effects;
    let operation = &state.operation;
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
        match plan.action { canic_host::PlanAction::UpgradeStore { .. } => "Root upgrades Store, preserving its closed catalog and pins", canic_host::PlanAction::CreateStore { .. } => "create Store, hand control to Root and verify readiness", canic_host::PlanAction::Publish(_) => "publish artifact to Store (no deployment)", canic_host::PlanAction::MaintainRoot {..} => "upgrade Root, preserving child custody", canic_host::PlanAction::UpgradeChild {..} => "Root upgrades its existing child", canic_host::PlanAction::CreateChild(_) => "Root creates and installs its child", canic_host::PlanAction::Upgrade(_) => "upgrade existing canister", canic_host::PlanAction::Create {..} => "create and install canister" },
        terminal_text(&network),
        plan.network.binding_id()?,
        terminal_text(plan.application_controller()),
        terminal_text(&plan.artifact.sha256),
        plan.artifact.bytes,
        plan.creation_cycles(),
        plan.expires_at_ns / 1_000_000_000,
        if now >= u128::from(plan.expires_at_ns) { "expired" } else { "within original deadline" },
    )
    .expect("writing to String");
    if let Some(root) = plan.store_root().or_else(|| plan.store_upgrade_root()) {
        writeln!(
            output,
            "Root: {}\nRoot module: {}",
            terminal_text(&root.canister),
            terminal_text(&root.module)
        )
        .expect("String write");
    }
    if plan.store_upgrade_root().is_some() {
        if let StatusDetail::StoreMaintenance {
            store_maintenance: Some(status),
        } = &state.detail
        {
            output.push_str(&store::render(status, Some(now), receipt.is_some()));
        } else {
            output.push_str("Store maintenance: no retained observation\n");
        }
    }
    if let Some(publication) = plan.publication() {
        writeln!(output, "Root: {}\nRoot module: {}\nStore: {}\nStore module: {}\nRelease: {} / {}\nPublished receipt retained: {}",
            terminal_text(&publication.root.canister), terminal_text(&publication.root.module),
            terminal_text(&publication.store.canister), terminal_text(&publication.store.module),
            terminal_text(&publication.release.name), terminal_text(&publication.release.version), matches!(&state.detail, StatusDetail::Publication { publication: Some(_) })).expect("String write");
    }
    for custody in plan.maintenance().into_iter().flatten() {
        writeln!(
            output,
            "Preserve: child {} creation and {} completed child upgrade(s)",
            terminal_text(custody.creation.worker.as_deref().unwrap_or("unknown")),
            custody.upgrades.len()
        )
        .expect("writing to String");
    }
    if let Some(store) = plan.maintenance_store() {
        writeln!(
            output,
            "Preserved Store: {}\nStore module: {}\nRetained artifact pins: {}",
            terminal_text(&store.store.canister),
            terminal_text(&store.store.module),
            store
                .sources
                .iter()
                .flatten()
                .map(|s| 1 + s.upgrades.len())
                .sum::<usize>()
        )
        .expect("String write");
    }
    if let Some(owner) = plan.management() {
        writeln!(
            output,
            "Root: {}\nRoot module: {}\nOperator: {}\nAutonomous dispatch: {}",
            terminal_text(&owner.root),
            terminal_text(&owner.root_module),
            terminal_text(&plan.controller),
            dispatch(plan.created_at_ns, owner.execute_at_ns)
        )
        .expect("writing to String");
        if let Some(source) = &owner.source {
            writeln!(
                output,
                "Artifact source: Store {}\nStore module: {}\nRelease: {} / {}",
                terminal_text(&source.store.canister),
                terminal_text(&source.store.module),
                terminal_text(&source.manifest.release.name),
                terminal_text(&source.manifest.release.version)
            )
            .expect("String write");
        }
        output.push_str(&management::render(
            state.management.as_ref(),
            &state.detail,
            receipt.is_some(),
        ));
    }
    if let Some(target) = plan.target() {
        writeln!(output, "Upgrade target: {}", terminal_text(target)).expect("writing to String");
    }
    if let Some(module) = plan.previous_module() {
        writeln!(
            output,
            "Required previous module: {}",
            terminal_text(module)
        )
        .expect("writing to String");
    }
    let store_upgrade = plan.store_upgrade_root().is_some();
    let rows: Vec<_> = if store_upgrade {
        // Scans retain many read/continuation pairs. Summarize their states here;
        // the original per-attempt evidence remains in Host JSON and the journal.
        let mut counts = BTreeMap::<&str, usize>::new();
        for evidence in effects.values().filter(|s| s.as_str() != "not_submitted") {
            *counts.entry(evidence.as_str()).or_default() += 1;
        }
        if counts.is_empty() {
            vec![["No retained request attempts".into(), "0".into()]]
        } else {
            counts
                .into_iter()
                .map(|(state, count)| {
                    [
                        evidence_description("", state, false).into(),
                        count.to_string(),
                    ]
                })
                .collect()
        }
    } else {
        effects
            .iter()
            .map(|(effect, evidence)| {
                [
                    effect.clone(),
                    evidence_description(effect, evidence, plan.target().is_some()).into(),
                ]
            })
            .collect()
    };
    writeln!(
        output,
        "\n{}",
        table::render(
            if store_upgrade {
                &["RETAINED EVIDENCE", "ATTEMPTS"]
            } else {
                &["EFFECT", "RETAINED EVIDENCE"]
            },
            &rows,
            &[
                Alignment::Left,
                if store_upgrade {
                    Alignment::Right
                } else {
                    Alignment::Left
                }
            ],
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
    } else if plan.publication().is_none() {
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

fn render_fleet(state: &canic_host::FleetStatus, journal: &Path) -> Result<String> {
    let mut output = fleet_preview(&state.operation, &state.fleet_plan, journal)?;
    if state.cancellation.is_some() {
        output.push_str("Cancelled before effect preparation. This approval cannot apply.\n");
        return Ok(output);
    }
    let evidence = |step: Option<&CanisterStatus>| -> String {
        let Some(step) = step else {
            return "Not yet admitted".into();
        };
        if step.receipt.is_some() {
            return "Completion receipt retained".into();
        }
        match &step.management {
            Some(ManagementStatus::Creation(s)) => format!("{:?}; no completion receipt", s.phase),
            Some(ManagementStatus::Upgrade(s)) => format!("{:?}; no completion receipt", s.phase),
            None => "No completion receipt; inspect host effects with --json".into(),
        }
    };
    let mut rows = vec![["Root".into(), evidence(Some(&state.root))]];
    if let Some(store) = &state.store {
        rows.push([store.name.clone(), evidence(store.state.as_ref())]);
    }
    for child in &state.children {
        if let Some(publication) = &child.publication {
            rows.push([
                format!("{} release", child.name),
                if matches!(
                    &publication.detail,
                    StatusDetail::Publication {
                        publication: Some(_)
                    }
                ) {
                    "Published release evidence retained"
                } else {
                    "Publication unfinished; resume fleet apply"
                }
                .into(),
            ]);
        }
        rows.push([child.name.clone(), evidence(child.state.as_ref())]);
    }
    output.push_str(&table::render(
        &["STEP", "RETAINED EVIDENCE"],
        &rows,
        &[Alignment::Left; 2],
        Options {
            column_gap: 2,
            header_separator: true,
        },
    ));
    output
        .push_str("\n\nLocal evidence only. Resume this same fleet operation with fleet apply.\n");
    for child in &state.children {
        if let Some(step) = &child.state
            && step.management.is_some()
        {
            writeln!(output, "\nChild: {}", terminal_text(&child.name)).expect("String write");
            output.push_str(&management::render(
                step.management.as_ref(),
                &step.detail,
                step.receipt.is_some(),
            ));
        }
    }
    Ok(output)
}

fn evidence_description<'a>(effect: &str, evidence: &'a str, upgrade: bool) -> &'a str {
    match evidence {
        "not_submitted" if effect == "create" && upgrade => {
            "Not applicable to this upgrade (not submitted)"
        }
        "not_submitted" => "Not submitted",
        "authority_retained_not_signed" => "Recovery request prepared; not signed",
        "signed_attempt_retained_outcome_unknown" => "Signed attempt retained; outcome UNKNOWN",
        "reply_retained" => "Reply retained; not itself proof of success",
        "rejection_retained" => "Rejection retained",
        _ => evidence,
    }
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
    let mut rows = vec![[
        root.canister.clone(),
        "operator".into(),
        format!("{} debit ceiling", cycles(plan.root.creation_cycles())),
    ]];
    let mut modules = format!("Root module: {}\n", plan.root.artifact.sha256);
    if let Some(store) = &plan.store {
        rows.push([
            format!("├─ {} [WASM STORE]", store.deployment.canister),
            format!("{}.result.canister", root.canister),
            format!("{} separate debit ceiling", cycles(store.creation_cycles)),
        ]);
        writeln!(
            modules,
            "{} module: {}",
            terminal_text(&store.deployment.canister),
            store.artifact.sha256
        )
        .expect("String write");
    }
    writeln!(
        modules,
        "Total operator debit ceiling: {}",
        cycles(plan.creation_cycles())
    )
    .expect("String write");
    for (index, child) in plan.children.iter().enumerate() {
        let branch = if index + 1 < plan.children.len() {
            "├─"
        } else {
            "└─"
        };
        rows.push([
            format!("{branch} {}", child.deployment.canister),
            format!("{}.result.canister", root.canister),
            format!("{} from Root", cycles(child.creation_cycles)),
        ]);
        writeln!(
            modules,
            "{} module: {}",
            terminal_text(&child.deployment.canister),
            child.artifact.sha256
        )
        .expect("String write");
        if let Some(release) = &child.release {
            writeln!(
                modules,
                "  Store release: {} / {} (publication before child admission)",
                terminal_text(&release.release.name),
                terminal_text(&release.release.version)
            )
            .expect("String write");
        }
    }
    Ok(format!(
        "CANIC FLEET PLAN\nOperation: {}\nApp: {}\nFleet: {}\nNetwork: {}\nOperator: {}\nJournal: {}\n\n{}\n\n{}Child dispatch: {}\nChild effect deadline: {}\n\n",
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
        modules,
        dispatch(plan.root.created_at_ns, plan.children[0].execute_at_ns),
        deadline(plan.children[0].execute_at_ns + 600_000_000_000)
    ))
}

fn dispatch(created: u64, earliest: u64) -> String {
    if earliest == created {
        "when approved prerequisites are ready".into()
    } else {
        format!("{} (original admission cutoff)", utc(earliest))
    }
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
    format!("{} ({relative})", utc(ns))
}

fn utc(ns: u64) -> String {
    // Every u64 nanosecond timestamp fits within Time's supported calendar range.
    let date = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ns))
        .expect("u64 nanoseconds are within the supported UTC range");
    format!(
        "{} {:02}:{:02}:{:02} UTC",
        date.date(),
        date.hour(),
        date.minute(),
        date.second()
    )
}

fn cycles(value: u64) -> String {
    let (divisor, width, unit) = if value >= 1_000_000_000_000 {
        (1_000_000_000_000, 12, "T")
    } else if value >= 1_000_000_000 {
        (1_000_000_000, 9, "B")
    } else {
        return format!("{value} cycles");
    };
    // Budgets are exact amounts: never round a fractional unit up to more authority.
    let fraction = format!("{:0width$}", value % divisor);
    let fraction = fraction.trim_end_matches('0');
    let whole = value / divisor;
    let amount = if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    };
    format!("{amount}{unit} cycles ({value})")
}

// Select against retained names before making a network call; configuration changes
// cannot redirect a completed operation to another application.
pub(crate) fn call_state<'a>(
    state: &'a OperationStatus,
    selected: Option<&str>,
) -> Result<&'a CanisterStatus> {
    match state {
        OperationStatus::Fleet(fleet) => {
            if let Some(child) = fleet
                .children
                .iter()
                .find(|child| selected.map_or(fleet.children.len() == 1, |name| child.name == name))
            {
                return child.state.as_ref().ok_or_else(|| {
                    Error::Invalid("operation has no completed deployment receipt".into())
                });
            }
            Err(Error::Invalid(format!(
                "select --canister from: {}",
                fleet
                    .children
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
        OperationStatus::Canister(state) => {
            if selected.is_some_and(|name| {
                state
                    .plan
                    .deployment
                    .as_ref()
                    .is_none_or(|d| d.canister != name)
            }) {
                return Err(Error::Invalid(
                    "--canister does not match this retained operation".into(),
                ));
            }
            Ok(state)
        }
    }
}
