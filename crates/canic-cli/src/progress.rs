//! Human command outcomes and explicit, shell-safe continuations.
use crate::args::FleetCommand;
use canic_host::{ApplyOutcome, BuiltCanister};
use ic_host_text::{
    escape::terminal_text,
    table::{self, Alignment, Options},
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

fn outcome(value: &ApplyOutcome, text: &mut String) -> bool {
    match value {
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
        ApplyOutcome::Fleet { root, children, .. } => {
            text.push_str(&format!(
                "Root installed: {}\n",
                terminal_text(&root.canister)
            ));
            let mut complete = true;
            for child in children {
                text.push_str(&format!("Application: {}\n", terminal_text(&child.name)));
                complete &= outcome(&child.outcome, text);
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
    let complete = outcome(value, &mut text);
    text.push_str(&next(
        command,
        digest,
        if complete { "status" } else { "apply" },
        Some(identity),
    ));
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser as _;

    #[test]
    fn follow_up_retains_scope_and_quotes_shell_input() {
        let cli = crate::args::Cli::try_parse_from([
            "canic",
            "fleet",
            "plan",
            "--config",
            "project's config.json",
            "--app",
            "demo",
            "--fleet",
            "mainnet",
            "--state-dir",
            "journal path",
            "--identity",
            "operator",
            "--icp-bin",
            "signer path",
        ])
        .unwrap();
        let crate::args::Commands::Fleet(command) = cli.command else {
            panic!("fleet")
        };
        let output = next(&command, "digest", "apply", Some("operator"));
        assert!(output.contains("--config 'project'\"'\"'s config.json'"));
        assert!(output.contains("--state-dir 'journal path' --app demo --fleet mainnet"));
        assert!(output.contains("--identity operator --icp-bin 'signer path'"));
        assert_eq!(quote("$(touch nope)"), "'$(touch nope)'");
    }

    #[test]
    fn planning_requires_exactly_one_controller_source() {
        let base = ["canic", "fleet", "plan"];
        for args in [
            vec![],
            vec!["--identity", "one", "--controller", "2vxsx-fae"],
            vec!["--controller", "2vxsx-fae", "--icp-bin", "unused"],
        ] {
            assert!(crate::args::Cli::try_parse_from(base.into_iter().chain(args)).is_err());
        }
        for args in [vec!["--identity", "one"], vec!["--controller", "2vxsx-fae"]] {
            assert!(crate::args::Cli::try_parse_from(base.into_iter().chain(args)).is_ok());
        }
    }
}
