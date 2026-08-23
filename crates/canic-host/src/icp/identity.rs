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

    /// Return the selected identity's exact default account in the requested ledger format.
    pub fn identity_account_id_text(
        &self,
        format: IcpIdentityAccountFormat,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.identity_account_id_command(format);
        run_output(&mut command)
    }

    fn identity_principal_command(&self) -> std::process::Command {
        let mut command = self.command();
        command.args(["identity", "principal"]);
        command
    }

    fn identity_account_id_command(
        &self,
        format: IcpIdentityAccountFormat,
    ) -> std::process::Command {
        let mut command = self.command();
        command.args(["identity", "account-id", "--format", format.label()]);
        command
    }
}

///
/// IcpIdentityAccountFormat
///
/// Ledger account representation selected for one ICP CLI identity observation.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IcpIdentityAccountFormat {
    IcpLedger,
    Icrc1,
}

impl IcpIdentityAccountFormat {
    const fn label(self) -> &'static str {
        match self {
            Self::IcpLedger => "ledger",
            Self::Icrc1 => "icrc1",
        }
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

    #[test]
    fn account_resolution_selects_the_exact_ledger_representation() {
        let icp = IcpCli::new("icp", Some("ic".to_string())).with_cwd("/workspace/app");

        assert_eq!(
            command_display(&icp.identity_account_id_command(IcpIdentityAccountFormat::Icrc1)),
            "icp --project-root-override /workspace/app identity account-id --format icrc1"
        );
        assert_eq!(
            command_display(&icp.identity_account_id_command(IcpIdentityAccountFormat::IcpLedger)),
            "icp --project-root-override /workspace/app identity account-id --format ledger"
        );
    }
}
