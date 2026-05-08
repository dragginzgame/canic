use crate::{
    args::{parse_matches, print_help_or_version},
    version_text,
};
use canic_host::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact,
    print_current_workspace_build_context_once,
};
use clap::{Arg, Command as ClapCommand};
use std::{ffi::OsString, time::Instant};
use thiserror::Error as ThisError;

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
    pub canister_name: String,
}

impl BuildOptions {
    /// Parse build options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BuildCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(build_command(), args).map_err(|_| BuildCommandError::Usage(usage()))?;
        let canister_name = matches
            .get_one::<String>("canister-name")
            .expect("clap requires canister-name")
            .clone();

        Ok(Self { canister_name })
    }
}

// Build the canister-artifact parser.
fn build_command() -> ClapCommand {
    ClapCommand::new("build")
        .bin_name("canic build")
        .about("Build one Canic canister artifact")
        .disable_help_flag(true)
        .arg(
            Arg::new("canister-name")
                .value_name("canister-name")
                .required(true),
        )
}

/// Run one Canic canister artifact build.
pub fn run<I>(args: I) -> Result<(), BuildCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = BuildOptions::parse(args)?;
    build_canister(options).map_err(BuildCommandError::from)
}

// Build the requested canister and print the artifact path for caller build scripts.
fn build_canister(options: BuildOptions) -> Result<(), Box<dyn std::error::Error>> {
    let profile = CanisterBuildProfile::current();
    print_current_workspace_build_context_once(profile)?;
    eprintln!(
        "Canic build start: canister={} profile={}",
        options.canister_name,
        profile.target_dir_name()
    );

    let started_at = Instant::now();
    let output = build_current_workspace_canister_artifact(&options.canister_name, profile)?;
    let elapsed = started_at.elapsed().as_secs_f64();

    println!("{}", output.wasm_gz_path.display());
    eprintln!(
        "Canic build done: canister={} elapsed={elapsed:.2}s",
        options.canister_name
    );
    eprintln!();
    Ok(())
}

// Return build command usage text.
fn usage() -> String {
    let mut command = build_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure build requires one canister name and preserves it exactly.
    #[test]
    fn parses_build_canister_name() {
        let options = BuildOptions::parse([OsString::from("root")]).expect("parse build");

        assert_eq!(options.canister_name, "root");
    }

    // Ensure build rejects missing canister names.
    #[test]
    fn rejects_missing_build_canister_name() {
        assert!(matches!(
            BuildOptions::parse([]),
            Err(BuildCommandError::Usage(_))
        ));
    }
}
