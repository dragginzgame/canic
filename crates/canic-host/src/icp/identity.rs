//! Module: icp::identity
//!
//! Responsibility: bind an operation's selected ICP identity and resolve its Principal.
//! Does not own: global identity selection, authentication policy, or controller authorization.
//! Boundary: callers compare the returned text with their own typed authority.

use super::{error::IcpCommandError, model::IcpCli, run::run_output};

impl IcpCli {
    /// Freeze the selected identity for this operation and every clone of its transport.
    /// Callers must still compare its Principal with the reviewed operator before effects.
    pub(crate) fn bind_selected_identity(&self) -> Result<(), IcpCommandError> {
        if self.selected_identity.get().is_none() {
            let identity = self.selected_identity_name()?;
            let _ = self.selected_identity.set(identity);
        }
        Ok(())
    }

    pub(super) fn selected_identity_name(&self) -> Result<String, IcpCommandError> {
        if let Some(identity) = self.selected_identity.get() {
            return Ok(identity.clone());
        }
        let mut command = self.request_command();
        command.args(["identity", "default"]);
        let identity = run_output(&mut command, self)?;
        if identity.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ICP returned an empty selected identity",
            )
            .into());
        }
        Ok(identity)
    }

    /// Return the Principal text for the identity that will execute ICP commands.
    pub fn identity_principal_text(&self) -> Result<String, IcpCommandError> {
        self.identity_lookups
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut command = self.identity_principal_command();
        run_output(&mut command, self)
    }

    /// Return the selected identity's exact default account in the requested ledger format.
    pub fn identity_account_id_text(
        &self,
        format: IcpIdentityAccountFormat,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.identity_account_id_command(format);
        run_output(&mut command, self)
    }

    fn identity_principal_command(&self) -> std::process::Command {
        let mut command = self.request_command();
        command.args(["identity", "principal"]);
        self.add_selected_identity_arg(&mut command);
        command
    }

    fn identity_account_id_command(
        &self,
        format: IcpIdentityAccountFormat,
    ) -> std::process::Command {
        let mut command = self.request_command();
        command.args(["identity", "account-id", "--format", format.label()]);
        self.add_selected_identity_arg(&mut command);
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

    #[cfg(unix)]
    #[test]
    fn bound_identity_survives_default_change_for_clones_and_later_calls() {
        use std::{fs, os::unix::fs::PermissionsExt};

        let root = crate::test_support::temp_dir("bound-icp-identity");
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("icp");
        fs::write(
            &executable,
            crate::test_support::tool_script(
                r#"#!/bin/sh
if [ "$1" = --version ]; then echo 'icp @ICP_VERSION@'; exit 0; fi
while [ "$1" = --project-root-override ] || [ "$1" = --identity-password-file ]; do shift 2; done
if [ "$1 $2" = 'identity default' ]; then cat selected; exit 0; fi
selected=$(cat selected)
for arg do
  if [ "$previous" = --identity ]; then selected=$arg; fi
  previous=$arg
done
printf '%s\n' "$selected"
"#,
            ),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(root.join("selected"), "reviewed").unwrap();
        let icp = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(&root);
        let clone = icp.clone();
        icp.bind_selected_identity().unwrap();
        assert_eq!(icp.identity_principal_text().unwrap(), "reviewed");
        fs::write(root.join("selected"), "unrelated-encrypted").unwrap();
        clone.bind_selected_identity().unwrap();
        assert_eq!(clone.identity_principal_text().unwrap(), "reviewed");
        assert_eq!(
            clone
                .identity_account_id_text(IcpIdentityAccountFormat::Icrc1)
                .unwrap(),
            "reviewed"
        );
        assert_eq!(clone.selected_identity_name().unwrap(), "reviewed");
        assert_eq!(
            clone
                .canister_metadata_output("canister", "candid:service")
                .unwrap(),
            "reviewed"
        );
        let unbound = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(&root);
        assert_eq!(
            unbound.identity_principal_text().unwrap(),
            "unrelated-encrypted"
        );
        assert_ne!(clone, unbound);
        fs::remove_dir_all(root).unwrap();
    }

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
