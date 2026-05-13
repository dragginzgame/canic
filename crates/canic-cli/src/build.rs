use crate::{
    cli::{
        clap::{parse_matches, string_option, value_arg},
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact, copy_icp_wasm_output,
    print_current_workspace_build_context_once,
};
use clap::Command as ClapCommand;
use std::{env, ffi::OsString};
use thiserror::Error as ThisError;

const BUILD_HELP_AFTER: &str = "\
Examples:
  canic build app
  canic --network local build root
  canic build --profile fast --workspace backend --icp-root . --config backend/src/canisters/canic.toml root

The selected role must have a canister manifest at the conventional fleet path.
The command writes .icp/local/canisters/<role>/<role>.wasm and .wasm.gz.";

///
/// BuildCommandError
///

#[derive(Debug, ThisError)]
pub enum BuildCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Build(#[from] Box<dyn std::error::Error>),
}

///
/// BuildOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildOptions {
    pub canister: String,
    pub network: String,
    pub profile: Option<CanisterBuildProfile>,
    pub workspace: Option<String>,
    pub icp_root: Option<String>,
    pub config: Option<String>,
}

impl BuildOptions {
    pub fn parse<I>(args: I) -> Result<Self, BuildCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(build_command(), args).map_err(|_| BuildCommandError::Usage(usage()))?;

        Ok(Self {
            canister: string_option(&matches, "canister").expect("clap requires canister"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            profile: string_option(&matches, "profile")
                .as_deref()
                .map(parse_profile)
                .transpose()?,
            workspace: string_option(&matches, "workspace"),
            icp_root: string_option(&matches, "icp-root"),
            config: string_option(&matches, "config"),
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
    let _guard = BuildEnvGuard::apply(&options);
    let profile = options
        .profile
        .unwrap_or_else(CanisterBuildProfile::current);
    print_current_workspace_build_context_once(profile)?;
    let output = build_current_workspace_canister_artifact(&options.canister, profile)?;
    copy_icp_wasm_output(&options.canister, &output)?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}

fn build_command() -> ClapCommand {
    ClapCommand::new("build")
        .bin_name("canic build")
        .about("Build one Canic canister artifact")
        .disable_help_flag(true)
        .override_usage("canic build <role>")
        .arg(
            value_arg("canister")
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
                .help("Canister wasm build profile; defaults to CANIC_WASM_PROFILE or release"),
        )
        .arg(internal_network_arg())
        .after_help(BUILD_HELP_AFTER)
}

fn usage() -> String {
    let mut command = build_command();
    command.render_help().to_string()
}

fn parse_profile(value: &str) -> Result<CanisterBuildProfile, BuildCommandError> {
    match value {
        "debug" => Ok(CanisterBuildProfile::Debug),
        "fast" => Ok(CanisterBuildProfile::Fast),
        "release" => Ok(CanisterBuildProfile::Release),
        _ => Err(BuildCommandError::Usage(format!(
            "invalid build profile: {value}\n\n{}",
            usage()
        ))),
    }
}

struct BuildEnvGuard {
    previous_network: Option<OsString>,
    previous_workspace: Option<OsString>,
    previous_icp_root: Option<OsString>,
    previous_config: Option<OsString>,
}

impl BuildEnvGuard {
    fn apply(options: &BuildOptions) -> Self {
        let guard = Self {
            previous_network: env::var_os("ICP_ENVIRONMENT"),
            previous_workspace: env::var_os("CANIC_WORKSPACE_ROOT"),
            previous_icp_root: env::var_os("CANIC_ICP_ROOT"),
            previous_config: env::var_os("CANIC_CONFIG_PATH"),
        };
        set_env("ICP_ENVIRONMENT", &options.network);
        set_optional_env("CANIC_WORKSPACE_ROOT", options.workspace.as_deref());
        set_optional_env("CANIC_ICP_ROOT", options.icp_root.as_deref());
        set_optional_env("CANIC_CONFIG_PATH", options.config.as_deref());
        guard
    }
}

impl Drop for BuildEnvGuard {
    fn drop(&mut self) {
        restore_env("ICP_ENVIRONMENT", self.previous_network.take());
        restore_env("CANIC_WORKSPACE_ROOT", self.previous_workspace.take());
        restore_env("CANIC_ICP_ROOT", self.previous_icp_root.take());
        restore_env("CANIC_CONFIG_PATH", self.previous_config.take());
    }
}

fn set_optional_env(key: &str, value: Option<&str>) {
    if let Some(value) = value {
        set_env(key, value);
    }
}

fn set_env<K, V>(key: K, value: V)
where
    K: AsRef<std::ffi::OsStr>,
    V: AsRef<std::ffi::OsStr>,
{
    // Artifact builds are single-threaded CLI orchestration; the scoped env
    // value selects the ICP artifact directory seen by Cargo build scripts.
    unsafe {
        env::set_var(key, value);
    }
}

fn restore_env(key: &str, value: Option<OsString>) {
    // See set_env: this restores the single-threaded artifact build context.
    unsafe {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_parses_required_role() {
        let options = BuildOptions::parse([OsString::from("app")]).expect("parse build options");

        assert_eq!(options.canister, "app");
        assert_eq!(options.network, "local");
        assert_eq!(options.profile, None);
        assert_eq!(options.workspace, None);
        assert_eq!(options.icp_root, None);
        assert_eq!(options.config, None);
    }

    #[test]
    fn build_accepts_internal_network() {
        let options = BuildOptions::parse([
            OsString::from("app"),
            OsString::from("--__canic-network"),
            OsString::from("demo"),
        ])
        .expect("parse build options");

        assert_eq!(options.network, "demo");
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
            OsString::from("root"),
        ])
        .expect("parse build options");

        assert_eq!(options.canister, "root");
        assert_eq!(options.profile, Some(CanisterBuildProfile::Fast));
        assert_eq!(options.workspace.as_deref(), Some("backend"));
        assert_eq!(options.icp_root.as_deref(), Some("."));
        assert_eq!(
            options.config.as_deref(),
            Some("backend/src/canisters/canic.toml")
        );
    }

    #[test]
    fn build_requires_role() {
        assert!(matches!(
            BuildOptions::parse(Vec::<OsString>::new()),
            Err(BuildCommandError::Usage(_))
        ));
    }

    #[test]
    fn build_rejects_invalid_profile() {
        assert!(matches!(
            BuildOptions::parse([
                OsString::from("--profile"),
                OsString::from("tiny"),
                OsString::from("app")
            ]),
            Err(BuildCommandError::Usage(_))
        ));
    }
}
