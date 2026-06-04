use super::super::{DeployCommandError, output_format::PromotionOutputFormat};
use crate::cli::clap::{parse_matches, required_path, typed_option};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// DeployPromoteReportOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployPromoteReportOptions {
    pub request: PathBuf,
    pub format: PromotionOutputFormat,
}
impl DeployPromoteReportOptions {
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
            format: typed_option(&matches, "format").unwrap_or(PromotionOutputFormat::Json),
        })
    }
}
