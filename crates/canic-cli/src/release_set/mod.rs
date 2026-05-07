use crate::{
    args::{
        first_arg_is_help, first_arg_is_version, flag_arg, parse_matches, path_option,
        string_option, value_arg,
    },
    version_text,
};
use canic_host::release_set::{
    config_path, configured_install_targets, dfx_root, emit_root_release_set_manifest,
    emit_root_release_set_manifest_if_ready, load_root_release_set_manifest, resolve_artifact_root,
    resume_root_bootstrap, root_release_set_manifest_path, stage_root_release_set, workspace_root,
};
use clap::{ArgMatches, Command as ClapCommand};
use std::{env, ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";

///
/// ReleaseSetCommandError
///

#[derive(Debug, ThisError)]
pub enum ReleaseSetCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error(transparent)]
    ReleaseSet(#[from] Box<dyn std::error::Error>),
}

///
/// ReleaseSetCommand
///

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReleaseSetCommand {
    Targets(TargetsOptions),
    Manifest(ManifestOptions),
    Stage(StageOptions),
}

///
/// TargetsOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct TargetsOptions {
    config_path: Option<PathBuf>,
    root_target: String,
}

///
/// ManifestOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManifestOptions {
    if_ready: bool,
}

///
/// StageOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct StageOptions {
    root_target: String,
}

/// Run the release-set command family.
pub fn run<I>(args: I) -> Result<(), ReleaseSetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if first_arg_is_help(&args) {
        println!("{}", usage());
        return Ok(());
    }
    if first_arg_is_version(&args) {
        println!("{}", version_text());
        return Ok(());
    }

    match ReleaseSetCommand::parse(args)? {
        ReleaseSetCommand::Targets(options) => run_targets(options),
        ReleaseSetCommand::Manifest(options) => run_manifest(options),
        ReleaseSetCommand::Stage(options) => run_stage(options),
    }
    .map_err(ReleaseSetCommandError::from)
}

impl ReleaseSetCommand {
    // Parse the selected release-set subcommand.
    fn parse<I>(args: I) -> Result<Self, ReleaseSetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut args = args.into_iter();
        let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
            return Err(ReleaseSetCommandError::Usage(usage()));
        };

        match command.as_str() {
            "targets" => Ok(Self::Targets(TargetsOptions::parse(args)?)),
            "manifest" => Ok(Self::Manifest(ManifestOptions::parse(args)?)),
            "stage" => Ok(Self::Stage(StageOptions::parse(args)?)),
            _ => Err(ReleaseSetCommandError::Usage(usage())),
        }
    }
}

impl TargetsOptions {
    // Parse install-target listing options.
    fn parse<I>(args: I) -> Result<Self, ReleaseSetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_release_set_options(targets_command(), args, targets_usage())?;

        Ok(Self {
            config_path: path_option(&matches, "config"),
            root_target: string_option(&matches, "root")
                .unwrap_or_else(|| DEFAULT_ROOT_TARGET.to_string()),
        })
    }
}

impl ManifestOptions {
    // Parse root release-set manifest emission options.
    fn parse<I>(args: I) -> Result<Self, ReleaseSetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_release_set_options(manifest_command(), args, manifest_usage())?;

        Ok(Self {
            if_ready: matches.get_flag("if-ready"),
        })
    }
}

impl StageOptions {
    // Parse root release-set staging options.
    fn parse<I>(args: I) -> Result<Self, ReleaseSetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_release_set_options(stage_command(), args, stage_usage())?;

        Ok(Self {
            root_target: string_option(&matches, "root-canister")
                .or_else(|| env::var("ROOT_CANISTER").ok())
                .unwrap_or_else(|| DEFAULT_ROOT_TARGET.to_string()),
        })
    }
}

// Parse one release-set subcommand option set.
fn parse_release_set_options<I>(
    command: ClapCommand,
    args: I,
    usage: &'static str,
) -> Result<ArgMatches, ReleaseSetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    parse_matches(command, args).map_err(|_| ReleaseSetCommandError::Usage(usage))
}

// Build the install-target parser.
fn targets_command() -> ClapCommand {
    ClapCommand::new("targets")
        .disable_help_flag(true)
        .arg(value_arg("config").long("config"))
        .arg(value_arg("root").long("root"))
}

// Build the manifest emission parser.
fn manifest_command() -> ClapCommand {
    ClapCommand::new("manifest")
        .disable_help_flag(true)
        .arg(flag_arg("if-ready").long("if-ready"))
}

// Build the release staging parser.
fn stage_command() -> ClapCommand {
    ClapCommand::new("stage")
        .disable_help_flag(true)
        .arg(value_arg("root-canister"))
}

// Print configured install targets in the order the install flow uses.
fn run_targets(options: TargetsOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let config_path = options
        .config_path
        .unwrap_or_else(|| config_path(&workspace_root));

    for role in configured_install_targets(&config_path, &options.root_target)? {
        println!("{role}");
    }

    Ok(())
}

// Emit the root release-set manifest from current build artifacts.
fn run_manifest(options: ManifestOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let dfx_root = dfx_root()?;
    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let manifest_path = if options.if_ready {
        emit_root_release_set_manifest_if_ready(&workspace_root, &dfx_root, &network)?
    } else {
        Some(emit_root_release_set_manifest(
            &workspace_root,
            &dfx_root,
            &network,
        )?)
    };

    if let Some(path) = manifest_path {
        println!("{}", path.display());
    }

    Ok(())
}

// Stage the current root release set and resume root bootstrap.
fn run_stage(options: StageOptions) -> Result<(), Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let artifact_root = resolve_artifact_root(&dfx_root, &network)?;
    let manifest_path = root_release_set_manifest_path(&artifact_root)?;
    let manifest = load_root_release_set_manifest(&manifest_path)?;

    stage_root_release_set(&dfx_root, &options.root_target, &manifest)?;
    resume_root_bootstrap(&options.root_target)?;
    Ok(())
}

// Return release-set command family usage.
const fn usage() -> &'static str {
    "usage: canic release-set <command> [<args>]\n\ncommands:\n  targets   List root plus ordinary install targets from canic.toml.\n  manifest  Emit the current root release-set manifest from local build artifacts.\n  stage     Stage the current root release set and resume root bootstrap."
}

// Return release-set target listing usage.
const fn targets_usage() -> &'static str {
    "usage: canic release-set targets [--config <canic.toml>] [--root <dfx-canister-name>]"
}

// Return release-set manifest usage.
const fn manifest_usage() -> &'static str {
    "usage: canic release-set manifest [--if-ready]"
}

// Return release-set stage usage.
const fn stage_usage() -> &'static str {
    "usage: canic release-set stage [root-canister]"
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure target listing options preserve config and root inputs.
    #[test]
    fn parses_targets_options() {
        let parsed = ReleaseSetCommand::parse([
            OsString::from("targets"),
            OsString::from("--config"),
            OsString::from("canisters/demo/canic.toml"),
            OsString::from("--root"),
            OsString::from("custom_root"),
        ])
        .expect("parse targets");

        let ReleaseSetCommand::Targets(options) = parsed else {
            panic!("expected targets command");
        };

        assert_eq!(
            options.config_path,
            Some(PathBuf::from("canisters/demo/canic.toml"))
        );
        assert_eq!(options.root_target, "custom_root");
    }

    // Ensure manifest emission accepts the readiness gate flag.
    #[test]
    fn parses_manifest_options() {
        let parsed =
            ReleaseSetCommand::parse([OsString::from("manifest"), OsString::from("--if-ready")])
                .expect("parse manifest");

        let ReleaseSetCommand::Manifest(options) = parsed else {
            panic!("expected manifest command");
        };

        assert!(options.if_ready);
    }

    // Ensure stage accepts an explicit root target.
    #[test]
    fn parses_stage_root_target() {
        let parsed =
            ReleaseSetCommand::parse([OsString::from("stage"), OsString::from("custom_root")])
                .expect("parse stage");

        let ReleaseSetCommand::Stage(options) = parsed else {
            panic!("expected stage command");
        };

        assert_eq!(options.root_target, "custom_root");
    }
}
