use super::super::{
    DeployCommandError, DeployTruthOptions,
    output_format::{ExternalOutputFormat, parse_external_output_format},
};
use crate::cli::clap::{parse_matches, path_option, string_option};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// DeployExternalOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployExternalOptions {
    pub truth: DeployTruthOptions,
    pub format: ExternalOutputFormat,
}

///
/// DeployExternalCriticalFixOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployExternalCriticalFixOptions {
    pub truth: DeployTruthOptions,
    pub format: ExternalOutputFormat,
    pub fix_id: String,
    pub severity: String,
}

///
/// DeployExternalVerifyOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployExternalVerifyOptions {
    pub request: PathBuf,
    pub format: ExternalOutputFormat,
}

///
/// DeployExternalInspectOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployExternalInspectOptions {
    pub request: PathBuf,
    pub format: ExternalOutputFormat,
}

impl DeployExternalOptions {
    pub fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches, usage)?,
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

impl DeployExternalCriticalFixOptions {
    pub fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches, usage)?,
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
            fix_id: string_option(&matches, "fix-id").expect("clap requires fix-id"),
            severity: string_option(&matches, "severity").expect("clap requires severity"),
        })
    }
}

impl DeployExternalVerifyOptions {
    pub fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            request: path_option(&matches, "request").expect("clap requires request"),
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

impl DeployExternalInspectOptions {
    pub fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            request: path_option(&matches, "request").expect("clap requires request"),
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}
