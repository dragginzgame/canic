use crate::{
    Error,
    cdk::{
        mgmt::{
            self, CanisterInstallMode, CanisterSettings, CanisterStatusArgs, CanisterStatusResult,
            CreateCanisterArgs, DeleteCanisterArgs, DepositCyclesArgs, InstallCodeArgs,
            UninstallCodeArgs, UpdateSettingsArgs, WasmModule,
        },
        utils::wasm::get_wasm_hash,
    },
    env::nns::NNS_REGISTRY_CANISTER,
    log,
    log::Topic,
    model::metrics::system::{SystemMetricKind, SystemMetrics},
    ops::ic::call::Call,
    spec::nns::{GetSubnetForCanisterRequest, GetSubnetForCanisterResponse},
    types::Cycles,
    workflow::CanisterInitPayload,
};
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};

/// Installs or reinstalls a Canic non-root canister with the standard init args.
pub async fn install_canic_code(
    mode: CanisterInstallMode,
    canister_pid: Principal,
    wasm: &[u8],
    payload: CanisterInitPayload,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    install_code(mode, canister_pid, wasm, (payload, extra_arg)).await
}
