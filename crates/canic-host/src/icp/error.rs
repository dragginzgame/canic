use ic_host_text::escape::terminal_text;
use thiserror::Error;

use super::{
    diagnostic::{IcpDiagnostic, classify_icp_diagnostic},
    model::{ICP_CLI_SUPPORTED_VERSION_RANGE, REQUIRED_ICP_CLI_VERSION},
};

///
/// IcpCommandError
///

#[derive(Debug, Error)]
pub enum IcpCommandError {
    #[error("{}", terminal_text(&.0.to_string()))]
    Io(#[source] std::io::Error),
    #[error(
        "icp-cli executable not found: {executable}\nrequired: icp-cli {ICP_CLI_SUPPORTED_VERSION_RANGE}\n{}",
        icp_cli_install_hint(),
        executable = terminal_text(.executable)
    )]
    MissingCli { executable: String },
    #[error(
        "unsupported icp-cli version for {executable}\nfound: {found}\nrequired: icp-cli {ICP_CLI_SUPPORTED_VERSION_RANGE}\n{}",
        icp_cli_install_hint(),
        executable = terminal_text(.executable),
        found = terminal_text(.found)
    )]
    IncompatibleCliVersion { executable: String, found: String },
    #[error("icp command failed: {command}\n{stderr}", command = terminal_text(.command), stderr = terminal_text(.stderr))]
    Failed { command: String, stderr: String },
    #[error("could not parse icp json output for {command}: {source}\n{output}", command = terminal_text(.command), output = terminal_text(.output))]
    Json {
        command: String,
        output: String,
        source: serde_json::Error,
    },
}

impl IcpCommandError {
    /// Return the typed meaning of external ICP CLI output when Canic recognizes it.
    #[must_use]
    pub fn diagnostic(&self) -> Option<IcpDiagnostic> {
        self.external_output().and_then(classify_icp_diagnostic)
    }

    /// Return raw external ICP CLI output carried by this error.
    #[must_use]
    pub fn external_output(&self) -> Option<&str> {
        match self {
            Self::Failed { stderr, .. } => Some(stderr),
            Self::Json { output, .. } => Some(output),
            Self::Io(_) | Self::MissingCli { .. } | Self::IncompatibleCliVersion { .. } => None,
        }
    }
}

fn icp_cli_install_hint() -> String {
    format!(
        "next: install checksum-verified icp-cli {REQUIRED_ICP_CLI_VERSION} or newer and ensure it is first on PATH\n  from a Canic checkout: make install-dev\nnote: `icp network update` updates the local network launcher, not the `icp` CLI binary; pass top-level --icp <path> to use a non-PATH install"
    )
}

impl From<std::io::Error> for IcpCommandError {
    // Convert process-spawn failures into the shared ICP CLI command error type.
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
mod error_contract_tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn command_error_messages_keep_version_requirements_and_install_hints() {
        let missing = IcpCommandError::MissingCli {
            executable: "custom-icp".into(),
        };
        assert_eq!(
            missing.to_string(),
            format!(
                "icp-cli executable not found: custom-icp\nrequired: icp-cli {ICP_CLI_SUPPORTED_VERSION_RANGE}\n{}",
                icp_cli_install_hint(),
            )
        );
        let incompatible = IcpCommandError::IncompatibleCliVersion {
            executable: "custom-icp".into(),
            found: "0.1.0".into(),
        };
        assert_eq!(
            incompatible.to_string(),
            format!(
                "unsupported icp-cli version for custom-icp\nfound: 0.1.0\nrequired: icp-cli {ICP_CLI_SUPPORTED_VERSION_RANGE}\n{}",
                icp_cli_install_hint(),
            )
        );
        for error in [missing, incompatible] {
            assert!(error.source().is_none());
            assert_eq!(error.external_output(), None);
            assert_eq!(error.diagnostic(), None);
        }
    }

    #[test]
    fn command_error_forwards_the_original_io_and_json_sources() {
        let io: IcpCommandError = std::io::Error::other("spawn refused").into();
        let IcpCommandError::Io(inner) = &io else {
            panic!("I/O conversion changed variant")
        };
        assert_eq!(io.to_string(), inner.to_string());
        assert!(std::ptr::eq(
            io.source()
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .unwrap(),
            inner
        ));
        assert_eq!(io.external_output(), None);
        assert_eq!(io.diagnostic(), None);

        let source = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let expected = format!("could not parse icp json output for status: {source}\nIC0207");
        let json = IcpCommandError::Json {
            command: "status".into(),
            output: "IC0207".into(),
            source,
        };
        let IcpCommandError::Json { source, .. } = &json else {
            unreachable!()
        };
        assert_eq!(json.to_string(), expected);
        assert!(std::ptr::eq(
            json.source()
                .unwrap()
                .downcast_ref::<serde_json::Error>()
                .unwrap(),
            source
        ));
        assert!(json.source().unwrap().source().is_none());
        assert_eq!(json.external_output(), Some("IC0207"));
        assert_eq!(json.diagnostic(), Some(IcpDiagnostic::CanisterOutOfCycles));
    }

    #[test]
    fn failed_command_retains_diagnostics_without_a_source() {
        let error = IcpCommandError::Failed {
            command: "status".into(),
            stderr: "IC0207".into(),
        };
        assert_eq!(error.to_string(), "icp command failed: status\nIC0207");
        assert!(error.source().is_none());
        assert_eq!(error.external_output(), Some("IC0207"));
        assert_eq!(error.diagnostic(), Some(IcpDiagnostic::CanisterOutOfCycles));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_rendering_escapes_external_fields_and_retains_original_diagnostics() {
        let raw = "failure\x1b[2J\r\tmessage";
        let error = IcpCommandError::Failed {
            command: "icp\x1b[2J status".into(),
            stderr: raw.into(),
        };
        let rendered = error.to_string();
        assert!(
            !rendered
                .chars()
                .any(|character| character.is_control() && character != '\n')
        );
        assert!(rendered.contains("message"));
        assert_eq!(error.external_output(), Some(raw));
    }

    #[test]
    fn version_and_json_errors_escape_untrusted_fields() {
        let errors = [
            IcpCommandError::IncompatibleCliVersion {
                executable: "icp\x1b".into(),
                found: "version\r\x1b".into(),
            },
            IcpCommandError::Json {
                command: "icp\x1b".into(),
                output: "output\r\x1b".into(),
                source: serde_json::from_str::<serde_json::Value>("invalid").unwrap_err(),
            },
            IcpCommandError::Io(std::io::Error::other("failure\r\x1b")),
        ];
        for error in errors {
            assert!(
                !error
                    .to_string()
                    .chars()
                    .any(|character| character.is_control() && character != '\n')
            );
        }
    }
}
