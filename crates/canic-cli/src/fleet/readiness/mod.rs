//! Parse and render the effect-free checks available before a Fleet release build.

use crate::{
    cli::{
        clap::{
            parse_matches, render_usage, required_string, string_option, string_option_or_else,
            value_arg,
        },
        defaults::default_icp,
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    fleet::{DEFAULT_CYCLES_LEDGER, FleetCommandError, format_cycles},
    version_text,
};
use candid::Principal;
use canic_core::cdk::types::Cycles;
use canic_host::fleet_ensure::workflow::readiness::{FleetReadinessRequest, inspect};
use canic_host::icp_config::resolve_current_canic_icp_root;
use clap::{ArgAction, Command};
use std::ffi::OsString;

pub(super) fn command() -> Command {
    Command::new("readiness")
        .bin_name("canic fleet readiness")
        .about("Check signer, network, retained work and funding before compiling")
        .disable_help_flag(true)
        .arg(value_arg("fleet").required(true).help("Fleet identity"))
        .arg(value_arg("operator").long("operator").required(true).help("Expected operator Principal"))
        .arg(value_arg("cycles-ledger").long("cycles-ledger").default_value(DEFAULT_CYCLES_LEDGER).help("Exact Cycles Ledger to query"))
        .arg(value_arg("estimated-cycles").long("estimated-cycles").help("Optional operator debit estimate, such as 90T; never spending approval"))
        .arg(value_arg("json").long("json").action(ArgAction::SetTrue).num_args(0))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .arg(super::identity_arg())
        .after_help("Example:\n  canic --environment staging fleet readiness staging --identity staging-operator --operator <principal> --estimated-cycles 90T\n\nNo build or payment occurs. A selected plan and fresh admission are still required.")
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), FleetCommandError> {
    if print_help_or_version(&args, || render_usage(command), version_text()) {
        return Ok(());
    }
    let matches = parse_matches(command(), args)
        .map_err(|error| FleetCommandError::Usage(error.to_string()))?;
    let environment = string_option(&matches, "environment")
        .ok_or_else(|| FleetCommandError::Usage("readiness requires --environment".into()))?;
    let root = resolve_current_canic_icp_root()?;
    let parse_principal = |key: &str| {
        Principal::from_text(required_string(&matches, key))
            .map_err(|_| FleetCommandError::Usage(format!("invalid {key} Principal")))
    };
    let estimated_required_cycles = string_option(&matches, "estimated-cycles")
        .map(|value| {
            value
                .parse::<Cycles>()
                .map(|cycles| cycles.to_u128())
                .map_err(|_| FleetCommandError::Usage("invalid estimated cycle amount".into()))
        })
        .transpose()?;
    let report = inspect(&FleetReadinessRequest {
        workspace: &root,
        environment: &environment,
        fleet: &required_string(&matches, "fleet"),
        icp_executable: &string_option_or_else(&matches, "icp", default_icp),
        signing_identity: string_option(&matches, "identity").as_deref(),
        operator: parse_principal("operator")?,
        cycles_ledger: parse_principal("cycles-ledger")?,
        estimated_required_cycles,
    })
    .map_err(|error| FleetCommandError::Readiness(Box::new(error)))?;
    if matches.get_flag("json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1, "readiness": report, "payment_authorized": false,
                "deployment_authorized": false, "funding_basis": "caller_estimate_not_selected_plan"
            }))?
        );
    } else {
        println!(
            "fleet: {}\nenvironment: {}\noperator: {}\nnetwork: {}\navailable_cycles: {}\nfunding_requirement: {}\nblockers: {:?}",
            report.fleet,
            report.environment,
            report.operator,
            report.network_identity,
            format_cycles(report.available_cycles),
            report.estimated_required_cycles.map_or_else(
                || "unknown until planning".into(),
                |amount| format!("{} (caller estimate)", format_cycles(amount))
            ),
            report.blockers
        );
        if let Some(operation) = &report.retained_operation {
            println!(
                "retained_operation: {} {:?}; plan={}",
                operation.operation_id, operation.completion, operation.plan_sha256
            );
        }
        println!(
            "Read-only snapshot. Resume retained work through Fleet ensure; preserve its plan, journal and selected build. Exact funding and authority are checked again before effects."
        );
    }
    if !report.blockers.is_empty() {
        return Err(FleetCommandError::ReadinessBlocked);
    }
    Ok(())
}
