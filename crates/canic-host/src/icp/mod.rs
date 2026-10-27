mod balance;
mod binary_arguments;
mod candid;
mod canister;
mod command;
pub(crate) mod cycles_ledger;
mod diagnostic;
mod error;
mod identity;
mod management;
mod model;
mod query;
mod replica;
mod response;
mod run;
mod snapshot;
mod timing;
mod version;
mod wire;
pub use timing::{IcpRequestKind, IcpRequestTiming};

pub use balance::IcpBalanceError;
pub(crate) use binary_arguments::{cleanup_binary_argument_file, write_binary_argument_file};
pub use candid::IcpCandidCallError;
pub use command::{
    CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV, add_target_args, command_display,
    existing_local_canister_candid_path, local_canister_candid_path,
};
pub use diagnostic::{IcpDiagnostic, classify_icp_diagnostic};
pub use error::IcpCommandError;
pub use identity::IcpIdentityAccountFormat;
pub use management::IcpManagementCallError;
pub(crate) use management::{SNAPSHOT_RESPONSE_BYTES, read_snapshot_ids};
pub use model::{
    IcpCanisterQueryStats, IcpCanisterStatusReport, IcpCanisterStatusSettings,
    IcpCanisterVisibility, IcpCli, IcpRawOutput, IcpSnapshot, LocalReplicaTarget,
};
pub use query::IcpQueryError;
pub use response::IcpJsonResponseError;
pub(crate) use response::response_bytes;
pub(crate) use run::run_status;
pub use run::{run_output_with_stderr, run_raw_output};
pub use wire::IcpWireQueryError;
#[cfg(test)]
pub(crate) use wire::decode_wire_response;

#[cfg(test)]
mod tests;
