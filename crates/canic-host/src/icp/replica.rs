use std::{path::Path, process::Command};

use super::{
    command::{add_debug_arg, add_local_network_target},
    error::IcpCommandError,
    model::IcpCli,
    run::{run_json, run_output_with_stderr, run_status_inherit, run_success},
};

impl IcpCli {
    /// Start the local ICP replica from one ICP project root.
    pub fn local_replica_start_in(
        &self,
        cwd: &Path,
        background: bool,
        debug: bool,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.local_replica_command_in("start", cwd);
        run_local_replica_start_command(&mut command, background, debug)
    }

    /// Return local ICP replica status from one ICP project root.
    pub fn local_replica_status_in(
        &self,
        cwd: &Path,
        debug: bool,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.local_replica_command_in("status", cwd);
        add_debug_arg(&mut command, debug);
        run_output_with_stderr(&mut command)
    }

    /// Return local ICP replica status JSON from one ICP project root.
    pub fn local_replica_status_json_in(
        &self,
        cwd: &Path,
        debug: bool,
    ) -> Result<serde_json::Value, IcpCommandError> {
        let mut command = self.local_replica_command_in("status", cwd);
        add_debug_arg(&mut command, debug);
        command.arg("--json");
        run_json(&mut command)
    }

    /// Return whether one ICP project root owns a running local ICP replica.
    pub fn local_replica_project_running_in(
        &self,
        cwd: &Path,
        debug: bool,
    ) -> Result<bool, IcpCommandError> {
        let mut command = self.local_replica_command_in("status", cwd);
        add_debug_arg(&mut command, debug);
        run_success(&mut command)
    }

    /// Return whether the local ICP replica responds to ping.
    pub fn local_replica_ping(&self, debug: bool) -> Result<bool, IcpCommandError> {
        let mut command = self.local_replica_command("ping");
        add_debug_arg(&mut command, debug);
        run_success(&mut command)
    }

    /// Stop the local ICP replica from one ICP project root.
    pub fn local_replica_stop_in(
        &self,
        cwd: &Path,
        debug: bool,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.local_replica_command_in("stop", cwd);
        add_debug_arg(&mut command, debug);
        run_output_with_stderr(&mut command)
    }

    fn local_replica_command(&self, action: &str) -> Command {
        let mut command = self.command();
        command.args(["network", action]);
        add_local_network_target(&mut command);
        command
    }

    fn local_replica_command_in(&self, action: &str, cwd: &Path) -> Command {
        let mut command = self.command_in(cwd);
        command.args(["network", action]);
        add_local_network_target(&mut command);
        command
    }
}

fn run_local_replica_start_command(
    command: &mut Command,
    background: bool,
    debug: bool,
) -> Result<String, IcpCommandError> {
    add_debug_arg(command, debug);
    if background {
        command.arg("--background");
        return run_output_with_stderr(command);
    }
    run_status_inherit(command)?;
    Ok(String::new())
}
