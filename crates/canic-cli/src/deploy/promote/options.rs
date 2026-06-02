use super::super::{
    DeployCommandError,
    output_format::{PromotionOutputFormat, parse_promotion_output_format},
};
use crate::cli::clap::{parse_matches, path_option, string_option};
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
            request: path_option(&matches, "request").expect("clap requires request"),
            format: parse_promotion_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}
