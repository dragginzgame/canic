//! Module: workflow::ic::mgmt
//!
//! Responsibility: expose management-canister status queries to workflow callers.
//! Does not own: management call execution, endpoint authorization, or DTO schemas.
//! Boundary: delegates management calls to ops and maps results into DTOs.

use crate::{
    InternalError, dto::canister::CanisterStatusResponse, ops::ic::mgmt::MgmtOps,
    workflow::prelude::*,
};

///
/// MgmtWorkflow
///
/// Workflow facade for management-canister operations.
///

pub struct MgmtWorkflow;

impl MgmtWorkflow {
    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusResponse, InternalError> {
        let status = MgmtOps::canister_status(pid).await?;

        Ok(MgmtOps::canister_status_to_dto(status))
    }
}
