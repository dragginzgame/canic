mod canister;
mod command;
mod diagnostic;
mod error;
mod model;
mod replica;
mod run;
mod snapshot;
mod version;

pub use command::{
    add_candid_arg, add_debug_arg, add_output_arg, add_target_args, command_display,
    default_command, default_command_in, ensure_command_compatible,
    existing_local_canister_candid_path, local_canister_candid_path,
};
pub use diagnostic::{IcpDiagnostic, classify_icp_diagnostic};
pub use error::IcpCommandError;
pub use model::{
    ICP_CLI_SUPPORTED_VERSION_RANGE, IcpCanisterStatusReport, IcpCanisterStatusSettings, IcpCli,
    IcpCliVersion, IcpRawOutput, IcpSnapshotCreateReceipt, LocalReplicaTarget,
    REQUIRED_ICP_CLI_VERSION,
};
pub use run::{
    run_json, run_output, run_output_with_stderr, run_raw_output, run_status, run_status_inherit,
    run_success,
};
pub use version::{is_supported_icp_cli_version, parse_icp_cli_version};

#[cfg(test)]
mod tests;
