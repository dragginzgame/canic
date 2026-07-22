use std::{io, path::Path, process::Command};

use super::{
    command::command_display,
    error::IcpCommandError,
    model::{IcpCli, IcpCliVersion},
    run::command_stderr,
};

impl IcpCli {
    /// Resolve and validate the installed ICP CLI version.
    pub fn compatible_version(&self) -> Result<String, IcpCommandError> {
        compatible_version_output(&self.executable, self.cwd.as_deref())
    }
}

/// Parse an ICP CLI semantic version from `icp --version` output.
#[must_use]
pub(super) fn parse_icp_cli_version(output: &str) -> Option<IcpCliVersion> {
    output
        .split_whitespace()
        .find_map(parse_icp_cli_version_token)
}

/// Return whether an ICP CLI version is supported by this Canic release.
#[must_use]
pub(super) const fn is_supported_icp_cli_version(version: IcpCliVersion) -> bool {
    version.major == 1 && version.minor >= 1
}

pub(super) fn compatible_version_output(
    executable: &str,
    cwd: Option<&Path>,
) -> Result<String, IcpCommandError> {
    let output = icp_version_output(executable, cwd)?;
    if let Some(version) = parse_icp_cli_version(&output)
        && is_supported_icp_cli_version(version)
    {
        return Ok(output);
    }
    Err(IcpCommandError::IncompatibleCliVersion {
        executable: executable.to_string(),
        found: output,
    })
}

fn icp_version_output(executable: &str, cwd: Option<&Path>) -> Result<String, IcpCommandError> {
    let mut command = Command::new(executable);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.arg("--version");
    let display = command_display(&command);
    let output = command.output().map_err(|err| {
        if err.kind() == io::ErrorKind::NotFound {
            IcpCommandError::MissingCli {
                executable: executable.to_string(),
            }
        } else {
            IcpCommandError::Io(err)
        }
    })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

fn parse_icp_cli_version_token(token: &str) -> Option<IcpCliVersion> {
    let token = token
        .trim_matches(|c: char| matches!(c, ',' | ';' | ')' | '('))
        .trim_start_matches('v');
    let mut parts = token.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch_token = parts.next()?;
    let patch_digits = patch_token
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if patch_digits.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(IcpCliVersion {
        major,
        minor,
        patch: patch_digits.parse::<u64>().ok()?,
    })
}
