//! Module: workflow::ic::mgmt
//!
//! Responsibility: expose management-canister status queries to workflow callers.
//! Does not own: management call execution, endpoint authorization, or DTO schemas.
//! Boundary: delegates management calls to ops and maps results into DTOs.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::canister::{CanisterHistoryResponse, CanisterStatusResponse},
    ops::ic::mgmt::MgmtOps,
};

///
/// MgmtWorkflow
///
/// Workflow facade for management-canister operations.
///

pub struct MgmtWorkflow;

impl MgmtWorkflow {
    /// Inspect the latest replicated management history of an authenticated target.
    pub async fn canister_history(
        pid: Principal,
    ) -> Result<CanisterHistoryResponse, InternalError> {
        MgmtOps::canister_history(pid).await
    }

    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusResponse, InternalError> {
        let status = MgmtOps::canister_status(pid).await?;

        Ok(MgmtOps::canister_status_to_dto(status))
    }
}
