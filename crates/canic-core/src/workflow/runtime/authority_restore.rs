//! Module: workflow::runtime::authority_restore
//!
//! Responsibility: coordinate authority snapshot sealing with IC history and timer suspension.
//! Does not own: controller authentication, stable record encoding, or external snapshot effects.
//! Boundary: authority endpoints delegate here before host stop/capture/start operations.

use crate::{
    InternalError,
    domain::policy::pure::{
        PolicyError,
        authority_restore::{
            require_command_variant_allowed as require_policy_command_variant_allowed,
            require_update_allowed as require_policy_update_allowed,
        },
    },
    dto::authority_restore::{AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest},
    ids::{EndpointCall, EndpointCallKind},
    ops::{
        ic::{IcOps, mgmt::MgmtOps},
        runtime::env::EnvOps,
        storage::authority_restore::AuthorityRestoreFenceOps,
    },
    workflow::runtime::timer::{TimerAuthorityWorkflow, TimerError},
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

    /// Seal Root mutation after suspending its exact native timer owners.
    pub async fn prepare_root_snapshot(
        request: AuthoritySnapshotRequest,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_root_authority_runtime()?;
        prepare_snapshot_with(request, TimerAuthorityWorkflow::suspend_root).await
    }

    /// Seal Coordinator mutation only when it has no private lifecycle work in flight.
    pub async fn prepare_coordinator_snapshot(
        request: AuthoritySnapshotRequest,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_coordinator_authority_runtime()?;
        prepare_snapshot_with(request, TimerAuthorityWorkflow::suspend_coordinator).await
    }

    /// Resume the live Root and reconstruct exact core-owned demand before opening mutation.
    pub async fn resume_root_snapshot(
        request: AuthoritySnapshotRequest,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_root_authority_runtime()?;
        resume_snapshot_with(
            request,
            TimerAuthorityWorkflow::resume_root,
            crate::workflow::runtime::RuntimeWorkflow::start_all_root,
            "reconcile root timer owners while authority remains sealed",
        )
        .await
    }

    /// Resume the live Coordinator, which owns no fixed background timer claims.
    pub async fn resume_coordinator_snapshot(
        request: AuthoritySnapshotRequest,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_coordinator_authority_runtime()?;
        resume_snapshot_with(
            request,
            TimerAuthorityWorkflow::resume_coordinator,
            || Ok(()),
            "reconcile Coordinator timer owners while authority remains sealed",
        )
        .await
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

    /// Apply the sealed-authority fence after the common command has been decoded.
    pub fn require_command_variant_allowed(recovery_command: bool) -> Result<(), InternalError> {
        if !is_authority_runtime()? {
            return Ok(());
        }
        let is_sealed = AuthorityRestoreFenceOps::is_sealed_for(IcOps::canister_self())?;
        require_policy_command_variant_allowed(is_sealed, recovery_command)
            .map_err(PolicyError::from)
            .map_err(InternalError::from)
    }
}

fn trap_timer_transition(context: &str, error: TimerError) -> ! {
    ic_cdk::trap(format!(
        "authority snapshot failed closed while attempting to {context}: {error}"
    ))
}

fn trap_authority_transition(context: &str, error: InternalError) -> ! {
    ic_cdk::trap(format!(
        "authority snapshot failed closed while attempting to {context}: {error}"
    ))
}

async fn prepare_snapshot_with(
    request: AuthoritySnapshotRequest,
    suspend: impl FnOnce() -> Result<(), TimerError>,
) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
    let authority = IcOps::canister_self();
    let history_total_num_changes = MgmtOps::canister_history_total_changes(authority).await?;
    AuthorityRestoreFenceOps::validate_prepare(request, authority)?;
    suspend().unwrap_or_else(|error| {
        trap_timer_transition("suspend timers before sealing authority", error)
    });
    let status = AuthorityRestoreFenceOps::prepare(
        request,
        authority,
        history_total_num_changes,
        IcOps::now_nanos(),
    )
    .unwrap_or_else(|error| {
        trap_authority_transition("commit sealed authority after timer suspension", error)
    });
    Ok(status)
}

async fn resume_snapshot_with(
    request: AuthoritySnapshotRequest,
    resume: impl FnOnce(),
    reconcile: impl FnOnce() -> Result<(), InternalError>,
    reconcile_context: &str,
) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
    let authority = IcOps::canister_self();
    let history_total_num_changes = MgmtOps::canister_history_total_changes(authority).await?;
    AuthorityRestoreFenceOps::validate_resume(request, authority, history_total_num_changes)?;
    resume();
    reconcile().unwrap_or_else(|error| trap_authority_transition(reconcile_context, error));
    let status = AuthorityRestoreFenceOps::resume(
        request,
        authority,
        history_total_num_changes,
        IcOps::now_nanos(),
    )
    .unwrap_or_else(|error| {
        trap_authority_transition("open authority after timer reconstruction", error)
    });
    Ok(status)
}

fn require_root_authority_runtime() -> Result<(), InternalError> {
    if EnvOps::is_root() {
        return Ok(());
    }
    Err(InternalError::forbidden())
}

fn require_coordinator_authority_runtime() -> Result<(), InternalError> {
    if EnvOps::is_fleet_coordinator_runtime() {
        return Ok(());
    }
    Err(InternalError::forbidden())
}

fn require_authority_runtime() -> Result<(), InternalError> {
    if is_authority_runtime()? {
        return Ok(());
    }
    Err(InternalError::forbidden())
}

fn is_authority_runtime() -> Result<bool, InternalError> {
    if EnvOps::is_fleet_coordinator_runtime() {
        return Ok(true);
    }
    EnvOps::canister_role().map(|role| role.is_root())
}
