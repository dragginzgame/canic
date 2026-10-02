//! Marshal effect-free exhausted Host attempt reviews and exact local approval.

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{parse_matches, render_usage, required_string, string_option, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    fleet::{FleetCommandError, parse_digest, quote_review_argument},
    version_text,
};
use canic_core::cdk::utils::hash::{decode_hex, hex_bytes};
use canic_host::{
    fleet_ensure::workflow::attempt_recovery, icp_config::resolve_current_canic_icp_root,
};
use clap::{ArgAction, ArgMatches, Command};
use std::ffi::OsString;

pub(super) fn command() -> Command {
    Command::new("recover-attempts").bin_name("canic fleet recover-attempts")
        .about("Review bounded continuation of exhausted Host attempt counters")
        .disable_help_flag(true)
        .arg(value_arg("fleet").required(true).help("Fleet identity"))
        .arg(value_arg("apply").long("apply").value_name("REVIEW_SHA256").value_parser(parse_digest)
            .help("Approve two additional attempts per exact reviewed exhausted counter"))
        .arg(value_arg("json").long("json").action(ArgAction::SetTrue).num_args(0)
            .help("Print exact resources, spent counts and continuation authority as JSON"))
        .arg(internal_environment_arg()).arg(internal_icp_arg())
        .after_help("Examples:\n  canic --environment staging fleet recover-attempts staging\n  canic --environment staging fleet recover-attempts staging --apply <review-sha256>\n\nReview and approval issue no IC calls. Repeat the original interrupted command after approval. Spent attempts, original plans, Root paid-call/debit caps and unresolved effects remain retained.")
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), FleetCommandError> {
    if print_help_or_version(&args, || render_usage(command), version_text()) {
        return Ok(());
    }
    let matches = parse_matches(command(), args)
        .map_err(|error| FleetCommandError::Usage(error.to_string()))?;
    execute(&matches).map_err(|source| if matches.get_flag("json") {
        FleetCommandError::JsonReported {
            report: serde_json::json!({"schema_version":1,"event":"fleet_attempt_recovery_error","message":source.to_string()}).to_string(),
            source: Box::new(source),
        }
    } else { source })
}

fn execute(matches: &ArgMatches) -> Result<(), FleetCommandError> {
    let environment = string_option(matches, "environment").ok_or_else(|| {
        FleetCommandError::Usage("fleet recover-attempts requires --environment".into())
    })?;
    let fleet = required_string(matches, "fleet");
    let workspace = resolve_current_canic_icp_root()?;
    let approved = string_option(matches, "apply")
        .map(|value| {
            decode_hex(&value)
                .ok()
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or_else(|| {
                    FleetCommandError::Usage("invalid continuation review SHA-256".into())
                })
        })
        .transpose()?;
    let review = match approved {
        Some(digest) => attempt_recovery::apply(&workspace, &environment, &fleet, digest),
        None => attempt_recovery::review(&workspace, &environment, &fleet),
    }
    .map_err(|error| FleetCommandError::CapacityImport(Box::new(error)))?;
    let digest = hex_bytes(review.review_sha256);
    if matches.get_flag("json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1, "event": "fleet_attempt_recovery", "approved": approved.is_some(),
                "review": review,
                "next_command": if approved.is_some() { None } else { Some(vec!["canic", "--environment", &environment, "fleet", "recover-attempts", &fleet, "--apply", &digest]) },
            }))?
        );
    } else {
        for owner in &review.owners {
            for grant in &owner.grants {
                println!(
                    "{} {}: {} spent; maximum {} → {}",
                    owner.relative_path,
                    grant.resource,
                    grant.spent_attempts,
                    grant.previous_maximum,
                    grant.previous_maximum + grant.additional_attempts
                );
            }
        }
        if approved.is_some() {
            println!("Continuation approved. Repeat the original interrupted command.");
        } else {
            println!(
                "Review {digest}\nApply: canic --environment {} fleet recover-attempts {} --apply {digest}",
                quote_review_argument(&environment),
                quote_review_argument(&fleet)
            );
        }
    }
    Ok(())
}
