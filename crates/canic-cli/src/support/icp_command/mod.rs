//! Execute or display an already constructed ICP command at the CLI boundary.
//!
//! Command owners retain argument parsing, recipient resolution and error presentation.

use std::process::Command;

use canic_host::icp::{IcpCommandError, command_display, run_output_with_stderr};

pub fn run_or_print(command: &mut Command, dry_run: bool) -> Result<(), IcpCommandError> {
    if dry_run {
        println!("{}", command_display(command));
        return Ok(());
    }
    let output = run_output_with_stderr(command)?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

pub fn append_optional_arg(command: &mut Command, flag: &str, value: Option<&str>) {
    if let Some(value) = value {
        command.args([flag, value]);
    }
}

pub fn append_flag(command: &mut Command, flag: &str, enabled: bool) {
    if enabled {
        command.arg(flag);
    }
}
