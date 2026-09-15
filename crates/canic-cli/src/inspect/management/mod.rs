//! Explicit management-status inspection; visibility does not imply control authority.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage},
        globals::{internal_environment_arg, internal_icp_arg},
    },
    inspect::InspectCommandError,
    support::icp_target::IcpTargetOptions,
};
use canic_host::{icp::IcpCanisterStatusReport, icp_config::resolve_current_canic_icp_root};
use clap::{Arg, Command};
use std::ffi::OsString;

pub(super) fn command() -> Command {
    Command::new("management").bin_name("canic inspect management")
        .about("Read management status, visibility and cumulative query counters")
        .arg(Arg::new("canister").required(true).value_name("principal").value_parser(clap::value_parser!(candid::Principal)))
        .arg(internal_environment_arg()).arg(internal_icp_arg())
        .arg(flag_arg("json").long("json").help("Print structured status"))
        .after_help("ICP performs a management status update, which may incur network charges. Visibility is reported without changing settings.\n\nExample:\n  canic --environment ic inspect management <principal> --json")
}

pub(super) fn usage() -> String {
    render_usage(command)
}

pub(super) fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), InspectCommandError> {
    let matches =
        parse_matches(command(), args).map_err(|_| InspectCommandError::Usage(usage()))?;
    let target = IcpTargetOptions::parse(&matches);
    let canister = *matches
        .get_one::<candid::Principal>("canister")
        .expect("required Principal");
    if canister == candid::Principal::anonymous()
        || canister == candid::Principal::management_canister()
    {
        return Err(InspectCommandError::Target(
            "management inspection requires a concrete canister".into(),
        ));
    }
    let root = resolve_current_canic_icp_root().map_err(InspectCommandError::IcpRoot)?;
    let report = target
        .icp_cli(&root)
        .canister_status_report(&canister.to_text())?;
    validate_report(&canister.to_text(), &report)?;
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_report(&report)?);
    }
    Ok(())
}

fn validate_report(
    expected: &str,
    report: &IcpCanisterStatusReport,
) -> Result<(), InspectCommandError> {
    if report.id != expected {
        return Err(InspectCommandError::Target(
            "management status returned another canister".into(),
        ));
    }
    Ok(())
}

fn render_report(report: &IcpCanisterStatusReport) -> Result<String, InspectCommandError> {
    let mut lines = vec![
        format!("Canister: {}", report.id),
        format!(
            "Status: {}",
            report.status.as_deref().unwrap_or("unavailable")
        ),
    ];
    if let Some(settings) = &report.settings {
        lines.push(format!("Controllers: {}", settings.controllers.join(", ")));
        for (name, visibility) in [
            ("Logs", &settings.log_visibility),
            ("Snapshots", &settings.snapshot_visibility),
            ("Status", &settings.status_visibility),
        ] {
            lines.push(format!(
                "{name} visibility: {}",
                visibility
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?
                    .unwrap_or_else(|| "unavailable".into())
            ));
        }
    } else if let Some(controllers) = &report.public_controllers {
        lines.push(format!(
            "Public controller list: {}",
            controllers.join(", ")
        ));
    }
    lines.push(format!(
        "Native cycles: {}",
        report.cycles.as_deref().unwrap_or("unavailable")
    ));
    if let Some(stats) = &report.query_stats {
        lines.push(format!(
            "Queries: {}; instructions: {}; request/response bytes: {}/{}",
            stats.num_calls_total,
            stats.num_instructions_total,
            stats.request_payload_bytes_total,
            stats.response_payload_bytes_total
        ));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_status_stays_unavailable_and_wrong_target_rejects() {
        let report: IcpCanisterStatusReport = serde_json::from_str(
            r#"{"id":"rrkah-fqaaa-aaaaa-aaaaq-cai","controllers":["2vxsx-fae"]}"#,
        )
        .unwrap();
        validate_report(&report.id, &report).unwrap();
        assert!(matches!(
            validate_report("ryjl3-tyaaa-aaaaa-aaaba-cai", &report),
            Err(InspectCommandError::Target(_))
        ));
        let rendered = render_report(&report).unwrap();
        assert!(rendered.contains("Status: unavailable"));
        assert!(rendered.contains("Native cycles: unavailable"));
    }
}
