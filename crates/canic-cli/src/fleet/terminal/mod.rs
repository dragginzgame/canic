use canic_host::Fleet;
use std::{collections::BTreeMap, fmt::Write as _};

mod roles;
#[cfg(test)]
mod tests;

// Provider diagnostics and journal labels must not control terminal rendering.
fn text(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if character.is_control()
            || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            write!(output, "{}", character.escape_unicode()).expect("writing to String");
        } else {
            output.push(character);
        }
    }
    output
}

fn paint(value: &str, style: &str, color: bool) -> String {
    let value = text(value);
    if color {
        format!("\x1b[{style}m{value}\x1b[0m")
    } else {
        value
    }
}

pub(crate) fn render(snapshot: &Fleet, links: &[String], details: bool, color: bool) -> String {
    let mut output = format!(
        "{}  |  {} canisters  |  {} operations  |  {} inventory issues\nSnapshot: {} Unix seconds. Read-only, sampled observations.\nScope: local journal and explicitly selected observations.\n",
        paint("CANIC FLEET", "1", color),
        snapshot.canisters.len(),
        snapshot.operations.len(),
        snapshot.issues.len(),
        snapshot.finished_at_ns / 1_000_000_000,
    );
    if let Some(config) = &snapshot.configuration {
        writeln!(
            output,
            "\n{} {}\n  {} {}",
            paint("App:", "1;34", color),
            text(&config.app),
            paint("Fleet:", "1", color),
            text(&config.fleet)
        )
        .expect("writing to String");
        writeln!(
            output,
            "  Configured network: {}",
            text(&super::configured_network(config))
        )
        .expect("writing to String");
        if config.config.canisters.is_empty() {
            output.push_str(
                "    No canisters declared yet. Add slots to this fleet before planning.\n",
            );
        }
        for slot in config.config.canisters.keys() {
            writeln!(
                output,
                "    {}: {}",
                text(slot),
                text(&super::slot_status(snapshot, slot))
            )
            .expect("writing to String");
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
        writeln!(
            output,
            "\nNetwork #{}: {}",
            network_numbers[network_id],
            text(&canisters[0].network),
        )
        .expect("writing to String");
        if details {
            writeln!(
                output,
                "Network identity (diagnostic): {}",
                text(network_id)
            )
            .expect("writing to String");
        }
        let mut external = BTreeMap::<&str, usize>::new();
        for target in &canisters {
            if target.observation_error.is_none() {
                for controller in target.controllers.iter().flatten() {
                    if !canisters.iter().any(|item| item.canister == *controller) {
                        *external.entry(controller).or_default() += 1;
                    }
                }
            }
        }
        for (controller, count) in external {
            writeln!(output, "  {} {}\n    Controls {count} known canister(s); principal outside this canister inventory.", paint("[EXTERNAL CONTROLLER]", "1;35", color), text(controller)).expect("writing to String");
        }
        for canister in canisters {
            let observed = canister.observation_error.is_none()
                && canister.module_hash.is_some()
                && canister.controllers.is_some();
            writeln!(
                output,
                "  {} {}  [{}]",
                roles::badges(snapshot, canister, links, color),
                text(&canister.canister),
                paint(
                    if observed {
                        "verified code/controllers"
                    } else {
                        "UNKNOWN metadata"
                    },
                    if observed { "32" } else { "1;33" },
                    color
                )
            )
            .expect("writing to String");
            if let Some(module) = &canister.module_hash
                && details
            {
                writeln!(output, "    Module: {}", text(module)).expect("writing to String");
            }
            if let Some(controllers) = &canister.controllers {
                let controllers = if controllers.is_empty() {
                    "none".into()
                } else {
                    controllers
                        .iter()
                        .map(|value| text(value))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                writeln!(output, "    Controlled by: {controllers}").expect("writing to String");
            }
            if let Some(error) = &canister.observation_error {
                writeln!(output, "    Observation: {}", text(error)).expect("writing to String");
            }
        }
    }
    if snapshot.canisters.is_empty() {
        output
            .push_str("\nNo known canisters. Retained plans may still have unresolved targets.\n");
    }
    output
        .push_str("\nMetadata does not establish readiness, running status or canister cycles.\n");
    output.push_str("Controller badges come from verified relationships. Manager roles are selected; worker roles are reported; entry routes are declared.\n");
    for manager in &snapshot.managers {
        writeln!(
            output,
            "\n{} {} (uncertified report, network #{})",
            paint("MANAGER", "1;36", color),
            text(&manager.manager),
            network_numbers[&manager.network_id]
        )
        .expect("writing to String");
        if let Some(status) = &manager.reported {
            writeln!(
                output,
                "  Phase: {:?}; create/upload/install attempts: {}/{}/{}\n  Worker: {}",
                status.phase,
                status.attempts[0],
                status.attempts[1],
                status.attempts[2],
                text(status.worker.as_deref().unwrap_or("No retained principal"))
            )
            .expect("writing to String");
            if status.phase == canic_host::ProfileGrowthPhase::CreateIssued {
                output.push_str("  Creation unresolved; no second creation is authorized.\n");
            }
            if let Some(failure) = &status.failure {
                writeln!(output, "  Failure: {failure:?}").expect("writing to String");
            }
        } else {
            output.push_str("  UNKNOWN manager state\n");
        }
        if let Some(error) = &manager.observation_error {
            writeln!(output, "  Observation: {}", text(error)).expect("writing to String");
        }
    }
    if let Some(wallet) = &snapshot.wallet {
        writeln!(
            output,
            "\nWallet: {} (uncertified mainnet balances)",
            text(&wallet.principal)
        )
        .expect("writing to String");
        for (label, balance) in [("ICP", &wallet.icp), ("Cycles ledger", &wallet.cycles)] {
            writeln!(
                output,
                "  {label}: {}",
                text(balance.display.as_deref().unwrap_or("UNKNOWN"))
            )
            .expect("writing to String");
            if let Some(error) = &balance.observation_error {
                writeln!(output, "    Observation: {}", text(error)).expect("writing to String");
            }
        }
        writeln!(
            output,
            "  Receive ICP: {}\n  Ledger balances exclude cycles held by canisters.",
            text(&wallet.icp_address)
        )
        .expect("writing to String");
    }
    if !links.is_empty() {
        output.push_str("\nDeclared routes (not verified on-chain):\n");
        for link in links {
            writeln!(output, "  {}", text(link)).expect("writing to String");
        }
    }
    let receipts = snapshot
        .operations
        .iter()
        .filter(|operation| operation.receipt_retained)
        .count();
    writeln!(
        output,
        "\n{receipts} retained deployment receipts (historical evidence)."
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
            text(&operation.operation),
            network_numbers[&operation.network_id],
            text(operation.canister.as_deref().unwrap_or("unresolved")),
            if operation.approval_expired {
                "expired"
            } else {
                "within original deadline"
            }
        )
        .expect("writing to String");
        for (effect, evidence) in &operation.effects {
            writeln!(output, "    {}: {}", text(effect), text(evidence))
                .expect("writing to String");
        }
        writeln!(
            output,
            "    Inspect: {}",
            text(&super::status_command(snapshot, &operation.operation))
        )
        .expect("writing to String");
    }
    if !snapshot.issues.is_empty() {
        output.push_str("\nPARTIAL INVENTORY:\n");
        for issue in &snapshot.issues {
            writeln!(output, "  {}", text(issue)).expect("writing to String");
        }
    }
    output.push_str(
        "\nUse --details for code hashes/network identities, --json for structured evidence,\nor --watch for a localhost live view. Retain any --config/--state-dir path when inspecting status.\n",
    );
    output
}
