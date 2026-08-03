use std::path::Path;

use super::{
    command::{add_candid_arg, add_output_arg, command_display},
    error::IcpCommandError,
    model::{IcpCanisterStatusReport, IcpCli},
    run::{run_json, run_output, run_output_with_stderr, run_status},
};

#[derive(Clone, Copy)]
enum CanisterCallMode {
    Query,
    Update,
}

impl IcpCli {
    /// Call one canister method with raw binary Candid arguments from a file.
    pub fn canister_call_binary_args_output_with_candid(
        &self,
        canister: &str,
        method: &str,
        args_file: &Path,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_binary_args_command(
            canister,
            method,
            args_file,
            output,
            candid_path,
            CanisterCallMode::Update,
        );
        run_output(&mut command)
    }

    /// Query one canister method with raw binary Candid arguments from a file.
    pub fn canister_query_binary_args_output_with_candid(
        &self,
        canister: &str,
        method: &str,
        args_file: &Path,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_binary_args_command(
            canister,
            method,
            args_file,
            output,
            candid_path,
            CanisterCallMode::Query,
        );
        run_output(&mut command)
    }

    fn canister_binary_args_command(
        &self,
        canister: &str,
        method: &str,
        args_file: &Path,
        output: Option<&str>,
        candid_path: Option<&Path>,
        mode: CanisterCallMode,
    ) -> std::process::Command {
        let mut command = self.canister_command();
        command.args(["call", canister, method, "--args-file"]);
        command.arg(args_file);
        command.args(["--args-format", "bin"]);
        if matches!(mode, CanisterCallMode::Query) {
            command.arg("--query");
        }
        add_candid_arg(&mut command, candid_path);
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        command
    }

    /// Call one canister method with an explicit Candid argument, optional local Candid, and optional JSON output.
    pub fn canister_call_arg_output_with_candid(
        &self,
        canister: &str,
        method: &str,
        arg: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_text_args_command(
            canister,
            method,
            arg,
            output,
            candid_path,
            CanisterCallMode::Update,
        );
        run_output(&mut command)
    }

    /// Query one canister method with no arguments, optional local Candid, and optional JSON output.
    pub fn canister_query_output_with_candid(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_text_args_command(
            canister,
            method,
            "()",
            output,
            candid_path,
            CanisterCallMode::Query,
        );
        run_output(&mut command)
    }

    /// Query one canister method with an explicit Candid argument, optional local Candid, and optional JSON output.
    pub fn canister_query_arg_output_with_candid(
        &self,
        canister: &str,
        method: &str,
        arg: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_text_args_command(
            canister,
            method,
            arg,
            output,
            candid_path,
            CanisterCallMode::Query,
        );
        run_output(&mut command)
    }

    fn canister_text_args_command(
        &self,
        canister: &str,
        method: &str,
        arg: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
        mode: CanisterCallMode,
    ) -> std::process::Command {
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        command.arg(arg);
        if matches!(mode, CanisterCallMode::Query) {
            command.arg("--query");
        }
        add_candid_arg(&mut command, candid_path);
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        command
    }

    /// Read one canister metadata section.
    pub fn canister_metadata_output(
        &self,
        canister: &str,
        metadata_name: &str,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["metadata", canister, metadata_name]);
        self.add_target_args(&mut command);
        run_output(&mut command)
    }

    /// Top up one canister with cycles.
    pub fn canister_top_up_output(
        &self,
        canister: &str,
        amount_cycles: u128,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["top-up", "--amount"]);
        command.arg(amount_cycles.to_string());
        command.arg(canister);
        self.add_target_args(&mut command);
        run_output_with_stderr(&mut command)
    }

    /// Return one canister status report from ICP CLI JSON output.
    pub fn canister_status_report(
        &self,
        canister: &str,
    ) -> Result<IcpCanisterStatusReport, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["status", canister]);
        command.arg("--json");
        self.add_target_args(&mut command);
        run_json(&mut command)
    }

    /// Stop one canister.
    pub fn stop_canister(&self, canister: &str) -> Result<(), IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["stop", canister]);
        self.add_target_args(&mut command);
        run_status(&mut command)
    }

    /// Delete one stopped canister without installing ICP CLI's cycle-recovery shim.
    pub fn delete_canister_without_cycle_recovery(
        &self,
        canister: &str,
    ) -> Result<(), IcpCommandError> {
        let mut command = self.delete_canister_without_cycle_recovery_command(canister);
        run_status(&mut command)
    }

    #[cfg(test)]
    pub(super) fn delete_canister_without_cycle_recovery_display(&self, canister: &str) -> String {
        command_display(&self.delete_canister_without_cycle_recovery_command(canister))
    }

    fn delete_canister_without_cycle_recovery_command(
        &self,
        canister: &str,
    ) -> std::process::Command {
        let mut command = self.canister_command();
        command.args(["delete", "--no-recover-cycles", canister]);
        self.add_target_args(&mut command);
        command
    }

    /// Start one canister.
    pub fn start_canister(&self, canister: &str) -> Result<(), IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["start", canister]);
        self.add_target_args(&mut command);
        run_status(&mut command)
    }

    /// Render a dry-run top-up command.
    #[must_use]
    pub fn canister_top_up_display(&self, canister: &str, amount_cycles: u128) -> String {
        let mut command = self.canister_command();
        command.args(["top-up", "--amount"]);
        command.arg(amount_cycles.to_string());
        command.arg(canister);
        self.add_target_args(&mut command);
        command_display(&command)
    }

    /// Render a dry-run argument query call with optional local Candid.
    #[must_use]
    pub fn canister_query_arg_output_display_with_candid(
        &self,
        canister: &str,
        method: &str,
        arg: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> String {
        let command = self.canister_text_args_command(
            canister,
            method,
            arg,
            output,
            candid_path,
            CanisterCallMode::Query,
        );
        command_display(&command)
    }

    /// Render a dry-run update call with an explicit Candid argument and optional local Candid.
    #[must_use]
    pub fn canister_call_arg_output_display_with_candid(
        &self,
        canister: &str,
        method: &str,
        arg: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> String {
        let command = self.canister_text_args_command(
            canister,
            method,
            arg,
            output,
            candid_path,
            CanisterCallMode::Update,
        );
        command_display(&command)
    }
}

#[cfg(test)]
mod binary_args_tests {
    use super::*;

    #[test]
    fn binary_query_command_keeps_file_format_and_query_mode() {
        let icp = IcpCli::new("icp", Some("local".to_string()));
        let command = icp.canister_binary_args_command(
            "root",
            "canic_status",
            Path::new("/state/args.bin"),
            Some("json"),
            None,
            CanisterCallMode::Query,
        );

        assert_eq!(
            command_display(&command),
            "icp canister call root canic_status --args-file /state/args.bin --args-format bin --query --json -e local"
        );
    }
}
