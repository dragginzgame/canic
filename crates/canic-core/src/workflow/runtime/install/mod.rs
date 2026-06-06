use crate::{
    InternalError,
    cdk::{
        candid::{CandidType, utils::ArgumentEncoder},
        types::Principal,
    },
    ops::{
        cost_guard::CostGuardPermit,
        ic::mgmt::{CanisterInstallMode, MgmtOps},
        runtime::install_source::{ApprovedModulePayload, ApprovedModuleSource},
    },
};

///
/// ModuleInstallWorkflow
///

pub struct ModuleInstallWorkflow;

impl ModuleInstallWorkflow {
    /// Install or reinstall one canister from an already resolved module source after a deployment permit.
    pub async fn install_with_payload_with_permit<P: CandidType>(
        permit: &CostGuardPermit,
        mode: CanisterInstallMode,
        target_canister: Principal,
        source: &ApprovedModuleSource,
        payload: P,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        match source.payload() {
            ApprovedModulePayload::Chunked {
                source_canister,
                chunk_hashes,
            } => {
                MgmtOps::install_chunked_code_with_permit(
                    permit,
                    mode,
                    target_canister,
                    *source_canister,
                    chunk_hashes.clone(),
                    source.module_hash().to_vec(),
                    (payload, extra_arg),
                )
                .await
            }
            ApprovedModulePayload::Embedded { wasm_module } => {
                MgmtOps::install_code_with_permit(
                    permit,
                    mode,
                    target_canister,
                    wasm_module.as_ref().to_vec(),
                    (payload, extra_arg),
                )
                .await
            }
        }
    }

    /// Install or upgrade one canister from an already resolved module source after a deployment permit.
    pub async fn install_code_with_permit<T: ArgumentEncoder>(
        permit: &CostGuardPermit,
        mode: CanisterInstallMode,
        target_canister: Principal,
        source: &ApprovedModuleSource,
        args: T,
    ) -> Result<(), InternalError> {
        match source.payload() {
            ApprovedModulePayload::Chunked {
                source_canister,
                chunk_hashes,
            } => {
                MgmtOps::install_chunked_code_with_permit(
                    permit,
                    mode,
                    target_canister,
                    *source_canister,
                    chunk_hashes.clone(),
                    source.module_hash().to_vec(),
                    args,
                )
                .await
            }
            ApprovedModulePayload::Embedded { wasm_module } => {
                MgmtOps::install_code_with_permit(
                    permit,
                    mode,
                    target_canister,
                    wasm_module.as_ref().to_vec(),
                    args,
                )
                .await
            }
        }
    }
}
