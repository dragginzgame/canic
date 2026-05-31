use crate::{
    cli::clap::{
        parse_matches, parse_subcommand, passthrough_subcommand, path_option, string_option,
        value_arg,
    },
    cli::defaults::local_network,
    cli::globals::internal_network_arg,
    cli::help::print_help_or_version,
    output, scaffold, version_text,
};
use canic_host::{
    adoption::{
        AdoptionArtifactStateV1, AdoptionAuthorityStateV1, AdoptionClassificationV1,
        AdoptionDeclarationStateV1, AdoptionMatchConfidenceV1, AdoptionObservationStateV1,
        AdoptionOperatorActionRequirementV1, AdoptionPackageMetadataV1, AdoptionPackageStateV1,
        AdoptionProfileV1, AdoptionRecommendationSeverityV1, AdoptionReportError,
        AdoptionReportRequest, AdoptionReportV1, AdoptionSuggestedActionAvailabilityV1,
        AdoptionSuggestedActionEffectV1, AdoptionSuggestedActionSupportV1, AdoptionTopologyStateV1,
        adoption_report_from_config_source,
    },
    deployment_truth::{DeploymentInventoryV1, RoleArtifactManifestV1, RoleArtifactV1},
    evidence_envelope::{
        CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
        EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, InputFingerprintV1,
        PayloadSchemaRefV1, adoption_report_schema, command_path_for_root, deployment_check_schema,
        evidence_envelope_schema, file_input_fingerprint, json_payload_sha256,
    },
    icp_config::{IcpConfigError, IcpProjectConfigReport, inspect_canic_icp_yaml},
    install_root::{
        current_canic_project_root, discover_current_canic_config_choices, project_fleet_roots,
    },
    release_set::{
        AttachedFleetRole, ConfiguredRoleLifecycle, DeclaredFleetRole, RenamedFleetRole,
        attach_fleet_role, configured_deployable_roles, configured_fleet_name,
        configured_role_lifecycle, declare_fleet_role, display_workspace_path,
        matching_fleet_config_paths, rename_fleet_role,
    },
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use serde::de::DeserializeOwned;
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const FLEET_HEADER: &str = "FLEET";
const NETWORK_HEADER: &str = "NETWORK";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const ROLE_PREVIEW_LIMIT: usize = 6;
const FLEET_HELP_AFTER: &str = "\
Examples:
  canic fleet list
  canic fleet adoption report demo --profile brownfield
  canic fleet role declare demo store --package store
  canic fleet role attach demo store --subnet prime
  canic fleet role rename demo hub router
  canic fleet role list demo
  canic fleet role inspect demo app
  canic fleet create demo
  canic fleet check test
  canic fleet delete demo";
const FLEET_LIST_HELP_AFTER: &str = "\
Examples:
  canic fleet list

Commands that operate on one fleet take the fleet name as a positional argument.";
const FLEET_CHECK_HELP_AFTER: &str = "\
Examples:
  canic fleet check test";
const FLEET_DELETE_HELP_AFTER: &str = "\
Examples:
  canic fleet delete demo

This removes the matching config-defined fleet directory after you type the
fleet name exactly.";
const FLEET_ROLE_HELP_AFTER: &str = "\
Examples:
  canic fleet role declare demo store --package store
  canic fleet role attach demo store --subnet prime
  canic fleet role rename demo hub router
  canic fleet role list demo
  canic fleet role inspect demo app";
const FLEET_ROLE_LIST_HELP_AFTER: &str = "\
Examples:
  canic fleet role list demo";
const FLEET_ROLE_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic fleet role inspect demo app";
const FLEET_ROLE_DECLARE_HELP_AFTER: &str = "\
Examples:
  canic fleet role declare demo store --package store";
const FLEET_ROLE_ATTACH_HELP_AFTER: &str = "\
Examples:
  canic fleet role attach demo store --subnet prime
  canic fleet role attach demo worker --subnet prime --kind replica";
const FLEET_ROLE_RENAME_HELP_AFTER: &str = "\
Examples:
  canic fleet role rename demo hub router";
const FLEET_ADOPTION_HELP_AFTER: &str = "\
Examples:
  canic fleet adoption report demo --profile brownfield
  canic fleet adoption report demo --profile minimal --format json
  canic fleet adoption report demo --profile minimal --format envelope-json

Adoption commands are read-only. They report recommendations and never update
fleet config, package manifests, topology, deployments, or controllers.";
const FLEET_ADOPTION_REPORT_HELP_AFTER: &str = "\
Examples:
  canic fleet adoption report demo --profile brownfield
  canic fleet adoption report demo --profile minimal --format json
  canic fleet adoption report demo --profile minimal --format envelope-json
  canic fleet adoption report demo --profile partial --deployment-check check.json
  canic fleet adoption report demo --profile partial --inventory inventory.json
  canic fleet adoption report demo --profile partial --cargo-metadata cargo-metadata.json
  canic fleet adoption report demo --profile partial --output adoption-report.txt

Profiles: brownfield, partial, standalone, leaf-only, hybrid-external-wasm,
minimal. --format json emits the raw experimental adoption report payload;
--format envelope-json emits the stable CI/GitOps evidence envelope with the
raw adoption payload nested inside. The report is read-only; --output writes
only the requested report artifact. Evidence inputs are JSON files and are
read-only. Use either --inventory or --deployment-check, not both. Use either
--package-metadata or --cargo-metadata, not both. Deployment-check evidence
also supplies plan role artifacts when present.";

///
/// FleetCommandError
///

#[derive(Debug, ThisError)]
pub enum FleetCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("no Canic fleet configs found under fleets; run canic fleet create <name>")]
    NoConfigChoices,

    #[error("unknown fleet {0}; run canic fleet list to inspect config-defined fleets")]
    UnknownFleet(String),

    #[error(
        "multiple configs declare fleet {0}; use distinct [fleet].name values before selecting it"
    )]
    DuplicateFleet(String),

    #[error("fleet delete cancelled")]
    DeleteCancelled,

    #[error("refusing to delete fleet {fleet}; target {target} is not under a fleet config root")]
    UnsafeDeleteTarget { fleet: String, target: String },

    #[error("fleet {0} config does not have a parent directory")]
    MissingFleetDirectory(String),

    #[error("unknown role {role} in fleet {fleet}; run canic fleet role list {fleet}")]
    UnknownRole { fleet: String, role: String },

    #[error("fleet create: {0}")]
    Create(String),

    #[error(transparent)]
    AdoptionReport(#[from] AdoptionReportError),

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),
}

///
/// FleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetOptions {
    network: String,
}

///
/// DeleteFleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeleteFleetOptions {
    fleet: String,
}

///
/// FleetCheckOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetCheckOptions {
    fleet: String,
}

///
/// RoleListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RoleListOptions {
    fleet: String,
}

///
/// RoleInspectOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RoleInspectOptions {
    fleet: String,
    role: String,
}

///
/// RoleDeclareOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RoleDeclareOptions {
    fleet: String,
    role: String,
    package: String,
}

///
/// RoleAttachOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RoleAttachOptions {
    fleet: String,
    role: String,
    subnet: String,
    kind: String,
}

///
/// RoleRenameOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RoleRenameOptions {
    fleet: String,
    old_role: String,
    new_role: String,
}

///
/// AdoptionReportOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct AdoptionReportOptions {
    fleet: String,
    profile: AdoptionProfileV1,
    format: AdoptionReportFormat,
    deployment_check: Option<PathBuf>,
    inventory: Option<PathBuf>,
    artifact_manifest: Option<PathBuf>,
    cargo_metadata: Option<PathBuf>,
    package_metadata: Option<PathBuf>,
    output: Option<PathBuf>,
}

///
/// AdoptionReportFormat
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AdoptionReportFormat {
    Text,
    Json,
    EnvelopeJson,
}

///
/// FleetListRow
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetListRow {
    fleet: String,
    network: String,
    config: String,
    canisters: String,
}

pub fn run<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(fleet_command(), args).map_err(|_| FleetCommandError::Usage(usage()))? {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "create" => run_create(args),
            "check" => run_check(args),
            "delete" => run_delete(args),
            "list" => run_list(args),
            "adoption" => run_adoption(args),
            "role" => run_role(args),
            _ => unreachable!("fleet dispatch command only defines known commands"),
        },
    }
}

fn run_adoption<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, adoption_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(fleet_adoption_command(), args)
        .map_err(|_| FleetCommandError::Usage(adoption_usage()))?
    {
        None => {
            println!("{}", adoption_usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "report" => run_adoption_report(args),
            _ => unreachable!("fleet adoption dispatch command only defines known commands"),
        },
    }
}

fn run_adoption_report<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, adoption_report_usage, version_text()) {
        return Ok(());
    }

    let options = AdoptionReportOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let generated_at = current_adoption_report_generated_at()?;
    let report = build_adoption_report_from_config_path(&config_path, &options, &generated_at)?;
    write_adoption_report(&config_path, &options, &report)
}

fn run_role<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(fleet_role_command(), args)
        .map_err(|_| FleetCommandError::Usage(role_usage()))?
    {
        None => {
            println!("{}", role_usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "declare" => run_role_declare(args),
            "attach" => run_role_attach(args),
            "rename" => run_role_rename(args),
            "list" => run_role_list(args),
            "inspect" => run_role_inspect(args),
            _ => unreachable!("fleet role dispatch command only defines known commands"),
        },
    }
}

fn run_role_declare<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_declare_usage, version_text()) {
        return Ok(());
    }

    let options = RoleDeclareOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let project_root = current_canic_project_root()?;
    let declared = declare_fleet_role(
        &config_path,
        &options.fleet,
        &options.role,
        &options.package,
    )?;
    println!(
        "{}",
        render_declared_role(&declared, &project_root, &config_path)
    );
    Ok(())
}

fn run_role_attach<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_attach_usage, version_text()) {
        return Ok(());
    }

    let options = RoleAttachOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let project_root = current_canic_project_root()?;
    let attached = attach_fleet_role(
        &config_path,
        &options.fleet,
        &options.role,
        &options.subnet,
        &options.kind,
    )?;
    println!(
        "{}",
        render_attached_role(&attached, &project_root, &config_path)
    );
    Ok(())
}

fn run_role_rename<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_rename_usage, version_text()) {
        return Ok(());
    }

    let options = RoleRenameOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let project_root = current_canic_project_root()?;
    let renamed = rename_fleet_role(
        &config_path,
        &options.fleet,
        &options.old_role,
        &options.new_role,
    )?;
    println!(
        "{}",
        render_renamed_role(&renamed, &project_root, &config_path)
    );
    Ok(())
}

fn run_role_list<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_list_usage, version_text()) {
        return Ok(());
    }

    let options = RoleListOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let rows = configured_role_lifecycle(&config_path)?;
    println!("{}", render_role_lifecycle_rows(&rows));
    Ok(())
}

fn run_role_inspect<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_inspect_usage, version_text()) {
        return Ok(());
    }

    let options = RoleInspectOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let rows = configured_role_lifecycle(&config_path)?;
    let row = rows
        .iter()
        .find(|row| row.role == options.role)
        .ok_or_else(|| FleetCommandError::UnknownRole {
            fleet: options.fleet.clone(),
            role: options.role.clone(),
        })?;
    println!("{}", render_role_inspection(row));
    Ok(())
}

fn run_check<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, check_usage, version_text()) {
        return Ok(());
    }

    let options = FleetCheckOptions::parse(args)?;
    let report = inspect_canic_icp_yaml(Some(&options.fleet))?;
    print_config_report(&report);
    Ok(())
}

fn run_create<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, create_usage, version_text()) {
        return Ok(());
    }

    scaffold::run_fleet_create(args).map_err(|err| FleetCommandError::Create(err.to_string()))
}

fn run_list<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, list_usage, version_text()) {
        return Ok(());
    }

    let options = FleetOptions::parse(args)?;
    let project_root = current_canic_project_root()?;
    let choices = discover_config_choices()?;
    if choices.is_empty() {
        return Err(FleetCommandError::NoConfigChoices);
    }
    println!(
        "{}",
        render_fleet_list(&project_root, &choices, &options.network)
    );
    Ok(())
}

fn run_delete<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, delete_usage, version_text()) {
        return Ok(());
    }

    let options = DeleteFleetOptions::parse(args)?;
    let project_root = current_canic_project_root()?;
    let target = delete_target_dir(&project_root, &options.fleet)?;
    confirm_delete_fleet(&options.fleet, &target, io::stdin().lock(), io::stdout())?;
    fs::remove_dir_all(&target)?;

    println!("Deleted Canic fleet:");
    println!("  fleet: {}", options.fleet);
    println!(
        "  path:  {}",
        display_workspace_path(&project_root, &target)
    );
    Ok(())
}

impl FleetOptions {
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_list_command(), args)
            .map_err(|_| FleetCommandError::Usage(list_usage()))?;

        Ok(Self {
            network: string_option(&matches, "network").unwrap_or_else(local_network),
        })
    }
}

impl DeleteFleetOptions {
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_delete_command(), args)
            .map_err(|_| FleetCommandError::Usage(delete_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
        })
    }
}

impl FleetCheckOptions {
    #[cfg(test)]
    fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_check_command(), args)
            .map_err(|_| FleetCommandError::Usage(check_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
        })
    }
}

impl RoleListOptions {
    #[cfg(test)]
    fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_list_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_list_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
        })
    }
}

impl RoleInspectOptions {
    #[cfg(test)]
    fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_inspect_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_inspect_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            role: string_option(&matches, "role").expect("clap requires role"),
        })
    }
}

impl RoleDeclareOptions {
    #[cfg(test)]
    fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_declare_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_declare_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            role: string_option(&matches, "role").expect("clap requires role"),
            package: string_option(&matches, "package").expect("clap requires package"),
        })
    }
}

impl RoleAttachOptions {
    #[cfg(test)]
    fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_attach_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_attach_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            role: string_option(&matches, "role").expect("clap requires role"),
            subnet: string_option(&matches, "subnet").expect("clap requires subnet"),
            kind: string_option(&matches, "kind").unwrap_or_else(|| "singleton".to_string()),
        })
    }
}

impl RoleRenameOptions {
    #[cfg(test)]
    fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_rename_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_rename_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            old_role: string_option(&matches, "old-role").expect("clap requires old role"),
            new_role: string_option(&matches, "new-role").expect("clap requires new role"),
        })
    }
}

impl AdoptionReportOptions {
    #[cfg(test)]
    fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_adoption_report_command(), args)
            .map_err(|_| FleetCommandError::Usage(adoption_report_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            profile: parse_adoption_profile(
                string_option(&matches, "profile").as_deref(),
                adoption_report_usage,
            )?,
            format: parse_adoption_report_format(
                string_option(&matches, "format").as_deref(),
                adoption_report_usage,
            )?,
            deployment_check: path_option(&matches, "deployment-check"),
            inventory: path_option(&matches, "inventory"),
            artifact_manifest: path_option(&matches, "artifact-manifest"),
            cargo_metadata: path_option(&matches, "cargo-metadata"),
            package_metadata: path_option(&matches, "package-metadata"),
            output: path_option(&matches, "output"),
        })
    }
}

fn parse_adoption_profile(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<AdoptionProfileV1, FleetCommandError> {
    match value {
        Some("brownfield") => Ok(AdoptionProfileV1::Brownfield),
        Some("partial") => Ok(AdoptionProfileV1::Partial),
        Some("standalone") => Ok(AdoptionProfileV1::Standalone),
        Some("leaf-only") => Ok(AdoptionProfileV1::LeafOnly),
        Some("hybrid-external-wasm") => Ok(AdoptionProfileV1::HybridExternalWasm),
        Some("minimal") => Ok(AdoptionProfileV1::Minimal),
        Some(other) => Err(FleetCommandError::Usage(format!(
            "invalid adoption profile: {other}\n\n{}",
            usage()
        ))),
        None => Err(FleetCommandError::Usage(usage())),
    }
}

fn parse_adoption_report_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<AdoptionReportFormat, FleetCommandError> {
    match value.unwrap_or("text") {
        "text" => Ok(AdoptionReportFormat::Text),
        "json" => Ok(AdoptionReportFormat::Json),
        "envelope-json" => Ok(AdoptionReportFormat::EnvelopeJson),
        other => Err(FleetCommandError::Usage(format!(
            "invalid adoption report output format: {other}\n\n{}",
            usage()
        ))),
    }
}

fn discover_config_choices() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    discover_current_canic_config_choices()
}

fn delete_target_dir(workspace_root: &Path, fleet: &str) -> Result<PathBuf, FleetCommandError> {
    let choices = discover_config_choices()?;
    delete_target_dir_from_choices(workspace_root, &choices, fleet)
}

fn selected_fleet_config_path(fleet: &str) -> Result<PathBuf, FleetCommandError> {
    let choices = discover_config_choices()?;
    if choices.is_empty() {
        return Err(FleetCommandError::NoConfigChoices);
    }
    selected_fleet_config_path_from_choices(&choices, fleet)
}

fn selected_fleet_config_path_from_choices(
    choices: &[PathBuf],
    fleet: &str,
) -> Result<PathBuf, FleetCommandError> {
    let matches = matching_fleet_config_paths(choices, fleet);

    match matches.as_slice() {
        [] => Err(FleetCommandError::UnknownFleet(fleet.to_string())),
        [path] => Ok(path.clone()),
        _ => Err(FleetCommandError::DuplicateFleet(fleet.to_string())),
    }
}

fn delete_target_dir_from_choices(
    workspace_root: &Path,
    choices: &[PathBuf],
    fleet: &str,
) -> Result<PathBuf, FleetCommandError> {
    let matches = matching_fleet_config_paths(choices, fleet);

    let config_path = match matches.as_slice() {
        [] => return Err(FleetCommandError::UnknownFleet(fleet.to_string())),
        [path] => path,
        _ => return Err(FleetCommandError::DuplicateFleet(fleet.to_string())),
    };
    let target = config_path
        .parent()
        .ok_or_else(|| FleetCommandError::MissingFleetDirectory(fleet.to_string()))?
        .to_path_buf();
    if !is_safe_delete_target(workspace_root, &target) {
        return Err(FleetCommandError::UnsafeDeleteTarget {
            fleet: fleet.to_string(),
            target: target.display().to_string(),
        });
    }

    Ok(target)
}

// Restrict destructive delete targets to one fleet directory, never the fleet root.
fn is_safe_delete_target(workspace_root: &Path, target: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(target) else {
        return false;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return false;
    }

    let Ok(target) = target.canonicalize() else {
        return false;
    };
    project_fleet_roots(workspace_root)
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| target != root && target.starts_with(root))
}

// Confirm destructive fleet directory deletion by requiring the exact fleet name.
fn confirm_delete_fleet<R, W>(
    fleet: &str,
    target: &Path,
    mut reader: R,
    mut writer: W,
) -> Result<(), FleetCommandError>
where
    R: BufRead,
    W: Write,
{
    writeln!(writer, "Delete Canic fleet?")?;
    writeln!(writer, "  fleet: {fleet}")?;
    writeln!(writer, "  target: {}", target.display())?;
    writeln!(writer, "This will permanently remove the fleet directory.")?;
    write!(writer, "Type the fleet name to confirm: ")?;
    writer.flush()?;

    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    if answer.trim() == fleet {
        return Ok(());
    }

    Err(FleetCommandError::DeleteCancelled)
}

fn fleet_command() -> ClapCommand {
    ClapCommand::new("fleet")
        .bin_name("canic fleet")
        .about("Manage Canic fleets")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Check icp.yaml for one Canic fleet")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("create")
                .about("Create a minimal Canic fleet")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List config-defined Canic fleets")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("adoption")
                .about("Report safe onboarding recommendations")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("role")
                .about("Manage fleet role lifecycle")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("delete")
                .about("Delete a config-defined Canic fleet")
                .disable_help_flag(true),
        ))
        .after_help(FLEET_HELP_AFTER)
}

fn fleet_adoption_command() -> ClapCommand {
    ClapCommand::new("adoption")
        .bin_name("canic fleet adoption")
        .about("Report safe onboarding recommendations")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("report")
                .about("Generate a read-only adoption report")
                .disable_help_flag(true),
        ))
        .after_help(FLEET_ADOPTION_HELP_AFTER)
}

fn fleet_adoption_report_command() -> ClapCommand {
    ClapCommand::new("report")
        .bin_name("canic fleet adoption report")
        .about("Generate a read-only adoption report")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name"),
        )
        .arg(
            clap::Arg::new("profile")
                .long("profile")
                .value_name("profile")
                .required(true)
                .help("Adoption profile to evaluate"),
        )
        .arg(
            clap::Arg::new("format")
                .long("format")
                .value_name("text|json|envelope-json")
                .help("Report output format"),
        )
        .arg(
            clap::Arg::new("inventory")
                .long("inventory")
                .value_name("path")
                .help("Read DeploymentInventoryV1 JSON evidence from this path"),
        )
        .arg(
            clap::Arg::new("deployment-check")
                .long("deployment-check")
                .value_name("path")
                .help("Read inventory evidence from a DeploymentCheckV1 JSON artifact"),
        )
        .arg(
            clap::Arg::new("artifact-manifest")
                .long("artifact-manifest")
                .value_name("path")
                .help("Read RoleArtifactManifestV1 JSON evidence from this path"),
        )
        .arg(
            clap::Arg::new("package-metadata")
                .long("package-metadata")
                .value_name("path")
                .help("Read AdoptionPackageMetadataV1 JSON array evidence from this path"),
        )
        .arg(
            clap::Arg::new("cargo-metadata")
                .long("cargo-metadata")
                .value_name("path")
                .help("Read package metadata evidence from cargo metadata JSON"),
        )
        .arg(
            clap::Arg::new("output")
                .long("output")
                .value_name("path")
                .help("Write the report artifact to this path"),
        )
        .after_help(FLEET_ADOPTION_REPORT_HELP_AFTER)
}

fn fleet_role_command() -> ClapCommand {
    ClapCommand::new("role")
        .bin_name("canic fleet role")
        .about("Manage fleet role lifecycle")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("declare")
                .about("Declare an existing package-backed role")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("attach")
                .about("Attach a declared role to direct topology")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("rename")
                .about("Rename a declared fleet role")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List declared fleet roles")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inspect")
                .about("Inspect one declared fleet role")
                .disable_help_flag(true),
        ))
        .after_help(FLEET_ROLE_HELP_AFTER)
}

fn fleet_role_declare_command() -> ClapCommand {
    ClapCommand::new("declare")
        .bin_name("canic fleet role declare")
        .about("Declare an existing package-backed role")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name"),
        )
        .arg(
            value_arg("role")
                .value_name("role")
                .required(true)
                .help("Local role name"),
        )
        .arg(
            clap::Arg::new("package")
                .long("package")
                .value_name("path")
                .required(true)
                .help("Package path recorded in [roles.<role>]"),
        )
        .after_help(FLEET_ROLE_DECLARE_HELP_AFTER)
}

fn fleet_role_attach_command() -> ClapCommand {
    ClapCommand::new("attach")
        .bin_name("canic fleet role attach")
        .about("Attach a declared role to direct topology")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name"),
        )
        .arg(
            value_arg("role")
                .value_name("role")
                .required(true)
                .help("Local role name"),
        )
        .arg(
            clap::Arg::new("subnet")
                .long("subnet")
                .value_name("subnet")
                .required(true)
                .help("Subnet to attach the role under"),
        )
        .arg(
            clap::Arg::new("kind")
                .long("kind")
                .value_name("kind")
                .help("Canister kind: singleton, shard, replica, or instance"),
        )
        .after_help(FLEET_ROLE_ATTACH_HELP_AFTER)
}

fn fleet_role_rename_command() -> ClapCommand {
    ClapCommand::new("rename")
        .bin_name("canic fleet role rename")
        .about("Rename a declared fleet role")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name"),
        )
        .arg(
            value_arg("old-role")
                .value_name("old-role")
                .required(true)
                .help("Existing local role name"),
        )
        .arg(
            value_arg("new-role")
                .value_name("new-role")
                .required(true)
                .help("New local role name"),
        )
        .after_help(FLEET_ROLE_RENAME_HELP_AFTER)
}

fn fleet_role_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic fleet role list")
        .about("List declared fleet roles")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name"),
        )
        .after_help(FLEET_ROLE_LIST_HELP_AFTER)
}

fn fleet_role_inspect_command() -> ClapCommand {
    ClapCommand::new("inspect")
        .bin_name("canic fleet role inspect")
        .about("Inspect one declared fleet role")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name"),
        )
        .arg(
            value_arg("role")
                .value_name("role")
                .required(true)
                .help("Local role name"),
        )
        .after_help(FLEET_ROLE_INSPECT_HELP_AFTER)
}

fn fleet_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic fleet list")
        .about("List config-defined Canic fleets")
        .disable_help_flag(true)
        .arg(internal_network_arg())
        .after_help(FLEET_LIST_HELP_AFTER)
}

fn fleet_check_command() -> ClapCommand {
    ClapCommand::new("check")
        .bin_name("canic fleet check")
        .about("Check icp.yaml for one Canic fleet")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("name")
                .required(true)
                .help("Config-defined fleet name to check"),
        )
        .after_help(FLEET_CHECK_HELP_AFTER)
}

fn fleet_delete_command() -> ClapCommand {
    ClapCommand::new("delete")
        .bin_name("canic fleet delete")
        .about("Delete a config-defined Canic fleet directory")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("name")
                .required(true)
                .help("Config-defined fleet name to delete"),
        )
        .after_help(FLEET_DELETE_HELP_AFTER)
}

fn render_fleet_list(workspace_root: &Path, choices: &[PathBuf], network: &str) -> String {
    render_fleet_rows(fleet_list_rows(workspace_root, choices, network))
}

fn render_fleet_rows(rows: Vec<FleetListRow>) -> String {
    let rows = rows
        .into_iter()
        .map(|row| [row.fleet, row.network, row.config, row.canisters])
        .collect::<Vec<_>>();
    render_table(
        &[
            FLEET_HEADER,
            NETWORK_HEADER,
            CONFIG_HEADER,
            CANISTERS_HEADER,
        ],
        &rows,
        &[ColumnAlign::Left; 4],
    )
}

fn fleet_list_rows(workspace_root: &Path, choices: &[PathBuf], network: &str) -> Vec<FleetListRow> {
    choices
        .iter()
        .map(|path| fleet_list_row(workspace_root, path, network))
        .collect()
}

fn fleet_list_row(workspace_root: &Path, path: &Path, network: &str) -> FleetListRow {
    let fleet = configured_fleet_name(path).unwrap_or_else(|_| "invalid config".to_string());
    FleetListRow {
        network: network.to_string(),
        fleet,
        config: display_workspace_path(workspace_root, path),
        canisters: configured_deployable_roles(path).map_or_else(
            |_| "invalid config".to_string(),
            |roles| format_canister_summary(&roles),
        ),
    }
}

fn format_canister_summary(roles: &[String]) -> String {
    if roles.is_empty() {
        return "0".to_string();
    }

    let preview = roles
        .iter()
        .take(ROLE_PREVIEW_LIMIT)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if roles.len() > ROLE_PREVIEW_LIMIT {
        ", ..."
    } else {
        ""
    };

    format!("{} ({preview}{suffix})", roles.len())
}

fn render_role_lifecycle_rows(rows: &[ConfiguredRoleLifecycle]) -> String {
    let rows = rows
        .iter()
        .map(|row| {
            [
                row.display.clone(),
                row.package.clone(),
                row.state.clone(),
                row.topology.clone().unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &["ROLE", "PACKAGE", "STATE", "TOPOLOGY"],
        &rows,
        &[ColumnAlign::Left; 4],
    )
}

fn render_role_inspection(row: &ConfiguredRoleLifecycle) -> String {
    let topology = row.topology.as_deref().unwrap_or("-");
    let package = row.package.as_str();
    let deploy = if row.attached {
        "eligible"
    } else {
        "blocked: role is declared-only"
    };
    let next_action = if row.attached {
        format!("canic build {} {}", row.fleet, row.role)
    } else {
        format!(
            "canic fleet role attach {} {} --subnet <subnet>",
            row.fleet, row.role
        )
    };

    [
        "Fleet role:".to_string(),
        format!("  role: {}", row.display),
        format!("  declaration: {}", row.declaration_kind),
        format!("  package: {package}"),
        format!("  state: {}", row.state),
        format!("  topology: {topology}"),
        "  cargo check: allowed".to_string(),
        format!("  deploy artifact: {deploy}"),
        format!("  next action: {next_action}"),
    ]
    .join("\n")
}

fn render_declared_role(
    role: &DeclaredFleetRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    [
        "Declared fleet role:".to_string(),
        format!("  role: {}", role.display),
        format!("  package: {}", role.package),
        format!(
            "  config: {}",
            display_workspace_path(workspace_root, config_path)
        ),
        "  state: declared".to_string(),
        format!(
            "  next action: canic fleet role attach {} {} --subnet <subnet>",
            role.fleet, role.role
        ),
    ]
    .join("\n")
}

fn render_attached_role(
    role: &AttachedFleetRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    [
        "Attached fleet role:".to_string(),
        format!("  role: {}", role.display),
        format!("  kind: {}", role.kind),
        format!("  topology: {}", role.topology),
        format!(
            "  config: {}",
            display_workspace_path(workspace_root, config_path)
        ),
        "  state: attached".to_string(),
        format!("  next action: canic build {} {}", role.fleet, role.role),
    ]
    .join("\n")
}

fn render_renamed_role(
    role: &RenamedFleetRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    let package = role.package_manifest.as_ref().map_or_else(
        || {
            role.package_manifest_note
                .as_deref()
                .unwrap_or("not updated")
                .to_string()
        },
        |path| display_workspace_path(workspace_root, path),
    );

    [
        "Renamed fleet role:".to_string(),
        format!("  old: {}", role.old_display),
        format!("  new: {}", role.new_display),
        format!(
            "  config: {}",
            display_workspace_path(workspace_root, config_path)
        ),
        format!("  package_manifest: {package}"),
        format!(
            "  next action: canic fleet role inspect {} {}",
            role.fleet, role.new_role
        ),
    ]
    .join("\n")
}

fn build_adoption_report_from_config_path(
    config_path: &Path,
    options: &AdoptionReportOptions,
    generated_at: &str,
) -> Result<AdoptionReportV1, FleetCommandError> {
    let config_source = fs::read_to_string(config_path)?;
    let inventory = adoption_inventory_from_options(options)?;
    let artifact_manifest = adoption_artifact_manifest_from_options(options)?;
    let package_metadata = adoption_package_metadata_from_options(config_path, options)?;
    let report_id = format!(
        "local:{}:{}:adoption-report",
        options.fleet,
        adoption_profile_label(options.profile)
    );

    adoption_report_from_config_source(AdoptionReportRequest {
        report_id: &report_id,
        generated_at,
        profile: options.profile,
        config_source: &config_source,
        inventory: inventory.as_ref(),
        artifact_manifest: artifact_manifest.as_ref(),
        package_metadata,
    })
    .map_err(FleetCommandError::from)
}

fn adoption_package_metadata_from_options(
    config_path: &Path,
    options: &AdoptionReportOptions,
) -> Result<Vec<AdoptionPackageMetadataV1>, FleetCommandError> {
    match (&options.package_metadata, &options.cargo_metadata) {
        (Some(_), Some(_)) => Err(FleetCommandError::Usage(
            "choose either --package-metadata or --cargo-metadata, not both".to_string(),
        )),
        (Some(path), None) => read_json_file(path),
        (None, Some(path)) => read_cargo_metadata_package_metadata(config_path, path),
        (None, None) => Ok(Vec::new()),
    }
}

fn adoption_artifact_manifest_from_options(
    options: &AdoptionReportOptions,
) -> Result<Option<RoleArtifactManifestV1>, FleetCommandError> {
    if let Some(path) = &options.artifact_manifest {
        return read_json_file(path).map(Some);
    }

    options
        .deployment_check
        .as_deref()
        .map(read_deployment_check_artifact_manifest)
        .transpose()
        .map(Option::flatten)
}

fn adoption_inventory_from_options(
    options: &AdoptionReportOptions,
) -> Result<Option<DeploymentInventoryV1>, FleetCommandError> {
    match (&options.inventory, &options.deployment_check) {
        (Some(_), Some(_)) => Err(FleetCommandError::Usage(
            "choose either --inventory or --deployment-check, not both".to_string(),
        )),
        (Some(path), None) => read_json_file(path).map(Some),
        (None, Some(path)) => read_deployment_check_inventory(path).map(Some),
        (None, None) => Ok(None),
    }
}

fn read_deployment_check_inventory(
    path: &Path,
) -> Result<DeploymentInventoryV1, FleetCommandError> {
    let value = read_json_file::<serde_json::Value>(path)?;
    let Some(inventory) = value.get("inventory") else {
        return Err(FleetCommandError::Usage(format!(
            "deployment check evidence {} is missing inventory",
            path.display()
        )));
    };

    Ok(serde_json::from_value(inventory.clone())?)
}

fn read_deployment_check_artifact_manifest(
    path: &Path,
) -> Result<Option<RoleArtifactManifestV1>, FleetCommandError> {
    let value = read_json_file::<serde_json::Value>(path)?;
    let Some(plan) = value.get("plan") else {
        return Ok(None);
    };
    let role_artifacts = plan
        .get("role_artifacts")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    let network = plan
        .get("deployment_identity")
        .and_then(|identity| identity.get("network"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let check_id = value
        .get("check_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown-check");

    Ok(Some(RoleArtifactManifestV1 {
        schema_version: 1,
        manifest_id: format!("deployment-check:{check_id}:role-artifacts"),
        network,
        artifact_root: None,
        role_artifacts: serde_json::from_value::<Vec<RoleArtifactV1>>(role_artifacts)?,
        unresolved_artifacts: Vec::new(),
    }))
}

fn read_cargo_metadata_package_metadata(
    config_path: &Path,
    path: &Path,
) -> Result<Vec<AdoptionPackageMetadataV1>, FleetCommandError> {
    let value = read_json_file::<serde_json::Value>(path)?;
    let packages = value
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            FleetCommandError::Usage(format!(
                "cargo metadata evidence {} is missing packages",
                path.display()
            ))
        })?;
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new(""));
    let mut metadata = Vec::new();

    for package in packages {
        let Some(canic_metadata) = package
            .get("metadata")
            .and_then(|metadata| metadata.get("canic"))
        else {
            continue;
        };
        let Some(package_path) = cargo_metadata_package_path(config_dir, package) else {
            continue;
        };
        metadata.push(AdoptionPackageMetadataV1 {
            package: package_path,
            fleet: canic_metadata
                .get("fleet")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            role: canic_metadata
                .get("role")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
        });
    }

    Ok(metadata)
}

fn cargo_metadata_package_path(config_dir: &Path, package: &serde_json::Value) -> Option<String> {
    let manifest_path = package.get("manifest_path")?.as_str()?;
    let package_dir = Path::new(manifest_path).parent()?;
    let relative = relative_package_dir(config_dir, package_dir);
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn relative_package_dir(config_dir: &Path, package_dir: &Path) -> PathBuf {
    if let Ok(relative) = package_dir.strip_prefix(config_dir) {
        return non_empty_relative_path(relative);
    }

    lexical_relative_path(config_dir, package_dir).unwrap_or_else(|| package_dir.to_path_buf())
}

fn non_empty_relative_path(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    }
}

fn lexical_relative_path(base: &Path, target: &Path) -> Option<PathBuf> {
    let base_components = normal_path_components(base);
    let target_components = normal_path_components(target);
    let common = base_components
        .iter()
        .zip(target_components.iter())
        .take_while(|(base, target)| base == target)
        .count();
    if common == 0 {
        return None;
    }

    let mut relative = PathBuf::new();
    for _ in common..base_components.len() {
        relative.push("..");
    }
    for component in &target_components[common..] {
        relative.push(component);
    }

    Some(non_empty_relative_path(&relative))
}

fn normal_path_components(path: &Path) -> Vec<String> {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir => components.push(String::new()),
            Component::CurDir => {}
            Component::ParentDir => components.push("..".to_string()),
            Component::Normal(segment) => components.push(segment.to_string_lossy().to_string()),
        }
    }

    components
}

fn read_json_file<T>(path: &Path) -> Result<T, FleetCommandError>
where
    T: DeserializeOwned,
{
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_adoption_report(
    config_path: &Path,
    options: &AdoptionReportOptions,
    report: &AdoptionReportV1,
) -> Result<(), FleetCommandError> {
    match options.format {
        AdoptionReportFormat::Text => {
            output::write_text(options.output.as_ref(), &render_adoption_report(report))
        }
        AdoptionReportFormat::Json => output::write_pretty_json(options.output.as_ref(), report),
        AdoptionReportFormat::EnvelopeJson => {
            let envelope = build_adoption_report_envelope(config_path, options, report)?;
            output::write_pretty_json(options.output.as_ref(), &envelope)
        }
    }
}

fn build_adoption_report_envelope(
    config_path: &Path,
    options: &AdoptionReportOptions,
    report: &AdoptionReportV1,
) -> Result<EvidenceEnvelopeV1, FleetCommandError> {
    let payload = serde_json::to_value(report)?;
    let payload_sha256 = Some(json_payload_sha256(report)?);
    let config_root = config_path.parent().unwrap_or_else(|| Path::new("."));

    Ok(EvidenceEnvelopeV1 {
        envelope_schema: evidence_envelope_schema(),
        canic_version: env!("CARGO_PKG_VERSION").to_string(),
        command: adoption_report_command_provenance(config_root, options),
        target: EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::FleetAdoption,
            deployment: None,
            fleet: Some(report.fleet.clone()),
            role: None,
            profile: Some(adoption_profile_label(report.profile).to_string()),
            network: None,
        },
        generated_at: report.generated_at.clone(),
        source_config: Some(file_input_fingerprint(
            "canic_config",
            config_path,
            config_root,
            Some(PayloadSchemaRefV1::internal("canic.config.toml", "1")),
            None,
        )?),
        inputs: adoption_report_input_fingerprints(config_root, options)?,
        payload_schema: adoption_report_schema(),
        payload_sha256,
        payload,
        summary: adoption_report_evidence_summary(report),
        exit_class: adoption_report_exit_class(report),
    })
}

fn adoption_report_command_provenance(
    config_root: &Path,
    options: &AdoptionReportOptions,
) -> CommandProvenanceV1 {
    let mut argv_normalized = vec![
        "canic".to_string(),
        "fleet".to_string(),
        "adoption".to_string(),
        "report".to_string(),
        options.fleet.clone(),
        "--profile".to_string(),
        adoption_profile_label(options.profile).to_string(),
        "--format".to_string(),
        "envelope-json".to_string(),
    ];

    let mut argv_redactions = Vec::new();

    push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--deployment-check",
        options.deployment_check.as_ref(),
        config_root,
    );
    push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--inventory",
        options.inventory.as_ref(),
        config_root,
    );
    push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--artifact-manifest",
        options.artifact_manifest.as_ref(),
        config_root,
    );
    push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--cargo-metadata",
        options.cargo_metadata.as_ref(),
        config_root,
    );
    push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--package-metadata",
        options.package_metadata.as_ref(),
        config_root,
    );

    CommandProvenanceV1 {
        name: "canic fleet adoption report".to_string(),
        argv_normalized,
        argv_redactions,
        format: "envelope-json".to_string(),
    }
}

fn push_optional_path_arg(
    args: &mut Vec<String>,
    redactions: &mut Vec<String>,
    flag: &str,
    path: Option<&PathBuf>,
    config_root: &Path,
) {
    if let Some(path) = path {
        args.push(flag.to_string());
        let display_path = command_path_for_root(path, config_root);
        if display_path.starts_with("<redacted:") {
            redactions.push(format!("{flag} absolute path outside config root"));
        }
        args.push(display_path);
    }
}

fn adoption_report_input_fingerprints(
    config_root: &Path,
    options: &AdoptionReportOptions,
) -> Result<Vec<InputFingerprintV1>, FleetCommandError> {
    let mut inputs = Vec::new();

    push_optional_input_fingerprint(
        &mut inputs,
        "deployment_check",
        options.deployment_check.as_deref(),
        config_root,
        Some(deployment_check_schema()),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "deployment_inventory",
        options.inventory.as_deref(),
        config_root,
        Some(PayloadSchemaRefV1::internal(
            "canic.deployment_inventory.v1",
            "1",
        )),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "role_artifact_manifest",
        options.artifact_manifest.as_deref(),
        config_root,
        Some(PayloadSchemaRefV1::internal(
            "canic.role_artifact_manifest.v1",
            "1",
        )),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "cargo_metadata",
        options.cargo_metadata.as_deref(),
        config_root,
        Some(PayloadSchemaRefV1::internal("cargo.metadata.v1", "1")),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "adoption_package_metadata",
        options.package_metadata.as_deref(),
        config_root,
        Some(PayloadSchemaRefV1::experimental(
            "canic.adoption_package_metadata.v1",
            "1",
        )),
    )?;

    Ok(inputs)
}

fn push_optional_input_fingerprint(
    inputs: &mut Vec<InputFingerprintV1>,
    kind: &str,
    path: Option<&Path>,
    config_root: &Path,
    schema: Option<PayloadSchemaRefV1>,
) -> Result<(), FleetCommandError> {
    if let Some(path) = path {
        inputs.push(file_input_fingerprint(
            kind,
            path,
            config_root,
            schema,
            None,
        )?);
    }
    Ok(())
}

fn adoption_report_evidence_summary(report: &AdoptionReportV1) -> EvidenceSummaryV1 {
    EvidenceSummaryV1 {
        warnings: report
            .warnings
            .iter()
            .map(|warning| {
                EvidenceMessageV1::new(
                    "adoption.warning",
                    warning.clone(),
                    EvidenceMessageSeverityV1::Warning,
                )
            })
            .collect(),
        blocked_actions: report
            .blocked_actions
            .iter()
            .map(|action| {
                EvidenceMessageV1::new(
                    "adoption.blocked_action",
                    action.clone(),
                    EvidenceMessageSeverityV1::Error,
                )
            })
            .collect(),
        missing_or_stale_evidence: report
            .inputs
            .missing_or_stale_evidence
            .iter()
            .map(|evidence| {
                EvidenceMessageV1::new(
                    "adoption.missing_or_stale_evidence",
                    evidence.clone(),
                    EvidenceMessageSeverityV1::Warning,
                )
            })
            .collect(),
        evidence_conflicts: adoption_evidence_conflict_messages(report),
    }
}

fn adoption_evidence_conflict_messages(report: &AdoptionReportV1) -> Vec<EvidenceMessageV1> {
    report
        .role_findings
        .iter()
        .filter(|finding| {
            finding
                .classifications
                .contains(&AdoptionClassificationV1::EvidenceConflict)
        })
        .map(|finding| {
            EvidenceMessageV1::new(
                "adoption.evidence_conflict",
                format!("role {} has conflicting adoption evidence", finding.role),
                EvidenceMessageSeverityV1::Error,
            )
        })
        .collect()
}

const fn adoption_report_exit_class(report: &AdoptionReportV1) -> ExitClassV1 {
    if report.summary.evidence_conflicts > 0 {
        return ExitClassV1::EvidenceConflict;
    }
    if !report.blocked_actions.is_empty() {
        return ExitClassV1::BlockedByPolicy;
    }
    if !report.warnings.is_empty() || !report.inputs.missing_or_stale_evidence.is_empty() {
        return ExitClassV1::SuccessWithWarnings;
    }

    ExitClassV1::Success
}

fn render_adoption_report(report: &AdoptionReportV1) -> String {
    let mut lines = vec![
        "Adoption report:".to_string(),
        format!("  fleet: {}", report.fleet),
        format!("  profile: {}", adoption_profile_label(report.profile)),
        format!("  report_id: {}", report.report_id),
        format!("  generated_at: {}", report.generated_at),
        "  read_only: true".to_string(),
        "Summary:".to_string(),
    ];

    lines.extend(adoption_summary_lines(report));
    lines.extend(adoption_missing_evidence_lines(report));
    lines.extend(adoption_role_finding_lines(report));
    lines.extend(adoption_observed_canister_lines(report));
    lines.extend(adoption_recommendation_lines(report));
    lines.extend(adoption_blocked_action_lines(report));
    lines.join("\n")
}

fn adoption_summary_lines(report: &AdoptionReportV1) -> Vec<String> {
    let summary = &report.summary;
    [
        format!(
            "  managed_configured_roles: {}",
            summary.managed_configured_roles
        ),
        format!("  declared_only_roles: {}", summary.declared_only_roles),
        format!(
            "  attached_unobserved_roles: {}",
            summary.attached_unobserved_roles
        ),
        format!(
            "  observed_only_canisters: {}",
            summary.observed_only_canisters
        ),
        format!(
            "  user_controlled_canisters: {}",
            summary.user_controlled_canisters
        ),
        format!(
            "  external_controller_required: {}",
            summary.external_controller_required
        ),
        format!("  evidence_conflicts: {}", summary.evidence_conflicts),
        format!(
            "  mutating_actions_performed: {}",
            summary.mutating_actions_performed
        ),
    ]
    .into()
}

fn adoption_missing_evidence_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Missing or stale evidence:".to_string()];
    if report.inputs.missing_or_stale_evidence.is_empty() {
        lines.push("  - none".to_string());
    } else {
        lines.extend(
            report
                .inputs
                .missing_or_stale_evidence
                .iter()
                .map(|evidence| format!("  - {evidence}")),
        );
    }
    lines
}

fn adoption_role_finding_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Role findings:".to_string()];
    if report.role_findings.is_empty() {
        lines.push("  - none".to_string());
        return lines;
    }

    for finding in &report.role_findings {
        lines.push(format!(
            "  - {}.{}: {}",
            finding.fleet,
            finding.role,
            format_adoption_classifications(&finding.classifications)
        ));
        lines.push(format!(
            "    state: declaration={}, topology={}, observation={}, authority={}, artifact={}, package={}",
            adoption_declaration_state_label(finding.declaration_state),
            adoption_topology_state_label(finding.topology_state),
            adoption_observation_state_label(finding.observation_state),
            adoption_authority_state_label(finding.authority_state),
            adoption_artifact_state_label(finding.artifact_state),
            adoption_package_state_label(finding.package_state)
        ));
        lines.extend(
            finding
                .warnings
                .iter()
                .map(|warning| format!("    warning: {warning}")),
        );
    }

    lines
}

fn adoption_observed_canister_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Observed canisters:".to_string()];
    if report.observed_canisters.is_empty() {
        lines.push("  - none".to_string());
        return lines;
    }

    for finding in &report.observed_canisters {
        let role = finding.matched_role.as_deref().map_or("-", |role| role);
        lines.push(format!(
            "  - {}: role={}, confidence={}, classifications={}",
            finding.canister_id,
            role,
            adoption_match_confidence_label(finding.confidence),
            format_adoption_classifications(&finding.classifications)
        ));
        if !finding.controllers.is_empty() {
            lines.push(format!(
                "    controllers: {}",
                finding.controllers.join(",")
            ));
        }
        if let Some(evidence) = &finding.wasm_evidence {
            lines.push(format!("    wasm_evidence: {evidence}"));
        }
        if let Some(evidence) = &finding.deployment_target_evidence {
            lines.push(format!("    deployment_target_evidence: {evidence}"));
        }
        lines.extend(
            finding
                .warnings
                .iter()
                .map(|warning| format!("    warning: {warning}")),
        );
    }
    lines
}

fn adoption_recommendation_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Recommendations (report-only; not executed):".to_string()];
    if report.recommendations.is_empty() {
        lines.push("  - none".to_string());
        return lines;
    }

    for recommendation in &report.recommendations {
        lines.push(format!(
            "  - {} [{}; {}; {}; {}; {}]",
            recommendation.description,
            adoption_recommendation_severity_label(recommendation.severity),
            adoption_action_effect_label(recommendation.suggested_action_effect),
            adoption_action_support_label(recommendation.suggested_action_support),
            adoption_action_availability_label(recommendation.suggested_action_availability),
            adoption_operator_requirement_label(recommendation.operator_action_requirement)
        ));
        if let Some(action) = &recommendation.suggested_action {
            lines.push(format!("    suggested_action_preview: {action}"));
            lines.push("    status: not executed by adoption report".to_string());
            lines.push(format!(
                "    support: {}",
                adoption_action_support_label(recommendation.suggested_action_support)
            ));
            lines.push(format!(
                "    availability: {}",
                adoption_action_availability_label(recommendation.suggested_action_availability)
            ));
        }
    }
    lines
}

fn adoption_blocked_action_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Blocked adoption actions (not executed by report):".to_string()];
    if report.blocked_actions.is_empty() {
        lines.push("  - none".to_string());
    } else {
        lines.extend(
            report
                .blocked_actions
                .iter()
                .map(|action| format!("  - {action}")),
        );
    }
    lines
}

fn format_adoption_classifications(classifications: &[AdoptionClassificationV1]) -> String {
    if classifications.is_empty() {
        return "-".to_string();
    }

    classifications
        .iter()
        .map(|classification| adoption_classification_label(*classification))
        .collect::<Vec<_>>()
        .join(",")
}

fn current_adoption_report_generated_at() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "unix:{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    ))
}

const fn adoption_profile_label(profile: AdoptionProfileV1) -> &'static str {
    match profile {
        AdoptionProfileV1::Brownfield => "brownfield",
        AdoptionProfileV1::Partial => "partial",
        AdoptionProfileV1::Standalone => "standalone",
        AdoptionProfileV1::LeafOnly => "leaf-only",
        AdoptionProfileV1::HybridExternalWasm => "hybrid-external-wasm",
        AdoptionProfileV1::Minimal => "minimal",
    }
}

const fn adoption_classification_label(classification: AdoptionClassificationV1) -> &'static str {
    match classification {
        AdoptionClassificationV1::Managed => "managed",
        AdoptionClassificationV1::DeclaredOnly => "declared-only",
        AdoptionClassificationV1::ObservedOnly => "observed-only",
        AdoptionClassificationV1::AttachedUnobserved => "attached-unobserved",
        AdoptionClassificationV1::UserControlled => "user-controlled",
        AdoptionClassificationV1::ExternalControllerRequired => "external-controller-required",
        AdoptionClassificationV1::ImportedPoolCandidate => "imported-pool-candidate",
        AdoptionClassificationV1::EvidenceConflict => "evidence-conflict",
    }
}

const fn adoption_declaration_state_label(state: AdoptionDeclarationStateV1) -> &'static str {
    match state {
        AdoptionDeclarationStateV1::Undeclared => "undeclared",
        AdoptionDeclarationStateV1::Declared => "declared",
    }
}

const fn adoption_topology_state_label(state: AdoptionTopologyStateV1) -> &'static str {
    match state {
        AdoptionTopologyStateV1::Unattached => "unattached",
        AdoptionTopologyStateV1::Attached => "attached",
    }
}

const fn adoption_observation_state_label(state: AdoptionObservationStateV1) -> &'static str {
    match state {
        AdoptionObservationStateV1::Unobserved => "unobserved",
        AdoptionObservationStateV1::Observed => "observed",
        AdoptionObservationStateV1::CandidateMatch => "candidate-match",
        AdoptionObservationStateV1::ConflictingMatch => "conflicting-match",
    }
}

const fn adoption_authority_state_label(state: AdoptionAuthorityStateV1) -> &'static str {
    match state {
        AdoptionAuthorityStateV1::CanicAuthorized => "canic-authorized",
        AdoptionAuthorityStateV1::UserControlled => "user-controlled",
        AdoptionAuthorityStateV1::External => "external",
        AdoptionAuthorityStateV1::Unknown => "unknown",
    }
}

const fn adoption_artifact_state_label(state: AdoptionArtifactStateV1) -> &'static str {
    match state {
        AdoptionArtifactStateV1::CanicBuilt => "canic-built",
        AdoptionArtifactStateV1::ExternalWasm => "external-wasm",
        AdoptionArtifactStateV1::Unknown => "unknown",
    }
}

const fn adoption_package_state_label(state: AdoptionPackageStateV1) -> &'static str {
    match state {
        AdoptionPackageStateV1::UndeclaredRole => "undeclared-role",
        AdoptionPackageStateV1::NotChecked => "not-checked",
        AdoptionPackageStateV1::Matches => "matches",
        AdoptionPackageStateV1::MissingFleet => "missing-fleet",
        AdoptionPackageStateV1::MissingRole => "missing-role",
        AdoptionPackageStateV1::Mismatch => "mismatch",
    }
}

const fn adoption_match_confidence_label(confidence: AdoptionMatchConfidenceV1) -> &'static str {
    match confidence {
        AdoptionMatchConfidenceV1::None => "none",
        AdoptionMatchConfidenceV1::Candidate => "candidate",
        AdoptionMatchConfidenceV1::ExplicitEvidence => "explicit-evidence",
        AdoptionMatchConfidenceV1::Conflict => "conflict",
    }
}

const fn adoption_recommendation_severity_label(
    severity: AdoptionRecommendationSeverityV1,
) -> &'static str {
    match severity {
        AdoptionRecommendationSeverityV1::Info => "info",
        AdoptionRecommendationSeverityV1::Warning => "warning",
        AdoptionRecommendationSeverityV1::Blocked => "blocked",
    }
}

const fn adoption_action_effect_label(effect: AdoptionSuggestedActionEffectV1) -> &'static str {
    match effect {
        AdoptionSuggestedActionEffectV1::ReadOnly => "read-only",
        AdoptionSuggestedActionEffectV1::MutatesState => "mutates-state",
    }
}

const fn adoption_action_support_label(support: AdoptionSuggestedActionSupportV1) -> &'static str {
    match support {
        AdoptionSuggestedActionSupportV1::SupportedByAdoption => "supported-by-adoption",
        AdoptionSuggestedActionSupportV1::UnsupportedByAdoption => "unsupported-by-adoption",
    }
}

const fn adoption_action_availability_label(
    availability: AdoptionSuggestedActionAvailabilityV1,
) -> &'static str {
    match availability {
        AdoptionSuggestedActionAvailabilityV1::AllowedIn0500 => "allowed-in-0.50.0",
        AdoptionSuggestedActionAvailabilityV1::BlockedIn0500 => "blocked-in-0.50.0",
    }
}

const fn adoption_operator_requirement_label(
    requirement: AdoptionOperatorActionRequirementV1,
) -> &'static str {
    match requirement {
        AdoptionOperatorActionRequirementV1::Required => "operator-action-required",
        AdoptionOperatorActionRequirementV1::NotRequired => "no-operator-action-required",
    }
}

fn print_config_report(report: &IcpProjectConfigReport) {
    println!("Checked ICP project config:");
    println!("  path: {}", report.path.display());
    println!("  canisters: {}", report.canisters.len());
    println!("  environments: {}", report.environments.len());
    println!(
        "  status: {}",
        if report.is_ready() {
            "ok"
        } else {
            "incomplete"
        }
    );
    for issue in report.issues() {
        println!("  issue: {issue}");
    }
}

fn usage() -> String {
    let mut command = fleet_command();
    command.render_help().to_string()
}

fn list_usage() -> String {
    let mut command = fleet_list_command();
    command.render_help().to_string()
}

fn check_usage() -> String {
    let mut command = fleet_check_command();
    command.render_help().to_string()
}

fn create_usage() -> String {
    scaffold::fleet_create_usage()
}

fn delete_usage() -> String {
    let mut command = fleet_delete_command();
    command.render_help().to_string()
}

fn role_usage() -> String {
    let mut command = fleet_role_command();
    command.render_help().to_string()
}

fn adoption_usage() -> String {
    let mut command = fleet_adoption_command();
    command.render_help().to_string()
}

fn adoption_report_usage() -> String {
    let mut command = fleet_adoption_report_command();
    command.render_help().to_string()
}

fn role_list_usage() -> String {
    let mut command = fleet_role_list_command();
    command.render_help().to_string()
}

fn role_inspect_usage() -> String {
    let mut command = fleet_role_inspect_command();
    command.render_help().to_string()
}

fn role_declare_usage() -> String {
    let mut command = fleet_role_declare_command();
    command.render_help().to_string()
}

fn role_attach_usage() -> String {
    let mut command = fleet_role_attach_command();
    command.render_help().to_string()
}

fn role_rename_usage() -> String {
    let mut command = fleet_role_rename_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests;
