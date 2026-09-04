//! Module: canic_cli::toolchain
//!
//! Responsibility: install checksum-authoritative external tools required by Canic builds.
//! Does not own: release-Wasm transformation policy, Cargo toolchains, or arbitrary tool versions.
//! Boundary: exposes one governed complete Wasm-tool installation without configuration axes.

use crate::{
    cli::{
        clap::{parse_required_subcommand, passthrough_subcommand, render_usage},
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::build_toolchain::{BuildToolchainError, install_required_build_toolchain};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

/// CLI boundary error for governed release-tool installation.
#[derive(Debug, ThisError)]
pub enum ToolchainCommandError {
    #[error(transparent)]
    BuildToolchain(#[from] BuildToolchainError),

    #[error("{0}")]
    Usage(String),
}

/// Install the complete checksum-authoritative Wasm toolchain projection.
pub fn run<I>(args: I) -> Result<(), ToolchainCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let (command, args) = parse_required_subcommand(toolchain_command(), args)
        .map_err(|_| ToolchainCommandError::Usage(usage()))?;
    match command.as_str() {
        "install" => run_install(args),
        _ => unreachable!("toolchain dispatch command only defines known commands"),
    }
}

fn run_install(args: Vec<OsString>) -> Result<(), ToolchainCommandError> {
    if print_help_or_version(&args, install_usage, version_text()) {
        return Ok(());
    }
    if !args.is_empty() {
        return Err(ToolchainCommandError::Usage(install_usage()));
    }
    let installed = install_required_build_toolchain()?;
    println!("ic-wasm={}", installed.ic_wasm().path().display());
    println!("wasm-opt={}", installed.binaryen().path().display());
    eprintln!(
        "Canic builds use these admitted executable paths directly. To invoke the tools yourself, place {} on PATH.",
        installed.install_directory().display()
    );
    if let Some(warning) = installed.root_home_warning() {
        eprintln!("{warning}");
    }
    Ok(())
}

fn usage() -> String {
    render_usage(toolchain_command)
}

fn install_usage() -> String {
    render_usage(install_command)
}

fn toolchain_command() -> ClapCommand {
    ClapCommand::new("toolchain")
        .bin_name("canic toolchain")
        .about("Install checksum-authoritative Canic release tools")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("install")
                .about("Install pinned ic-wasm and Binaryen tools")
                .disable_help_flag(true),
        ))
}

fn install_command() -> ClapCommand {
    ClapCommand::new("install")
        .bin_name("canic toolchain install")
        .about("Install pinned ic-wasm and Binaryen tools")
        .disable_help_flag(true)
        .after_help("Output: prints the absolute admitted ic-wasm and wasm-opt paths.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_is_available_without_installing_tools() {
        run([OsString::from("--help")]).expect("toolchain help");
        run([OsString::from("install"), OsString::from("--help")]).expect("toolchain install help");
    }

    #[test]
    fn install_rejects_configuration_arguments() {
        assert!(matches!(
            run([
                OsString::from("install"),
                OsString::from("--channel"),
                OsString::from("custom"),
            ]),
            Err(ToolchainCommandError::Usage(_))
        ));
    }
}
