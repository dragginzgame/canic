//! Operator snapshot and curated downstream report adapter.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, required_string, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_nested_help,
    },
    support::icp_target::IcpTargetOptions,
};
use canic_host::{
    icp_config::resolve_current_canic_icp_root,
    observatory::{
        model::{ObservatoryOptions, ObservatoryProfile},
        ops::{self, presentation},
        workflow,
    },
};
use clap::Command;
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::{Read, Write},
};

/// Adapter errors preserve host diagnostics and local publication failures.
#[derive(Debug, thiserror::Error)]
pub enum ObservatoryCommandError {
    #[error(transparent)]
    Clap(#[from] clap::Error),
    #[error(transparent)]
    Host(#[from] canic_host::observatory::ObservatoryError),
    #[error(transparent)]
    Root(#[from] canic_host::icp_config::IcpConfigError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn command() -> Command {
    Command::new("observatory").bin_name("canic observatory")
        .about("Collect Fleet observations, compare saved costs and render public reports")
        .arg(internal_environment_arg().global(true)).arg(internal_icp_arg().global(true))
        .subcommand_required(true)
        .subcommand(Command::new("compare").about("Compare two saved private cost snapshots without remote calls")
            .arg(value_arg("before").required(true))
            .arg(value_arg("after").required(true))
            .arg(value_arg("maximum-snapshot-bytes").long("maximum-snapshot-bytes").default_value("1048576").value_parser(clap::value_parser!(u32).range(1024..=16_777_216)))
            .arg(value_arg("maximum-report-bytes").long("maximum-report-bytes").default_value("1048576").value_parser(clap::value_parser!(u32).range(1024..=16_777_216)))
            .arg(value_arg("out").long("out").help("Create a new private comparison file; otherwise write stdout"))
            .after_help("Example:\n  canic observatory compare before.json after.json --out comparison.json"))
        .subcommand(Command::new("snapshot").about("Query one terminal Fleet once; preserve independent failures")
            .arg(value_arg("fleet").required(true))
            .arg(flag_arg("costs").long("costs").conflicts_with("public").help("Include cached timer, balance and grant evidence in the private report"))
            .arg(value_arg("freshness-secs").long("freshness-secs").default_value("30").value_parser(clap::value_parser!(u32).range(1..=3600)))
            .arg(value_arg("maximum-collection-secs").long("maximum-collection-secs").default_value("60").value_parser(clap::value_parser!(u32).range(1..=3600)))
            .arg(value_arg("maximum-canisters").long("maximum-canisters").default_value("128").value_parser(clap::value_parser!(u32).range(1..=4096)))
            .arg(value_arg("maximum-response-bytes").long("maximum-response-bytes").default_value("1048576").value_parser(clap::value_parser!(u32).range(1024..=16_777_216)))
            .arg(value_arg("maximum-report-bytes").long("maximum-report-bytes").default_value("1048576").value_parser(clap::value_parser!(u32).range(1024..=16_777_216)))
            .arg(value_arg("query-timeout-secs").long("query-timeout-secs").default_value("5").value_parser(clap::value_parser!(u32).range(1..=60)))
            .arg(value_arg("profile").long("profile").help("Data-only public labels JSON; required for --public"))
            .arg(flag_arg("public").long("public").requires("profile").help("Omit exact authority, identities, placement and funding"))
            .arg(flag_arg("html").long("html").requires("public").help("Render escaped public HTML instead of JSON"))
            .arg(value_arg("out").long("out").help("Create a new report file; otherwise write stdout")))
        .after_help("Examples:\n  canic observatory compare before.json after.json\n  canic observatory snapshot demo\n  canic observatory snapshot demo --public --profile observatory.json --html --out report.html")
}

/// Collect a snapshot or compare saved evidence and emit the selected report surface.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), ObservatoryCommandError> {
    let args = args.into_iter().collect::<Vec<_>>();
    if print_nested_help(&args, command()) {
        return Ok(());
    }
    let matches = parse_matches(command(), args)?;
    let (name, leaf) = matches.subcommand().expect("required observatory command");
    if name == "compare" {
        let before = required_string(leaf, "before");
        let after = required_string(leaf, "after");
        let report = workflow::compare_files(
            std::path::Path::new(&before),
            std::path::Path::new(&after),
            *leaf.get_one::<u32>("maximum-snapshot-bytes").unwrap() as usize,
        )?;
        let maximum = *leaf.get_one::<u32>("maximum-report-bytes").unwrap() as usize;
        return emit_report(leaf, &presentation::json_bytes(&report, maximum)?);
    }
    let target = IcpTargetOptions::parse(&matches);
    let options = ObservatoryOptions {
        environment: target.environment.clone(),
        fleet: required_string(leaf, "fleet"),
        collect_costs: leaf.get_flag("costs"),
        freshness_secs: *leaf.get_one::<u32>("freshness-secs").unwrap(),
        maximum_canisters: *leaf.get_one::<u32>("maximum-canisters").unwrap() as usize,
        maximum_response_bytes: *leaf.get_one::<u32>("maximum-response-bytes").unwrap() as usize,
        maximum_collection_secs: *leaf.get_one::<u32>("maximum-collection-secs").unwrap(),
        query_timeout_secs: *leaf.get_one::<u32>("query-timeout-secs").unwrap(),
    };
    let maximum = *leaf.get_one::<u32>("maximum-report-bytes").unwrap() as usize;
    // Validate the profile before any remote observation.
    let profile = leaf
        .get_one::<String>("profile")
        .map(|path| read_profile(path))
        .transpose()?;
    let root = resolve_current_canic_icp_root()?;
    let snapshot = workflow::snapshot(&root, &target.icp_cli(&root), &options)?;
    let bytes = if leaf.get_flag("public") {
        presentation::http_response(
            &snapshot,
            profile.as_ref().expect("required public profile"),
            if leaf.get_flag("html") {
                "/"
            } else {
                "/snapshot.json"
            },
            ops::now_ms(),
            maximum,
        )?
        .body
    } else {
        presentation::json_bytes(&snapshot, maximum)?
    };
    emit_report(leaf, &bytes)
}

fn emit_report(leaf: &clap::ArgMatches, bytes: &[u8]) -> Result<(), ObservatoryCommandError> {
    if let Some(path) = leaf.get_one::<String>("out") {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    } else {
        std::io::stdout().lock().write_all(bytes)?;
        println!();
    }
    Ok(())
}

fn read_profile(path: &str) -> Result<ObservatoryProfile, ObservatoryCommandError> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(canic_host::observatory::ObservatoryError::Bound("profile bytes").into());
    }
    let profile = serde_json::from_slice(&bytes)?;
    canic_host::observatory::policy::validate_profile(&profile)?;
    Ok(profile)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn comparison_snapshot(at: u64) -> serde_json::Value {
        let unavailable = serde_json::json!({
            "state": "unavailable", "observed_at_unix_ms": at, "failure": {"kind": "unsupported"}
        });
        let timer_instructions = serde_json::json!({
            "state": "observed", "observed_at_unix_ms": at,
            "source": "public_metric_cache", "value": {
                "state": "fresh", "sampled_at_ns": at, "stale_after_ns": 500, "truncated": false,
                "rows": [{"name": "perf.timer.app.jobs.tick.work", "canister_id": null,
                    "value": at.to_string(), "unit": "instructions", "observed_at_ns": at,
                    "measurement": {"kind": "timer_counter", "registration": {
                        "canister_version": 1, "started_at_ns": 40, "sequence": 3
                    }, "saturated": false}}]
            }
        });
        let costs = serde_json::json!({
            "balance": unavailable, "funding_and_callbacks": unavailable,
            "timer_instructions": timer_instructions,
            "window": {"state": "observed", "observed_at_unix_ms": at,
                "source": "public_metric_cache", "value": {"canister_version": 1, "heap_started_at_ns": 50}},
            "limitations": []
        });
        serde_json::json!({
            "schema_version": 1, "environment": "local", "fleet": "demo",
            "collected_at_unix_ms": at, "freshness_secs": 30,
            "collection_elapsed_ms": 0, "remote_call_attempts": 0,
            "authority": {"state": "observed", "observed_at_unix_ms": at,
                "source": "retained_terminal_review", "value": {
                    "app": "app", "canonical_network_id": "network", "plan_sha256": "b".repeat(64),
                    "registry_revision": 1, "admission_principals": 1
                }},
            "operation": unavailable,
            "roles": [{"role": "root", "canister_id": candid::Principal::from_slice(&[1]).to_text(),
                "parent_canister_id": null, "subnet_id": null, "release_identity": "release-a",
                "expected_module_sha256": "a".repeat(64), "overview": unavailable,
                "funding": unavailable, "estate": unavailable, "store": unavailable, "costs": costs}]
        })
    }

    #[test]
    fn comparison_runs_offline_and_creates_private_output_without_overwriting() {
        let dir = crate::test_support::TempDir::new("canic-cli-cost-comparison");
        std::fs::create_dir_all(&dir).unwrap();
        let before = dir.join("before.json");
        let after = dir.join("after.json");
        let output = dir.join("comparison.json");
        std::fs::write(
            &before,
            serde_json::to_vec(&comparison_snapshot(100)).unwrap(),
        )
        .unwrap();
        std::fs::write(
            &after,
            serde_json::to_vec(&comparison_snapshot(200)).unwrap(),
        )
        .unwrap();
        let arguments = vec![
            OsString::from("compare"),
            before.into_os_string(),
            after.into_os_string(),
            OsString::from("--out"),
            output.clone().into_os_string(),
            OsString::from("--__canic-icp"),
            OsString::from("/nonexistent-canic-comparison-icp"),
        ];
        run(arguments.clone()).unwrap();
        let bytes = std::fs::read(&output).unwrap();
        let report: canic_host::observatory::view::ObservatoryComparisonView =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report.before_collected_at_unix_ms, 100);
        assert!(matches!(
            report.roles[0].balance_change,
            canic_host::observatory::view::CostComparisonResult::Unavailable {
                reason: canic_host::observatory::view::CostComparisonFailure::SnapshotUnavailable
            }
        ));
        let canic_host::observatory::view::CostComparisonResult::Available { value: timers } =
            &report.roles[0].timer_measurements
        else {
            panic!("timer evidence must survive offline JSON");
        };
        let canic_host::observatory::view::CostComparisonResult::Available { value } =
            &timers[0].movement
        else {
            panic!("timer delta must be available");
        };
        assert_eq!(value.amount, "100");
        assert_eq!(value.unit, "instructions");
        assert_eq!(value.registration.sequence, 3);
        assert!(
            matches!(run(arguments), Err(ObservatoryCommandError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists)
        );
        assert_eq!(std::fs::read(&output).unwrap(), bytes);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(output).unwrap().permissions().mode() & 0o077,
                0
            );
        }
    }

    #[test]
    fn cost_evidence_is_an_explicit_private_collection() {
        let args = command()
            .try_get_matches_from(["observatory", "snapshot", "demo", "--costs"])
            .unwrap();
        assert!(args.subcommand().unwrap().1.get_flag("costs"));
        let error = command()
            .try_get_matches_from([
                "observatory",
                "snapshot",
                "demo",
                "--costs",
                "--public",
                "--profile",
                "profile.json",
            ])
            .unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
