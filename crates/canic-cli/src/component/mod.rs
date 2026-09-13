//! Operator command adapter for one reviewed top-level Component allocation.
//!
//! The host workflow owns persistence and reconciliation; Root owns lifecycle effects.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, required_string, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_nested_help,
    },
    support::icp_target::IcpTargetOptions,
};
use canic_core::ids::ComponentSpecId;
use canic_host::{
    component_operation::{
        ComponentOperationError, ops::transport::IcpComponentTransport, workflow,
    },
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
};
use clap::Command;
use std::{
    ffi::OsString,
    time::{Duration, Instant},
};
use thiserror::Error;

/// Component command boundary failures preserve the host's typed recovery diagnostic.
#[derive(Debug, Error)]
pub enum ComponentCommandError {
    #[error(transparent)]
    Clap(#[from] clap::Error),

    #[error(transparent)]
    Host(#[from] ComponentOperationError),

    #[error(transparent)]
    Root(#[from] IcpConfigError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Usage(String),
}

fn command() -> Command {
    let leaf = |name: &'static str, about: &'static str| {
        Command::new(name)
            .about(about)
            .arg(value_arg("fleet").required(true))
            .arg(
                value_arg("name")
                    .required(true)
                    .help("Retained local operation name; reuse it to resume"),
            )
            .arg(
                flag_arg("json")
                    .long("json")
                    .help("Print the complete reviewed operation record"),
            )
    };
    Command::new("component").bin_name("canic component")
        .about("Review, create and reconcile one admitted top-level Component")
        .arg(internal_environment_arg().global(true))
        .arg(internal_icp_arg().global(true))
        .subcommand_required(true)
        .subcommand(leaf("apply", "Submit or resume the exact reviewed operation")
            .arg(value_arg("review").long("review").required(true))
            .arg(value_arg("wait-secs").long("wait-secs").default_value("60").value_parser(clap::value_parser!(u64).range(0..=3600)).help("Maximum polling time; 0 advances once")))
        .subcommand(leaf("plan", "Retain a read-only review against the current Fleet")
            .arg(value_arg("root").long("root").required(true).help("Logical Root name in Fleet desired state"))
            .arg(value_arg("spec").long("spec").required(true).help("Admitted Component Spec ID")))
        .subcommand(leaf("status", "Refresh progress without submitting a command"))
        .after_help("Examples:\n  canic component plan demo core --root root --spec core\n  canic component apply demo core --review <sha256>\n  canic component status demo core --json")
}

/// Parse operator input and delegate to the same durable host workflow on each retry.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), ComponentCommandError> {
    let args = args.into_iter().collect::<Vec<_>>();
    if print_nested_help(&args, command()) {
        return Ok(());
    }
    let matches = parse_matches(command(), args)?;
    let target = IcpTargetOptions::parse(&matches);
    let (action, leaf) = matches
        .subcommand()
        .ok_or_else(|| ComponentCommandError::Usage("select plan, apply or status".to_string()))?;
    let fleet = required_string(leaf, "fleet");
    let name = required_string(leaf, "name");
    let root = resolve_current_canic_icp_root()?;
    let mut transport = IcpComponentTransport::new(&root, target.icp_cli(&root));
    let record = match action {
        "plan" => {
            let spec =
                ComponentSpecId::try_from(required_string(leaf, "spec")).map_err(|error| {
                    ComponentCommandError::Usage(format!("invalid Component Spec: {error}"))
                })?;
            let authority = transport.authority(
                &target.environment,
                &fleet,
                &required_string(leaf, "root"),
                &spec,
            )?;
            workflow::plan(&root, &name, authority, &mut transport)?
        }
        "apply" => {
            let review = required_string(leaf, "review");
            let wait =
                Duration::from_secs(*leaf.get_one::<u64>("wait-secs").expect("default wait"));
            let started = Instant::now();
            let mut record = workflow::apply(
                &root,
                &target.environment,
                &fleet,
                &name,
                &review,
                &mut transport,
            )?;
            loop {
                if record
                    .progress
                    .as_ref()
                    .is_some_and(|progress| progress.complete)
                    || started.elapsed() >= wait
                {
                    break record;
                }
                std::thread::sleep(Duration::from_secs(1));
                record =
                    workflow::status(&root, &target.environment, &fleet, &name, &mut transport)?;
            }
        }
        "status" => workflow::status(&root, &target.environment, &fleet, &name, &mut transport)?,
        _ => unreachable!("Clap limits commands"),
    };
    if leaf.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&record)?);
    } else {
        println!("review: {}", record.plan.review_sha256);
        println!("role: {}", record.plan.authority.role);
        match &record.progress {
            Some(progress) => {
                println!(
                    "phase: {:?}; complete: {}",
                    progress.phase, progress.complete
                );
                if let Some(binding) = &progress.binding {
                    println!("canister: {}", binding.canister_id);
                }
            }
            None if record.submission_attempts == 0 => println!("phase: reviewed"),
            None => println!(
                "phase: awaiting Root observation; submissions: {}",
                record.submission_attempts
            ),
        }
        if !record
            .progress
            .as_ref()
            .is_some_and(|progress| progress.complete)
        {
            println!(
                "resume: canic --environment {} component apply {} {} --review {}",
                target.environment, fleet, name, record.plan.review_sha256
            );
        }
    }
    Ok(())
}
