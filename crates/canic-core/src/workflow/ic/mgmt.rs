//! Module: workflow::ic::mgmt
//!
//! Responsibility: expose management-canister status queries to workflow callers.
//! Does not own: management call execution, endpoint authorization, or DTO schemas.
//! Boundary: delegates management calls to ops and maps results into DTOs.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::canister::{
        CanisterHistoryResponse, CanisterInspectionOutcome, CanisterInspectionReserveResponse,
    },
    ops::ic::mgmt::MgmtOps,
};

///
/// MgmtWorkflow
///
/// Workflow facade for management-canister operations.
///

pub struct MgmtWorkflow;

impl MgmtWorkflow {
    /// Observe the current inspection reserve without scheduling the inspection.
    pub fn canister_inspection_reserve(
        pid: Principal,
    ) -> Result<CanisterInspectionReserveResponse, InternalError> {
        MgmtOps::canister_inspection_reserve(pid)
    }

    /// Inspect the latest replicated management history of an authenticated target.
    pub async fn canister_history(
        pid: Principal,
    ) -> Result<CanisterHistoryResponse, InternalError> {
        MgmtOps::canister_history(pid).await
    }

    /// Inspect a target while retaining protected numerical reserve failures.
    pub async fn canister_inspection(
        pid: Principal,
    ) -> Result<CanisterInspectionOutcome, InternalError> {
        MgmtOps::canister_inspection(pid).await
    }
}
