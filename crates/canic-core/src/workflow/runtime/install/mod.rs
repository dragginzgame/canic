use crate::{
    InternalError,
    cdk::{candid::CandidType, types::Principal},
    ops::{
        cost_guard::CostGuardPermit, ic::mgmt::MgmtOps,
        runtime::install_source::ApprovedModuleSource,
    },
};

///
/// ModuleInstallWorkflow
///

pub struct ModuleInstallWorkflow;

impl ModuleInstallWorkflow {
    /// Install one Canister whose Candid init boundary accepts exactly one payload.
    pub async fn install_single_payload_with_permit<P: CandidType>(
        permit: &CostGuardPermit,
        target_canister: Principal,
        source: &ApprovedModuleSource,
        payload: P,
    ) -> Result<(), InternalError> {
        MgmtOps::install_chunked_code_with_permit(
            permit,
            target_canister,
            *source.source_canister(),
            source.chunk_hashes().to_vec(),
            source.module_hash().to_vec(),
            (payload,),
        )
        .await
    }

    /// Install one canister from an already resolved module source after a deployment permit.
    pub async fn install_with_payload_with_permit<P: CandidType>(
        permit: &CostGuardPermit,
        target_canister: Principal,
        source: &ApprovedModuleSource,
        payload: P,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        MgmtOps::install_chunked_code_with_permit(
            permit,
            target_canister,
            *source.source_canister(),
            source.chunk_hashes().to_vec(),
            source.module_hash().to_vec(),
            (payload, extra_arg),
        )
        .await
    }
}
