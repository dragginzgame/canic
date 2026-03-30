use crate::{
    InternalError,
    api::runtime::install::ApprovedModuleSource,
    cdk::{
        candid::{CandidType, utils::ArgumentEncoder},
        types::Principal,
    },
    ops::ic::mgmt::{CanisterInstallMode, MgmtOps},
};

///
/// ModuleInstallWorkflow
///

pub struct ModuleInstallWorkflow;

impl ModuleInstallWorkflow {
    /// Install or reinstall one canister from an already resolved module source.
    pub async fn install_with_payload<P: CandidType>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        source: &ApprovedModuleSource,
        payload: P,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        MgmtOps::install_chunked_canister_with_payload(
            mode,
            target_canister,
            source.source_canister,
            source.chunk_hashes.clone(),
            source.module_hash.clone(),
            payload,
            extra_arg,
        )
        .await
    }

    /// Install or upgrade one canister from an already resolved module source.
    pub async fn install_code<T: ArgumentEncoder>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        source: &ApprovedModuleSource,
        args: T,
    ) -> Result<(), InternalError> {
        MgmtOps::install_chunked_code(
            mode,
            target_canister,
            source.source_canister,
            source.chunk_hashes.clone(),
            source.module_hash.clone(),
            args,
        )
        .await
    }
}
