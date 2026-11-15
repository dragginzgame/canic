//! Human command outcomes and explicit, shell-safe continuations.
use crate::args::FleetCommand;
use canic_host::{ApplyOutcome, BuiltCanister};
use ic_host_text::{
    escape::terminal_text,
    table::{self, Alignment, Options},
};
use std::time::{SystemTime, UNIX_EPOCH};

fn quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_/.:".contains(&b))
    {
        value.into()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

pub(crate) fn next(
    command: &FleetCommand,
    digest: &str,
    action: &str,
    identity: Option<&str>,
) -> String {
    let mut parts = vec!["canic".into(), "fleet".into(), action.into(), quote(digest)];
    if let Some(scope) = command.scope() {
        if scope.no_config {
            parts.push("--no-config".into());
        }
        for (flag, value) in [
            ("--config", scope.config.as_deref()),
            ("--state-dir", scope.state_dir.as_deref()),
        ] {
            if let Some(value) = value {
                parts.extend([flag.into(), quote(&value.to_string_lossy())]);
            }
        }
        for (flag, value) in [
            ("--app", scope.app.as_deref()),
            ("--fleet", scope.fleet.as_deref()),
        ] {
            if let Some(value) = value {
                parts.extend([flag.into(), quote(value)]);
            }
        }
    }
    if matches!(action, "apply" | "reconcile") {
        parts.extend(["--identity".into(), quote(identity.unwrap_or("IDENTITY"))]);
        if let FleetCommand::Plan { signer, .. }
        | FleetCommand::Apply { signer, .. }
        | FleetCommand::Reconcile { signer, .. } = command
            && signer.icp_bin != crate::args::checkout().join(".tools/ic/bin/icp")
        {
            parts.extend(["--icp-bin".into(), quote(&signer.icp_bin.to_string_lossy())]);
        }
    }
    format!("\nNext:\n{}\n", terminal_text(&parts.join(" ")))
}

pub(crate) fn built(built: &[BuiltCanister]) -> String {
    let rows: Vec<_> = built
        .iter()
        .map(|item| {
            [
                item.canister.clone(),
                item.wasm.display().to_string(),
                item.artifact.bytes.to_string(),
            ]
        })
        .collect();
    format!(
        "Built app canisters\n\n{}\n\nNothing deployed.\n",
        table::render(
            &["CANISTER", "WASM", "BYTES"],
            &rows,
            &[Alignment::Left, Alignment::Left, Alignment::Right],
            Options {
                column_gap: 2,
                header_separator: true
            }
        )
    )
}

fn outcome(value: &ApplyOutcome, text: &mut String, now: Option<u128>) -> bool {
    match value {
        ApplyOutcome::StorePending {
            root,
            store_maintenance,
            ..
        } => {
            text.push_str(&format!("Root: {}\n", terminal_text(root),));
            text.push_str(&crate::status::store::render(store_maintenance, now, false));
            false
        }
        ApplyOutcome::Publication { publication } => {
            let published = publication.status.as_ref().is_some_and(|s| s.published);
            text.push_str(&format!(
                "Store: {}\nPublication: {}\n",
                terminal_text(&publication.store),
                if published {
                    "published; receipt retained"
                } else {
                    "incomplete; retained observation only"
                }
            ));
            published
        }
        ApplyOutcome::Fleet {
            root,
            store,
            children,
            ..
        } => {
            text.push_str(&format!(
                "Root installed: {}\n",
                terminal_text(&root.canister)
            ));
            if let Some(store) = store {
                text.push_str(&format!(
                    "Store ready: {} (controlled by Root)\n",
                    terminal_text(&store.canister)
                ));
            }
            let mut complete = true;
            for child in children {
                text.push_str(&format!("Application: {}\n", terminal_text(&child.name)));
                complete &= outcome(&child.outcome, text, now);
            }
            complete
        }
        ApplyOutcome::Complete(receipt) => {
            text.push_str(&format!(
                "Canister installed: {}\nVerified completion receipt retained.\n",
                terminal_text(&receipt.canister)
            ));
            true
        }
        ApplyOutcome::Pending { root, status, .. } => {
            text.push_str(&format!("Root: {}\nApplication: {:?}; completion not yet retained.\nResume this operation after dispatch to retain its outcome.\n", terminal_text(root), status.phase));
            false
        }
        ApplyOutcome::UpgradePending { root, upgrade, .. } => {
            text.push_str(&format!(
                "Root: {}\nUpgrade: {:?}; completion not yet retained.\n",
                terminal_text(root),
                upgrade.phase
            ));
            text.push_str(crate::status::management::upgrade_continuation(upgrade, false).1);
            text.push('\n');
            false
        }
    }
}

pub(crate) fn applied(value: &ApplyOutcome, command: &FleetCommand) -> String {
    let (FleetCommand::Apply {
        digest, identity, ..
    }
    | FleetCommand::Reconcile {
        digest, identity, ..
    }) = command
    else {
        unreachable!("apply renderer")
    };
    let mut text = format!("Fleet operation {digest}\n\n");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|time| time.as_nanos());
    let complete = outcome(value, &mut text, now);
    let action = match value {
        ApplyOutcome::StorePending {
            store_maintenance, ..
        } => crate::status::store::continuation(store_maintenance, now).0,
        ApplyOutcome::UpgradePending { upgrade, .. } => {
            crate::status::management::upgrade_continuation(upgrade, false).0
        }
        _ if complete => "status",
        _ => "apply",
    };
    text.push_str(&next(command, digest, action, Some(identity)));
    text
}

#[cfg(test)]
mod tests;
