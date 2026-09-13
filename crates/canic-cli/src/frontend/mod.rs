//! Public CLI adapter for frontend bundle export and integrity verification.
//!
//! The host owns source validation, generation and durable publication.

use crate::cli::{
    clap::{flag_arg, parse_matches, required_string, string_option, value_arg},
    defaults::local_environment,
    globals::{internal_environment_arg, internal_icp_arg},
    help::print_nested_help,
};
use crate::support::icp_target::IcpTargetOptions;
use canic_host::{
    frontend::{FrontendError, model::FrontendAssetCapacityInput, ops, workflow},
    icp_config::resolve_current_canic_icp_root,
};
use clap::Command;
use std::{ffi::OsString, path::Path};
use thiserror::Error;

/// CLI boundary errors retain the host's exact typed diagnostic.
#[derive(Debug, Error)]
pub enum FrontendCommandError {
    #[error(transparent)]
    Clap(#[from] clap::Error),

    #[error(transparent)]
    Host(#[from] FrontendError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Root(#[from] canic_host::icp_config::IcpConfigError),
}

fn command() -> Command {
    Command::new("frontend").bin_name("canic frontend")
        .about("Export and verify exact browser bindings from a terminal Fleet")
        .arg(internal_environment_arg().global(true))
        .arg(internal_icp_arg().global(true))
        .arg(flag_arg("json").long("json").global(true).help("Print structured JSON"))
        .subcommand_required(true)
        .subcommand(Command::new("capacity").about("Check external asset native cycles against an explicit upload floor")
            .arg(value_arg("canister").required(true).value_parser(clap::value_parser!(candid::Principal)))
            .arg(value_arg("payload").long("payload").required(true))
            .arg(value_arg("minimum-native-cycles").long("minimum-native-cycles").required(true).value_parser(clap::value_parser!(u128)))
            .arg(value_arg("maximum-payload-bytes").long("maximum-payload-bytes").required(true).value_parser(clap::value_parser!(u64).range(1..)))
            .arg(value_arg("maximum-files").long("maximum-files").required(true).value_parser(clap::value_parser!(u32).range(1..))))
        .subcommand(Command::new("export").about("Write a reviewed environment and binding bundle")
            .arg(value_arg("fleet").required(true))
            .arg(value_arg("input").long("input").required(true).help("Selected-environment frontend JSON"))
            .arg(value_arg("out").long("out").required(true).help("New or identical bundle directory")))
        .subcommand(Command::new("verify").about("Verify files against an independently retained digest")
            .arg(value_arg("directory").required(true))
            .arg(value_arg("sha256").long("sha256").required(true)))
        .after_help("Examples:\n  canic frontend export demo --input frontend-local.json --out browser/canic\n  canic frontend verify browser/canic --sha256 <digest>\n  canic --environment ic frontend export demo --input frontend-ic.json --out browser/canic --json")
}

/// Parse the selected environment and delegate to the host handoff owner.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), FrontendCommandError> {
    let args = args.into_iter().collect::<Vec<_>>();
    if print_nested_help(&args, command()) {
        return Ok(());
    }
    let matches = parse_matches(command(), args)?;
    let (action, leaf) = matches.subcommand().expect("required frontend subcommand");
    if action == "capacity" {
        return capacity(&matches, leaf);
    }
    let manifest = match action {
        "export" => {
            let input = ops::read_input(Path::new(&required_string(leaf, "input")))?;
            let environment =
                string_option(&matches, "environment").unwrap_or_else(local_environment);
            if input.environment != environment {
                return Err(FrontendError::Environment.into());
            }
            workflow::prepare_handoff(
                &resolve_current_canic_icp_root()?,
                &required_string(leaf, "fleet"),
                &input,
                Path::new(&required_string(leaf, "out")),
            )?
        }
        "verify" => ops::verify_bundle(
            Path::new(&required_string(leaf, "directory")),
            &required_string(leaf, "sha256"),
        )?,
        _ => unreachable!("Clap selects the maintained subcommands"),
    };
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        println!(
            "Frontend bundle: {} / {}",
            manifest.environment, manifest.fleet
        );
        println!("sha256: {}", manifest.manifest_sha256);
        println!("role instances: {}", manifest.roles.len());
    }
    Ok(())
}

fn capacity(
    matches: &clap::ArgMatches,
    leaf: &clap::ArgMatches,
) -> Result<(), FrontendCommandError> {
    let target = IcpTargetOptions::parse(matches);
    let report = ops::asset_capacity(
        &target.icp_cli(&resolve_current_canic_icp_root()?),
        &FrontendAssetCapacityInput {
            environment: target.environment,
            canister_id: *leaf
                .get_one::<candid::Principal>("canister")
                .expect("required Principal"),
            payload_directory: required_string(leaf, "payload").into(),
            minimum_native_cycles: *leaf
                .get_one::<u128>("minimum-native-cycles")
                .expect("required native floor"),
            maximum_payload_bytes: *leaf
                .get_one::<u64>("maximum-payload-bytes")
                .expect("required byte bound"),
            maximum_files: *leaf
                .get_one::<u32>("maximum-files")
                .expect("required file bound"),
        },
    )?;
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Asset {}: {} native cycles; required {}",
            report.canister_id, report.native_cycles, report.minimum_native_cycles
        );
        println!(
            "Payload: {} files, {} bytes, sha256 {}",
            report.payload.files, report.payload.bytes, report.payload.sha256
        );
    }
    if !report.sufficient {
        return Err(FrontendError::NativeCapacity {
            available: report
                .native_cycles
                .parse()
                .map_err(|_| FrontendError::NativeBalance)?,
            required: report
                .minimum_native_cycles
                .parse()
                .map_err(|_| FrontendError::NativeBalance)?,
        }
        .into());
    }
    Ok(())
}
