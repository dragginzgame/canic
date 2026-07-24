mod canister;
mod command;
mod diagnostic;
mod error;
mod model;
mod replica;
mod response;
mod run;
mod snapshot;
mod version;

pub(crate) use command::default_command_in;
pub use command::{
    add_target_args, command_display, existing_local_canister_candid_path,
    local_canister_candid_path,
};
pub use diagnostic::{IcpDiagnostic, classify_icp_diagnostic};
pub use error::IcpCommandError;
pub use model::{
    IcpCanisterStatusReport, IcpCanisterStatusSettings, IcpCli, IcpRawOutput, IcpSnapshot,
    LocalReplicaTarget,
};
pub(crate) use response::decode_json_response;
pub use response::{IcpJsonResponseError, decode_json_result_response};
pub(crate) use run::{run_output, run_status, run_success};
pub use run::{run_output_with_stderr, run_raw_output};

#[cfg(test)]
mod tests;
