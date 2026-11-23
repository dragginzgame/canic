//! Human command outcomes and explicit, shell-safe continuations.

#[cfg(test)]
mod tests;

use crate::args::FleetCommand;
use canic_host::{ApplyOutcome, BuiltCanister};
use ic_host_text::{Alignment, escape, plain_table};
use std::{
    fmt::Write as _,
    time::{SystemTime, UNIX_EPOCH},
};

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
    if matches!(action, "apply" | "reconcile" | "activate") {
        parts.extend(["--identity".into(), quote(identity.unwrap_or("IDENTITY"))]);
        if let FleetCommand::Plan { signer, .. }
        | FleetCommand::Apply { signer, .. }
        | FleetCommand::Reconcile { signer, .. }
        | FleetCommand::Activate { signer, .. } = command
            && signer.icp_bin != crate::args::default_signer()
        {
            parts.extend(["--icp-bin".into(), quote(&signer.icp_bin.to_string_lossy())]);
        }
    }
    format!("\nNext:\n{}\n", escape(&parts.join(" ")))
}

fn build_row(mut label: String, item: &BuiltCanister) -> [String; 5] {
    if item.role.as_deref() == Some("wasm-store") {
        label.push_str(" [WASM STORE]");
    }

    [
        label,
        item.wasm.display().to_string(),
        format!(
            "{:.2} KiB",
            f64::from(u32::try_from(item.artifact.bytes).expect("bounded Wasm size")) / 1024.0
        ),
        if item.modules.is_empty() {
            "—".into()
        } else {
            item.modules.join(", ")
        },
        format!(
            "{}.{:02} s",
            item.build_ms / 1000,
            item.build_ms % 1000 / 10
        ),
    ]
}

pub(crate) fn built(built: &[BuiltCanister]) -> String {
    use Alignment::{Left, Right};

    let ordered = crate::topology::labeled(built, |item| {
        (item.canister.as_str(), item.parent.as_deref())
    });
    let table = if built.iter().any(|item| item.client.is_some()) {
        let rows: Vec<_> = ordered
            .into_iter()
            .map(|(label, item)| {
                let [canister, wasm, size, modules, time] = build_row(label, item);
                let codec = item
                    .client
                    .as_ref()
                    .map_or_else(|| "—".into(), |path| path.display().to_string());
                [canister, wasm, size, modules, time, codec]
            })
            .collect();
        plain_table(
            &[
                "CANISTER",
                "WASM",
                "SIZE",
                "MODULES",
                "BUILD TIME",
                "COMMAND CODEC",
            ],
            &rows,
            &[Left, Left, Right, Left, Right, Left],
        )
    } else {
        let rows: Vec<_> = ordered
            .into_iter()
            .map(|(label, item)| build_row(label, item))
            .collect();
        plain_table(
            &["CANISTER", "WASM", "SIZE", "MODULES", "BUILD TIME"],
            &rows,
            &[Left, Left, Right, Left, Right],
        )
    };
    format!("Built app canisters\n\n{table}\n\nNothing deployed.\n")
}

fn outcome(value: &ApplyOutcome, text: &mut String, now: Option<u128>) -> bool {
    match value {
        ApplyOutcome::Call { call } => {
            writeln!(
                text,
                "Application reply retained: {} ({} bytes).",
                call.operation,
                call.bytes.len()
            )
            .expect("String write");
            true
        }
        ApplyOutcome::Auth { auth } => {
            writeln!(
                text,
                "Auth {} reply retained; receiver readiness is not established.",
                if auth.revoked {
                    "revocation"
                } else {
                    "configuration"
                }
            )
            .expect("String write");
            true
        }
        ApplyOutcome::Removal { removal } => {
            writeln!(text, "Removal result retained: {} / {}\nPreserve this journal; restoration needs a separate plan after the original dispatch deadline.",
                escape(&removal.manifest.release.name), escape(&removal.manifest.release.version)).expect("String write");
            true
        }
        ApplyOutcome::StorePending {
            root,
            store_maintenance,
            ..
        } => {
            text.push_str(&format!("Root: {}\n", escape(root),));
            text.push_str(&crate::status::store::render(store_maintenance, now, false));
            false
        }
        ApplyOutcome::Publication { publication } => {
            let published = publication.status.as_ref().is_some_and(|s| s.published);
            text.push_str(&format!(
                "Store: {}\nPublication: {}\n",
                escape(&publication.store),
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
            text.push_str(&format!("Root installed: {}\n", escape(&root.canister)));
            if let Some(store) = store {
                text.push_str(&format!(
                    "Store ready: {} (controlled by Root)\n",
                    escape(&store.canister)
                ));
            }
            let mut complete = true;
            for child in children {
                text.push_str(&format!("Application: {}\n", escape(&child.name)));
                complete &= outcome(&child.outcome, text, now);
            }
            complete
        }
        ApplyOutcome::Complete(receipt) => {
            text.push_str(&format!(
                "Canister installed: {}\nVerified completion receipt retained.\n",
                escape(&receipt.canister)
            ));
            true
        }
        ApplyOutcome::Pending { root, status, .. } => {
            text.push_str(&format!("Root: {}\nApplication: {:?}; completion not yet retained.\nResume this operation after dispatch to retain its outcome.\n", escape(root), status.phase));
            false
        }
        ApplyOutcome::UpgradePending {
            root,
            upgrade,
            maintenance,
            ..
        } => {
            text.push_str(&format!(
                "Root: {}\nUpgrade: {:?}; completion not yet retained.\n",
                escape(root),
                upgrade.phase
            ));
            if let Some(maintenance) = maintenance {
                text.push_str(&format!(
                    "Blob attempt {}: {:?}. ",
                    maintenance.attempt, maintenance.phase
                ));
            }
            if maintenance.as_ref().is_some_and(|m| {
                m.phase == canic_management_protocol::application::MaintenancePhase::Inspected
            }) {
                text.push_str(
                    "Reopened with admission closed; activation requires separate approval.",
                );
            } else {
                text.push_str(crate::status::management::upgrade_continuation(upgrade, false).1);
            }
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
    }
    | FleetCommand::Activate {
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
        ApplyOutcome::UpgradePending { maintenance, .. }
            if maintenance.as_ref().is_some_and(|m| {
                m.phase == canic_management_protocol::application::MaintenancePhase::Inspected
            }) =>
        {
            "activate"
        }
        ApplyOutcome::UpgradePending { upgrade, .. } => {
            crate::status::management::upgrade_continuation(upgrade, false).0
        }
        _ if complete => "status",
        _ => "apply",
    };
    text.push_str(&next(command, digest, action, Some(identity)));
    text
}
