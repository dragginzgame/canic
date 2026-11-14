use canic_host::Fleet;
use ic_host_text::escape::terminal_text;
use std::{collections::BTreeMap, fmt::Write as _};

mod inventory;
mod roles;
#[cfg(test)]
mod tests;

fn paint(value: &str, style: &str, color: bool) -> String {
    let value = terminal_text(value);
    if color {
        format!("\x1b[{style}m{value}\x1b[0m")
    } else {
        value.into_owned()
    }
}

pub(crate) fn render(snapshot: &Fleet, links: &[String], details: bool, color: bool) -> String {
    let mut output = if let Some(config) = &snapshot.configuration {
        format!(
            "{} / {} · {}\n",
            paint(&config.app, "1", color),
            terminal_text(&config.fleet),
            terminal_text(&super::configured_network(config))
        )
    } else {
        "Canic fleet\n".into()
    };
    if let Some(config) = &snapshot.configuration {
        if config.config.canisters.is_empty() {
            output.push_str("No canisters declared yet.\n");
        }
        for slot in config.config.canisters.keys() {
            if !snapshot.canisters.iter().any(|target| {
                config.config.network.binding_id().ok().as_deref() == Some(&target.network_id)
                    && super::slot_state(snapshot, slot).identity()
                        == Some(target.canister.as_str())
            }) {
                writeln!(
                    output,
                    "  {}: {}",
                    terminal_text(slot),
                    terminal_text(&super::slot_status(snapshot, slot))
                )
                .expect("writing to String");
            }
        }
    }
    let mut networks = BTreeMap::<_, Vec<_>>::new();
    for canister in &snapshot.canisters {
        networks
            .entry(&canister.network_id)
            .or_default()
            .push(canister);
    }
    // Local display numbers keep equal URLs with different trust roots separate.
    let network_numbers: BTreeMap<_, _> = snapshot
        .canisters
        .iter()
        .map(|item| &item.network_id)
        .chain(snapshot.operations.iter().map(|item| &item.network_id))
        .chain(snapshot.managers.iter().map(|item| &item.network_id))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(index, id)| (id, index + 1))
        .collect();
    for (network_id, canisters) in networks {
        if snapshot.configuration.as_ref().is_none_or(|config| {
            network_numbers.len() != 1
                || config.config.network.binding_id().ok().as_deref() != Some(network_id)
        }) {
            writeln!(
                output,
                "\nNetwork #{}: {}",
                network_numbers[network_id],
                terminal_text(&canisters[0].network),
            )
            .expect("writing to String");
        } else {
            output.push('\n');
        }
        if details {
            writeln!(
                output,
                "Network identity (diagnostic): {}",
                terminal_text(network_id)
            )
            .expect("writing to String");
        }
        output.push_str(&inventory::render(
            snapshot, &canisters, links, details, color,
        ));
    }
    if snapshot.canisters.is_empty() {
        output.push_str("\nNo known canisters.\n");
    }
    if details {
        writeln!(output, "\nSnapshot: {} Unix seconds. Read-only sampled observations.\nCode/control verification does not establish readiness, running status or canister cycles.\nTree and explicit roles are declared; fallback roles come from manager reports or routes. Controller relationships are observed separately.", snapshot.finished_at_ns / 1_000_000_000).expect("writing to String");
    }
    for manager in &snapshot.managers {
        let name = roles::name(snapshot, &manager.network_id, &manager.manager)
            .unwrap_or_else(|| manager.manager.clone());
        write!(
            output,
            "\n{} report (uncertified): {}",
            super::manager_kind(snapshot, manager),
            terminal_text(&name)
        )
        .expect("writing to String");
        if network_numbers.len() > 1 {
            write!(
                output,
                " (network #{})",
                network_numbers[&manager.network_id]
            )
            .expect("writing to String");
        }
        if let Some(status) = &manager.reported {
            writeln!(output, " — {:?}", status.phase).expect("writing to String");
            if details {
                writeln!(
                    output,
                    "  Create/upload/install attempts: {}/{}/{}; child: {}",
                    status.attempts[0],
                    status.attempts[1],
                    status.attempts[2],
                    terminal_text(status.worker.as_deref().unwrap_or("unresolved"))
                )
                .expect("writing to String");
            }
            if status.phase == canic_host::ProfileGrowthPhase::CreateIssued {
                output.push_str("  Creation unresolved; no second creation is authorized.\n");
            }
            if let Some(failure) = &status.failure {
                writeln!(output, "  Failure: {failure:?}").expect("writing to String");
            }
        } else {
            output.push_str(" — UNKNOWN\n");
        }
        if let Some(error) = &manager.observation_error {
            writeln!(output, "  Observation: {}", terminal_text(error)).expect("writing to String");
        }
    }
    if let Some(wallet) = &snapshot.wallet {
        writeln!(
            output,
            "\nWallet: {} (uncertified mainnet balances)",
            terminal_text(&wallet.principal)
        )
        .expect("writing to String");
        for (label, balance) in [("ICP", &wallet.icp), ("Cycles ledger", &wallet.cycles)] {
            writeln!(
                output,
                "  {label}: {}",
                terminal_text(balance.display.as_deref().unwrap_or("UNKNOWN"))
            )
            .expect("writing to String");
            if let Some(error) = &balance.observation_error {
                writeln!(output, "    Observation: {}", terminal_text(error))
                    .expect("writing to String");
            }
        }
        writeln!(
            output,
            "  Receive ICP: {}\n  Ledger balances exclude cycles held by canisters.",
            terminal_text(&wallet.icp_address)
        )
        .expect("writing to String");
    }
    if !links.is_empty() {
        output.push_str("\nDeclared routes (not verified on-chain):\n");
        for link in links {
            writeln!(output, "  {}", terminal_text(link)).expect("writing to String");
        }
    }
    let receipts = snapshot
        .operations
        .iter()
        .filter(|operation| operation.receipt_retained)
        .count();
    writeln!(
        output,
        "\n{} canisters · {receipts} retained receipts",
        snapshot.canisters.len()
    )
    .expect("writing to String");
    for operation in snapshot
        .operations
        .iter()
        .filter(|operation| !operation.receipt_retained)
    {
        writeln!(
            output,
            "  No terminal receipt: {}\n    Network #{}; target: {}; approval {}",
            terminal_text(&operation.operation),
            network_numbers[&operation.network_id],
            terminal_text(operation.canister.as_deref().unwrap_or("unresolved")),
            if operation.approval_expired {
                "expired"
            } else {
                "within original deadline"
            }
        )
        .expect("writing to String");
        for (effect, evidence) in &operation.effects {
            writeln!(
                output,
                "    {}: {}",
                terminal_text(effect),
                terminal_text(evidence)
            )
            .expect("writing to String");
        }
        if details {
            writeln!(
                output,
                "    Inspect: {}",
                terminal_text(&super::status_command(snapshot, &operation.operation))
            )
            .expect("writing to String");
        }
    }
    if !snapshot.issues.is_empty() {
        output.push_str("\nPARTIAL INVENTORY:\n");
        for issue in &snapshot.issues {
            writeln!(output, "  {}", terminal_text(issue)).expect("writing to String");
        }
    }

    output
}
