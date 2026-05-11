use crate::{
    cli::clap::{parse_matches, parse_subcommand, passthrough_subcommand, path_option, value_arg},
    cli::help::print_help_or_version,
    output, version_text,
};
use canic_backup::manifest::{
    FleetBackupManifest, ManifestValidationError, manifest_validation_summary,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, fs, path::PathBuf};
use thiserror::Error as ThisError;

///
/// ManifestCommandError
///

#[derive(Debug, ThisError)]
pub enum ManifestCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),
}

///
/// ManifestValidateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestValidateOptions {
    pub manifest: PathBuf,
    pub out: Option<PathBuf>,
}

impl ManifestValidateOptions {
    pub fn parse<I>(args: I) -> Result<Self, ManifestCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(manifest_validate_command(), args)
            .map_err(|_| ManifestCommandError::Usage(validate_usage()))?;

        Ok(Self {
            manifest: path_option(&matches, "manifest").expect("clap requires manifest"),
            out: path_option(&matches, "out"),
        })
    }
}

fn manifest_validate_command() -> ClapCommand {
    ClapCommand::new("validate")
        .bin_name("canic manifest validate")
        .about("Validate a fleet backup manifest")
        .disable_help_flag(true)
        .arg(
            value_arg("manifest")
                .long("manifest")
                .value_name("file")
                .required(true),
        )
        .arg(value_arg("out").long("out").value_name("file"))
}

/// Run a manifest subcommand.
pub fn run<I>(args: I) -> Result<(), ManifestCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) = parse_subcommand(manifest_command(), args)
        .map_err(|_| ManifestCommandError::Usage(usage()))?
    else {
        return Err(ManifestCommandError::Usage(usage()));
    };

    match command.as_str() {
        "validate" => {
            if print_help_or_version(&args, validate_usage, version_text()) {
                return Ok(());
            }
            let options = ManifestValidateOptions::parse(args)?;
            let manifest = validate_manifest(&options)?;
            write_validation_summary(&options, &manifest)?;
            Ok(())
        }
        _ => unreachable!("manifest dispatch command only defines known commands"),
    }
}

/// Read and validate a fleet backup manifest from disk.
pub fn validate_manifest(
    options: &ManifestValidateOptions,
) -> Result<FleetBackupManifest, ManifestCommandError> {
    let data = fs::read_to_string(&options.manifest)?;
    let manifest: FleetBackupManifest = serde_json::from_str(&data)?;
    manifest.validate()?;
    Ok(manifest)
}

fn write_validation_summary(
    options: &ManifestValidateOptions,
    manifest: &FleetBackupManifest,
) -> Result<(), ManifestCommandError> {
    let summary = manifest_validation_summary(manifest);

    output::write_pretty_json(options.out.as_ref(), &summary)
}

fn usage() -> String {
    let mut command = manifest_command();
    command.render_help().to_string()
}

fn validate_usage() -> String {
    let mut command = manifest_validate_command();
    command.render_help().to_string()
}

fn manifest_command() -> ClapCommand {
    ClapCommand::new("manifest")
        .bin_name("canic manifest")
        .about("Validate fleet backup manifests")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("validate")
                .about("Validate a fleet backup manifest")
                .disable_help_flag(true),
        ))
}

#[cfg(test)]
mod tests;
