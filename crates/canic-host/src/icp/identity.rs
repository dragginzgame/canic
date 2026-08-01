//! Module: icp::identity
//!
//! Responsibility: resolve the Principal of the active ICP CLI identity.
//! Does not own: identity selection, authentication policy, or controller authorization.
//! Boundary: callers compare the returned text with their own typed authority.

use super::{error::IcpCommandError, model::IcpCli, run::run_output};

impl IcpCli {
    /// Return the Principal text for the identity that will execute ICP commands.
    pub fn identity_principal_text(&self) -> Result<String, IcpCommandError> {
        let mut command = self.identity_principal_command();
        run_output(&mut command)
    }

    fn identity_principal_command(&self) -> std::process::Command {
        let mut command = self.command();
        command.args(["identity", "principal"]);
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icp::command::command_display;

    #[test]
    fn identity_resolution_uses_the_active_identity_without_network_selection() {
        let icp = IcpCli::new("icp", Some("ic".to_string())).with_cwd("/workspace/app");

        assert_eq!(
            command_display(&icp.identity_principal_command()),
            "icp --project-root-override /workspace/app identity principal"
        );
    }
}
