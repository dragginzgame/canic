//! Operator review and digest-approved execution of exact-ID capacity enrollment.

#[cfg(test)]
mod tests;

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
    fleet::{FleetCommandError, identity_arg, parse_digest, quote_review_argument},
    version_text,
};
use candid::Principal;
use canic_core::cdk::{
    types::Cycles,
    utils::hash::{decode_hex, hex_bytes},
};
use canic_host::{
    fleet_ensure::{
        dto::capacity_import::{CapacityImportFundingCreditRequest, CapacityImportReviewRequest},
        model::capacity_import::CapacityImportJournalRecord,
        ops::capacity_import::{
            admission::survey::MAXIMUM_ATTEMPTS, journal::CapacityImportJournalError, publication,
        },
        workflow::capacity_import::review,
    },
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
};
use clap::{ArgAction, ArgMatches, Command};
use std::{ffi::OsString, path::PathBuf};

pub(super) fn command() -> Command {
    let mut command = Command::new("import").bin_name("canic fleet import")
        .about("Review or apply supplied capacity into initialized current Fleet infrastructure")
        .disable_help_flag(true)
        .arg(value_arg("fleet").required(true).help("Fleet identity"))
        .arg(value_arg("apply").long("apply").value_name("REVIEW_SHA256").value_parser(parse_digest)
            .help("Resume the exact retained review; completed replay is local"))
        .arg(value_arg("canister").long("canister").value_name("PRINCIPAL").action(ArgAction::Append)
            .value_parser(clap::value_parser!(Principal)).required_unless_present("apply").conflicts_with("apply")
            .help("Exact supplied canister; repeat for every source"))
        .arg(value_arg("funding-credit").long("funding-credit").value_name("CANISTER=CYCLES").action(ArgAction::Append)
            .value_parser(parse_funding_credit).conflicts_with("apply")
            .help("Review an already received credit against original observations; repeat per source or Root"))
        .arg(value_arg("root").long("root").value_name("PRINCIPAL")
            .value_parser(clap::value_parser!(Principal)).conflicts_with("apply")
            .help("Select a destination Root explicitly; otherwise require a unique Root on the source subnet"))
        .arg(value_arg("source").long("source").value_name("PATH").conflicts_with("apply")
            .help("Fleet policy; defaults to deployments/<fleet>.toml"))
        .arg(value_arg("seed").long("seed").value_name("PATH").conflicts_with("apply")
            .help("Retained estate seed; defaults to deployments/<fleet>.estate.toml"))
        .arg(value_arg("json").long("json").action(ArgAction::SetTrue).num_args(0)
            .help("Print complete review, journal and next command as JSON"))
        .arg(internal_environment_arg()).arg(internal_icp_arg()).arg(identity_arg());
    for (name, label, help) in [
        (
            "declarations",
            "PATH",
            "Required TOML: original source identities and disposition evidence",
        ),
        (
            "maximum-source-debit",
            "CYCLES",
            "Maximum burn per source from its retained initial balance, e.g. 1T",
        ),
        (
            "maximum-root-debit",
            "CYCLES",
            "Maximum aggregate Root burn, e.g. 2T",
        ),
        (
            "maximum-root-paid-calls",
            "COUNT",
            "Finite Root management-call allowance",
        ),
    ] {
        command = command.arg(
            value_arg(name)
                .long(name)
                .value_name(label)
                .required_unless_present("apply")
                .conflicts_with("apply")
                .help(help),
        );
    }
    command.after_help("Examples:\n  canic --environment staging fleet import staging --canister <id> --declarations deployments/import.toml --maximum-source-debit 1T --maximum-root-debit 2T --maximum-root-paid-calls 36\n  canic --environment staging fleet import staging --apply <review-sha256>\n\nReview retains bounded status observations. After adding funds, repeat an unapproved review with --funding-credit <id>=1T for the exact received amount. Apply hands sources to Root, clears code/state, publishes both policy/seed input paths and releases capacity. Use mutable operator copies for frozen release inputs. No replacement IDs or funding transfers are created. This requires initialized current infrastructure.")
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), FleetCommandError> {
    if print_help_or_version(&args, || render_usage(command), version_text()) {
        return Ok(());
    }
    let matches = parse_matches(command(), args)
        .map_err(|error| FleetCommandError::Usage(error.to_string()))?;
    let json = matches.get_flag("json");
    execute(&matches).map_err(|source| if json {
        FleetCommandError::JsonReported {
            report: serde_json::json!({"schema_version":1,"event":"fleet_import_error","message":source.to_string()}).to_string(),
            source: Box::new(source),
        }
    } else { source })
}

fn execute(matches: &ArgMatches) -> Result<(), FleetCommandError> {
    let environment = string_option(matches, "environment")
        .ok_or_else(|| FleetCommandError::Usage("fleet import requires --environment".into()))?;
    let fleet = required_string(matches, "fleet");
    let executable = string_option_or_else(matches, "icp", default_icp);
    let identity = string_option(matches, "identity");
    let workspace = resolve_current_canic_icp_root()?;
    let icp = IcpCli::new(&executable, Some(environment.clone()))
        .with_identity(identity.as_deref())
        .with_cwd(&workspace);
    let record = if let Some(digest) = string_option(matches, "apply") {
        let digest = decode_hex(&digest)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| FleetCommandError::Usage("invalid import review SHA-256".into()))?;
        review::apply(&workspace, &environment, &fleet, digest, &icp)
    } else {
        review::plan(&workspace, &request(matches, &environment, &fleet)?, &icp)
    }
    .map_err(failure)?;
    let operation = record
        .operation
        .as_ref()
        .ok_or_else(|| failure(CapacityImportJournalError::Integrity))?;
    let mut command = format!(
        "canic --environment {} --icp {} fleet import {}",
        quote_review_argument(&environment),
        quote_review_argument(&executable),
        quote_review_argument(&fleet)
    );
    if let Some(identity) = &identity {
        command.push_str(" --identity ");
        command.push_str(&quote_review_argument(identity));
    }
    command.push_str(" --apply ");
    command.push_str(&hex_bytes(operation.review.review_sha256));
    if matches.get_flag("json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version":1, "stage":"capacity_import", "completed":publication::completed(&record),
                "maximum_operator_debit_cycles":"0", "maximum_initial_status_attempts_per_canister":MAXIMUM_ATTEMPTS,
                "journal":record, "apply_command":command,
                "automation": super::automation::import(
                    hex_bytes(operation.review.review_sha256), publication::completed(&record),
                    matches.get_one::<String>("apply").is_some(),
                    super::automation::command(&environment, &executable, identity.as_deref(), "import", &fleet), false),
            }))?
        );
    } else {
        println!("{}", render(&record, &command));
    }
    Ok(())
}

fn request(
    matches: &ArgMatches,
    environment: &str,
    fleet: &str,
) -> Result<CapacityImportReviewRequest, FleetCommandError> {
    let cycles = |key: &str| {
        Cycles::from_human_config_str(&required_string(matches, key))
            .map(|amount| amount.to_u128())
            .map_err(|_| {
                FleetCommandError::Usage(format!(
                    "--{key} requires an exact cycle amount such as 1T"
                ))
            })
    };
    Ok(CapacityImportReviewRequest {
        funding_credits: matches
            .get_many::<CapacityImportFundingCreditRequest>("funding-credit")
            .into_iter()
            .flatten()
            .cloned()
            .collect(),
        environment: environment.into(),
        fleet: fleet.into(),
        canisters: matches
            .get_many::<Principal>("canister")
            .into_iter()
            .flatten()
            .copied()
            .collect(),
        root: matches.get_one::<Principal>("root").copied(),
        declarations: PathBuf::from(required_string(matches, "declarations")),
        policy: string_option(matches, "source").map_or_else(
            || PathBuf::from(format!("deployments/{fleet}.toml")),
            PathBuf::from,
        ),
        seed: string_option(matches, "seed").map_or_else(
            || PathBuf::from(format!("deployments/{fleet}.estate.toml")),
            PathBuf::from,
        ),
        maximum_source_debit_cycles: cycles("maximum-source-debit")?,
        maximum_root_debit_cycles: cycles("maximum-root-debit")?,
        maximum_root_paid_calls: required_string(matches, "maximum-root-paid-calls")
            .parse()
            .map_err(|_| {
                FleetCommandError::Usage(
                    "--maximum-root-paid-calls requires a positive integer".into(),
                )
            })?,
    })
}

fn parse_funding_credit(value: &str) -> Result<CapacityImportFundingCreditRequest, String> {
    let (canister, cycles) = value
        .split_once('=')
        .ok_or("funding credit requires CANISTER=CYCLES, e.g. <id>=1T")?;
    let canister = Principal::from_text(canister)
        .map_err(|_| "funding credit requires a valid canister Principal")?;
    let cycles = Cycles::from_human_config_str(cycles)
        .map_err(|_| "funding credit requires an exact amount such as 1T")?
        .to_u128();
    if cycles == 0 {
        return Err("funding credit must be positive".into());
    }
    Ok(CapacityImportFundingCreditRequest { canister, cycles })
}

fn failure(error: CapacityImportJournalError) -> FleetCommandError {
    FleetCommandError::CapacityImport(Box::new(error))
}

fn render(record: &CapacityImportJournalRecord, command: &str) -> String {
    let plan = &record.plan;
    let mut lines = vec![
        format!(
            "Capacity import: {}",
            if publication::completed(record) {
                "complete"
            } else {
                "review / resume"
            }
        ),
        format!(
            "Root: {} on {}",
            plan.authority.root,
            plan.authority.subnet.into_principal()
        ),
        "Maximum operator payment: 0 cycles; no creation or funding transfers".into(),
        format!(
            "Root: {} native + {} reserved cycles; maximum debit {}; minimum retained {}; maximum paid calls {}",
            plan.root_budget.observed_cycles,
            plan.root_budget.observed_reserved_cycles,
            plan.root_budget.maximum_debit_cycles,
            plan.root_budget.minimum_retained_cycles,
            plan.root_budget.maximum_paid_calls
        ),
        format!(
            "Initial status attempts: at most {MAXIMUM_ATTEMPTS} per canister; successful balances retained across restart"
        ),
    ];
    for credit in &plan.funding_credits {
        lines.push(format!("Additional funding for {}: {} cycles; original native {}; observed native {}; original debit allowance retained",
            credit.before.binding.canister_id, credit.credited_cycles, credit.before.cycles, credit.observed.cycles));
    }
    for source in &plan.sources {
        lines.push(format!("{}: CLEAR CODE/STATE; module={}; version={}; {} native + {} reserved cycles; maximum debit {}; Ready floor {}",
            source.binding.canister_id, source.binding.module_sha256.map_or_else(|| "empty".into(), hex_bytes),
            source.binding.canister_version, source.observed_cycles, source.observed_reserved_cycles,
            source.maximum_debit_cycles, source.minimum_ready_cycles));
        lines.push(format!(
            "  controllers: {:?} -> {:?} -> {:?}",
            source
                .binding
                .controllers
                .iter()
                .map(Principal::to_text)
                .collect::<Vec<_>>(),
            plan.transitional_controllers
                .iter()
                .map(Principal::to_text)
                .collect::<Vec<_>>(),
            plan.final_controllers
                .iter()
                .map(Principal::to_text)
                .collect::<Vec<_>>()
        ));
    }
    if let Some(operation) = &record.operation {
        lines.push(format!(
            "Review: {}",
            hex_bytes(operation.review.review_sha256)
        ));
        lines.push(format!(
            "Apply attempts: {} per step; management observations: {} per canister",
            operation.review.maximum_submissions_per_step,
            operation
                .review
                .maximum_management_observations_per_canister
        ));
        for document in [&operation.review.policy, &operation.review.seed] {
            lines.push(format!(
                "Publish {}: {} -> {}",
                document.relative_path,
                hex_bytes(document.before_sha256),
                hex_bytes(document.after_sha256)
            ));
        }
    }
    lines.push(command.into());
    lines.join("\n")
}
