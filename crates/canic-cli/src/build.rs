//! Module: canic_cli::build
//!
//! Responsibility: build a complete App-plus-infrastructure artifact set or one selected App role.
//! Does not own: canister build execution, app config schema, or evidence envelope schemas.
//! Boundary: resolves CLI build context and delegates configuration-backed artifact creation.

#[cfg(test)]
use crate::cli::clap::render_usage;
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, required_string, required_typed, string_option,
            string_option_or_else, value_arg,
        },
        defaults::local_environment,
        globals::internal_environment_arg,
    },
    evidence_support::current_evidence_timestamp,
    output,
};
use canic_core::ids::{BuildNetwork, CanisterRole, ReleaseBuildId};
use canic_host::build_provenance::{BuildProvenanceRequest, build_provenance_envelope};
use canic_host::canister_build::{
    CanisterArtifactBuildOptions, CanisterBuildProfile, ConfiguredCanisterArtifactBuildOutput,
    WorkspaceBuildContext, build_workspace_canister_artifact,
    build_workspace_canister_artifact_with_options, build_workspace_configured_canister_artifacts,
    copy_icp_wasm_output, print_workspace_build_context_once,
};
use canic_host::evidence_envelope::{CommandProvenanceV1, command_path_for_root};
use canic_host::{
    config_discovery::{
        ConfigDiscoveryError, current_canic_workspace_root,
        discover_workspace_canic_config_choices, select_discovered_app_config_path,
    },
    format::wasm_size_label,
    icp_config::{resolve_current_canic_icp_root, resolve_icp_build_network_from_root},
    release_build::{
        finalize_release_build_from_manifest, plan_release_build_for_profile_and_network,
    },
    release_set::{
        AppConfigError, AppConfigSnapshot, ApplicationArtifactBuildTarget,
        ApplicationArtifactFileBuildOutput, CanicInfrastructureArtifactBuildOutput,
        CanicInfrastructureRole, WorkspaceDiscoveryError,
        compile_and_persist_application_artifact_union,
        compile_and_persist_canic_infrastructure_artifact_manifest,
        compile_and_persist_current_release_set_manifest, display_workspace_path, workspace_root,
    },
    table::{ColumnAlign, render_bordered_table},
    terminal::{TerminalActivity, TerminalStyle},
};
use clap::{ArgAction, Command as ClapCommand};
use std::{
    collections::BTreeSet,
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use thiserror::Error as ThisError;

const BUILD_HELP_AFTER: &str = "\
Examples:
  canic build demo
  canic build demo app --standalone-local --features standalone-local

Builds the configured Fleet Subnet Root, every attached Component role, and
Canic's built-in Coordinator and Wasm Store by default. Pass a role for one
focused configured artifact.";

///
/// BuildCommandError
///
/// CLI boundary error for build option parsing, config selection, artifact
/// creation, and provenance output.
///

#[derive(Debug, ThisError)]
pub enum BuildCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("no Canic app configs found under apps; run canic app create <name>")]
    NoConfigChoices,

    #[error("unknown app {0}; run canic app list to inspect config-defined apps")]
    UnknownApp(String),

    #[error(
        "complete App build resolved {actual} Fleet Subnet Root artifacts; expected exactly one"
    )]
    FleetSubnetRootArtifactCount { actual: usize },

    #[error("failed to discover Canic workspace App configs: {0}")]
    ConfigDiscovery(#[from] ConfigDiscoveryError),

    #[error("failed to resolve Cargo workspace: {0}")]
    WorkspaceDiscovery(#[from] WorkspaceDiscoveryError),

    #[error(transparent)]
    Build(#[from] Box<dyn std::error::Error>),

    #[error(transparent)]
    Clap(#[from] clap::Error),

    #[error(transparent)]
    AppConfig(#[from] AppConfigError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl BuildCommandError {
    /// Return the shell exit code for this build failure.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Clap(error) => error.exit_code(),
            Self::Usage(_) => 2,
            Self::AppConfig(_)
            | Self::Build(_)
            | Self::ConfigDiscovery(_)
            | Self::FleetSubnetRootArtifactCount { .. }
            | Self::Io(_)
            | Self::Json(_)
            | Self::NoConfigChoices
            | Self::UnknownApp(_)
            | Self::WorkspaceDiscovery(_) => 1,
        }
    }
}

/// Parsed `canic build` command options.

#[derive(Clone, Debug, Eq, PartialEq)]
struct BuildOptions {
    app: String,
    role: Option<String>,
    environment: String,
    profile: CanisterBuildProfile,
    workspace: Option<String>,
    icp_root: Option<String>,
    config: Option<String>,
    features: BTreeSet<String>,
    no_default_features: bool,
    provenance: Option<PathBuf>,
    standalone_local: bool,
}

impl BuildOptions {
    fn parse<I>(args: I) -> Result<Self, BuildCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(build_command(), args)?;

        Ok(Self {
            app: required_string(&matches, "app"),
            role: string_option(&matches, "role"),
            environment: string_option_or_else(&matches, "environment", local_environment),
            profile: required_typed(&matches, "profile"),
            workspace: string_option(&matches, "workspace"),
            icp_root: string_option(&matches, "icp-root"),
            config: string_option(&matches, "config"),
            features: matches
                .get_many::<String>("features")
                .into_iter()
                .flatten()
                .cloned()
                .collect(),
            no_default_features: matches.get_flag("no-default-features"),
            provenance: string_option(&matches, "provenance").map(PathBuf::from),
            standalone_local: matches.get_flag("standalone-local"),
        })
    }

    fn artifact_build_options(&self) -> CanisterArtifactBuildOptions {
        CanisterArtifactBuildOptions {
            cargo_features: self.features.clone(),
            default_features: !self.no_default_features,
            sidecar_only_candid: self.standalone_local,
        }
    }
}

/// Build configured Canic App artifacts through the installed CLI.
pub fn run<I>(args: I) -> Result<(), BuildCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    let started_at = Instant::now();
    let options = match BuildOptions::parse(args) {
        Ok(options) => options,
        Err(BuildCommandError::Clap(error)) if !error.use_stderr() => {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    let config_path = resolve_build_config_path(&options)?.canonicalize()?;
    let roles = selected_build_roles(&options, &config_path)?;
    let mut context = resolve_build_context(&options, config_path, &roles[0])?;

    if let Some(role) = &options.role {
        if options.standalone_local && context.build_network != BuildNetwork::Local {
            return Err(BuildCommandError::Usage(
                "--standalone-local requires a local ICP environment".to_string(),
            ));
        }
        if options.standalone_local && role == CanisterRole::ROOT.as_str() {
            return Err(BuildCommandError::Usage(
                "--standalone-local requires a non-Root application role".to_string(),
            ));
        }
        print_workspace_build_context_once(&context)?;
        let output = build_workspace_canister_artifact_with_options(
            &context,
            &options.artifact_build_options(),
        )?;
        copy_icp_wasm_output(role, &output)?;
        write_build_provenance_if_requested(&options, &context, output.clone())?;
        TerminalStyle::detected().print_section(
            "Build complete",
            &build_completion_detail(1, "role", "roles", started_at.elapsed()),
        );
        println!("{}", output.wasm_gz_path.display());
    } else {
        let release = plan_release_build_for_profile_and_network(
            &context.icp_root,
            context.profile,
            context.build_network,
        )
        .map_err(|error| BuildCommandError::Build(Box::new(error)))?;
        context = context.with_release_build_id(release.record.release_build_id);
        build_app(&options, &context, &roles, started_at)?;
    }
    Ok(())
}

fn build_command() -> ClapCommand {
    ClapCommand::new("build")
        .bin_name("canic build")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Build Canic App and infrastructure artifacts")
        .override_usage("canic build [OPTIONS] <app> [role]")
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Config-defined app name to build from"),
        )
        .arg(
            value_arg("role")
                .value_name("role")
                .required(false)
                .help("Build only this deployable configured canister role"),
        )
        .arg(
            value_arg("workspace")
                .long("workspace")
                .value_name("dir")
                .num_args(1)
                .help("Cargo workspace root; inferred from the current directory when omitted"),
        )
        .arg(
            value_arg("icp-root")
                .long("icp-root")
                .value_name("dir")
                .num_args(1)
                .help("ICP project root for .icp artifacts; inferred when omitted"),
        )
        .arg(
            value_arg("config")
                .long("config")
                .value_name("file")
                .num_args(1)
                .help("Canic config path; inferred from the workspace when omitted"),
        )
        .arg(
            value_arg("features")
                .long("features")
                .value_name("feature,...")
                .num_args(1)
                .action(ArgAction::Append)
                .value_delimiter(',')
                .requires("role")
                .help("Cargo features used identically for declaration and runtime builds"),
        )
        .arg(
            flag_arg("no-default-features")
                .long("no-default-features")
                .requires("role")
                .help("Disable Cargo default features for both build passes"),
        )
        .arg(
            value_arg("profile")
                .long("profile")
                .value_name("debug|fast|release")
                .num_args(1)
                .value_parser(clap::value_parser!(CanisterBuildProfile))
                .default_value("fast")
                .help("Canister wasm build profile"),
        )
        .arg(
            value_arg("provenance")
                .long("provenance")
                .value_name("file")
                .num_args(1)
                .requires("role")
                .help("Write an EvidenceEnvelopeV1 build provenance artifact to this file"),
        )
        .arg(
            flag_arg("standalone-local")
                .long("standalone-local")
                .requires("role")
                .help("Emit a local runtime with Candid retained only in the adjacent .did"),
        )
        .arg(internal_environment_arg())
        .after_help(BUILD_HELP_AFTER)
}

#[cfg(test)]
fn usage() -> String {
    render_usage(build_command)
}

fn selected_build_roles(
    options: &BuildOptions,
    config_path: &Path,
) -> Result<Vec<String>, BuildCommandError> {
    let config = AppConfigSnapshot::load(config_path)?;
    let roles = config.role_lifecycle();
    let Some(role) = &options.role else {
        let selected = config.deployable_roles();
        if selected.is_empty() {
            return Err(BuildCommandError::Usage(format!(
                "App {} has no deployable roles to build",
                options.app
            )));
        }
        return Ok(selected);
    };
    let Some(row) = roles.iter().find(|row| row.role == *role) else {
        return Err(BuildCommandError::Usage(format!(
            "role {}.{} is not declared in {}",
            options.app,
            role,
            config_path.display()
        )));
    };
    if !row.attached {
        return Err(BuildCommandError::Usage(format!(
            "role {}.{} is declared but not attached to topology; run `canic app role attach {} {} --component-spec <component-spec>` before building an artifact",
            options.app, role, options.app, role
        )));
    }
    Ok(vec![role.clone()])
}

fn build_app(
    options: &BuildOptions,
    context: &WorkspaceBuildContext,
    roles: &[String],
    started_at: Instant,
) -> Result<(), BuildCommandError> {
    let style = TerminalStyle::detected();
    style.print_section(
        "Build App",
        &format!(
            "{} | {} profile | {} network",
            options.app,
            context.profile.target_dir_name(),
            context.build_network
        ),
    );
    println!(
        "App config: {}",
        display_workspace_path(&context.workspace_root, &context.config_path)
    );
    println!("Root Wasm: App-config-bound | Subnet-unbound until Fleet ensure");
    println!();

    let release_build_id = context
        .release_build_id
        .expect("complete App builds own one durable release-build identity");
    let mut infrastructure = build_builtin_infrastructure(context)?;

    let configured_started_at = Instant::now();
    let activity = TerminalActivity::start(format!(
        "{} configured roles | {} profile | shared Cargo batch",
        roles.len(),
        context.profile.target_dir_name()
    ));
    let build = build_workspace_configured_canister_artifacts(context, roles);
    activity.finish();
    let outputs = build?;
    let configured_elapsed = configured_started_at.elapsed();
    let artifacts = classify_configured_artifacts(outputs)?;
    infrastructure.insert(
        1,
        InfrastructureCanisterArtifactBuildOutput {
            role: artifacts.fleet_subnet_root.role.clone(),
            deployment_scope: InfrastructureDeploymentScope::FleetSubnet,
            output: artifacts.fleet_subnet_root.output.clone(),
            timing: InfrastructureArtifactTiming::SharedConfiguredBatch(configured_elapsed),
        },
    );
    let release_manifest =
        persist_complete_release_set(context, release_build_id, &artifacts, &infrastructure)?;

    style.print_section(
        "Infrastructure Wasm",
        "placement comes from current desired Fleet state during ensure",
    );
    println!(
        "{}",
        render_infrastructure_build_table(&infrastructure, style)?
    );
    println!();

    style.print_section(
        "Application Wasm",
        &format!(
            "{} | {} Component artifacts | {:.2}s shared batch",
            options.app,
            artifacts.application.len(),
            configured_elapsed.as_secs_f64()
        ),
    );
    println!("{}", render_app_build_table(&artifacts.application, style)?);
    println!();

    style.print_section(
        "Build complete",
        &build_completion_detail(
            artifacts.application.len() + infrastructure.len(),
            "artifact",
            "artifacts",
            started_at.elapsed(),
        ),
    );
    println!(
        "Release build: {release_build_id}\nArtifacts: {}",
        context
            .icp_root
            .join(".canic/release-builds")
            .join(release_build_id.to_string())
            .join("artifacts")
            .display()
    );
    println!("Release manifest: {}", release_manifest.display());
    Ok(())
}

fn persist_complete_release_set(
    context: &WorkspaceBuildContext,
    release_build_id: ReleaseBuildId,
    artifacts: &ConfiguredArtifactClassification,
    infrastructure: &[InfrastructureCanisterArtifactBuildOutput],
) -> Result<PathBuf, BuildCommandError> {
    let config = AppConfigSnapshot::load(&context.config_path)?;
    let application_targets = artifacts
        .application
        .iter()
        .map(|built| application_target(&context.icp_root, built))
        .collect::<Result<Vec<_>, _>>()?;
    let application_outputs = artifacts
        .application
        .iter()
        .map(|built| ApplicationArtifactFileBuildOutput {
            role: CanisterRole::from(built.role.clone()),
            package: built.output.package_name.clone(),
            release_build_id,
            wasm_path: built.output.wasm_path.clone(),
            wasm_gz_path: built.output.wasm_gz_path.clone(),
            candid_sha256: built.output.candid_sha256,
            protocol_profile_digest: built.output.protocol_profile_digest,
        })
        .collect::<Vec<_>>();
    let application = compile_and_persist_application_artifact_union(
        &context.icp_root,
        config.component_topology(),
        release_build_id,
        &application_targets,
        &application_outputs,
    )
    .map_err(|error| BuildCommandError::Build(Box::new(error)))?;
    let infrastructure_outputs = infrastructure
        .iter()
        .map(|built| infrastructure_output(release_build_id, built))
        .collect::<Result<Vec<_>, _>>()?;
    let infrastructure = compile_and_persist_canic_infrastructure_artifact_manifest(
        &context.icp_root,
        release_build_id,
        &infrastructure_outputs,
    )
    .map_err(|error| BuildCommandError::Build(Box::new(error)))?;
    let complete = compile_and_persist_current_release_set_manifest(
        &context.icp_root,
        release_build_id,
        &application,
        &infrastructure,
    )
    .map_err(|error| BuildCommandError::Build(Box::new(error)))?;
    finalize_release_build_from_manifest(&context.icp_root, release_build_id, &complete.path)
        .map_err(|error| BuildCommandError::Build(Box::new(error)))?;
    Ok(complete.path)
}

fn application_target(
    icp_root: &Path,
    built: &ConfiguredCanisterArtifactBuildOutput,
) -> Result<ApplicationArtifactBuildTarget, BuildCommandError> {
    Ok(ApplicationArtifactBuildTarget {
        role: CanisterRole::from(built.role.clone()),
        package: built.output.package_name.clone(),
        wasm_relative_path: artifact_relative_path(icp_root, &built.output.wasm_path)?,
        wasm_gz_relative_path: artifact_relative_path(icp_root, &built.output.wasm_gz_path)?,
    })
}

fn infrastructure_output(
    release_build_id: ReleaseBuildId,
    built: &InfrastructureCanisterArtifactBuildOutput,
) -> Result<CanicInfrastructureArtifactBuildOutput, BuildCommandError> {
    let role = match built.role.as_str() {
        "fleet_coordinator" => CanicInfrastructureRole::FleetCoordinator,
        "root" => CanicInfrastructureRole::FleetSubnetRoot,
        "wasm_store" => CanicInfrastructureRole::WasmStore,
        role => {
            return Err(BuildCommandError::Build(
                format!("unexpected infrastructure build role {role}").into(),
            ));
        }
    };
    Ok(CanicInfrastructureArtifactBuildOutput {
        role,
        package: built.output.package_name.clone(),
        protocol_release_identity: built.output.protocol_release_identity.clone(),
        protocol_role: built.output.protocol_role.clone(),
        protocol_capabilities: built.output.protocol_capabilities.clone(),
        release_build_id,
        wasm_path: built.output.wasm_path.clone(),
        wasm_gz_path: built.output.wasm_gz_path.clone(),
        candid_sha256: built.output.candid_sha256,
        protocol_profile_digest: built.output.protocol_profile_digest,
    })
}

fn artifact_relative_path(icp_root: &Path, path: &Path) -> Result<String, BuildCommandError> {
    path.strip_prefix(icp_root)
        .map_err(|_| {
            BuildCommandError::Build(
                format!("build artifact is outside the ICP root: {}", path.display()).into(),
            )
        })?
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| {
            BuildCommandError::Build(
                format!("build artifact path is not UTF-8: {}", path.display()).into(),
            )
        })
}

fn build_builtin_infrastructure(
    context: &WorkspaceBuildContext,
) -> Result<Vec<InfrastructureCanisterArtifactBuildOutput>, BuildCommandError> {
    const BUILT_INS: [(&str, InfrastructureDeploymentScope); 2] = [
        ("fleet_coordinator", InfrastructureDeploymentScope::Fleet),
        ("wasm_store", InfrastructureDeploymentScope::FleetSubnet),
    ];

    let mut outputs = Vec::with_capacity(BUILT_INS.len());
    for (index, (role, deployment_scope)) in BUILT_INS.iter().enumerate() {
        let activity = TerminalActivity::start(format!(
            "[{}/{} infrastructure] {role} | {} profile",
            index + 1,
            BUILT_INS.len(),
            context.profile.target_dir_name()
        ));
        let started_at = Instant::now();
        let build = build_workspace_canister_artifact(&context.with_role(*role));
        activity.finish();
        outputs.push(InfrastructureCanisterArtifactBuildOutput {
            role: (*role).to_string(),
            deployment_scope: *deployment_scope,
            output: build?,
            timing: InfrastructureArtifactTiming::Dedicated(started_at.elapsed()),
        });
    }
    Ok(outputs)
}

fn build_completion_detail(
    item_count: usize,
    singular: &str,
    plural: &str,
    elapsed: Duration,
) -> String {
    let noun = if item_count == 1 { singular } else { plural };
    format!(
        "{item_count} {noun} | {:.2}s elapsed",
        elapsed.as_secs_f64()
    )
}

#[derive(Debug)]
struct ConfiguredArtifactClassification {
    application: Vec<ConfiguredCanisterArtifactBuildOutput>,
    fleet_subnet_root: ConfiguredCanisterArtifactBuildOutput,
}

fn classify_configured_artifacts(
    outputs: Vec<ConfiguredCanisterArtifactBuildOutput>,
) -> Result<ConfiguredArtifactClassification, BuildCommandError> {
    let (mut roots, application): (Vec<_>, Vec<_>) = outputs
        .into_iter()
        .partition(|built| built.role == CanisterRole::ROOT.as_str());
    if roots.len() != 1 {
        return Err(BuildCommandError::FleetSubnetRootArtifactCount {
            actual: roots.len(),
        });
    }
    let fleet_subnet_root = roots
        .pop()
        .ok_or(BuildCommandError::FleetSubnetRootArtifactCount { actual: 0 })?;

    Ok(ConfiguredArtifactClassification {
        application,
        fleet_subnet_root,
    })
}

enum InfrastructureArtifactTiming {
    SharedConfiguredBatch(Duration),
    Dedicated(Duration),
}

impl InfrastructureArtifactTiming {
    fn label(&self) -> String {
        match self {
            Self::SharedConfiguredBatch(elapsed) => {
                format!("{:.2}s shared", elapsed.as_secs_f64())
            }
            Self::Dedicated(elapsed) => format!("{:.2}s", elapsed.as_secs_f64()),
        }
    }
}

#[derive(Clone, Copy)]
enum InfrastructureDeploymentScope {
    Fleet,
    FleetSubnet,
}

impl InfrastructureDeploymentScope {
    const fn label(self) -> &'static str {
        match self {
            Self::Fleet => "1 / Fleet",
            Self::FleetSubnet => "1 / Fleet Subnet",
        }
    }
}

struct InfrastructureCanisterArtifactBuildOutput {
    role: String,
    deployment_scope: InfrastructureDeploymentScope,
    output: canic_host::canister_build::CanisterArtifactBuildOutput,
    timing: InfrastructureArtifactTiming,
}

fn render_app_build_table(
    outputs: &[ConfiguredCanisterArtifactBuildOutput],
    style: TerminalStyle,
) -> Result<String, BuildCommandError> {
    let rows = outputs
        .iter()
        .map(|built| {
            let wasm = std::fs::metadata(&built.output.wasm_path)?.len();
            let gzip = std::fs::metadata(&built.output.wasm_gz_path)
                .ok()
                .map(|metadata| metadata.len());
            Ok([
                built.role.clone(),
                built.output.package_version.clone(),
                style.success("done"),
                wasm_size_label(Some(wasm), gzip),
            ])
        })
        .collect::<Result<Vec<_>, BuildCommandError>>()?;
    Ok(render_bordered_table(
        &["ROLE", "VERSION", "STATUS", "WASM"],
        &rows,
        &[
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Right,
        ],
    ))
}

fn render_infrastructure_build_table(
    outputs: &[InfrastructureCanisterArtifactBuildOutput],
    style: TerminalStyle,
) -> Result<String, BuildCommandError> {
    let rows = outputs
        .iter()
        .map(|built| {
            let wasm = std::fs::metadata(&built.output.wasm_path)?.len();
            let gzip = std::fs::metadata(&built.output.wasm_gz_path)
                .ok()
                .map(|metadata| metadata.len());
            Ok([
                built.role.clone(),
                built.output.package_version.clone(),
                built.deployment_scope.label().to_string(),
                style.success("done"),
                wasm_size_label(Some(wasm), gzip),
                built.timing.label(),
            ])
        })
        .collect::<Result<Vec<_>, BuildCommandError>>()?;
    Ok(render_bordered_table(
        &[
            "CANISTER",
            "VERSION",
            "INSTANCES",
            "STATUS",
            "WASM",
            "ELAPSED",
        ],
        &rows,
        &[
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Right,
            ColumnAlign::Right,
        ],
    ))
}

fn write_build_provenance_if_requested(
    options: &BuildOptions,
    context: &WorkspaceBuildContext,
    output: canic_host::canister_build::CanisterArtifactBuildOutput,
) -> Result<(), BuildCommandError> {
    let Some(path) = &options.provenance else {
        return Ok(());
    };
    let role = options.role.as_ref().ok_or_else(|| {
        BuildCommandError::Usage("--provenance requires one selected role".to_string())
    })?;

    let request = BuildProvenanceRequest {
        app: options.app.clone(),
        role: role.clone(),
        environment: options.environment.clone(),
        build_network: context.build_network,
        profile: context.profile,
        workspace_root: context.workspace_root.clone(),
        config_path: context.config_path.clone(),
        output,
        command: build_command_provenance(options, &context.workspace_root),
        generated_at: current_evidence_timestamp()?,
        canic_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let envelope = build_provenance_envelope(&request)?;
    output::write_pretty_json_file::<_, BuildCommandError>(path, &envelope)?;
    Ok(())
}

fn build_command_provenance(options: &BuildOptions, workspace_root: &Path) -> CommandProvenanceV1 {
    let mut argv_normalized = vec![
        "canic".to_string(),
        "build".to_string(),
        options.app.clone(),
    ];
    if let Some(role) = &options.role {
        argv_normalized.push(role.clone());
    }
    argv_normalized.push("--profile".to_string());
    argv_normalized.push(options.profile.target_dir_name().to_string());
    if let Some(workspace) = &options.workspace {
        push_path_arg(
            &mut argv_normalized,
            "--workspace",
            workspace,
            workspace_root,
        );
    }
    if let Some(icp_root) = &options.icp_root {
        push_path_arg(&mut argv_normalized, "--icp-root", icp_root, workspace_root);
    }
    if let Some(config) = &options.config {
        push_path_arg(&mut argv_normalized, "--config", config, workspace_root);
    }
    if options.environment != local_environment() {
        argv_normalized.push("--environment".to_string());
        argv_normalized.push(options.environment.clone());
    }
    if !options.features.is_empty() {
        argv_normalized.push("--features".to_string());
        argv_normalized.push(
            options
                .features
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    if options.no_default_features {
        argv_normalized.push("--no-default-features".to_string());
    }
    if let Some(provenance) = &options.provenance {
        argv_normalized.push("--provenance".to_string());
        argv_normalized.push(command_path_for_root(provenance, workspace_root));
    }
    if options.standalone_local {
        argv_normalized.push("--standalone-local".to_string());
    }

    CommandProvenanceV1 {
        name: "canic build".to_string(),
        argv_normalized,
        argv_redactions: Vec::new(),
        format: "provenance".to_string(),
    }
}

fn push_path_arg(argv_normalized: &mut Vec<String>, name: &str, path: &str, root: &Path) {
    argv_normalized.push(name.to_string());
    argv_normalized.push(command_path_for_root(Path::new(path), root));
}

fn resolve_build_config_path(options: &BuildOptions) -> Result<PathBuf, BuildCommandError> {
    if let Some(config) = &options.config {
        let path = normalize_build_path(config)?;
        validate_config_app(&path, &options.app)?;
        return Ok(path);
    }

    let workspace_root = options.workspace.as_ref().map_or_else(
        || current_canic_workspace_root().map_err(BuildCommandError::from),
        |workspace| normalize_build_path(workspace),
    )?;
    let choices = discover_workspace_canic_config_choices(&workspace_root)?;
    if choices.is_empty() {
        return Err(BuildCommandError::NoConfigChoices);
    }

    select_discovered_app_config_path(&choices, &options.app)?
        .ok_or_else(|| BuildCommandError::UnknownApp(options.app.clone()))
}

fn validate_config_app(config_path: &Path, expected_app: &str) -> Result<(), BuildCommandError> {
    let actual_app = AppConfigSnapshot::load(config_path)?.app_id().to_string();
    if actual_app != expected_app {
        return Err(BuildCommandError::Usage(format!(
            "selected config declares app {actual_app:?}, not {expected_app:?}"
        )));
    }
    Ok(())
}

fn normalize_build_path(path: &str) -> Result<PathBuf, BuildCommandError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        env::current_dir()
            .map(|current_dir| current_dir.join(path))
            .map_err(BuildCommandError::from)
    }
}

fn resolve_build_context(
    options: &BuildOptions,
    config_path: PathBuf,
    selected_role: &str,
) -> Result<WorkspaceBuildContext, BuildCommandError> {
    let workspace_root = match &options.workspace {
        Some(workspace) => normalize_build_path(workspace)?.canonicalize()?,
        None => workspace_root()?,
    };
    let icp_root = match &options.icp_root {
        Some(root) => normalize_build_path(root)?.canonicalize()?,
        None => resolve_current_canic_icp_root()
            .map_err(|err| BuildCommandError::Build(Box::new(err)))?,
    };
    let build_network = resolve_build_network(&options.environment, &icp_root)?;
    Ok(WorkspaceBuildContext {
        role: selected_role.to_string(),
        profile: options.profile,
        environment: options.environment.clone(),
        build_network,
        workspace_root,
        icp_root,
        config_path,
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    })
}

fn resolve_build_network(
    environment: &str,
    icp_root: &Path,
) -> Result<BuildNetwork, BuildCommandError> {
    resolve_icp_build_network_from_root(icp_root, environment)
        .map_err(|err| BuildCommandError::Build(Box::new(err)))
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn build_parses_app_and_optional_role() {
        let options = BuildOptions::parse([OsString::from("demo"), OsString::from("app")])
            .expect("parse build options");

        assert_eq!(options.app, "demo");
        assert_eq!(options.role.as_deref(), Some("app"));
        assert_eq!(options.environment, "local");
        assert_eq!(options.profile, CanisterBuildProfile::Fast);
        assert_eq!(options.workspace, None);
        assert_eq!(options.icp_root, None);
        assert_eq!(options.config, None);
        assert!(options.features.is_empty());
        assert!(!options.no_default_features);
        assert_eq!(options.provenance, None);
        assert!(!options.standalone_local);
    }

    #[test]
    fn build_accepts_feature_selected_sidecar_only_runtime() {
        let options = BuildOptions::parse([
            OsString::from("demo"),
            OsString::from("app"),
            OsString::from("--standalone-local"),
            OsString::from("--features"),
            OsString::from("standalone-local,qualification"),
            OsString::from("--no-default-features"),
        ])
        .expect("parse standalone-local build options");

        assert_eq!(
            options.features,
            ["qualification", "standalone-local"]
                .map(str::to_string)
                .into_iter()
                .collect()
        );
        assert!(options.no_default_features);
        assert!(options.standalone_local);
        assert_eq!(
            options.artifact_build_options(),
            CanisterArtifactBuildOptions {
                cargo_features: options.features,
                default_features: false,
                sidecar_only_candid: true,
            }
        );
    }

    #[test]
    fn build_accepts_explicit_release_profile() {
        let options = BuildOptions::parse([
            OsString::from("demo"),
            OsString::from("app"),
            OsString::from("--profile"),
            OsString::from("release"),
        ])
        .expect("parse explicit release build profile");

        assert_eq!(options.profile, CanisterBuildProfile::Release);
    }

    #[test]
    fn build_accepts_internal_environment() {
        let options = BuildOptions::parse([
            OsString::from("demo"),
            OsString::from("app"),
            OsString::from("--__canic-environment"),
            OsString::from("localnet"),
        ])
        .expect("parse build options");

        assert_eq!(options.environment, "localnet");
    }

    #[test]
    fn build_resolves_named_ic_build_network_from_icp_yaml() {
        let root = temp_dir("canic-cli-build-environment");
        fs::create_dir_all(&root).expect("create root");
        fs::write(
            root.join("icp.yaml"),
            "environments:\n  - name: staging\n    network: ic\n",
        )
        .expect("write icp yaml");
        let mut options = build_options(&root, "demo", "app");
        options.environment = "staging".to_string();
        options.icp_root = Some(root.display().to_string());

        let build_network =
            resolve_build_network(&options.environment, &root).expect("resolve build network");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(build_network, BuildNetwork::Ic);
    }

    #[test]
    fn build_rejects_undeclared_named_environment() {
        let root = temp_dir("canic-cli-build-environment-missing");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("icp.yaml"), "environments: []\n").expect("write icp yaml");
        let mut options = build_options(&root, "demo", "app");
        options.environment = "staging".to_string();
        options.icp_root = Some(root.display().to_string());

        let err = resolve_build_network(&options.environment, &root)
            .expect_err("missing environment should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(err.to_string().contains("is not declared"));
    }

    #[test]
    fn build_accepts_explicit_context_paths() {
        let options = BuildOptions::parse([
            OsString::from("--workspace"),
            OsString::from("backend"),
            OsString::from("--icp-root"),
            OsString::from("."),
            OsString::from("--config"),
            OsString::from("backend/src/canisters/canic.toml"),
            OsString::from("--profile"),
            OsString::from("fast"),
            OsString::from("--provenance"),
            OsString::from("artifacts/root-provenance.json"),
            OsString::from("demo"),
            OsString::from("root"),
        ])
        .expect("parse build options");

        assert_eq!(options.app, "demo");
        assert_eq!(options.role.as_deref(), Some("root"));
        assert_eq!(options.profile, CanisterBuildProfile::Fast);
        assert_eq!(options.workspace.as_deref(), Some("backend"));
        assert_eq!(options.icp_root.as_deref(), Some("."));
        assert_eq!(
            options.config.as_deref(),
            Some("backend/src/canisters/canic.toml")
        );
        assert_eq!(
            options.provenance.as_deref(),
            Some(Path::new("artifacts/root-provenance.json"))
        );
    }

    #[test]
    fn build_preserves_workspace_discovery_causes() {
        let error = BuildCommandError::from(WorkspaceDiscoveryError::UnsupportedPath {
            path: PathBuf::from("/project/socket"),
        });

        std::assert_matches!(
            error,
            BuildCommandError::WorkspaceDiscovery(WorkspaceDiscoveryError::UnsupportedPath { .. })
        );
    }

    #[test]
    fn build_accepts_whole_app_selection() {
        let options =
            BuildOptions::parse([OsString::from("demo")]).expect("parse whole-App build options");

        assert_eq!(options.app, "demo");
        assert_eq!(options.role, None);
    }

    #[test]
    fn whole_app_build_rejects_role_provenance_output() {
        std::assert_matches!(
            BuildOptions::parse([
                OsString::from("demo"),
                OsString::from("--provenance"),
                OsString::from("build-provenance.json")
            ]),
            Err(BuildCommandError::Clap(_))
        );
    }

    #[test]
    fn whole_app_build_rejects_focused_feature_flags() {
        for flag_args in [
            vec!["--features", "standalone-local"],
            vec!["--no-default-features"],
            vec!["--standalone-local"],
        ] {
            let args = std::iter::once(OsString::from("demo"))
                .chain(flag_args.into_iter().map(OsString::from))
                .collect::<Vec<_>>();
            std::assert_matches!(BuildOptions::parse(args), Err(BuildCommandError::Clap(_)));
        }
    }

    #[test]
    fn build_rejects_invalid_profile() {
        let error = BuildOptions::parse([
            OsString::from("--profile"),
            OsString::from("tiny"),
            OsString::from("demo"),
            OsString::from("app"),
        ])
        .expect_err("invalid profile must fail");
        let BuildCommandError::Clap(error) = error else {
            panic!("expected Clap error");
        };

        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        assert!(error.to_string().starts_with("error: invalid value 'tiny'"));
        assert_eq!(BuildCommandError::Clap(error).exit_code(), 2);
    }

    #[test]
    fn build_help_and_version_are_clap_actions() {
        for (arg, kind) in [
            ("--help", clap::error::ErrorKind::DisplayHelp),
            ("--version", clap::error::ErrorKind::DisplayVersion),
        ] {
            let error = BuildOptions::parse([OsString::from(arg)])
                .expect_err("Clap display action must stop option parsing");
            let BuildCommandError::Clap(error) = error else {
                panic!("expected Clap display action");
            };

            assert_eq!(error.kind(), kind);
            assert_eq!(error.exit_code(), 0);
        }
    }

    #[test]
    fn build_usage_lists_app_and_optional_role() {
        let text = usage();

        assert!(text.contains("Usage: canic build [OPTIONS] <app> [role]"));
        assert!(text.contains("canic build demo"));
        assert!(text.contains("canic build demo app --standalone-local"));
        assert!(text.contains("[default: fast]"));
        assert!(text.contains("--features <feature,...>"));
        assert!(text.contains("--no-default-features"));
        assert!(text.contains("--provenance <file>"));
        assert!(text.contains("--standalone-local"));
        assert!(text.contains("Builds the configured Fleet Subnet Root"));
        assert!(text.contains("every attached Component role"));
        assert_eq!(text.matches("  canic build ").count(), 2);
    }

    #[test]
    fn whole_app_build_table_renders_padded_role_artifacts() {
        let root = temp_dir("canic-cli-build-table");
        let outputs = [ConfiguredCanisterArtifactBuildOutput {
            role: "app".to_string(),
            output: test_artifact_output(&root, "app", 2048, 512),
        }];

        let table = render_app_build_table(&outputs, TerminalStyle::detected())
            .expect("render build table");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(table.starts_with('+'));
        let headers = table
            .lines()
            .nth(1)
            .expect("build table header")
            .split('|')
            .filter_map(|cell| {
                let cell = cell.trim();
                (!cell.is_empty()).then_some(cell)
            })
            .collect::<Vec<_>>();
        assert_eq!(headers, ["ROLE", "VERSION", "STATUS", "WASM"]);
        assert!(table.contains("| app  |"));
        assert!(table.contains("done"));
        assert!(table.contains("2.00 KiB (gz 512.00 B)"));
        assert!(table.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn build_completion_reports_role_count_and_elapsed_time() {
        assert_eq!(
            build_completion_detail(1, "role", "roles", Duration::from_millis(1250)),
            "1 role | 1.25s elapsed"
        );
        assert_eq!(
            build_completion_detail(6, "artifact", "artifacts", Duration::from_secs(25)),
            "6 artifacts | 25.00s elapsed"
        );
    }

    #[test]
    fn infrastructure_build_table_includes_the_configured_root() {
        let root = temp_dir("canic-cli-infrastructure-build-table");
        let outputs = [
            InfrastructureCanisterArtifactBuildOutput {
                role: "fleet_coordinator".to_string(),
                deployment_scope: InfrastructureDeploymentScope::Fleet,
                output: test_artifact_output(&root, "fleet_coordinator", 4096, 1024),
                timing: InfrastructureArtifactTiming::Dedicated(Duration::from_millis(2750)),
            },
            InfrastructureCanisterArtifactBuildOutput {
                role: "root".to_string(),
                deployment_scope: InfrastructureDeploymentScope::FleetSubnet,
                output: test_artifact_output(&root, "root", 2048, 512),
                timing: InfrastructureArtifactTiming::SharedConfiguredBatch(Duration::from_secs(
                    15,
                )),
            },
        ];

        let table = render_infrastructure_build_table(&outputs, TerminalStyle::detected())
            .expect("render infrastructure table");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(table.contains("| CANISTER"));
        assert!(table.contains("| VERSION"));
        assert!(table.contains("| INSTANCES"));
        assert!(table.contains("fleet_coordinator"));
        assert!(table.contains("root"));
        assert!(table.contains("1 / Fleet Subnet"));
        assert!(table.contains("4.00 KiB (gz 1.00 KiB)"));
        assert!(table.contains("2.75s"));
        assert!(table.contains("15.00s shared"));
    }

    #[test]
    fn configured_artifacts_classify_root_as_infrastructure() {
        let root = temp_dir("canic-cli-configured-artifact-classification");
        let outputs = vec![
            ConfiguredCanisterArtifactBuildOutput {
                role: "root".to_string(),
                output: test_artifact_output(&root, "root", 2048, 512),
            },
            ConfiguredCanisterArtifactBuildOutput {
                role: "app".to_string(),
                output: test_artifact_output(&root, "app", 1024, 256),
            },
        ];

        let artifacts = classify_configured_artifacts(outputs).expect("classify artifacts");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(artifacts.fleet_subnet_root.role, "root");
        assert_eq!(
            artifacts
                .application
                .iter()
                .map(|built| built.role.as_str())
                .collect::<Vec<_>>(),
            ["app"]
        );
    }

    #[test]
    fn configured_artifacts_require_exactly_one_root() {
        let root = temp_dir("canic-cli-configured-artifact-root-count");
        let app_only = vec![ConfiguredCanisterArtifactBuildOutput {
            role: "app".to_string(),
            output: test_artifact_output(&root, "app", 1024, 256),
        }];
        std::assert_matches!(
            classify_configured_artifacts(app_only),
            Err(BuildCommandError::FleetSubnetRootArtifactCount { actual: 0 })
        );

        let duplicate_roots = vec![
            ConfiguredCanisterArtifactBuildOutput {
                role: "root".to_string(),
                output: test_artifact_output(&root, "root-a", 2048, 512),
            },
            ConfiguredCanisterArtifactBuildOutput {
                role: "root".to_string(),
                output: test_artifact_output(&root, "root-b", 2048, 512),
            },
        ];
        std::assert_matches!(
            classify_configured_artifacts(duplicate_roots),
            Err(BuildCommandError::FleetSubnetRootArtifactCount { actual: 2 })
        );

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn build_command_provenance_redacts_paths_outside_workspace() {
        let root = temp_dir("canic-cli-build-provenance-command");
        fs::create_dir_all(&root).expect("create root");
        let outside = temp_dir("canic-cli-build-provenance-outside");
        fs::create_dir_all(&outside).expect("create outside");
        let mut options = build_options(&root, "demo", "app");
        options.provenance = Some(outside.join("build-provenance.json"));
        options.features = ["qualification", "standalone-local"]
            .map(str::to_string)
            .into_iter()
            .collect();
        options.no_default_features = true;
        options.standalone_local = true;

        let provenance = build_command_provenance(&options, &root);

        fs::remove_dir_all(root).expect("remove root");
        fs::remove_dir_all(outside).expect("remove outside");
        assert!(
            provenance
                .argv_normalized
                .contains(&"<redacted:absolute-outside-root>".to_string())
        );
        assert!(
            provenance
                .argv_normalized
                .windows(2)
                .any(|args| args[0] == "--profile" && args[1] == "fast")
        );
        assert!(provenance.argv_normalized.windows(2).any(|args| {
            args[0] == "--features" && args[1] == "qualification,standalone-local"
        }));
        assert!(
            provenance
                .argv_normalized
                .contains(&"--no-default-features".to_string())
        );
        assert!(
            provenance
                .argv_normalized
                .contains(&"--standalone-local".to_string())
        );
    }

    #[test]
    fn build_resolves_config_from_selected_app() {
        let root = temp_dir("canic-cli-build-config");
        let config_path = write_build_config(&root, true);
        let options = build_options(&root, "demo", "app");

        let resolved = resolve_build_config_path(&options).expect("resolve build config");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(resolved, config_path);
    }

    #[test]
    fn build_preflight_rejects_declared_only_role() {
        let root = temp_dir("canic-cli-build-declared-only");
        write_build_config(&root, false);
        let options = build_options(&root, "demo", "app");

        let config_path = resolve_build_config_path(&options).expect("resolve config");
        selected_build_roles(&options, &config_path).expect_err("declared-only role should fail");

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn build_preflight_accepts_attached_role() {
        let root = temp_dir("canic-cli-build-attached");
        write_build_config(&root, true);
        let options = build_options(&root, "demo", "app");

        let config_path = resolve_build_config_path(&options).expect("resolve config");
        assert_eq!(
            selected_build_roles(&options, &config_path).expect("attached role should pass"),
            ["app"]
        );

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn explicit_build_config_must_match_selected_app() {
        let root = temp_dir("canic-cli-build-app-mismatch");
        let config_path = write_build_config(&root, true);
        let mut options = build_options(&root, "other", "app");
        options.config = Some(config_path.display().to_string());

        resolve_build_config_path(&options).expect_err("app mismatch should fail");

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn whole_app_build_selects_root_and_every_attached_component_role() {
        let root = temp_dir("canic-cli-build-whole-app");
        let config_path = write_build_config(&root, true);
        let mut options = build_options(&root, "demo", "app");
        options.role = None;

        let roles = selected_build_roles(&options, &config_path).expect("select deployable roles");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(roles, ["root", "app"]);
    }

    fn build_options(root: &std::path::Path, app: &str, role: &str) -> BuildOptions {
        BuildOptions {
            app: app.to_string(),
            role: Some(role.to_string()),
            environment: "local".to_string(),
            profile: CanisterBuildProfile::Fast,
            workspace: Some(root.display().to_string()),
            icp_root: None,
            config: None,
            features: BTreeSet::new(),
            no_default_features: false,
            provenance: None,
            standalone_local: false,
        }
    }

    fn test_artifact_output(
        root: &Path,
        role: &str,
        wasm_size: usize,
        gzip_size: usize,
    ) -> canic_host::canister_build::CanisterArtifactBuildOutput {
        let artifact_root = root.join(role);
        fs::create_dir_all(&artifact_root).expect("create artifact root");
        let wasm_path = artifact_root.join(format!("{role}.wasm"));
        let wasm_gz_path = artifact_root.join(format!("{role}.wasm.gz"));
        fs::write(&wasm_path, vec![0_u8; wasm_size]).expect("write wasm");
        fs::write(&wasm_gz_path, vec![0_u8; gzip_size]).expect("write gzip");
        canic_host::canister_build::CanisterArtifactBuildOutput {
            package_name: format!("canister_{role}"),
            package_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_release_identity: env!("CARGO_PKG_VERSION").to_string(),
            protocol_role: canic_core::ids::CanisterRole::owned(role.to_string()),
            protocol_capabilities: std::collections::BTreeSet::new(),
            artifact_root: artifact_root.clone(),
            wasm_path,
            wasm_gz_path,
            did_path: artifact_root.join(format!("{role}.did")),
            candid_sha256: [0; 32],
            protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
                [0; 32],
            ),
            transforms: Vec::new(),
        }
    }

    fn write_build_config(root: &std::path::Path, attach_app: bool) -> PathBuf {
        let app_dir = root.join("apps/demo");
        fs::create_dir_all(&app_dir).expect("create app dir");
        fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n")
            .expect("write workspace manifest");
        let mut config = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[auth.delegated_tokens]
enabled = false


"#
        .to_string();
        if attach_app {
            config.push_str(
                r#"
[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
            );
        }
        let config_path = app_dir.join("canic.toml");
        fs::write(&config_path, config).expect("write canic config");
        config_path
    }
}
