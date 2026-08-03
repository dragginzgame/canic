//! Module: workflow::runtime::authority_restore
//!
//! Responsibility: coordinate authority snapshot sealing with IC history and timer suspension.
//! Does not own: controller authentication, stable record encoding, or external snapshot effects.
//! Boundary: authority endpoints delegate here before host stop/capture/start operations.

use crate::{
    InternalError, InternalErrorOrigin,
    domain::policy::pure::{
        PolicyError, authority_restore::require_update_allowed as require_policy_update_allowed,
    },
    dto::authority_restore::{AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest},
    ids::{EndpointCall, EndpointCallKind},
    ops::{
        ic::{IcOps, mgmt::MgmtOps},
        runtime::env::EnvOps,
        storage::authority_restore::AuthorityRestoreFenceOps,
    },
    workflow::runtime::timer::TimerWorkflow,
};

/// Runtime coordinator for Fleet authority snapshot sealing and live resume.
pub struct AuthorityRestoreWorkflow;

impl AuthorityRestoreWorkflow {
    /// Initialize the durable fence for one freshly installed authority Canister.
    pub fn initialize(
        authority_canister: crate::cdk::types::Principal,
    ) -> Result<(), InternalError> {
        AuthorityRestoreFenceOps::initialize(authority_canister)
    }

    /// Return one authority Canister's exact durable fence state.
    pub fn status() -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_authority_runtime()?;
        AuthorityRestoreFenceOps::status()
    }

    /// Seal mutation and suspend Canic timers before the host stops and captures the Canister.
    pub async fn prepare_snapshot(
        request: AuthoritySnapshotRequest,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_authority_runtime()?;
        let authority = IcOps::canister_self();
        let history_total_num_changes = MgmtOps::canister_history_total_changes(authority).await?;
        require_resumable_timer_state()?;
        let status = AuthorityRestoreFenceOps::prepare(
            request,
            authority,
            history_total_num_changes,
            IcOps::now_nanos(),
        )?;
        TimerWorkflow::suspend_all();
        Ok(status)
    }

    /// Resume only the live Canister whose independent management history still matches the seal.
    pub async fn resume_snapshot(
        request: AuthoritySnapshotRequest,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_authority_runtime()?;
        require_resumable_timer_state()?;
        let authority = IcOps::canister_self();
        let history_total_num_changes = MgmtOps::canister_history_total_changes(authority).await?;
        let status = AuthorityRestoreFenceOps::resume(
            request,
            authority,
            history_total_num_changes,
            IcOps::now_nanos(),
        )?;
        TimerWorkflow::resume_all();
        Ok(status)
    }

    /// Apply the durable mutation fence before access evaluation on authority updates.
    pub fn require_endpoint_allowed(call: EndpointCall) -> Result<(), InternalError> {
        if call.kind != EndpointCallKind::Update || !is_authority_runtime()? {
            return Ok(());
        }
        let is_sealed = AuthorityRestoreFenceOps::is_sealed_for(IcOps::canister_self())?;
        require_policy_update_allowed(is_sealed, call.endpoint.name)
            .map_err(PolicyError::from)
            .map_err(InternalError::from)
    }
}

fn require_resumable_timer_state() -> Result<(), InternalError> {
    TimerWorkflow::require_resumable()
        .map_err(|reason| InternalError::invariant(InternalErrorOrigin::Workflow, reason))
}

fn require_authority_runtime() -> Result<(), InternalError> {
    if is_authority_runtime()? {
        return Ok(());
    }
    Err(InternalError::forbidden(
        "authority snapshot fencing is available only on the Fleet Coordinator and Fleet Subnet Root",
    ))
}

fn is_authority_runtime() -> Result<bool, InternalError> {
    if EnvOps::is_fleet_coordinator_runtime() {
        return Ok(true);
    }
    EnvOps::canister_role().map(|role| role.is_root())
}
