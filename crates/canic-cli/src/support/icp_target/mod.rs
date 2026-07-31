//! Module: canic_cli::support::icp_target
//!
//! Responsibility: own parsed ICP CLI target context shared by CLI commands.
//! Does not own: command-specific arguments, output handling, or Fleet resolution.
//! Boundary: command modules parse target options here, then add their own ICP arguments.

use crate::cli::{
    clap::string_option_or_else,
    defaults::{default_icp, local_environment},
};
use canic_host::icp::{IcpCli, add_target_args};
use std::{path::Path, process::Command};

///
/// IcpTargetOptions
///
/// Parsed ICP executable and environment context shared by CLI commands.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpTargetOptions {
    pub environment: String,
    pub icp: String,
}

impl IcpTargetOptions {
    /// Parse the shared ICP executable and environment options.
    pub fn parse(matches: &clap::ArgMatches) -> Self {
        Self {
            environment: string_option_or_else(matches, "environment", local_environment),
            icp: string_option_or_else(matches, "icp", default_icp),
        }
    }

    /// Build one ICP command context rooted at the current project.
    pub fn icp_cli(&self, root: &Path) -> IcpCli {
        IcpCli::new(&self.icp, Some(self.environment.clone())).with_cwd(root)
    }

    /// Append the selected environment to a command built from this context.
    pub fn append_target_args(&self, command: &mut Command) {
        add_target_args(command, Some(&self.environment), None);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::globals::{
        INTERNAL_ENVIRONMENT_OPTION, INTERNAL_ICP_OPTION, internal_environment_arg,
        internal_icp_arg,
    };

    #[test]
    fn parsed_target_builds_one_exact_project_command_context() {
        let matches = clap::Command::new("test")
            .arg(internal_environment_arg())
            .arg(internal_icp_arg())
            .try_get_matches_from([
                "test",
                INTERNAL_ENVIRONMENT_OPTION,
                "fixture",
                INTERNAL_ICP_OPTION,
                "custom-icp",
            ])
            .expect("parse target context");
        let target = IcpTargetOptions::parse(&matches);
        let root = Path::new("/tmp/canic-icp-target-test");
        let mut command = target.icp_cli(root).command();
        target.append_target_args(&mut command);

        assert_eq!(command.get_program(), "custom-icp");
        assert_eq!(command.get_current_dir(), Some(root));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "--project-root-override",
                "/tmp/canic-icp-target-test",
                "-e",
                "fixture"
            ]
        );
    }
}
