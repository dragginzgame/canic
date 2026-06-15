use std::{error::Error, fmt};

use super::model::{ICP_CLI_SUPPORTED_VERSION_RANGE, REQUIRED_ICP_CLI_VERSION};

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
    SnapshotIdUnavailable {
        output: String,
    },
}

impl fmt::Display for IcpCommandError {
    // Render ICP CLI command failures with the command line and captured diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "{err}"),
            Self::MissingCli { executable } => {
                write!(
                    formatter,
                    "icp-cli executable not found: {executable}\nrequired: icp-cli {ICP_CLI_SUPPORTED_VERSION_RANGE}\nnext: install icp-cli {REQUIRED_ICP_CLI_VERSION} from https://github.com/dfinity/icp-cli/releases/tag/v{REQUIRED_ICP_CLI_VERSION}, or pass top-level --icp <path>"
                )
            }
            Self::IncompatibleCliVersion { executable, found } => {
                write!(
                    formatter,
                    "unsupported icp-cli version for {executable}\nfound: {found}\nrequired: icp-cli {ICP_CLI_SUPPORTED_VERSION_RANGE}\nnext: install icp-cli {REQUIRED_ICP_CLI_VERSION} from https://github.com/dfinity/icp-cli/releases/tag/v{REQUIRED_ICP_CLI_VERSION}, or pass top-level --icp <path>"
                )
            }
            Self::Failed { command, stderr } => {
                write!(formatter, "icp command failed: {command}\n{stderr}")
            }
            Self::Json {
                command,
                output,
                source,
            } => {
                write!(
                    formatter,
                    "could not parse icp json output for {command}: {source}\n{output}"
                )
            }
            Self::SnapshotIdUnavailable { output } => {
                write!(
                    formatter,
                    "could not parse snapshot id from icp output: {output}"
                )
            }
        }
    }
}

impl Error for IcpCommandError {
    // Preserve the underlying I/O error as the source when command execution fails locally.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json { source, .. } => Some(source),
            Self::Failed { .. }
            | Self::IncompatibleCliVersion { .. }
            | Self::MissingCli { .. }
            | Self::SnapshotIdUnavailable { .. } => None,
        }
    }
}

impl From<std::io::Error> for IcpCommandError {
    // Convert process-spawn failures into the shared ICP CLI command error type.
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
