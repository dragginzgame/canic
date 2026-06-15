use std::path::Path;

use super::{
    command::{add_candid_arg, add_output_arg, command_display},
    error::IcpCommandError,
    model::{IcpCanisterStatusReport, IcpCli},
    run::{run_json, run_output, run_output_with_stderr, run_status},
};

impl IcpCli {
    /// Call one canister method with optional JSON output.
    pub fn canister_call_output(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
    ) -> Result<String, IcpCommandError> {
        self.canister_call_output_with_candid(canister, method, output, None)
    }

    /// Call one canister method with optional local Candid and JSON output.
    pub fn canister_call_output_with_candid(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        command.arg("()");
        add_candid_arg(&mut command, candid_path);
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        run_output(&mut command)
    }

    /// Call one canister method with an explicit Candid argument and optional JSON output.
    pub fn canister_call_arg_output(
        &self,
        canister: &str,
        method: &str,
        arg: &str,
        output: Option<&str>,
    ) -> Result<String, IcpCommandError> {
        self.canister_call_arg_output_with_candid(canister, method, arg, output, None)
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
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        command.arg(arg);
        add_candid_arg(&mut command, candid_path);
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        run_output(&mut command)
    }

    /// Query one canister method with no arguments and optional JSON output.
    pub fn canister_query_output(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
    ) -> Result<String, IcpCommandError> {
        self.canister_query_output_with_candid(canister, method, output, None)
    }

    /// Query one canister method with no arguments, optional local Candid, and optional JSON output.
    pub fn canister_query_output_with_candid(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        command.arg("()");
        command.arg("--query");
        add_candid_arg(&mut command, candid_path);
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        run_output(&mut command)
    }

    /// Query one canister method with an explicit Candid argument and optional JSON output.
    pub fn canister_query_arg_output(
        &self,
        canister: &str,
        method: &str,
        arg: &str,
        output: Option<&str>,
    ) -> Result<String, IcpCommandError> {
        self.canister_query_arg_output_with_candid(canister, method, arg, output, None)
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
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        command.arg(arg);
        command.arg("--query");
        add_candid_arg(&mut command, candid_path);
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        run_output(&mut command)
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

    /// Return one canister status report.
    pub fn canister_status(&self, canister: &str) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["status", canister]);
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

    /// Start one canister.
    pub fn start_canister(&self, canister: &str) -> Result<(), IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["start", canister]);
        self.add_target_args(&mut command);
        run_status(&mut command)
    }

    /// Render a dry-run stop command.
    #[must_use]
    pub fn stop_canister_display(&self, canister: &str) -> String {
        let mut command = self.canister_command();
        command.args(["stop", canister]);
        self.add_target_args(&mut command);
        command_display(&command)
    }

    /// Render a dry-run start command.
    #[must_use]
    pub fn start_canister_display(&self, canister: &str) -> String {
        let mut command = self.canister_command();
        command.args(["start", canister]);
        self.add_target_args(&mut command);
        command_display(&command)
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

    /// Render a dry-run no-argument query call.
    #[must_use]
    pub fn canister_query_output_display(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
    ) -> String {
        self.canister_query_output_display_with_candid(canister, method, output, None)
    }

    /// Render a dry-run no-argument query call with optional local Candid.
    #[must_use]
    pub fn canister_query_output_display_with_candid(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
        candid_path: Option<&Path>,
    ) -> String {
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        command.arg("()");
        command.arg("--query");
        add_candid_arg(&mut command, candid_path);
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        command_display(&command)
    }

    /// Render a dry-run update call with an explicit Candid argument.
    #[must_use]
    pub fn canister_call_arg_output_display(
        &self,
        canister: &str,
        method: &str,
        arg: &str,
        output: Option<&str>,
    ) -> String {
        self.canister_call_arg_output_display_with_candid(canister, method, arg, output, None)
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
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        command.arg(arg);
        add_candid_arg(&mut command, candid_path);
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        command_display(&command)
    }
}
