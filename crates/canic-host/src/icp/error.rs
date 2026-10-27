use ic_host_text::escape::terminal_text;
use std::{error::Error, fmt};

use super::{
    diagnostic::{IcpDiagnostic, classify_icp_diagnostic},
    model::{ICP_CLI_SUPPORTED_VERSION_RANGE, REQUIRED_ICP_CLI_VERSION},
};

///
/// IcpCommandError
///

#[derive(Debug)]
pub enum IcpCommandError {
    Io(std::io::Error),
    MissingCli {
        executable: String,
    },
    IncompatibleCliVersion {
        executable: String,
        found: String,
    },
    Failed {
        command: String,
        stderr: String,
    },
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

impl fmt::Display for IcpCommandError {
    // Render ICP CLI command failures with the command line and captured diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "{}", terminal_text(&err.to_string())),
            Self::MissingCli { executable } => {
                let executable = terminal_text(executable);
                write!(
                    formatter,
                    "icp-cli executable not found: {executable}\nrequired: icp-cli {ICP_CLI_SUPPORTED_VERSION_RANGE}\n{}",
                    icp_cli_install_hint(),
                )
            }
            Self::IncompatibleCliVersion { executable, found } => {
                let executable = terminal_text(executable);
                let found = terminal_text(found);
                write!(
                    formatter,
                    "unsupported icp-cli version for {executable}\nfound: {found}\nrequired: icp-cli {ICP_CLI_SUPPORTED_VERSION_RANGE}\n{}",
                    icp_cli_install_hint(),
                )
            }
            Self::Failed { command, stderr } => {
                let command = terminal_text(command);
                let stderr = terminal_text(stderr);
                write!(formatter, "icp command failed: {command}\n{stderr}")
            }
            Self::Json {
                command,
                output,
                source,
            } => {
                let command = terminal_text(command);
                let output = terminal_text(output);
                write!(
                    formatter,
                    "could not parse icp json output for {command}: {source}\n{output}"
                )
            }
        }
    }
}

fn icp_cli_install_hint() -> String {
    format!(
        "next: install checksum-verified icp-cli {REQUIRED_ICP_CLI_VERSION} or newer and ensure it is first on PATH\n  from a Canic checkout: make install-dev\nnote: `icp network update` updates the local network launcher, not the `icp` CLI binary; pass top-level --icp <path> to use a non-PATH install"
    )
}

impl Error for IcpCommandError {
    // Preserve the underlying I/O error as the source when command execution fails locally.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json { source, .. } => Some(source),
            Self::Failed { .. } | Self::IncompatibleCliVersion { .. } | Self::MissingCli { .. } => {
                None
            }
        }
    }
}

impl From<std::io::Error> for IcpCommandError {
    // Convert process-spawn failures into the shared ICP CLI command error type.
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
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
