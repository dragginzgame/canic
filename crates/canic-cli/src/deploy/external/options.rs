use super::super::{DeployCommandError, DeployTruthOptions, output_format::JsonTextOutputFormat};
use super::command::TEXT_ARG;
use crate::cli::clap::{parse_matches, required_path, required_string};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// DeployExternalOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployExternalOptions {
    pub truth: DeployTruthOptions,
    pub format: JsonTextOutputFormat,
}

///
/// DeployExternalCriticalFixOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployExternalCriticalFixOptions {
    pub truth: DeployTruthOptions,
    pub format: JsonTextOutputFormat,
    pub fix_id: String,
    pub severity: String,
}

///
/// DeployExternalVerifyOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployExternalVerifyOptions {
    pub request: PathBuf,
    pub format: JsonTextOutputFormat,
}

///
/// DeployExternalInspectOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployExternalInspectOptions {
    pub request: PathBuf,
    pub format: JsonTextOutputFormat,
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
            truth: DeployTruthOptions::from_matches(&matches),
            format: JsonTextOutputFormat::from_text_flag(matches.get_flag(TEXT_ARG)),
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
            truth: DeployTruthOptions::from_matches(&matches),
            format: JsonTextOutputFormat::from_text_flag(matches.get_flag(TEXT_ARG)),
            fix_id: required_string(&matches, "fix-id"),
            severity: required_string(&matches, "severity"),
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
            request: required_path(&matches, "request"),
            format: JsonTextOutputFormat::from_text_flag(matches.get_flag(TEXT_ARG)),
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
            request: required_path(&matches, "request"),
            format: JsonTextOutputFormat::from_text_flag(matches.get_flag(TEXT_ARG)),
        })
    }
}
