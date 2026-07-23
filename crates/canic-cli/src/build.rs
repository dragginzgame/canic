//! Module: canic_cli::build
//!
//! Responsibility: build one role artifact and optionally emit build provenance.
//! Does not own: canister build execution, app config schema, or evidence envelope schemas.
//! Boundary: resolves CLI build context, validates attached roles, and delegates artifact creation.

use crate::{
    cli::{
        clap::{
            parse_matches, render_usage, required_string, string_option, string_option_or_else,
            typed_option, value_arg,
        },
        defaults::local_environment,
        globals::internal_environment_arg,
        help::print_help_or_version,
    },
    output, version_text,
};
use canic_core::ids::BuildNetwork;
use canic_host::build_provenance::{BuildProvenanceRequest, build_provenance_envelope};
use canic_host::canister_build::{
    CanisterBuildProfile, WorkspaceBuildContext, build_workspace_canister_artifact,
    copy_icp_wasm_output, print_workspace_build_context_once,
};
use canic_host::evidence_envelope::{CommandProvenanceV1, command_path_for_root};
use canic_host::{
    icp_config::{resolve_current_canic_icp_root, resolve_icp_build_network_from_root},
    install_root::{
        ConfigDiscoveryError, current_canic_project_root, discover_project_canic_config_choices,
        select_discovered_app_config_path,
    },
    release_set::{AppConfigError, AppConfigSnapshot, WorkspaceDiscoveryError, workspace_root},
};
use clap::Command as ClapCommand;
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const BUILD_HELP_AFTER: &str = "\
Examples:
  canic build demo app
  canic build demo app --provenance artifacts/app-provenance.json
  canic --environment local build demo root
  canic build --profile fast --workspace backend --icp-root . --config backend/apps/demo/canic.toml demo root

The selected app must have a matching canic.toml, and the selected role must
be attached to topology before an artifact build is allowed.
The command writes .icp/local/canisters/<role>/<role>.wasm and .wasm.gz.
Use --provenance <path> to additionally write a stable EvidenceEnvelopeV1
containing canic.build_provenance.v1.";

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

    #[error("failed to discover Canic project configs: {0}")]
    ConfigDiscovery(#[from] ConfigDiscoveryError),

    #[error("failed to resolve Cargo workspace: {0}")]
    WorkspaceDiscovery(#[from] WorkspaceDiscoveryError),

    #[error(transparent)]
    Build(#[from] Box<dyn std::error::Error>),

    #[error(transparent)]
    AppConfig(#[from] AppConfigError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Parsed `canic build` command options.

#[derive(Clone, Debug, Eq, PartialEq)]
struct BuildOptions {
    app: String,
    role: String,
    environment: String,
    profile: Option<CanisterBuildProfile>,
    workspace: Option<String>,
    icp_root: Option<String>,
    config: Option<String>,
    provenance: Option<PathBuf>,
}

impl BuildOptions {
    fn parse<I>(args: I) -> Result<Self, BuildCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(build_command(), args).map_err(|_| BuildCommandError::Usage(usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
            role: required_string(&matches, "role"),
            environment: string_option_or_else(&matches, "environment", local_environment),
            profile: typed_option(&matches, "profile"),
            workspace: string_option(&matches, "workspace"),
            icp_root: string_option(&matches, "icp-root"),
            config: string_option(&matches, "config"),
            provenance: string_option(&matches, "provenance").map(PathBuf::from),
        })
    }
}

/// Build one Canic canister artifact through the installed CLI.
pub fn run<I>(args: I) -> Result<(), BuildCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = BuildOptions::parse(args)?;
    let context = resolve_build_context(&options)?;
    print_workspace_build_context_once(&context)?;
    validate_attached_role(&options, &context.config_path)?;
    let output = build_workspace_canister_artifact(&context)?;
    copy_icp_wasm_output(&options.role, &output)?;
    write_build_provenance_if_requested(&options, &context, output.clone())?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}

fn build_command() -> ClapCommand {
    ClapCommand::new("build")
        .bin_name("canic build")
        .about("Build one Canic canister artifact")
        .disable_help_flag(true)
        .override_usage("canic build [OPTIONS] <app> <role>")
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Config-defined app name to build from"),
        )
        .arg(
            value_arg("role")
                .value_name("role")
                .required(true)
                .help("Config-defined canister role to build"),
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
            value_arg("profile")
                .long("profile")
                .value_name("debug|fast|release")
                .num_args(1)
                .value_parser(clap::value_parser!(CanisterBuildProfile))
                .help("Canister wasm build profile; defaults to release"),
        )
        .arg(
            value_arg("provenance")
                .long("provenance")
                .value_name("file")
                .num_args(1)
                .help("Write an EvidenceEnvelopeV1 build provenance artifact to this file"),
        )
        .arg(internal_environment_arg())
        .after_help(BUILD_HELP_AFTER)
}

fn usage() -> String {
    render_usage(build_command)
}

fn validate_attached_role(
    options: &BuildOptions,
    config_path: &Path,
) -> Result<(), BuildCommandError> {
    let roles = AppConfigSnapshot::load(config_path)?.role_lifecycle();
    let Some(row) = roles.iter().find(|row| row.role == options.role) else {
        return Err(BuildCommandError::Usage(format!(
            "role {}.{} is not declared in {}",
            options.app,
            options.role,
            config_path.display()
        )));
    };
    if !row.attached {
        return Err(BuildCommandError::Usage(format!(
            "role {}.{} is declared but not attached to topology; run `canic app role attach {} {} --subnet <subnet>` before building an artifact",
            options.app, options.role, options.app, options.role
        )));
    }
    Ok(())
}

fn write_build_provenance_if_requested(
    options: &BuildOptions,
    context: &WorkspaceBuildContext,
    output: canic_host::canister_build::CanisterArtifactBuildOutput,
) -> Result<(), BuildCommandError> {
    let Some(path) = &options.provenance else {
        return Ok(());
    };

    let request = BuildProvenanceRequest {
        app: options.app.clone(),
        role: options.role.clone(),
        environment: options.environment.clone(),
        build_network: context.build_network,
        profile: context.profile,
        workspace_root: context.workspace_root.clone(),
        config_path: context.config_path.clone(),
        output,
        command: build_command_provenance(options, &context.workspace_root),
        generated_at: current_build_generated_at()?,
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
        options.role.clone(),
    ];
    if let Some(profile) = options.profile {
        argv_normalized.push("--profile".to_string());
        argv_normalized.push(profile.target_dir_name().to_string());
    }
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
    if let Some(provenance) = &options.provenance {
        argv_normalized.push("--provenance".to_string());
        argv_normalized.push(command_path_for_root(provenance, workspace_root));
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

fn current_build_generated_at() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "unix:{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    ))
}

fn resolve_build_config_path(options: &BuildOptions) -> Result<PathBuf, BuildCommandError> {
    if let Some(config) = &options.config {
        let path = normalize_build_path(config)?;
        validate_config_app(&path, &options.app)?;
        return Ok(path);
    }

    let project_root = options.workspace.as_ref().map_or_else(
        || current_canic_project_root().map_err(BuildCommandError::from),
        |workspace| normalize_build_path(workspace),
    )?;
    let choices = discover_project_canic_config_choices(&project_root)?;
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
) -> Result<WorkspaceBuildContext, BuildCommandError> {
    let config_path = resolve_build_config_path(options)?.canonicalize()?;
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
    let profile = options.profile.unwrap_or(CanisterBuildProfile::Release);

    Ok(WorkspaceBuildContext {
        role: options.role.clone(),
        profile,
        environment: options.environment.clone(),
        build_network,
        workspace_root,
        icp_root,
        config_path,
        local_replica: None,
        refresh_canonical_wasm_store_did: false,
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
    fn build_parses_required_app_and_role() {
        let options = BuildOptions::parse([OsString::from("demo"), OsString::from("app")])
            .expect("parse build options");

        assert_eq!(options.app, "demo");
        assert_eq!(options.role, "app");
        assert_eq!(options.environment, "local");
        assert_eq!(options.profile, None);
        assert_eq!(options.workspace, None);
        assert_eq!(options.icp_root, None);
        assert_eq!(options.config, None);
        assert_eq!(options.provenance, None);
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
        assert_eq!(options.role, "root");
        assert_eq!(options.profile, Some(CanisterBuildProfile::Fast));
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
    fn build_requires_role() {
        std::assert_matches!(
            BuildOptions::parse([OsString::from("demo")]),
            Err(BuildCommandError::Usage(_))
        );
    }

    #[test]
    fn build_rejects_invalid_profile() {
        std::assert_matches!(
            BuildOptions::parse([
                OsString::from("--profile"),
                OsString::from("tiny"),
                OsString::from("demo"),
                OsString::from("app")
            ]),
            Err(BuildCommandError::Usage(_))
        );
    }

    #[test]
    fn build_usage_lists_app_and_role() {
        let text = usage();

        assert!(text.contains("Usage: canic build [OPTIONS] <app> <role>"));
        assert!(text.contains("canic build demo app"));
        assert!(text.contains("--provenance <file>"));
        assert!(text.contains("be attached to topology"));
    }

    #[test]
    fn build_command_provenance_redacts_paths_outside_workspace() {
        let root = temp_dir("canic-cli-build-provenance-command");
        fs::create_dir_all(&root).expect("create root");
        let outside = temp_dir("canic-cli-build-provenance-outside");
        fs::create_dir_all(&outside).expect("create outside");
        let mut options = build_options(&root, "demo", "app");
        options.provenance = Some(outside.join("build-provenance.json"));

        let provenance = build_command_provenance(&options, &root);

        fs::remove_dir_all(root).expect("remove root");
        fs::remove_dir_all(outside).expect("remove outside");
        assert!(
            provenance
                .argv_normalized
                .contains(&"<redacted:absolute-outside-root>".to_string())
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
        validate_attached_role(&options, &config_path).expect_err("declared-only role should fail");

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn build_preflight_accepts_attached_role() {
        let root = temp_dir("canic-cli-build-attached");
        write_build_config(&root, true);
        let options = build_options(&root, "demo", "app");

        let config_path = resolve_build_config_path(&options).expect("resolve config");
        validate_attached_role(&options, &config_path).expect("attached role should pass");

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

    fn build_options(root: &std::path::Path, app: &str, role: &str) -> BuildOptions {
        BuildOptions {
            app: app.to_string(),
            role: role.to_string(),
            environment: "local".to_string(),
            profile: None,
            workspace: Some(root.display().to_string()),
            icp_root: None,
            config: None,
            provenance: None,
        }
    }

    fn write_build_config(root: &std::path::Path, attach_app: bool) -> PathBuf {
        let app_dir = root.join("apps/demo");
        fs::create_dir_all(&app_dir).expect("create app dir");
        fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n")
            .expect("write workspace manifest");
        let mut config = r#"
controllers = []
[services.fleet]
roles = []

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

[subnets.default.canisters.root]
kind = "root"
"#
        .to_string();
        if attach_app {
            config.push_str(
                r#"
[subnets.default.canisters.app]
kind = "service"
"#,
            );
        }
        let config_path = app_dir.join("canic.toml");
        fs::write(&config_path, config).expect("write canic config");
        config_path
    }
}
