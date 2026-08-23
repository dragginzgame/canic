//! Module: canic_cli::scaffold
//!
//! Responsibility: create local app and declared canister source scaffolds.
//! Does not own: deployment execution, canister install/upgrade, or runtime
//! state mutation.
//! Boundary: validates CLI input and writes new local source/config files only.

#[cfg(test)]
mod tests;

use crate::{
    cli::clap::{
        flag_arg, parse_matches, parse_subcommand, passthrough_subcommand, render_usage,
        required_string, required_typed, string_option_or_else, typed_values,
    },
    cli::defaults::local_environment,
    cli::globals::internal_environment_arg,
    cli::help::print_help_or_version,
    cli::render::append_dry_run_footer,
    version_text,
};
use candid::Principal;
use canic_core::ids::{FleetFundingProfile, SubnetId};
use canic_core::shared_support::is_ascii_snake_case;
use canic_host::{
    durable_io::write_bytes,
    fleet_install_input::{
        FleetFundingProfileRootScaffold, FleetFundingProfileScaffold,
        PROFILE_CYCLE_ROUNDING_QUANTUM, STANDARD_PROFILE_NODE_COUNT,
        resolve_fleet_funding_profile_node_counts, scaffold_fleet_funding_profile,
    },
    install_root::{
        ConfigDiscoveryError, current_canic_workspace_root,
        discover_workspace_canic_config_choices, select_discovered_app_config_path,
    },
    release_set::{
        AppConfigError, declare_app_role, display_workspace_path, icp_root as resolve_icp_root,
        plan_declare_app_role,
    },
};
use clap::{Arg, ArgAction, Command as ClapCommand};
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;
use toml::Value as TomlValue;

const APP_CREATE_HELP_AFTER: &str = "\
Examples:
  canic app create demo
  canic app create demo --yes
  canic app create demo --dry-run";
const SCAFFOLD_HELP_AFTER: &str = "\
Examples:
  canic scaffold canister demo store
  canic scaffold fleet-input preview_multi_subnet --coordinator-node-count 34 --root-node-count 13

Mutation notes:
  canic scaffold canister writes a new local role crate, appends the workspace
  Cargo.toml member when present, and declares the role in canic.toml.
  canic scaffold fleet-input is a funding-policy authoring aid; it reads no
  identity or balance and writes no files.
  Use --dry-run to validate and preview without changing files.";
const SCAFFOLD_CANISTER_HELP_AFTER: &str = "\
Examples:
  canic scaffold canister demo store
  canic scaffold canister demo store --dry-run";
const SCAFFOLD_FLEET_INPUT_HELP_AFTER: &str = "\
Examples:
  canic --environment ic scaffold fleet-input preview_multi_subnet --coordinator-subnet <id> --root-subnet <id>
  canic scaffold fleet-input preview_multi_subnet --coordinator-node-count 34 --root-node-count 13

This command materializes exact node-scaled funding values and the fee-complete
maximum operator debit without reading an ICP identity or ledger balance. On
IC, exact Subnet IDs resolve node counts from Canic's trusted Registry catalog;
--refresh-catalog refreshes missing or invalid evidence. Explicit node counts
remain available for offline authoring. The emitted TOML is a funding fragment;
add topology, admissions, limits, pool policy and exact Subnet IDs, then use
canic deploy plan for live identity, balance and install admission.";

///
/// ScaffoldCommandError
///

#[derive(Debug, ThisError)]
pub enum ScaffoldCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("scaffold target already exists: {0}")]
    TargetExists(String),

    #[error("app create cancelled")]
    Cancelled,

    #[error("no Canic app configs found under apps; run canic app create <name>")]
    NoConfigChoices,

    #[error("unknown app {0}; run canic app list to inspect config-defined apps")]
    UnknownApp(String),

    #[error("app {0} config does not have a parent directory")]
    MissingAppDirectory(String),

    #[error("failed to discover Canic workspace App configs: {0}")]
    ConfigDiscovery(#[from] ConfigDiscoveryError),

    #[error(transparent)]
    AppConfig(#[from] AppConfigError),

    #[error(transparent)]
    FleetFundingProfile(#[from] canic_host::fleet_install_input::FleetFundingProfileScaffoldError),

    #[error(transparent)]
    FleetInstallInput(#[from] canic_host::fleet_install_input::FleetInstallInputError),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("scaffold failed: {operation}; rollback failed: {rollback}")]
    Rollback {
        operation: Box<Self>,
        rollback: io::Error,
    },
}

///
/// ScaffoldOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScaffoldOptions {
    name: String,
    yes: bool,
    dry_run: bool,
}

///
/// CanisterScaffoldOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanisterScaffoldOptions {
    app: String,
    role: String,
    dry_run: bool,
}

///
/// FleetInputScaffoldOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetInputScaffoldOptions {
    profile: FleetFundingProfile,
    node_counts: FleetInputScaffoldNodeCounts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FleetInputScaffoldNodeCounts {
    Explicit {
        coordinator: u64,
        roots: Vec<u64>,
    },
    Registry {
        environment: String,
        coordinator_subnet: SubnetId,
        root_subnets: Vec<SubnetId>,
        refresh_catalog: bool,
    },
}

impl ScaffoldOptions {
    #[cfg(test)]
    fn parse<I>(args: I) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_with(args, app_create_command(), app_create_usage)
    }

    fn parse_with<I>(
        args: I,
        command: ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command, args).map_err(|_| ScaffoldCommandError::Usage(usage()))?;
        Ok(Self {
            name: required_string(&matches, "name"),
            yes: matches.get_flag("yes"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl CanisterScaffoldOptions {
    #[cfg(test)]
    fn parse<I>(args: I) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_with(args, scaffold_canister_command(), scaffold_canister_usage)
    }

    fn parse_with<I>(
        args: I,
        command: ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command, args).map_err(|_| ScaffoldCommandError::Usage(usage()))?;
        Ok(Self {
            app: required_string(&matches, "app"),
            role: required_string(&matches, "role"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl FleetInputScaffoldOptions {
    #[cfg(test)]
    fn parse<I>(args: I) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_with(
            args,
            scaffold_fleet_input_command(),
            scaffold_fleet_input_usage,
        )
    }

    fn parse_with<I>(
        args: I,
        command: ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command, args).map_err(|_| ScaffoldCommandError::Usage(usage()))?;
        let profile = required_typed(&matches, "profile");
        let coordinator_node_count = matches.get_one::<u64>("coordinator-node-count").copied();
        let root_node_counts = typed_values(&matches, "root-node-count");
        let coordinator_subnet = matches.get_one::<SubnetId>("coordinator-subnet").copied();
        let root_subnets = typed_values(&matches, "root-subnet");
        let refresh_catalog = matches.get_flag("refresh-catalog");
        let environment = string_option_or_else(&matches, "environment", local_environment);
        let node_counts = match (
            coordinator_node_count,
            root_node_counts.as_slice(),
            coordinator_subnet,
            root_subnets.as_slice(),
        ) {
            (Some(coordinator), [_, ..], None, []) if !refresh_catalog => {
                FleetInputScaffoldNodeCounts::Explicit {
                    coordinator,
                    roots: root_node_counts,
                }
            }
            (None, [], Some(coordinator_subnet), [_, ..]) => {
                FleetInputScaffoldNodeCounts::Registry {
                    environment,
                    coordinator_subnet,
                    root_subnets,
                    refresh_catalog,
                }
            }
            _ => {
                return Err(ScaffoldCommandError::Usage(format!(
                    "choose either Coordinator/Root Subnet IDs or Coordinator/Root node counts; do not mix the two modes\n\n{}",
                    usage()
                )));
            }
        };
        Ok(Self {
            profile,
            node_counts,
        })
    }
}

/// Run the top-level scaffold command.
pub fn run<I>(args: I) -> Result<(), ScaffoldCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(scaffold_command(), args)
        .map_err(|_| ScaffoldCommandError::Usage(usage()))?
    {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "canister" => run_canister(args),
            "fleet-input" => run_fleet_input(args),
            _ => unreachable!("scaffold dispatch command only defines known commands"),
        },
    }
}

/// Run the app create command.
pub fn run_app_create<I>(args: I) -> Result<(), ScaffoldCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, app_create_usage, version_text()) {
        return Ok(());
    }

    let options = ScaffoldOptions::parse_with(args, app_create_command(), app_create_usage)?;
    run_scaffold(options)
}

fn run_canister<I>(args: I) -> Result<(), ScaffoldCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, scaffold_canister_usage, version_text()) {
        return Ok(());
    }

    let options = CanisterScaffoldOptions::parse_with(
        args,
        scaffold_canister_command(),
        scaffold_canister_usage,
    )?;
    let result = if options.dry_run {
        let plan = plan_scaffold_canister(&options)?;
        println!("{}", render_canister_scaffold_plan(&plan));
        return Ok(());
    } else {
        scaffold_canister(&options)?
    };
    println!("Created Canic canister role:");
    println!("  role: {}.{}", result.app, result.role);
    println!("  package: {}", result.package);
    println!("  crate: {}", result.canister_dir.display());
    println!("  config: {}", result.config_path.display());
    println!("  state: declared");
    println!();
    println!("Next:");
    println!("  cargo check -p {}", result.package_name);
    println!("  canic medic --ci");
    println!(
        "  canic app role attach {} {} --component-spec <component-spec>",
        result.app, result.role
    );
    println!(
        "  if medic reports required auth features, edit {} manually",
        result.canister_dir.join("Cargo.toml").display()
    );
    Ok(())
}

fn run_fleet_input<I>(args: I) -> Result<(), ScaffoldCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, scaffold_fleet_input_usage, version_text()) {
        return Ok(());
    }

    let options = FleetInputScaffoldOptions::parse_with(
        args,
        scaffold_fleet_input_command(),
        scaffold_fleet_input_usage,
    )?;
    let (coordinator_node_count, root_node_counts, source) = match options.node_counts {
        FleetInputScaffoldNodeCounts::Explicit { coordinator, roots } => {
            (coordinator, roots, "explicit_node_counts")
        }
        FleetInputScaffoldNodeCounts::Registry {
            environment,
            coordinator_subnet,
            root_subnets,
            refresh_catalog,
        } => {
            let icp_root =
                resolve_icp_root().map_err(|error| ScaffoldCommandError::Host(Box::new(error)))?;
            let resolution = resolve_fleet_funding_profile_node_counts(
                &icp_root,
                &environment,
                options.profile,
                coordinator_subnet,
                &root_subnets,
                refresh_catalog,
            )?;
            (
                resolution.coordinator_node_count,
                resolution.root_node_counts,
                "trusted_ic_registry_catalog",
            )
        }
    };
    let scaffold =
        scaffold_fleet_funding_profile(options.profile, coordinator_node_count, &root_node_counts)?;
    println!("{}", render_fleet_input_scaffold(&scaffold, source));
    Ok(())
}

fn run_scaffold(options: ScaffoldOptions) -> Result<(), ScaffoldCommandError> {
    let workspace_root = scaffold_workspace_root()?;
    if options.dry_run {
        let plan = plan_scaffold_app_at(&workspace_root, &options)?;
        println!("{}", render_scaffold_app_plan(&plan));
        return Ok(());
    }
    if !options.yes {
        confirm_scaffold(&options, &workspace_root, io::stdin().lock(), io::stdout())?;
    }

    let result = scaffold_app_at(&workspace_root, &options)?;
    println!("Created Canic app:");
    println!("  {}", result.app_root.display());
    println!("  {}", result.root_dir.display());
    println!("  {}", result.app_dir.display());
    println!("  {}", result.config_path.display());
    println!();
    println!("Next:");
    println!("  edit icp.yaml");
    println!("  canic medic --ci");
    println!("  canic status");
    println!("  canic install {} <fleet>", options.name);
    Ok(())
}

///
/// CanisterScaffoldResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanisterScaffoldResult {
    app: String,
    role: String,
    package: String,
    package_name: String,
    canister_dir: PathBuf,
    config_path: PathBuf,
}

///
/// CanisterScaffoldPlan
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanisterScaffoldPlan {
    result: CanisterScaffoldResult,
    canister_dir: PathBuf,
    config_path: PathBuf,
    workspace_member: String,
    workspace_manifest_path: Option<PathBuf>,
}

///
/// ScaffoldResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScaffoldResult {
    app_root: PathBuf,
    root_dir: PathBuf,
    app_dir: PathBuf,
    config_path: PathBuf,
}

///
/// ScaffoldAppPlan
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScaffoldAppPlan {
    result: ScaffoldResult,
    files: Vec<PathBuf>,
}

fn plan_scaffold_app_at(
    workspace_root: &Path,
    options: &ScaffoldOptions,
) -> Result<ScaffoldAppPlan, ScaffoldCommandError> {
    let app_root = workspace_root.join("apps").join(&options.name);
    if app_root.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            app_root.display().to_string(),
        ));
    }

    let root_dir = app_root.join("root");
    let app_dir = app_root.join("app");
    let config_path = app_root.join("canic.toml");
    let files = vec![
        config_path.clone(),
        root_dir.join("Cargo.toml"),
        root_dir.join("build.rs"),
        root_dir.join("src/lib.rs"),
        app_dir.join("Cargo.toml"),
        app_dir.join("build.rs"),
        app_dir.join("src/lib.rs"),
    ];

    Ok(ScaffoldAppPlan {
        result: ScaffoldResult {
            app_root,
            root_dir,
            app_dir,
            config_path,
        },
        files,
    })
}

fn scaffold_app_at(
    workspace_root: &Path,
    options: &ScaffoldOptions,
) -> Result<ScaffoldResult, ScaffoldCommandError> {
    let plan = plan_scaffold_app_at(workspace_root, options)?;
    let result = &plan.result;
    let root_src_dir = result.root_dir.join("src");
    let app_src_dir = result.app_dir.join("src");

    let write_result = (|| {
        write_new_file(&result.config_path, &canic_toml(&options.name))?;
        write_new_file(
            &result.root_dir.join("Cargo.toml"),
            &root_cargo_toml(&options.name),
        )?;
        write_new_file(&result.root_dir.join("build.rs"), ROOT_BUILD_RS)?;
        write_new_file(&root_src_dir.join("lib.rs"), ROOT_LIB_RS)?;
        write_new_file(
            &result.app_dir.join("Cargo.toml"),
            &app_cargo_toml(&options.name),
        )?;
        write_new_file(&result.app_dir.join("build.rs"), APP_BUILD_RS)?;
        write_new_file(&app_src_dir.join("lib.rs"), APP_LIB_RS)?;
        Ok::<(), ScaffoldCommandError>(())
    })();

    if let Err(operation) = write_result {
        return match rollback_scaffold(&result.app_root, &[]) {
            Ok(()) => Err(operation),
            Err(rollback) => Err(ScaffoldCommandError::Rollback {
                operation: Box::new(operation),
                rollback,
            }),
        };
    }

    Ok(plan.result)
}

/// Create a declared-only canister crate under an existing app config.
fn scaffold_canister(
    options: &CanisterScaffoldOptions,
) -> Result<CanisterScaffoldResult, ScaffoldCommandError> {
    scaffold_canister_at(&scaffold_workspace_root()?, options)
}

fn plan_scaffold_canister(
    options: &CanisterScaffoldOptions,
) -> Result<CanisterScaffoldPlan, ScaffoldCommandError> {
    plan_scaffold_canister_at(&scaffold_workspace_root()?, options)
}

fn plan_scaffold_canister_at(
    workspace_root: &Path,
    options: &CanisterScaffoldOptions,
) -> Result<CanisterScaffoldPlan, ScaffoldCommandError> {
    let config_path = selected_app_config_path(workspace_root, &options.app)?;
    let app_dir = config_path
        .parent()
        .ok_or_else(|| ScaffoldCommandError::MissingAppDirectory(options.app.clone()))?;
    let canister_dir = app_dir.join(&options.role);
    if canister_dir.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            canister_dir.display().to_string(),
        ));
    }

    let package = options.role.clone();
    plan_declare_app_role(&config_path, &options.app, &options.role, &package)
        .map_err(|err| ScaffoldCommandError::Usage(err.to_string()))?;
    let workspace_member = display_workspace_path(workspace_root, &canister_dir);
    validate_workspace_member_update(workspace_root, &workspace_member)?;
    let workspace_manifest_path = workspace_manifest_path(workspace_root);
    let package_name = canister_package_name(&options.app, &options.role);

    Ok(CanisterScaffoldPlan {
        result: CanisterScaffoldResult {
            app: options.app.clone(),
            role: options.role.clone(),
            package,
            package_name,
            canister_dir: display_path(workspace_root, &canister_dir),
            config_path: display_path(workspace_root, &config_path),
        },
        canister_dir,
        config_path,
        workspace_member,
        workspace_manifest_path,
    })
}

fn scaffold_canister_at(
    workspace_root: &Path,
    options: &CanisterScaffoldOptions,
) -> Result<CanisterScaffoldResult, ScaffoldCommandError> {
    let plan = plan_scaffold_canister_at(workspace_root, options)?;
    let src_dir = plan.canister_dir.join("src");
    let mut originals = vec![(plan.config_path.clone(), fs::read(&plan.config_path)?)];
    if let Some(manifest_path) = &plan.workspace_manifest_path {
        originals.push((manifest_path.clone(), fs::read(manifest_path)?));
    }

    let write_result = (|| {
        write_new_file(
            &plan.canister_dir.join("Cargo.toml"),
            &canister_cargo_toml(&options.app, &options.role),
        )?;
        write_new_file(&plan.canister_dir.join("build.rs"), CANISTER_BUILD_RS)?;
        write_new_file(&src_dir.join("lib.rs"), CANISTER_LIB_RS)?;
        append_workspace_member(workspace_root, &plan.workspace_member)?;
        declare_app_role(
            &plan.config_path,
            &options.app,
            &options.role,
            &plan.result.package,
        )?;
        Ok::<(), ScaffoldCommandError>(())
    })();

    if let Err(operation) = write_result {
        return match rollback_scaffold(&plan.canister_dir, &originals) {
            Ok(()) => Err(operation),
            Err(rollback) => Err(ScaffoldCommandError::Rollback {
                operation: Box::new(operation),
                rollback,
            }),
        };
    }

    Ok(plan.result)
}

fn append_workspace_member(
    workspace_root: &Path,
    member: &str,
) -> Result<(), ScaffoldCommandError> {
    let Some(manifest_path) = workspace_manifest_path(workspace_root) else {
        return Ok(());
    };
    let source = fs::read_to_string(&manifest_path)?;
    let updated = append_workspace_member_source(&source, member)?;
    if updated != source {
        write_bytes(&manifest_path, updated.as_bytes())?;
    }
    Ok(())
}

fn rollback_scaffold(created_dir: &Path, originals: &[(PathBuf, Vec<u8>)]) -> io::Result<()> {
    let mut first_error = None;

    for (path, bytes) in originals {
        if let Err(error) = write_bytes(path, bytes)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }

    if created_dir.exists()
        && let Err(error) = fs::remove_dir_all(created_dir)
        && first_error.is_none()
    {
        first_error = Some(error);
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn validate_workspace_member_update(
    workspace_root: &Path,
    member: &str,
) -> Result<(), ScaffoldCommandError> {
    let Some(manifest_path) = workspace_manifest_path(workspace_root) else {
        return Ok(());
    };
    let source = fs::read_to_string(&manifest_path)?;
    append_workspace_member_source(&source, member)?;
    Ok(())
}

fn workspace_manifest_path(workspace_root: &Path) -> Option<PathBuf> {
    let manifest_path = workspace_root.join("Cargo.toml");
    if !manifest_path.is_file() {
        return None;
    }
    Some(manifest_path)
}

fn append_workspace_member_source(
    source: &str,
    member: &str,
) -> Result<String, ScaffoldCommandError> {
    if workspace_members_contains(source, member)? {
        return Ok(source.to_string());
    }

    let member_literal = toml_string_literal(member);
    let workspace_line = source
        .lines()
        .position(|line| line.trim() == "[workspace]")
        .ok_or_else(|| {
            ScaffoldCommandError::Usage("Cargo.toml is missing [workspace]".to_string())
        })?;
    let workspace_start = line_start_offset(source, workspace_line + 1);
    let workspace_end = section_end_offset(source, workspace_start);
    let section = &source[workspace_start..workspace_end];

    let updated = if let Some(members_offset) = find_members_array_offset(section) {
        insert_workspace_member(source, workspace_start + members_offset, &member_literal)?
    } else {
        let mut updated = source.to_string();
        updated.insert_str(
            workspace_start,
            &format!("members = [\n    {member_literal},\n]\n"),
        );
        updated
    };
    ensure_workspace_member_present(&updated, member)?;
    Ok(updated)
}

fn workspace_members_contains(source: &str, member: &str) -> Result<bool, ScaffoldCommandError> {
    let manifest = workspace_manifest_value(source)?;
    let Some(members) = workspace_members(&manifest)? else {
        return Ok(false);
    };
    Ok(members.contains(&member))
}

fn ensure_workspace_member_present(source: &str, member: &str) -> Result<(), ScaffoldCommandError> {
    if workspace_members_contains(source, member)? {
        return Ok(());
    }
    Err(ScaffoldCommandError::Usage(format!(
        "failed to append workspace member {member}"
    )))
}

fn workspace_manifest_value(source: &str) -> Result<TomlValue, ScaffoldCommandError> {
    toml::from_str::<TomlValue>(source)
        .map_err(|err| ScaffoldCommandError::Usage(format!("invalid Cargo.toml: {err}")))
}

fn workspace_members(manifest: &TomlValue) -> Result<Option<Vec<&str>>, ScaffoldCommandError> {
    let workspace = manifest
        .get("workspace")
        .and_then(TomlValue::as_table)
        .ok_or_else(|| {
            ScaffoldCommandError::Usage("Cargo.toml is missing [workspace]".to_string())
        })?;
    let Some(members) = workspace.get("members") else {
        return Ok(None);
    };
    let members = members
        .as_array()
        .ok_or_else(|| {
            ScaffoldCommandError::Usage("workspace members must be an array".to_string())
        })?
        .iter()
        .map(|member| {
            member.as_str().ok_or_else(|| {
                ScaffoldCommandError::Usage(
                    "workspace members must be an array of strings".to_string(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(members))
}

fn insert_workspace_member(
    source: &str,
    members_offset: usize,
    member_literal: &str,
) -> Result<String, ScaffoldCommandError> {
    let array_start = source[members_offset..]
        .find('[')
        .map(|offset| members_offset + offset)
        .ok_or_else(|| {
            ScaffoldCommandError::Usage("workspace members must be an array".to_string())
        })?;
    let array_end = source[array_start..]
        .find(']')
        .map(|offset| array_start + offset)
        .ok_or_else(|| {
            ScaffoldCommandError::Usage("workspace members array is not closed".to_string())
        })?;
    if !source[array_start + 1..array_end].contains('\n') {
        return Ok(rewrite_single_line_members_array(
            source,
            array_start,
            array_end,
            member_literal,
        ));
    }

    let insert_at = source[..array_end]
        .rfind('\n')
        .map_or(array_end, |offset| offset + 1);
    let mut updated = source.to_string();
    updated.insert_str(insert_at, &format!("    {member_literal},\n"));
    Ok(updated)
}

fn rewrite_single_line_members_array(
    source: &str,
    array_start: usize,
    array_end: usize,
    member_literal: &str,
) -> String {
    let existing = source[array_start + 1..array_end]
        .trim()
        .trim_end_matches(',');
    let replacement = if existing.is_empty() {
        format!("[\n    {member_literal},\n]")
    } else {
        format!("[\n    {existing},\n    {member_literal},\n]")
    };
    let mut updated = source.to_string();
    updated.replace_range(array_start..=array_end, &replacement);
    updated
}

fn line_start_offset(source: &str, line_index: usize) -> usize {
    source
        .match_indices('\n')
        .nth(line_index.saturating_sub(1))
        .map_or(0, |(offset, _)| offset + 1)
}

fn section_end_offset(source: &str, section_start: usize) -> usize {
    source[section_start..]
        .match_indices('\n')
        .find_map(|(offset, _)| {
            let line_start = section_start + offset + 1;
            let line = source[line_start..].lines().next().unwrap_or_default();
            line.trim_start().starts_with('[').then_some(line_start)
        })
        .unwrap_or(source.len())
}

fn find_members_array_offset(section: &str) -> Option<usize> {
    let mut offset = 0;
    for line in section.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = line_without_newline.trim_start();
        let Some((key, _)) = trimmed.split_once('=') else {
            offset += line.len();
            continue;
        };
        if key.trim() == "members" {
            let indentation = line_without_newline.len() - trimmed.len();
            return Some(offset + indentation);
        }
        offset += line.len();
    }
    None
}

fn scaffold_command() -> ClapCommand {
    ClapCommand::new("scaffold")
        .bin_name("canic scaffold")
        .about("Scaffold Canic source files and deployment input")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("canister")
                .about("Create a declared-only canister role")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("fleet-input")
                .about("Materialize node-scaled Fleet funding input")
                .disable_help_flag(true),
        ))
        .after_help(SCAFFOLD_HELP_AFTER)
}

fn app_create_command() -> ClapCommand {
    ClapCommand::new("create")
        .bin_name("canic app create")
        .about("Create a minimal Canic app")
        .disable_help_flag(true)
        .arg(
            Arg::new("name")
                .value_name("name")
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_snake_case_name))
                .help("Snake-case app name to create"),
        )
        .arg(
            flag_arg("yes")
                .long("yes")
                .short('y')
                .help("Create the app without prompting for confirmation"),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Validate and print planned source files without writing them"),
        )
        .after_help(APP_CREATE_HELP_AFTER)
}

fn scaffold_canister_command() -> ClapCommand {
    ClapCommand::new("canister")
        .bin_name("canic scaffold canister")
        .about("Create a declared-only canister role")
        .disable_help_flag(true)
        .arg(
            Arg::new("app")
                .value_name("app")
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_snake_case_name))
                .help("Config-defined app name"),
        )
        .arg(
            Arg::new("role")
                .value_name("role")
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_snake_case_name))
                .help("Snake-case role name to scaffold"),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Validate and print planned source/config writes without changing files"),
        )
        .after_help(SCAFFOLD_CANISTER_HELP_AFTER)
}

fn scaffold_fleet_input_command() -> ClapCommand {
    ClapCommand::new("fleet-input")
        .bin_name("canic scaffold fleet-input")
        .about("Materialize node-scaled Fleet funding input")
        .disable_help_flag(true)
        .arg(
            Arg::new("profile")
                .value_name("single_subnet|preview_multi_subnet|multi_subnet")
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_funding_profile))
                .help("Protected funding profile to materialize"),
        )
        .arg(
            Arg::new("coordinator-node-count")
                .long("coordinator-node-count")
                .value_name("COUNT")
                .value_parser(clap::value_parser!(u64).range(1..))
                .help("Explicit current Coordinator node count for offline authoring"),
        )
        .arg(
            Arg::new("coordinator-subnet")
                .long("coordinator-subnet")
                .value_name("SUBNET")
                .value_parser(clap::builder::ValueParser::new(parse_subnet_id))
                .help("IC Coordinator Subnet resolved through trusted Registry evidence"),
        )
        .arg(
            Arg::new("root-node-count")
                .long("root-node-count")
                .value_name("COUNT")
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(u64).range(1..))
                .help("Explicit current node count for one Root; repeat per Root"),
        )
        .arg(
            Arg::new("root-subnet")
                .long("root-subnet")
                .value_name("SUBNET")
                .action(ArgAction::Append)
                .value_parser(clap::builder::ValueParser::new(parse_subnet_id))
                .help("IC Root Subnet resolved through trusted Registry evidence; repeat per Root"),
        )
        .arg(
            flag_arg("refresh-catalog")
                .long("refresh-catalog")
                .help("Refresh missing or invalid trusted Registry evidence in Subnet-ID mode"),
        )
        .arg(internal_environment_arg())
        .after_help(SCAFFOLD_FLEET_INPUT_HELP_AFTER)
}

fn usage() -> String {
    render_usage(scaffold_command)
}

pub fn app_create_usage() -> String {
    render_usage(app_create_command)
}

fn scaffold_canister_usage() -> String {
    render_usage(scaffold_canister_command)
}

fn scaffold_fleet_input_usage() -> String {
    render_usage(scaffold_fleet_input_command)
}

fn parse_subnet_id(value: &str) -> Result<SubnetId, String> {
    let principal = Principal::from_text(value)
        .map_err(|error| format!("invalid Subnet principal {value:?}: {error}"))?;
    if principal == Principal::anonymous() || principal.to_text() != value {
        return Err(format!(
            "Subnet principal must be canonical and non-anonymous: {value:?}"
        ));
    }
    Ok(SubnetId::from_principal(principal))
}

fn render_scaffold_app_plan(plan: &ScaffoldAppPlan) -> String {
    let mut lines = vec![
        "Planned Canic app scaffold:".to_string(),
        format!("  source: {}", plan.result.app_root.display()),
        format!("  root: {}", plan.result.root_dir.display()),
        format!("  app: {}", plan.result.app_dir.display()),
        format!("  config: {}", plan.result.config_path.display()),
    ];
    append_dry_run_footer(&mut lines);
    lines.push("  would_create:".to_string());
    lines.extend(
        plan.files
            .iter()
            .map(|path| format!("    {}", path.display())),
    );
    lines.join("\n")
}

fn render_canister_scaffold_plan(plan: &CanisterScaffoldPlan) -> String {
    let workspace = plan
        .workspace_manifest_path
        .as_ref()
        .map_or_else(|| "none".to_string(), |path| path.display().to_string());

    let mut lines = vec![
        "Planned Canic canister role scaffold:".to_string(),
        format!("  role: {}.{}", plan.result.app, plan.result.role),
        format!("  package: {}", plan.result.package),
        format!("  crate: {}", plan.result.canister_dir.display()),
        format!("  config: {}", plan.result.config_path.display()),
        format!("  workspace_member: {}", plan.workspace_member),
        format!("  would_write_workspace: {workspace}"),
        "  would_write_role_files: Cargo.toml, build.rs, src/lib.rs".to_string(),
    ];
    append_dry_run_footer(&mut lines);
    lines.join("\n")
}

fn render_fleet_input_scaffold(
    scaffold: &FleetFundingProfileScaffold,
    node_count_source: &str,
) -> String {
    let mut lines = vec![
        "Fleet-input funding scaffold:".to_string(),
        "  authority: authoring_only".to_string(),
        "  funded_identity_required: false".to_string(),
        format!(
            "  funding_profile: {}",
            funding_profile_name(scaffold.profile)
        ),
        format!(
            "  coordinator_node_count: {}",
            scaffold.coordinator_node_count
        ),
        format!("  root_count: {}", scaffold.roots.len()),
        format!("  node_count_source: {node_count_source}"),
        format!("  standard_node_count: {STANDARD_PROFILE_NODE_COUNT}"),
        format!("  rounding_quantum_cycles: {PROFILE_CYCLE_ROUNDING_QUANTUM}"),
        format!(
            "  operator_creation_amount_cycles: {}",
            scaffold.operator_creation_amount_cycles
        ),
        format!(
            "  operator_creation_count: {}",
            scaffold.operator_creation_count
        ),
        format!(
            "  operator_creation_fee_cycles: {}",
            scaffold.operator_creation_fee_cycles
        ),
        format!(
            "  maximum_operator_debit_cycles: {}",
            scaffold.maximum_operator_debit_cycles
        ),
        String::new(),
        "Formulas:".to_string(),
    ];
    lines.extend(scaffold.formulas.iter().map(|formula| {
        format!(
            "  {} = {} = {} cycles",
            formula.field, formula.expression, formula.result
        )
    }));
    lines.extend([
        String::new(),
        "Exact funding TOML:".to_string(),
        "# Funding-only authoring fragment; this is not an install-admission result.".to_string(),
        format!(
            "funding_profile = \"{}\"",
            funding_profile_name(scaffold.profile)
        ),
        String::new(),
        "[coordinator.creation_funding]".to_string(),
        "kind = \"cycles\"".to_string(),
        format!("cycles = \"{}\"", scaffold.coordinator.creation_cycles),
        String::new(),
        "[coordinator.root_funding]".to_string(),
        format!(
            "minimum_reserve_cycles = \"{}\"",
            scaffold.coordinator.minimum_reserve_cycles
        ),
        format!("window_secs = {}", scaffold.coordinator.window_secs),
        format!(
            "maximum_cycles = \"{}\"",
            scaffold.coordinator.maximum_cycles
        ),
        format!(
            "maximum_automatic_grants = {}",
            scaffold.coordinator.maximum_automatic_grants
        ),
        format!(
            "maximum_automatic_cycles = \"{}\"",
            scaffold.coordinator.maximum_automatic_cycles
        ),
    ]);
    for (index, root) in scaffold.roots.iter().enumerate() {
        append_fleet_input_root_toml(&mut lines, index, root);
    }
    lines.extend([
        String::new(),
        "Next:".to_string(),
        "  merge this fragment with exact operator, Subnet, admission, pool and limit authority"
            .to_string(),
        "  run canic deploy plan with the complete Fleet input for live admission".to_string(),
    ]);
    lines.join("\n")
}

fn append_fleet_input_root_toml(
    lines: &mut Vec<String>,
    index: usize,
    root: &FleetFundingProfileRootScaffold,
) {
    lines.extend([
        String::new(),
        format!("# Root {}: node_count={}", index + 1, root.node_count),
        "[[fleet_subnet_roots]]".to_string(),
        String::new(),
        "[fleet_subnet_roots.root_funding]".to_string(),
        format!("request_threshold = \"{}\"", root.request_threshold_cycles),
        format!("target_balance = \"{}\"", root.target_balance_cycles),
        format!("cooldown_secs = {}", root.cooldown_secs),
        format!("window_secs = {}", root.window_secs),
        format!("maximum_cycles = \"{}\"", root.maximum_cycles),
        format!(
            "maximum_automatic_grants = {}",
            root.maximum_automatic_grants
        ),
        format!(
            "maximum_automatic_cycles = \"{}\"",
            root.maximum_automatic_cycles
        ),
        String::new(),
        "[fleet_subnet_roots.root_creation_funding]".to_string(),
        "kind = \"cycles\"".to_string(),
        format!("cycles = \"{}\"", root.root_creation_cycles),
        String::new(),
        "[fleet_subnet_roots.wasm_store_creation_funding]".to_string(),
        "kind = \"cycles\"".to_string(),
        format!("cycles = \"{}\"", root.wasm_store_creation_cycles),
    ]);
}

fn parse_funding_profile(value: &str) -> Result<FleetFundingProfile, String> {
    match value {
        "single_subnet" => Ok(FleetFundingProfile::SingleSubnet),
        "preview_multi_subnet" => Ok(FleetFundingProfile::PreviewMultiSubnet),
        "multi_subnet" => Ok(FleetFundingProfile::MultiSubnet),
        _ => Err(format!("unknown funding profile {value:?}")),
    }
}

const fn funding_profile_name(profile: FleetFundingProfile) -> &'static str {
    match profile {
        FleetFundingProfile::SingleSubnet => "single_subnet",
        FleetFundingProfile::PreviewMultiSubnet => "preview_multi_subnet",
        FleetFundingProfile::MultiSubnet => "multi_subnet",
    }
}

fn confirm_scaffold<R, W>(
    options: &ScaffoldOptions,
    workspace_root: &Path,
    mut reader: R,
    mut writer: W,
) -> Result<(), ScaffoldCommandError>
where
    R: BufRead,
    W: Write,
{
    let app_root = workspace_root.join("apps").join(&options.name);
    if app_root.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            app_root.display().to_string(),
        ));
    }

    writeln!(writer, "Create Canic app?")?;
    writeln!(writer, "  app:     {}", options.name)?;
    writeln!(writer, "  target:  {}", app_root.display())?;
    writeln!(writer, "  install: canic install {} <fleet>", options.name)?;
    write!(writer, "Continue? [y/N] ")?;
    writer.flush()?;

    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    if matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        return Ok(());
    }

    Err(ScaffoldCommandError::Cancelled)
}

fn scaffold_workspace_root() -> Result<PathBuf, ScaffoldCommandError> {
    current_canic_workspace_root().map_err(Into::into)
}

fn selected_app_config_path(
    workspace_root: &Path,
    app: &str,
) -> Result<PathBuf, ScaffoldCommandError> {
    let choices = discover_workspace_canic_config_choices(workspace_root)?;
    if choices.is_empty() {
        return Err(ScaffoldCommandError::NoConfigChoices);
    }

    select_discovered_app_config_path(&choices, app)?
        .ok_or_else(|| ScaffoldCommandError::UnknownApp(app.to_string()))
}

fn parse_snake_case_name(name: &str) -> Result<String, String> {
    if !is_ascii_snake_case(name) {
        return Err(format!("name must be snake_case: {name}"));
    }

    Ok(name.to_string())
}

fn canister_package_name(app: &str, role: &str) -> String {
    format!("canister_{app}_{role}").replace('-', "_")
}

fn display_path(workspace_root: &Path, path: &Path) -> PathBuf {
    PathBuf::from(display_workspace_path(workspace_root, path))
}

fn toml_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

fn write_new_file(path: &Path, contents: &str) -> Result<(), ScaffoldCommandError> {
    if path.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            path.display().to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents).map_err(ScaffoldCommandError::from)
}

fn canister_cargo_toml(app: &str, role: &str) -> String {
    let canic_version = env!("CARGO_PKG_VERSION");
    let package_name = canister_package_name(app, role);
    format!(
        r#"[package]
name = "{package_name}"
edition = "2024"
rust-version = "1.91.0"
version = "0.1.0"
publish = false

[package.metadata.canic]
app = "{app}"
role = "{role}"

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = {{ version = "0.10", default-features = false }}
# Add runtime Canic features here when canic.toml enables auth settings.
canic = "{canic_version}"
ic-cdk = "0.20"

[build-dependencies]
canic = "{canic_version}"
"#
    )
}

fn canic_toml(name: &str) -> String {
    format!(
        r#"# Minimal Canic App config.

[app]
name = "{name}"

[auth.delegated_tokens]
enabled = false

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"



[component_specs.app]
component_role = "app"
maximum_instances = 1
"#
    )
}

fn root_cargo_toml(name: &str) -> String {
    let canic_version = env!("CARGO_PKG_VERSION");
    format!(
        r#"[package]
name = "canister_{name}_root"
edition = "2024"
rust-version = "1.91.0"
version = "0.1.0"
publish = false

[package.metadata.canic]
app = "{name}"
role = "root"

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = {{ version = "0.10", default-features = false }}
# Add runtime Canic features here when canic.toml enables auth settings.
canic = {{ version = "{canic_version}", features = ["control-plane"] }}
ic-cdk = "0.20"

[build-dependencies]
canic = "{canic_version}"
"#
    )
}

fn app_cargo_toml(name: &str) -> String {
    let canic_version = env!("CARGO_PKG_VERSION");
    format!(
        r#"[package]
name = "canister_{name}_app"
edition = "2024"
rust-version = "1.91.0"
version = "0.1.0"
publish = false

[package.metadata.canic]
app = "{name}"
role = "app"

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = {{ version = "0.10", default-features = false }}
# Add runtime Canic features here when canic.toml enables auth settings.
canic = "{canic_version}"
ic-cdk = "0.20"

[build-dependencies]
canic = "{canic_version}"
"#
    )
}

const ROOT_BUILD_RS: &str = r#"fn main() {
    canic::build!("../canic.toml");
}
"#;

const APP_BUILD_RS: &str = r#"fn main() {
    canic::build!("../canic.toml");
}
"#;

const CANISTER_BUILD_RS: &str = r#"fn main() {
    canic::build!("../canic.toml");
}
"#;

const ROOT_LIB_RS: &str = r"#![expect(clippy::unused_async)]

//
// CANIC
//

canic::start!();

/// Run no-op setup for this scaffolded root.
async fn canic_setup() {}

/// Run no-op install handling for this scaffolded root.
async fn canic_install() {}

/// Run no-op upgrade handling for this scaffolded root.
async fn canic_upgrade() {}

canic::finish!();
";

const APP_LIB_RS: &str = r"#![expect(clippy::unused_async)]

/// Run no-op setup for this scaffolded app.
async fn canic_setup() {}

/// Accept no install payload for this scaffolded app.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for this scaffolded app.
async fn canic_upgrade() {}

canic::start!();

canic::finish!();
";

const CANISTER_LIB_RS: &str = r"#![expect(clippy::unused_async)]

canic::start!();

pub async fn canic_setup() {}

pub async fn canic_install(_: Option<Vec<u8>>) {}

pub async fn canic_upgrade() {}

canic::finish!();
";
