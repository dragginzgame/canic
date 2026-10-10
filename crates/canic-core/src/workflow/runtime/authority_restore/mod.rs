//! Module: workflow::runtime::authority_restore
//!
//! Responsibility: coordinate authority snapshot/release sealing and timer suspension.
//! Does not own: controller authentication, stable record encoding, or external snapshot effects.
//! Boundary: authority endpoints delegate here before host stop/capture/start operations.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    domain::policy::pure::{
        PolicyError,
        authority_restore::{
            require_command_variant_allowed as require_policy_command_variant_allowed,
            require_update_allowed as require_policy_update_allowed,
        },
    },
    dto::authority_restore::{
        AuthorityReleaseRequest, AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest,
    },
    ops::{
        ic::{IcOps, mgmt::MgmtOps},
        runtime::env::EnvOps,
        storage::authority_restore::AuthorityRestoreFenceOps,
    },
    protocol::{CANIC_COORDINATOR_COMMAND, CANIC_ROOT_COMMAND},
    view::authority_restore::AuthorityMutationFence,
    workflow::runtime::timer::{TimerAuthorityWorkflow, TimerError},
};
use canic_contracts::ids::{EndpointCall, EndpointCallKind};

/// Runtime coordinator for Fleet authority release sealing and snapshot recovery.
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

    /// Seal a Root in one message after its Control Plane owner proves all paid
    /// obligations settled without mutation. Producer suspension and the commit
    /// share one message; any suspension failure traps to roll both back.
    /// This internal hook exposes no release endpoint.
    ///
    /// # Panics
    /// Traps on suspension or commit failure so the message rolls back atomically.
    pub fn prepare_root_release(
        request: AuthorityReleaseRequest,
        require_settled: impl FnOnce() -> Result<(), InternalError>,
        suspend_role: impl FnOnce() -> Result<(), TimerError>,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_root_authority_runtime()?;
        prepare_release_with(
            request,
            TimerAuthorityWorkflow::require_root_resumable,
            require_settled,
            || {
                suspend_role()?;
                TimerAuthorityWorkflow::suspend_root()
            },
        )
    }

    /// Coordinator counterpart; the caller owns Registry/funding/provisioning
    /// quiescence, while Core owns the durable fence and native timer suspension.
    ///
    /// # Panics
    /// Traps on suspension or commit failure so the message rolls back atomically.
    pub fn prepare_coordinator_release(
        request: AuthorityReleaseRequest,
        require_settled: impl FnOnce() -> Result<(), InternalError>,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_coordinator_authority_runtime()?;
        prepare_release_with(
            request,
            TimerAuthorityWorkflow::require_coordinator_resumable,
            require_settled,
            TimerAuthorityWorkflow::suspend_coordinator,
        )
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
        if call.kind != EndpointCallKind::Update {
            return Ok(());
        }
        let Some(command_endpoint) = authority_command_endpoint()? else {
            return Ok(());
        };
        let fence = AuthorityRestoreFenceOps::mutation_fence_for(IcOps::canister_self())?;
        require_policy_update_allowed(fence, call.endpoint.name, command_endpoint)
            .map_err(PolicyError::from)
            .map_err(InternalError::from)
    }

    /// Apply the sealed-authority fence after the role command has been decoded.
    pub fn require_command_variant_allowed(recovery_command: bool) -> Result<(), InternalError> {
        if !is_authority_runtime()? {
            return Ok(());
        }
        let fence = AuthorityRestoreFenceOps::mutation_fence_for(IcOps::canister_self())?;
        require_policy_command_variant_allowed(fence, recovery_command)
            .map_err(PolicyError::from)
            .map_err(InternalError::from)
    }
}

fn prepare_release_with(
    request: AuthorityReleaseRequest,
    preflight: impl FnOnce() -> Result<(), TimerError>,
    require_settled: impl FnOnce() -> Result<(), InternalError>,
    suspend: impl FnOnce() -> Result<(), TimerError>,
) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
    let authority = IcOps::canister_self();
    AuthorityRestoreFenceOps::validate_release(request, authority)?;
    // A terminal seal is replayed without calling producers or observing IC history.
    if AuthorityRestoreFenceOps::mutation_fence_for(authority)? == AuthorityMutationFence::Release {
        return AuthorityRestoreFenceOps::status();
    }
    quiesce_release(preflight, require_settled, suspend)?;
    Ok(
        AuthorityRestoreFenceOps::seal_release(request, authority, IcOps::now_nanos())
            .unwrap_or_else(|error| {
                trap_authority_transition("commit release after producer suspension", error)
            }),
    )
}

// Both checks are read-only and synchronous. A busy refusal must leave every
// producer intact; only a failure after cancellation starts requires rollback.
fn quiesce_release(
    preflight: impl FnOnce() -> Result<(), TimerError>,
    require_settled: impl FnOnce() -> Result<(), InternalError>,
    suspend: impl FnOnce() -> Result<(), TimerError>,
) -> Result<(), InternalError> {
    preflight().map_err(|error| match error {
        TimerError::ActiveJob(_) | TimerError::RunningClaim(_) | TimerError::CustodyBusy => {
            InternalError::conflict()
        }
        error => error.into(),
    })?;
    require_settled()?;
    suspend().unwrap_or_else(|error| {
        trap_timer_transition("suspend timers before sealing Fleet release", error)
    });
    Ok(())
}

fn trap_timer_transition(context: &str, error: TimerError) -> ! {
    ic_cdk::trap(format!(
        "authority transition failed closed while attempting to {context}: {error}"
    ))
}

fn trap_authority_transition(context: &str, error: InternalError) -> ! {
    ic_cdk::trap(format!(
        "authority transition failed closed while attempting to {context}: {error}"
    ))
}

async fn prepare_snapshot_with(
    request: AuthoritySnapshotRequest,
    suspend: impl FnOnce() -> Result<(), TimerError>,
) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
    let authority = IcOps::canister_self();
    AuthorityRestoreFenceOps::validate_prepare(request, authority)?;
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
    if AuthorityRestoreFenceOps::mutation_fence_for(authority)? == AuthorityMutationFence::Release {
        return Err(InternalError::conflict());
    }
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
    authority_command_endpoint().map(|endpoint| endpoint.is_some())
}

fn authority_command_endpoint() -> Result<Option<&'static str>, InternalError> {
    if EnvOps::is_fleet_coordinator_runtime() {
        return Ok(Some(CANIC_COORDINATOR_COMMAND));
    }
    EnvOps::canister_role().map(|role| role.is_root().then_some(CANIC_ROOT_COMMAND))
}
