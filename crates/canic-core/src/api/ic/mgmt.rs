//! Module: api::ic::mgmt
//!
//! Responsibility: expose management observations to authenticated endpoint adapters.
//! Does not own: transport, authorization, or effect recovery decisions.
//! Boundary: delegates immediately to the management workflow.

use crate::{
    cdk::types::Principal,
    dto::{
        canister::{
            CanisterHistoryResponse, CanisterInspectionOutcome, CanisterInspectionReserveResponse,
        },
        error::Error,
    },
    workflow::ic::mgmt::MgmtWorkflow,
};

///
/// MgmtApi
///

pub struct MgmtApi;

impl MgmtApi {
    /// Observe a controller-selected target's inspection reserve without an IC call.
    pub fn canister_inspection_reserve(
        pid: Principal,
    ) -> Result<CanisterInspectionReserveResponse, Error> {
        MgmtWorkflow::canister_inspection_reserve(pid).map_err(Error::from)
    }

    /// Inspect the latest replicated management history of an authenticated target.
    pub async fn canister_history(pid: Principal) -> Result<CanisterHistoryResponse, Error> {
        MgmtWorkflow::canister_history(pid)
            .await
            .map_err(Error::from)
    }

    /// Inspect a target while retaining protected numerical reserve failures.
    pub async fn canister_inspection(pid: Principal) -> Result<CanisterInspectionOutcome, Error> {
        MgmtWorkflow::canister_inspection(pid)
            .await
            .map_err(Error::from)
    }
}
