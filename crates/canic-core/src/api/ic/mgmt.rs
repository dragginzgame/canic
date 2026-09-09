//! Module: api::ic::mgmt
//!
//! Responsibility: expose management observations to authenticated endpoint adapters.
//! Does not own: transport, authorization, or effect recovery decisions.
//! Boundary: delegates immediately to the management workflow.

use crate::{
    cdk::types::Principal,
    dto::{
        canister::{CanisterHistoryResponse, CanisterStatusResponse},
        error::Error,
    },
    workflow::ic::mgmt::MgmtWorkflow,
};

///
/// MgmtApi
///

pub struct MgmtApi;

impl MgmtApi {
    /// Inspect the latest replicated management history of an authenticated target.
    pub async fn canister_history(pid: Principal) -> Result<CanisterHistoryResponse, Error> {
        MgmtWorkflow::canister_history(pid)
            .await
            .map_err(Error::from)
    }

    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusResponse, Error> {
        MgmtWorkflow::canister_status(pid)
            .await
            .map_err(Error::from)
    }
}
